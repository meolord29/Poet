//! Dev-build-only helpers (adr/0015): the QA sandbox lifecycle and the
//! `--capture-example` authoring loop. Compiled exclusively under the `dev`
//! feature, so none of this surface reaches a release binary, `poet howto`,
//! or the QA agent's release-parity probes.
//!
//! `dev setup`/`dev clean` own the `.sandbox/` directory so the QA agent needs
//! no filesystem permissions of its own — the CLI is the only thing that
//! touches it (carpenter adr/016). `--dev-home <DIR>` redirects the session
//! repository for one invocation, keeping `session.json` inside the sandbox
//! (Poet's analog of carpenter's `--root`; a release build rejects the flag).
//!
//! The capture loop runs a command for real, then writes the worked-example
//! atom — invocation (dev flags stripped) + the real rendered envelope + a
//! TODO for the behavioral note — so example atoms carry observed output, not
//! hand-guessed JSON. Capture is best-effort: the command's own envelope on
//! stdout is authoritative.

use std::path::{Path, PathBuf};

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::models::data::Data;

/// The dev-only global flag that redirects the session home (`--dev-home`).
pub const DEV_HOME_FLAG: &str = "--dev-home";

/// The dev-only capture flag (`--capture-example <PATH>`).
pub const CAPTURE_FLAG: &str = "--capture-example";

/// The validation sandbox directory name (cwd-relative). Fixed convention so
/// `setup`/`clean` and the agent's `--dev-home` prefix agree.
pub const SANDBOX: &str = ".sandbox";

/// Production context for one invocation, honoring `--dev-home` when present:
/// the session repository moves to `<DIR>` (auto-open included), so sandboxed
/// runs never touch the real `$HOME/.poet`.
pub fn ctx_from_argv(argv: &[String]) -> Ctx {
    match scan_flag_value(argv, DEV_HOME_FLAG) {
        Some(home) => Ctx::at_dir(Path::new(&home)),
        None => Ctx::from_env(),
    }
}

/// Extract a flag's value from raw argv — `"--flag value"` or
/// `"--flag=value"` form. `None` when absent. clap validates the parse; this
/// pre-scan only feeds the dev hooks in [`crate::app::run`], which run before
/// `Ctx` construction (and so before `run_with` re-parses).
pub fn scan_flag_value(argv: &[String], flag: &str) -> Option<String> {
    let flag_eq = format!("{flag}=");
    let mut next = false;
    for arg in argv {
        if next {
            return Some(arg.clone());
        }
        if arg == flag {
            next = true;
        } else if let Some(value) = arg.strip_prefix(&flag_eq) {
            return Some(value.to_string());
        }
    }
    None
}

/// Shell-quote one argv token for the atom's invocation line: safe tokens
/// pass through untouched; anything with whitespace, quotes, or emptiness is
/// single-quoted (embedded `'` as `'\''`), so the atom's command re-runs
/// exactly as captured.
fn shell_quote(token: &str) -> String {
    let safe = !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
    if safe {
        return token.to_string();
    }
    format!("'{}'", token.replace('\'', "'\\''"))
}

/// Rebuild the `poet …` invocation line from argv, dropping arg[0] (replaced
/// by the literal `poet`) and both dev-only flags with their values, so the
/// atom shows the command a user re-runs on the release surface.
pub fn filter_dev_flags_from_argv(args: &[String]) -> String {
    let dev_home_eq = format!("{DEV_HOME_FLAG}=");
    let capture_eq = format!("{CAPTURE_FLAG}=");
    let mut keep: Vec<&str> = Vec::new();
    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == DEV_HOME_FLAG || arg == CAPTURE_FLAG {
            skip_next = true;
            continue;
        }
        if arg.starts_with(&dev_home_eq) || arg.starts_with(&capture_eq) {
            continue;
        }
        keep.push(arg.as_str());
    }
    let quoted: Vec<String> = keep.iter().map(|s| shell_quote(s)).collect();
    format!("poet {}", quoted.join(" "))
}

/// Build the worked-example atom for a real invocation and write it to
/// `out_path`. `envelope` is the already-rendered stdout string for the run.
/// Best-effort: failures are swallowed because the command's own envelope on
/// stdout is authoritative — capture is a convenience, not a contract.
pub fn write_capture_example(argv: &[String], out_path: &str, envelope: &str) {
    let invocation = filter_dev_flags_from_argv(argv);
    let atom = assemble_atom(&invocation, envelope);
    let path = Path::new(out_path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, atom);
}

/// Compose the worked-example markdown atom: invocation fence, the real
/// envelope, and the TODO the authoring loop cannot derive (the behavioral
/// note needs a human/LLM).
fn assemble_atom(invocation: &str, envelope: &str) -> String {
    let mut out = String::new();
    out.push_str("**example:**\n\n```sh\n");
    out.push_str(invocation);
    out.push_str("\n```\n\nResult (one envelope on stdout):\n```json\n");
    out.push_str(envelope.trim_end());
    out.push_str("\n```\n\n<!-- TODO: author the behavioral note -->\n");
    out
}

/// `dev setup` core: create `.sandbox/` (+ its `.poet` session home) under
/// `root`. Idempotent — `created:false` when the sandbox already existed.
pub fn setup_at(root: &Path) -> Result<Data, PoetError> {
    let dir = root.join(SANDBOX);
    let home = dir.join(".poet");
    let existed = dir.is_dir();
    std::fs::create_dir_all(&home)
        .map_err(|e| PoetError::File(format!("dev setup failed: {e}")))?;
    Ok(Data::DevSetup {
        path: dir.display().to_string(),
        home: home.display().to_string(),
        created: !existed,
    })
}

/// `dev clean` core: remove `.sandbox/` under `root`, session home included.
/// Idempotent — `removed:false` when it was already absent.
pub fn clean_at(root: &Path) -> Result<Data, PoetError> {
    let dir = root.join(SANDBOX);
    if !dir.is_dir() {
        return Ok(Data::DevClean {
            removed: false,
            path: dir.display().to_string(),
        });
    }
    std::fs::remove_dir_all(&dir).map_err(|e| PoetError::File(format!("dev clean failed: {e}")))?;
    Ok(Data::DevClean {
        removed: true,
        path: dir.display().to_string(),
    })
}

/// Resolve the current directory as the sandbox root, mapping failure to the
/// file error (env state, not a logic bug).
pub fn cwd() -> Result<PathBuf, PoetError> {
    std::env::current_dir().map_err(|e| PoetError::File(format!("current_dir failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn scan_flag_value_finds_space_and_equals_forms() {
        let a = argv(&["poet", "--dev-home", ".sandbox/.poet", "dev", "setup"]);
        assert_eq!(
            scan_flag_value(&a, DEV_HOME_FLAG).as_deref(),
            Some(".sandbox/.poet")
        );
        let b = argv(&["poet", "--dev-home=.sandbox/.poet", "document", "info"]);
        assert_eq!(
            scan_flag_value(&b, DEV_HOME_FLAG).as_deref(),
            Some(".sandbox/.poet")
        );
        assert_eq!(scan_flag_value(&a, CAPTURE_FLAG), None);
    }

    #[test]
    fn scan_flag_value_absent_flag_is_none() {
        let a = argv(&["poet", "table", "list"]);
        assert_eq!(scan_flag_value(&a, DEV_HOME_FLAG), None);
        assert_eq!(scan_flag_value(&a, CAPTURE_FLAG), None);
    }

    #[test]
    fn filter_strips_both_dev_flags_in_both_forms() {
        let a = argv(&[
            "target/debug/poet",
            "--dev-home",
            ".sandbox/.poet",
            "paragraph",
            "add",
            "Hello",
            "--capture-example",
            "docs/examples/paragraph/add.md",
        ]);
        assert_eq!(filter_dev_flags_from_argv(&a), "poet paragraph add Hello");
        let b = argv(&[
            "poet",
            "--dev-home=.sandbox/.poet",
            "table",
            "list",
            "--capture-example=out.md",
        ]);
        assert_eq!(filter_dev_flags_from_argv(&b), "poet table list");
    }

    #[test]
    fn filter_quotes_tokens_with_spaces_and_quotes() {
        let a = argv(&[
            "poet",
            "paragraph",
            "add",
            "Revenue grew 12%.",
            "--id",
            "rev",
        ]);
        assert_eq!(
            filter_dev_flags_from_argv(&a),
            "poet paragraph add 'Revenue grew 12%.' --id rev"
        );
        let b = argv(&["poet", "meta", "set-document", r#"{"title": "Q3"}"#]);
        assert_eq!(
            filter_dev_flags_from_argv(&b),
            "poet meta set-document '{\"title\": \"Q3\"}'"
        );
        let c = argv(&["poet", "paragraph", "add", "it's"]);
        assert_eq!(
            filter_dev_flags_from_argv(&c),
            "poet paragraph add 'it'\\''s'"
        );
        let d = argv(&["poet", "paragraph", "add", ""]);
        assert_eq!(filter_dev_flags_from_argv(&d), "poet paragraph add ''");
    }

    #[test]
    fn atom_has_invocation_envelope_and_todo() {
        let a = assemble_atom("poet paragraph add Hello", "{\"status\":\"ok\"}");
        assert!(a.starts_with("**example:**\n\n```sh\npoet paragraph add Hello\n```\n"));
        assert!(a.contains("```json\n{\"status\":\"ok\"}\n```"));
        assert!(a.contains("TODO: author the behavioral note"));
        assert!(!a.contains("```yaml"), "Poet has no --spec blocks");
    }

    #[test]
    fn setup_creates_sandbox_then_reports_created_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = setup_at(tmp.path()).expect("setup");
        let Data::DevSetup {
            path,
            home,
            created,
        } = data
        else {
            panic!("expected DevSetup");
        };
        assert!(created);
        assert!(path.ends_with(".sandbox"));
        assert!(home.ends_with(".sandbox/.poet"));
        assert!(tmp.path().join(".sandbox/.poet").is_dir());

        let Data::DevSetup { created, .. } = setup_at(tmp.path()).expect("setup 2") else {
            panic!("expected DevSetup");
        };
        assert!(!created, "second setup must be idempotent");
    }

    #[test]
    fn clean_removes_sandbox_then_reports_removed_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        setup_at(tmp.path()).expect("setup");

        let Data::DevClean { removed, path } = clean_at(tmp.path()).expect("clean") else {
            panic!("expected DevClean");
        };
        assert!(removed);
        assert!(path.ends_with(".sandbox"));
        assert!(!tmp.path().join(".sandbox").exists());

        let Data::DevClean { removed, .. } = clean_at(tmp.path()).expect("clean 2") else {
            panic!("expected DevClean");
        };
        assert!(!removed, "second clean must be idempotent");
    }

    #[test]
    fn clean_before_setup_is_a_noop() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let Data::DevClean { removed, .. } = clean_at(tmp.path()).expect("clean") else {
            panic!("expected DevClean");
        };
        assert!(!removed);
    }
}
