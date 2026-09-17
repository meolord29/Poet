//! Phase-2 integration: chained in-process commands build a full report
//! (heading + paragraphs + 4×3 table via `set-range --header` + toc +
//! image), save, reopen from disk, and verify the read→pack→read round
//! trip (`docs/plans/phase-2-content.md` §3).

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
    (json.expect("dispatch always emits a document"), code)
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let (json, code) = run(dir, args);
    assert_eq!(code, ExitCode::SUCCESS, "command failed: {args:?} → {json}");
    json
}

fn png(dir: &Path) -> String {
    let path = dir.join("chart.png");
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(8, 4)
        .write_to(&mut png, image::ImageFormat::Png)
        .expect("png");
    std::fs::write(&path, png.into_inner()).expect("write");
    path.to_string_lossy().into_owned()
}

#[test]
fn full_report_build_survives_save_reopen_round_trip() {
    let (ctx, dir) = setup();
    let path = dir.join("report.docx");
    let path_str = path.to_string_lossy().into_owned();

    // Build the report through chained commands (fresh "process" each).
    ok(&dir, &["document", "new", &path_str]);
    ok(
        &dir,
        &["heading", "add", "Quarterly Report", "--level", "1"],
    );
    ok(&dir, &["heading", "add", "Summary", "--level", "2"]);
    ok(&dir, &["paragraph", "add", "Revenue grew in the quarter."]);
    ok(
        &dir,
        &["paragraph", "add", "Details follow.", "--id", "details"],
    );
    ok(&dir, &["list", "add", "first point"]);
    ok(&dir, &["list", "add", "second point", "--ordered"]);

    let table_json =
        r#"[["Region","Q1","Q2"],["North","10","20"],["South","5","8"],["East","7","3"]]"#;
    ok(&dir, &["table", "add", "4", "3"]);
    ok(
        &dir,
        &["table", "set-range", table_json, "--index", "0", "--header"],
    );
    ok(&dir, &["toc", "add", "--levels", "1-2"]);
    ok(&dir, &["image", "add", &png(&dir)]);
    let _ = ctx;

    // Verify everything from disk (no in-memory state carries over).
    let json = ok(&dir, &["document", "info"]);
    assert!(json.contains("\"table_count\": 1"));
    assert!(json.contains("\"image_count\": 1"));
    // h1,h2,p1,details,l1,l2,t1,toc1,img1 — every created element wrapped.
    assert!(
        json.contains("\"bookmark_count\": 9"),
        "ids resolvable: {json}"
    );

    // Headings still list with their styles.
    let json = ok(&dir, &["heading", "list"]);
    assert!(json.contains("\"Heading 1\""));
    assert!(json.contains("\"Heading 2\""));

    // Paragraph ids resolvable across invocations.
    let json = ok(&dir, &["paragraph", "get", "--id", "details"]);
    assert!(json.contains("Details follow."));
    let json = ok(&dir, &["paragraph", "get", "--id", "h1"]);
    assert!(json.contains("Quarterly Report"));

    // Table contents survive.
    let json = ok(&dir, &["table", "get", "--index", "0"]);
    assert!(json.contains("Region"));
    assert!(json.contains("North"));
    let json = ok(&dir, &["table", "get", "--id", "t1"]);
    assert!(json.contains("South"));

    // List numbering still applies to the reopened items.
    let json = ok(&dir, &["list", "set-level", "2", "--index", "4"]);
    assert!(json.contains("List level set to 2"));

    // TOC field survives (and a resave does not corrupt the file). The
    // cached hint text is part of the paragraph text, like python-docx.
    let json = ok(&dir, &["paragraph", "get", "--id", "toc1"]);
    assert!(json.contains("Update this field"), "TOC field text: {json}");
    ok(&dir, &["document", "save"]);

    // Images still enumerate with geometry.
    let json = ok(&dir, &["image", "list"]);
    assert!(json.contains("\"count\": 1"));
    assert!(json.contains("\"width_inches\""));

    // Export the reopened document (md walker over restored styles).
    let out = dir.join("report.md");
    ok(
        &dir,
        &[
            "document",
            "export",
            &path_str,
            "md",
            "--out",
            &out.to_string_lossy(),
        ],
    );
    let md = std::fs::read_to_string(&out).expect("md");
    assert!(md.contains("# Quarterly Report"));
    assert!(md.contains("## Summary"));
    assert!(md.contains("| Region | Q1 | Q2 |"));
    assert!(md.contains("| North | 10 | 20 |"));

    // Text export contains table rows as TSV.
    let out = dir.join("report.txt");
    ok(
        &dir,
        &[
            "document",
            "export",
            &path_str,
            "txt",
            "--out",
            &out.to_string_lossy(),
        ],
    );
    let txt = std::fs::read_to_string(&out).expect("txt");
    assert!(txt.contains("Region\tQ1\tQ2"));
}

#[test]
fn autosave_persists_chained_mutations_to_session_path() {
    let (ctx, dir) = setup();
    let path = dir.join("chain.docx");
    let path_str = path.to_string_lossy().into_owned();
    let _ = ctx;

    ok(&dir, &["document", "new", &path_str]);
    // Mutating commands autosave (no explicit `document save` here).
    ok(&dir, &["paragraph", "add", "persisted"]);
    let json = ok(&dir, &["paragraph", "count"]);
    assert!(
        json.contains("\"count\": 1"),
        "autosave made it to disk: {json}"
    );
}

#[test]
fn error_envelopes_report_words_error_codes() {
    let (ctx, dir) = setup();
    let path = dir.join("err.docx");
    let _ = ctx;
    ok(&dir, &["document", "new", &path.to_string_lossy()]);

    // Unknown id → not_found.
    let (json, code) = run(&dir, &["paragraph", "get", "--id", "ghost"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert!(json.contains("\"code\": \"not_found\""));
    assert!(json.contains("No paragraph with id 'ghost'"));

    // Missing addressing → validation_error.
    let (json, code) = run(&dir, &["paragraph", "get"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert!(json.contains("\"code\": \"validation_error\""));

    // Out-of-range index → not_found with Words' message.
    let (json, _) = run(&dir, &["paragraph", "get", "--index", "7"]);
    assert!(json.contains("Paragraph index 7 out of range (0..-1)"));

    // A table id that wraps a paragraph is rejected.
    ok(&dir, &["paragraph", "add", "solo", "--id", "solo"]);
    let (json, _) = run(&dir, &["table", "get", "--id", "solo"]);
    assert!(json.contains("does not wrap a table"));

    // Cell addressing without row/col → validation.
    let (json, _) = run(&dir, &["paragraph", "get", "--table", "0"]);
    assert!(json.contains("row and col are required"));
}
