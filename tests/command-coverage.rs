//! Whole-surface integration coverage (adr/0015 Gate C): every command must
//! be exercised through in-process dispatch at least once. This file chains
//! the commands the phase suites don't reach into one realistic report build,
//! then verifies the round-trip.

use std::process::ExitCode;

use poet::app::run_with;
use poet::core::Ctx;

fn run(dir: &std::path::Path, args: &[&str]) -> (serde_json::Value, ExitCode) {
    let mut argv = vec!["poet".to_string()];
    argv.extend(args.iter().map(|s| s.to_string()));
    let ctx = Ctx::at_dir(dir);
    let (json, code) = run_with(ctx, argv);
    let parsed: serde_json::Value = json
        .as_deref()
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or(serde_json::Value::Null);
    (parsed, code)
}

fn ok(dir: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let (json, code) = run(dir, args);
    assert_eq!(code, ExitCode::SUCCESS, "{args:?}: {}", json);
    json
}

#[test]
fn every_command_is_reachable_through_dispatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let d = dir.path();
    let doc = d.join("report.docx").to_string_lossy().into_owned();

    // Seed: doc + paragraph + heading + table grid.
    ok(d, &["document", "new", &doc]);
    ok(
        d,
        &["heading", "add", "Report", "--level", "1", "--id", "top"],
    );
    ok(
        d,
        &["paragraph", "add", "Intro paragraph.", "--id", "intro"],
    );

    // paragraph: insert / update / find / replace / move / clear / delete.
    ok(d, &["paragraph", "insert", "0", "Executive Summary"]);
    ok(
        d,
        &["paragraph", "update", "Intro rewritten.", "--id", "intro"],
    );
    ok(d, &["paragraph", "find", "Intro"]);
    ok(d, &["paragraph", "replace", "Intro", "Opening"]);
    ok(d, &["paragraph", "move", "up", "--id", "intro"]);
    ok(d, &["paragraph", "add", "Doomed.", "--id", "doomed"]);
    ok(d, &["paragraph", "clear", "--id", "doomed"]);
    ok(d, &["paragraph", "delete", "--id", "doomed"]);

    // heading: set-level.
    ok(d, &["heading", "set-level", "2", "--id", "top"]);

    // list: add-item / convert / set-level.
    ok(d, &["list", "add", "First item", "--ordered"]);
    ok(d, &["list", "add-item", "Second item"]);
    ok(d, &["list", "set-level", "2", "--id", "l2"]);
    ok(d, &["list", "convert", "--ordered", "--id", "intro"]);

    // table: set-range / set-cell / add-row / add-column / get / delete-row /
    // delete-column — the shape calc consumes.
    ok(d, &["table", "add", "2", "2", "--id", "sales"]);
    ok(
        d,
        &[
            "table",
            "set-range",
            "--id",
            "sales",
            "--header",
            r#"[["Region","Q1"],["EMEA",10]]"#,
        ],
    );
    ok(d, &["table", "set-cell", "--id", "sales", "1", "1", "12"]);
    ok(
        d,
        &[
            "table",
            "add-row",
            "--id",
            "sales",
            "--values",
            r#"["APAC",8]"#,
        ],
    );
    ok(d, &["table", "add-column", "--id", "sales"]);
    ok(d, &["table", "get", "--id", "sales"]);
    ok(d, &["table", "delete-column", "--id", "sales", "2"]);
    ok(d, &["table", "delete-row", "--id", "sales", "2"]);

    // run: clear (add/get/format/emphasize covered by phase suites).
    ok(d, &["run", "add", "tail text", "--id", "intro"]);
    ok(d, &["run", "clear", "--id", "intro"]);

    // image: add / list / get / resize / delete (real 1x1 PNG).
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xfc,
        0xcf, 0xc0, 0x50, 0x0f, 0x00, 0x04, 0x85, 0x01, 0x80, 0x84, 0xa9, 0x8c, 0x21, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let logo = d.join("logo.png");
    std::fs::write(&logo, PNG).expect("write png");
    ok(
        d,
        &["image", "add", &logo.to_string_lossy(), "--width", "2.0"],
    );
    ok(d, &["image", "list"]);
    ok(d, &["image", "get", "0"]);
    ok(d, &["image", "resize", "0", "--width", "3.0"]);
    ok(d, &["image", "delete", "0"]);

    // section: page-break.
    ok(d, &["section", "page-break"]);

    // page: size / page-numbers (margins/orientation/header/footer/columns
    // covered by the phase-3 suite).
    ok(d, &["page", "size", "--width", "8.5", "--height", "11"]);
    ok(d, &["page", "page-numbers", "--align", "center"]);

    // toc: update (the documented no-op hint).
    ok(d, &["toc", "update"]);

    // Persist, then drive meta + calc against the saved file.
    ok(d, &["document", "save"]);

    ok(d, &["meta", "set-document", r#"{"title": "Report"}"#]);
    ok(d, &["meta", "get-document"]);
    ok(
        d,
        &["meta", "set-section", "Body", r#"{"purpose": "content"}"#],
    );
    ok(d, &["meta", "get-section", "Body"]);
    ok(d, &["meta", "set-table", "sales", r#"{"name": "Sales"}"#]);
    ok(d, &["meta", "get-table", "sales"]);

    ok(
        d,
        &[
            "calc",
            "aggregate",
            &doc,
            "--id",
            "sales",
            "--group-by",
            "Region",
            "--agg-column",
            "Q1",
            "--agg-func",
            "sum",
        ],
    );
    ok(
        d,
        &[
            "calc",
            "filter",
            &doc,
            "--id",
            "sales",
            "--column",
            "Q1",
            "--operator",
            ">",
            "--value",
            "5",
        ],
    );
    ok(
        d,
        &[
            "calc",
            "transform",
            &doc,
            "--id",
            "sales",
            "--operations",
            r#"[{"type":"sort","by":"Q1","descending":true}]"#,
        ],
    );

    // The chain left a consistent, reopenable document behind.
    ok(d, &["document", "close"]);
    let info = ok(d, &["document", "open", &doc]);
    assert_eq!(info["status"], "ok");
}
