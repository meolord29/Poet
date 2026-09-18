//! Phase-4 integration: metadata round-trips through save→reopen, batch
//! workflows end-to-end (including the Words' integration case: batch-built
//! table analyzed by `calc stats` with mean 20.0), session persistence and
//! error-after-close, and the meta part surviving the package round trip
//! (`docs/plans/phase-4-misc.md`, adr/0012, adr/0013).

use std::path::Path;
use std::process::ExitCode;

use poet::commands::testutil::setup;
use poet::core::Ctx;

/// One CLI invocation against a fresh context rooted at `dir` — the same
/// shape as a shell `&&` chain, where every process auto-opens the session
/// document from disk.
fn run(dir: &Path, args: &[&str]) -> (String, ExitCode) {
    let ctx = Ctx::at_dir(dir);
    let argv: Vec<String> = std::iter::once("poet".to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    let (json, code) = poet::app::run_with(ctx, argv);
    (
        json.unwrap_or_else(|| panic!("clap rejected {args:?}: no document emitted")),
        code,
    )
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let (json, code) = run(dir, args);
    assert_eq!(code, ExitCode::SUCCESS, "command failed: {args:?} → {json}");
    json
}

/// Package-level probe: the metadata part (the docx-rs reader cannot see
/// custom-XML parts — adr/0012).
fn part_xml(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).expect("open package");
    let mut archive = zip::ZipArchive::new(file).expect("zip");
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut archive.by_name(name).expect("part"), &mut xml)
        .expect("utf8");
    xml
}

fn json_field(json: &str, pointer: &str) -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(json)
        .unwrap_or_else(|e| panic!("envelope is not JSON: {e}\n{json}"))
        .pointer(pointer)
        .unwrap_or_else(|| panic!("missing {pointer} in {json}"))
        .clone()
}

/// Write a batch script, relocating every relative `data.docx` reference to
/// the test dir (the templates ship with Words' relative paths; the process
/// cwd stays the repo root, so runs must use absolute paths).
fn hermetic_script(source: &str, dir: &Path, docx_name: &str) -> String {
    let absolute = dir.join(docx_name).to_string_lossy().into_owned();
    source.replace(&format!("\"{docx_name}\""), &format!("\"{absolute}\""))
}

#[test]
fn meta_round_trips_through_save_reopen() {
    let (_ctx, dir) = setup();
    let path = dir.join("meta.docx");
    let path_str = path.to_string_lossy().into_owned();

    ok(&dir, &["document", "new", &path_str]);
    ok(&dir, &["paragraph", "add", "Annotated.", "--id", "p1"]);
    ok(
        &dir,
        &[
            "meta",
            "set-document",
            r#"{"title": "T", "author": "A", "tags": ["x"]}"#,
        ],
    );
    ok(
        &dir,
        &["meta", "set-section", "Body", r#"{"purpose": "main"}"#],
    );
    ok(
        &dir,
        &[
            "meta",
            "set-table",
            "p1",
            r#"{"name": "Odd", "description": "d"}"#,
        ],
    );
    ok(&dir, &["document", "save"]);

    // Reopen (fresh process semantics) and verify everything survived.
    let described = ok(&dir, &["meta", "describe"]);
    assert_eq!(json_field(&described, "/data/document/title"), "T");
    assert_eq!(json_field(&described, "/data/sections/0/purpose"), "main");
    assert_eq!(
        json_field(&described, "/data/history_count"),
        1,
        "only the paragraph add annotates history"
    );

    let got = ok(&dir, &["meta", "get-table", "p1"]);
    assert_eq!(json_field(&got, "/data/schema/name"), "Odd");

    // The part bytes are in the package with Words' wrapper shape.
    let xml = part_xml(&path, "customXml/words_meta.xml");
    assert!(xml.starts_with(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<meta xmlns=\"https://words.local/meta\">"
    ));
    assert!(xml.contains("\"version\":\"words_meta_v1\""));
    assert!(xml.ends_with("</meta>"));
}

#[test]
fn annotator_records_history_and_infers_schema_end_to_end() {
    let (_ctx, dir) = setup();
    let path = dir.join("annotated.docx");
    let path_str = path.to_string_lossy().into_owned();

    ok(&dir, &["document", "new", &path_str]);
    ok(&dir, &["heading", "add", "H1", "--level", "1"]);
    ok(&dir, &["list", "add", "Item one", "--ordered"]);
    ok(&dir, &["run", "add", "tail", "--index", "0"]);
    ok(&dir, &["table", "add", "3", "2", "--id", "sales"]);
    ok(
        &dir,
        &[
            "table",
            "set-range",
            "--id",
            "sales",
            "--header",
            r#"[["Product","Q1"],["Widget A","$1500"],["Widget B","$1200"]]"#,
        ],
    );

    let described = ok(&dir, &["meta", "describe"]);
    let history = json_field(&described, "/data/history_count");
    assert!(
        history.as_u64().expect("count") >= 5,
        "history: {described}"
    );

    let schema = ok(&dir, &["meta", "get-table", "sales"]);
    assert_eq!(
        json_field(&schema, "/data/schema/columns/0/data_type"),
        "string"
    );
    assert_eq!(
        json_field(&schema, "/data/schema/columns/1/data_type"),
        "currency"
    );
    assert_eq!(json_field(&schema, "/data/schema/header_row"), true);

    let hist = ok(&dir, &["meta", "history", "--limit", "50"]);
    let commands: Vec<String> = json_field(&hist, "/data/history")
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| e["command"].as_str().expect("command").to_string())
        .collect();
    for expected in [
        "heading add",
        "list add",
        "run add",
        "table add",
        "table data",
    ] {
        assert!(
            commands.contains(&expected.to_string()),
            "{expected} missing in {commands:?}"
        );
    }
}

#[test]
fn batch_two_phase_workflow_rebuilds_and_formats() {
    let (_ctx, dir) = setup();
    let docx = dir.join("report.docx");
    let phase1 = dir.join("report.phase1.json");
    let phase2 = dir.join("report.phase2.json");
    let path = docx.to_string_lossy().into_owned();
    std::fs::write(
        &phase1,
        format!(
            r#"[
  {{"cmd": "document", "action": "new", "path": "{path}"}},
  {{"cmd": "heading", "action": "add", "text": "Annual Report", "level": 1}},
  {{"cmd": "paragraph", "action": "add", "text": "Revenue grew 15% YoY.", "id": "summary"}},
  {{"cmd": "document", "action": "save", "path": "{path}"}}
]"#
        ),
    )
    .expect("write phase1");
    std::fs::write(
        &phase2,
        format!(
            r#"[
  {{"cmd": "document", "action": "open", "path": "{path}"}},
  {{"cmd": "run", "action": "add", "id": "summary", "text": " Strong quarter.", "bold": true}},
  {{"cmd": "page", "action": "footer", "text": "Confidential"}},
  {{"cmd": "page", "action": "page-numbers", "align": "center"}},
  {{"cmd": "document", "action": "save", "path": "{path}"}}
]"#
        ),
    )
    .expect("write phase2");

    let phase1_commands = 4;
    let phase2_commands = 5;
    for (phase, expected) in [(&phase1, phase1_commands), (&phase2, phase2_commands)] {
        let json = ok(&dir, &["batch", "run", &phase.to_string_lossy()]);
        assert_eq!(json_field(&json, "/data/message"), "Batch script executed");
        assert_eq!(json_field(&json, "/data/commands_executed"), expected);
    }

    // Reopen and verify content + formatting survived both passes.
    ok(&dir, &["document", "open", &path]);
    let got = ok(&dir, &["paragraph", "get", "--id", "summary"]);
    assert_eq!(
        json_field(&got, "/data/text"),
        "Revenue grew 15% YoY. Strong quarter."
    );
    let runs = ok(&dir, &["run", "get", "--id", "summary"]);
    assert_eq!(json_field(&runs, "/data/runs/1/bold"), true);
}

#[test]
fn batch_data_table_template_run_persists_and_is_readable() {
    let (_ctx, dir) = setup();
    // Generate Words' data_table template, run it, then read the result.
    let template = dir.join("data.json");
    ok(
        &dir,
        &[
            "batch",
            "template",
            "data_table",
            &template.to_string_lossy(),
        ],
    );
    let raw = std::fs::read_to_string(&template).expect("template");
    let script = hermetic_script(&raw, &dir, "data.docx");
    let script_path = dir.join("data.hermetic.json");
    std::fs::write(&script_path, script).expect("rewrite script");

    let json = ok(&dir, &["batch", "run", &script_path.to_string_lossy()]);
    assert_eq!(json_field(&json, "/data/commands_executed"), 5);
    assert!(
        dir.join("data.docx").exists(),
        "template run persists data.docx"
    );

    // calc reads fresh from disk (no session needed).
    let data_path = dir.join("data.docx").to_string_lossy().into_owned();
    let read = ok(&dir, &["calc", "read", &data_path, "--id", "sales"]);
    assert_eq!(json_field(&read, "/data/rows"), 3);
    assert_eq!(json_field(&read, "/data/columns/0"), "Product");

    // Range windows keep the header and use 1-based inclusive row numbers
    // counted over the full list (header = row 0): rows 2..3 → B, Total.
    let window = ok(
        &dir,
        &["calc", "read", &data_path, "--index", "0", "--range", "2:3"],
    );
    assert_eq!(json_field(&window, "/data/rows"), 2);
    assert_eq!(json_field(&window, "/data/data/0/Product"), "Widget B");
}

/// Words' integration case (`test_full_document_with_table_and_calc`): a
/// batch-built K/V table analyzed by `calc stats` with mean exactly 20.0.
#[test]
fn batch_build_table_then_calc_stats_mean_20() {
    let (_ctx, dir) = setup();
    let docx = dir.join("data.docx");
    let path = docx.to_string_lossy().into_owned();
    let script = dir.join("build.json");
    std::fs::write(
        &script,
        format!(
            r#"[
  {{"cmd": "document", "action": "new", "path": "{path}"}},
  {{"cmd": "table", "action": "add", "rows": 4, "cols": 2, "id": "data"}},
  {{"cmd": "table", "action": "set-range", "id": "data", "header": true,
    "values": [["K", "V"], ["a", 10], ["b", 20], ["c", 30]]}},
  {{"cmd": "document", "action": "save", "path": "{path}"}}
]"#
        ),
    )
    .expect("write script");
    let json = ok(&dir, &["batch", "run", &script.to_string_lossy()]);
    assert_eq!(json_field(&json, "/data/commands_executed"), 4);

    let stats = ok(&dir, &["calc", "stats", &path, "--id", "data"]);
    assert_eq!(json_field(&stats, "/data/statistics/V/count"), 3);
    assert_eq!(json_field(&stats, "/data/statistics/V/mean"), 20.0);
    assert_eq!(json_field(&stats, "/data/statistics/V/min"), 10.0);
    assert_eq!(json_field(&stats, "/data/statistics/V/max"), 30.0);
}

#[test]
fn batch_stop_on_first_error_reports_the_failure() {
    let (_ctx, dir) = setup();
    let script = dir.join("bad.json");
    let docx = dir.join("never.docx");
    let path = docx.to_string_lossy().into_owned();
    std::fs::write(
        &script,
        format!(
            r#"[
  {{"cmd": "document", "action": "new", "path": "{path}"}},
  {{"cmd": "nope", "action": "x"}},
  {{"cmd": "paragraph", "action": "count"}}
]"#
        ),
    )
    .expect("write script");
    let json = ok(&dir, &["batch", "run", &script.to_string_lossy()]);
    assert_eq!(
        json_field(&json, "/status"),
        "ok",
        "outer envelope stays ok"
    );
    assert_eq!(json_field(&json, "/data/commands_executed"), 2);
    assert!(
        json_field(&json, "/data/results/1/message")
            .as_str()
            .expect("message")
            .contains("Unknown command type: nope")
    );
    assert!(!docx.exists(), "the script had no explicit save");
}

#[test]
fn session_persistence_and_error_after_close() {
    let (_ctx, dir) = setup();
    let path = dir.join("session.docx");
    let path_str = path.to_string_lossy().into_owned();

    // new starts the session; later invocations auto-open.
    ok(&dir, &["document", "new", &path_str]);
    ok(&dir, &["paragraph", "add", "Persisted."]);
    let count = ok(&dir, &["paragraph", "count"]);
    assert_eq!(json_field(&count, "/data/count"), 1);

    // close ends the session; the next command fails with the state error.
    ok(&dir, &["document", "close"]);
    let (json, code) = run(&dir, &["paragraph", "count"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(json_field(&json, "/code"), "document_state");
}
