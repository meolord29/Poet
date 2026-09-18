//! Whole-process contract tests (adr/0015): the real binary is spawned, so
//! process-level behavior — exit codes, clap's stderr path, exactly-one-JSON-
//! document on stdout, `POET_HOME` resolution — is pinned, not assumed.
//! In-process tests (`run_with`) cannot see these layers.

use std::process::{Command, Output};

fn run_in(dir: &std::path::Path, poet_home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poet"))
        .args(args)
        .current_dir(dir)
        .env("POET_HOME", poet_home)
        .env_remove("POET_DEV")
        .output()
        .expect("spawn poet binary")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn version_is_plain_text_and_exit_zero() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = run_in(tmp.path(), tmp.path(), &["--version"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.contains("poet"), "{text}");
    assert!(
        serde_json::from_str::<serde_json::Value>(&text).is_err(),
        "--version must not print JSON"
    );
}

#[test]
fn no_subcommand_prints_the_howto_raw() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = run_in(tmp.path(), tmp.path(), &[]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.starts_with("# Poet"), "{text}");
    assert!(
        serde_json::from_str::<serde_json::Value>(&text).is_err(),
        "howto must not print JSON"
    );
}

#[test]
fn unknown_action_is_clap_text_exit_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = run_in(tmp.path(), tmp.path(), &["paragraph", "frobnicate"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stdout(&out).is_empty(), "clap errors go to stderr");
    assert!(stderr(&out).contains("unrecognized subcommand"));
}

#[test]
fn unknown_global_flag_is_exit_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = run_in(tmp.path(), tmp.path(), &["--nope", "paragraph", "count"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("unexpected argument"));
}

#[test]
fn success_prints_exactly_one_json_document() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = run_in(tmp.path(), tmp.path(), &["document", "new", "report.docx"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    let env: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("stdout must be exactly one JSON document: {e}\n{text}"));
    assert_eq!(env["status"], "ok");
    assert!(text.ends_with('\n'));
}

#[test]
fn error_prints_one_json_document_and_exits_one() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = run_in(tmp.path(), tmp.path(), &["paragraph", "count"]);
    assert_eq!(out.status.code(), Some(1));
    let env: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("one JSON document");
    assert_eq!(env["status"], "error");
    assert_eq!(env["code"], "document_state");
}

#[test]
fn session_chains_across_processes_via_poet_home() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("home");
    let first = run_in(tmp.path(), &home, &["document", "new", "report.docx"]);
    assert_eq!(first.status.code(), Some(0), "{}", stdout(&first));
    assert!(
        home.join("session.json").exists(),
        "session.json must live under POET_HOME"
    );
    // A fresh process auto-opens the session document (the `&&` contract).
    let second = run_in(tmp.path(), &home, &["paragraph", "add", "Hello"]);
    assert_eq!(second.status.code(), Some(0), "{}", stdout(&second));
    let env: serde_json::Value = serde_json::from_str(&stdout(&second)).expect("json");
    assert_eq!(env["status"], "ok");
}

#[cfg(feature = "dev")]
#[test]
fn dev_sandbox_lifecycle_end_to_end() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("home");
    let bin = || env!("CARGO_BIN_EXE_poet");
    let run = |args: &[&str]| {
        Command::new(bin())
            .args(args)
            .current_dir(tmp.path())
            .env("POET_HOME", &home)
            .output()
            .expect("spawn poet")
    };
    let out = run(&["--dev-home", ".sandbox/.poet", "dev", "setup"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    assert!(tmp.path().join(".sandbox/.poet").is_dir());
    // Documents and session state land inside the sandbox only — the runbook
    // convention is explicit `.sandbox/<name>.docx` document paths.
    let doc = run(&[
        "--dev-home",
        ".sandbox/.poet",
        "document",
        "new",
        ".sandbox/x.docx",
    ]);
    assert_eq!(doc.status.code(), Some(0), "{}", stdout(&doc));
    assert!(tmp.path().join(".sandbox/x.docx").exists());
    assert!(tmp.path().join(".sandbox/.poet/session.json").exists());
    assert!(!home.join("session.json").exists(), "no real-home leakage");
    let clean = run(&["--dev-home", ".sandbox/.poet", "dev", "clean"]);
    assert_eq!(clean.status.code(), Some(0), "{}", stdout(&clean));
    assert!(!tmp.path().join(".sandbox").exists());
}

#[cfg(feature = "dev")]
#[test]
fn capture_example_writes_the_atom() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let capture = tmp.path().join("captured/add.md");
    let capture_str = capture.to_str().expect("utf8 temp path");
    let out = Command::new(env!("CARGO_BIN_EXE_poet"))
        .args([
            "--dev-home",
            ".sandbox/.poet",
            "--capture-example",
            capture_str,
            "document",
            "new",
            "x.docx",
        ])
        .current_dir(tmp.path())
        .env("POET_HOME", tmp.path().join("home"))
        .output()
        .expect("spawn poet");
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    let atom = std::fs::read_to_string(&capture).expect("atom written");
    assert!(
        atom.contains("```sh\npoet document new x.docx\n```"),
        "{atom}"
    );
    assert!(atom.contains("\"status\": \"ok\""), "{atom}");
    assert!(atom.contains("TODO: author the behavioral note"), "{atom}");
    assert!(!atom.contains("--capture-example"), "{atom}");
    assert!(!atom.contains("--dev-home"), "{atom}");
}

#[cfg(not(feature = "dev"))]
#[test]
fn release_binary_rejects_the_dev_surface() {
    let tmp = tempfile::tempdir().expect("tempdir");
    for args in [
        vec!["dev", "setup"],
        vec!["--dev-home", "x", "paragraph", "count"],
        vec!["--capture-example", "x.md", "paragraph", "count"],
    ] {
        let out = run_in(tmp.path(), tmp.path(), &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stdout(&out));
        assert!(stdout(&out).is_empty(), "{args:?}");
    }
}
