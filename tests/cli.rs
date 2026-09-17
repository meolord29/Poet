//! Whole-process integration tests through the CLI dispatch layer.
//!
//! These exercise the same code paths as a shell invocation (`run_with`)
//! without spawning processes; session state lives in per-test temp dirs.

use std::process::ExitCode;

use poet::commands::testutil::setup;
use poet::core::Ctx;

fn run(ctx: &Ctx, args: &[&str]) -> (String, ExitCode) {
    let argv: Vec<String> = std::iter::once("poet".to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    let (json, code) = poet::app::run_with(ctx.clone(), argv);
    (json.expect("dispatch always emits a document"), code)
}

#[test]
fn document_lifecycle_round_trip_via_cli() {
    let (ctx, dir) = setup();
    let path = dir.join("report.docx");
    let path_str = path.to_string_lossy().into_owned();

    // new: persists the file and starts the session (`&&` chaining support).
    let (json, code) = run(&ctx, &["document", "new", &path_str]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(json.contains("\"status\": \"ok\""));
    assert!(json.contains("Document created"));
    assert!(path.exists(), "new persists so chained calls can continue");
    assert!(ctx.session.get().is_some(), "new starts a session");

    // info through a fresh context (fresh process): session auto-opens.
    let ctx2 = Ctx::at_dir(&dir);
    let (json, code) = run(&ctx2, &["document", "info"]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(json.contains("paragraph_count"));

    // save with no path: falls back to the session-opened current path.
    let (json, code) = run(&ctx2, &["document", "save"]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(json.contains("Document saved"));

    // close: deletes the session.
    let (json, code) = run(&ctx2, &["document", "close"]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(json.contains("Document closed"));
    assert!(ctx2.session.get().is_none(), "close ends the session");
}

#[test]
fn unimplemented_actions_return_error_envelope_and_exit_1() {
    let (ctx, _dir) = setup();
    // Phase-4 stubs; `meta` is ported now and gets the state error instead.
    for args in [
        vec!["batch", "run", "script.json"],
        vec!["calc", "read", "sheet.xlsx"],
    ] {
        let (json, code) = run(&ctx, &args);
        assert_eq!(code, ExitCode::FAILURE);
        assert!(json.contains("\"status\": \"error\""));
        assert!(json.contains("\"code\": \"not_implemented\""));
        assert!(json.contains("\"details\": {}"));
    }
}

#[test]
fn meta_actions_without_an_open_document_return_state_error() {
    let (ctx, _dir) = setup();
    let (json, code) = run(&ctx, &["meta", "get-document"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert!(json.contains("\"code\": \"document_state\""));
}

#[test]
fn open_missing_document_is_not_found_envelope() {
    let (ctx, _dir) = setup();
    let (json, code) = run(&ctx, &["document", "open", "/nonexistent/x.docx"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert!(json.contains("\"code\": \"not_found\""));
}

#[test]
fn no_subcommand_prints_howto_text_and_exits_0() {
    let (ctx, _dir) = setup();
    let (json, code) = run(&ctx, &[]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(json.contains("phase 4"), "placeholder howto until phase 4");
}

#[test]
fn unknown_action_exits_with_clap_usage_error_code() {
    let (ctx, _dir) = setup();
    let argv: Vec<String> = ["poet", "document", "frobnicate"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (json, code) = poet::app::run_with(ctx, argv);
    assert!(json.is_none(), "usage errors print plain text");
    assert_eq!(code, ExitCode::from(2));
}

#[test]
fn version_flag_prints_without_json() {
    let (ctx, _dir) = setup();
    let argv: Vec<String> = ["poet", "--version"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (json, code) = poet::app::run_with(ctx, argv);
    assert!(json.is_none());
    assert_eq!(code, ExitCode::SUCCESS);
}
