//! `uninstall` — remove the poet skill from every registered agent app and
//! delete the installed binary (adr/0021). Ordering: skill first (recoverable
//! via `register`), binary last (point of no return). Removing the running
//! binary is safe on Linux/macOS — unlink keeps the inode alive for the
//! running process (the same property `upgrade`'s copy-replace relies on).
//! Poet session data (`~/.poet`) is never touched. Skill mechanics live in
//! [`crate::core::skill`]; this file is the signature wrapper.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::core::error::PoetError;
use crate::core::skill::{self, App};
use crate::models::data::Data;

/// Arguments for `poet uninstall`.
#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    /// Directory holding the `poet` binary (default `~/.local/bin`).
    #[arg(long)]
    pub bin_dir: Option<String>,
}

/// `poet uninstall [--bin-dir <p>]`.
pub fn uninstall(_ctx: &crate::core::Ctx, args: &UninstallArgs) -> Result<Data, PoetError> {
    let target_dir = match &args.bin_dir {
        Some(d) => PathBuf::from(d),
        None => skill::home_dir()?.join(".local").join("bin"),
    };
    let bin = target_dir.join("poet");
    let (xdg, home) = (skill::xdg_root()?, skill::home_dir()?);
    let skill_registered = App::all()
        .iter()
        .any(|a| a.skill_path(&xdg, &home).exists());
    if !skill_registered && !bin.exists() {
        return Err(PoetError::NotFound(format!(
            "poet is not installed (no registered skill and no binary at {})",
            bin.display()
        )));
    }
    let skill_outcome = remove_skill_at(&xdg, &home);
    let removed_bin = if bin.exists() {
        std::fs::remove_file(&bin)
            .map_err(|e| PoetError::File(format!("cannot remove {}: {e}", bin.display())))?;
        Some(bin.display().to_string())
    } else {
        None
    };
    Ok(Data::Uninstall {
        uninstalled: true,
        bin: removed_bin,
        skill: skill_outcome,
    })
}

/// Remove the poet skill from every registered app, best-effort: a missing or
/// failing removal never fails the uninstall (mirrors `upgrade`'s outcome).
/// Anchor-explicit so tests pass temp dirs.
fn remove_skill_at(root: &std::path::Path, home: &std::path::Path) -> Value {
    let outcomes: Vec<Value> = App::all()
        .into_iter()
        .filter(|app| app.skill_path(root, home).exists())
        .map(|app| match skill::deregister(app, root, home) {
            Ok(d) => json!({"removed": true, "app": d.app, "path": d.path}),
            Err(e) => json!({"removed": false, "reason": "not_registered", "error": e.to_string()}),
        })
        .collect();
    if outcomes.is_empty() {
        return json!({"removed": false, "reason": "not_registered"});
    }
    Value::Array(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn anchors() -> (std::path::PathBuf, std::path::PathBuf) {
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("poet-uninstall-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let xdg = base.join("xdg");
        let home = base.join("home");
        std::fs::create_dir_all(&xdg).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        (xdg, home)
    }

    #[test]
    fn uninstall_removes_skill_and_binary() {
        let (xdg, home) = anchors();
        skill::register(App::Opencode, &xdg, &home).expect("register");
        let bin_dir = xdg.parent().unwrap().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("poet");
        std::fs::write(&bin, b"fake binary").unwrap();
        let outcome = remove_skill_at(&xdg, &home);
        let arr = outcome.as_array().expect("per-app outcomes");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["removed"], json!(true));
        assert_eq!(arr[0]["app"], json!("opencode"));
        assert!(!xdg.join("opencode/skills/poet/SKILL.md").exists());
        assert!(
            bin.exists(),
            "binary removal is the command's job, not the helper's"
        );
        let removed = std::fs::remove_file(&bin).is_ok();
        assert!(removed);
        assert!(!bin.exists());
        let _ = std::fs::remove_dir_all(bin_dir.parent().unwrap());
    }

    #[test]
    fn uninstall_helper_reports_not_registered_when_nothing_registered() {
        let (xdg, home) = anchors();
        let out = remove_skill_at(&xdg, &home);
        assert_eq!(out["removed"], json!(false));
        assert_eq!(out["reason"], json!("not_registered"));
        let _ = std::fs::remove_dir_all(xdg.parent().unwrap());
    }

    #[test]
    fn uninstall_helper_removes_both_registered_apps() {
        let (xdg, home) = anchors();
        skill::register(App::Opencode, &xdg, &home).expect("register");
        skill::register(App::ClaudeCode, &xdg, &home).expect("register");
        let out = remove_skill_at(&xdg, &home);
        let arr = out.as_array().expect("per-app outcomes");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["app"], json!("opencode"));
        assert_eq!(arr[1]["app"], json!("claude-code"));
        assert!(!xdg.join("opencode/skills/poet/SKILL.md").exists());
        assert!(!home.join(".claude/skills/poet/SKILL.md").exists());
        let _ = std::fs::remove_dir_all(xdg.parent().unwrap());
    }
}
