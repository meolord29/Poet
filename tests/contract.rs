//! Hostile-input battery (adr/0015): `run_with` must never panic and must
//! always produce either clap's rendered text (`None`) or exactly one JSON
//! envelope (`Some`). `AGENTS.md` §2 pins the never-panic guarantee; these
//! cases try to break it.

use std::process::ExitCode;

use poet::app::run_with;
use poet::commands::testutil;
use poet::core::Ctx;

fn argv(items: &[&str]) -> Vec<String> {
    let mut out = vec!["poet".to_string()];
    out.extend(items.iter().map(|s| s.to_string()));
    out
}

/// Every case: no panic (the test completing is the proof) and the output is
/// either clap's (`None`) or an envelope whose first byte is `{`/`#`.
fn drive(dir: &std::path::Path, args: &[&str]) {
    let ctx: Ctx = Ctx::at_dir(dir);
    let (json, _code): (Option<String>, ExitCode) = run_with(ctx, argv(args));
    if let Some(text) = json {
        assert!(
            text.starts_with('{') || text.starts_with('#'),
            "unexpected raw output for {args:?}: {text:.80}"
        );
    }
}

#[test]
fn empty_and_whitespace_arguments_never_panic() {
    let (_ctx, dir) = testutil::setup();
    for args in [
        vec!["paragraph", "add", ""],
        vec!["paragraph", "add", "   "],
        vec![""],
        vec!["paragraph", "add", "  \t\n "],
    ] {
        drive(&dir, &args);
    }
}

#[test]
fn hostile_indices_never_panic() {
    let (_ctx, dir) = testutil::setup();
    for args in [
        vec!["paragraph", "get", "--index", "18446744073709551615"],
        vec!["paragraph", "get", "--index", "-1"],
        vec!["paragraph", "get", "--index", "abc"],
        vec![
            "table", "set-cell", "--index", "0", "--row", "0", "--col", "0", "--value", "x",
        ],
        vec!["table", "delete-row", "--index", "999999999"],
        vec!["heading", "add", "--level", "255"],
        vec!["heading", "add", "--level", "-3"],
    ] {
        drive(&dir, &args);
    }
}

#[test]
fn unicode_and_controlish_text_never_panic() {
    let (_ctx, dir) = testutil::setup();
    for text in [
        "中文标题 — emoji ✓",
        "line1\nline2\ttabbed",
        "\u{0000}ignored-in-text",
        "repeated ",
    ] {
        drive(&dir, &["paragraph", "add", text]);
    }
}

#[test]
fn missing_and_duplicated_arguments_never_panic() {
    let (_ctx, dir) = testutil::setup();
    for args in [
        vec!["paragraph", "add", "--style"],
        vec!["document", "new"],
        vec!["table"],
        vec!["paragraph", "add", "a", "a", "a"],
        vec!["paragraph", "get", "--index", "0", "--index", "1"],
        vec!["calc", "stats", "--path", "/nonexistent/nope.docx"],
    ] {
        drive(&dir, &args);
    }
}

#[test]
fn long_input_never_panics() {
    let (_ctx, dir) = testutil::setup();
    let big = "x".repeat(100_000);
    drive(&dir, &["paragraph", "add", &big]);
}

#[test]
fn hostile_paths_never_panic() {
    let (_ctx, dir) = testutil::setup();
    for args in [
        vec!["document", "open", "/nonexistent/deep/path.docx"],
        vec!["document", "open", ""],
        vec![
            "document",
            "export",
            "--format",
            "md",
            "--output",
            "/nonexistent/out.md",
        ],
        vec!["document", "open", "a\0b.docx"],
    ] {
        drive(&dir, &args);
    }
}

#[test]
fn chained_hostile_sequence_stays_enveloped() {
    let (_ctx, dir) = testutil::setup();
    let doc = dir.join("d.docx").to_string_lossy().into_owned();
    for args in [
        vec!["document", "new", doc.as_str()],
        vec!["paragraph", "add", "ok"],
        vec!["table", "add", "--rows", "1", "--cols", "1"],
        vec!["paragraph", "get", "--index", "9999"],
        vec!["document", "close"],
        vec!["paragraph", "count"],
        vec!["document", "close"],
    ] {
        drive(&dir, &args);
    }
}
