//! `upgrade` — replace the installed poet binary, then refresh the skill of
//! every registered agent app (adr/0020). Release mode only: fetch a
//! published tarball for the running platform, verify its checksum, extract,
//! probe, atomically replace. The channel picks the release (`stable` =
//! GitHub Latest, `nightly` = the rolling prerelease, adr/0002). Mechanics
//! live in [`crate::core::release`]; this file is the signature wrapper.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::core::error::PoetError;
use crate::core::release::{self, Channel};
use crate::core::skill::{self, App};
use crate::models::data::Data;

/// The single-sourced "not registered" warning.
const NOT_REGISTERED_WARNING: &str =
    "CLI not registered in any agent app — nothing to refresh. Run `poet register`.";

/// Arguments for `poet upgrade`.
#[derive(Debug, clap::Args)]
pub struct UpgradeArgs {
    /// Release channel to follow (`stable` default, `nightly` canary).
    #[arg(long, default_value = "stable")]
    pub channel: String,
    /// Directory holding the `poet` binary (default `~/.local/bin`).
    #[arg(long)]
    pub bin_dir: Option<String>,
    /// Skip the skill refresh.
    #[arg(long)]
    pub no_skill: bool,
}

/// `poet upgrade [--channel stable|nightly] [--bin-dir <p>] [--no-skill]`.
pub fn upgrade(_ctx: &crate::core::Ctx, args: &UpgradeArgs) -> Result<Data, PoetError> {
    let channel = Channel::parse(&args.channel)?;
    let target = release::platform_target().ok_or_else(|| {
        PoetError::Validation(format!(
            "no release asset for {} {} — build from source with cargo \
             (published: Linux x86_64, macOS Apple Silicon)",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;
    // `_staged` keeps the download dir alive until the binary is copied out.
    let staged = {
        let dir = release::stage_dir()?;
        release::fetch_release(&release::download_base(channel), target, &dir)?
    };
    let version = release::probe_version(&staged.bin)?;
    let target_dir = match &args.bin_dir {
        Some(d) => PathBuf::from(d),
        None => skill::home_dir()?.join(".local").join("bin"),
    };
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| PoetError::File(format!("cannot create {}: {e}", target_dir.display())))?;
    let dest = target_dir.join("poet");
    let tmp = dest.with_extension("poet-tmp");
    std::fs::copy(&staged.bin, &tmp)
        .map_err(|e| PoetError::File(format!("cannot stage {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, &dest)
        .map_err(|e| PoetError::File(format!("cannot finalize {}: {e}", dest.display())))?;
    let skill_outcome = if args.no_skill {
        None
    } else {
        Some(skill_outcome())
    };
    Ok(Data::Upgrade {
        upgraded: true,
        version,
        bin: dest.display().to_string(),
        source: staged.url.clone(),
        skill: skill_outcome,
    })
}

/// Refresh the skill of every registered app (skill file present), best-
/// effort: a failure never fails the upgrade, and no app is registered that
/// wasn't already (the installer only registers on confirmation — parity).
fn skill_outcome() -> Value {
    match (skill::xdg_root(), skill::home_dir()) {
        (Ok(root), Ok(home)) => skill_outcome_at(&root, &home),
        (Err(e), _) | (_, Err(e)) => {
            json!({"refreshed": false, "reason": "no_anchor", "error": e.to_string()})
        }
    }
}

/// Anchor-explicit core of [`skill_outcome`] (tests pass temp dirs).
fn skill_outcome_at(root: &std::path::Path, home: &std::path::Path) -> Value {
    let registered: Vec<App> = App::all()
        .into_iter()
        .filter(|app| app.skill_path(root, home).exists())
        .collect();
    if registered.is_empty() {
        return json!({"refreshed": false, "reason": "not_registered", "warning": NOT_REGISTERED_WARNING});
    }
    Value::Array(
        registered
            .into_iter()
            .map(|app| match skill::register(app, root, home) {
                Ok(r) => json!({"refreshed": true, "app": r.app, "path": r.path}),
                Err(e) => {
                    json!({"refreshed": false, "reason": "refresh_failed", "error": e.to_string()})
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_rejects_unknown_channel() {
        let args = UpgradeArgs {
            channel: "edge".into(),
            bin_dir: None,
            no_skill: true,
        };
        let err = Channel::parse(&args.channel).unwrap_err();
        assert!(matches!(err, PoetError::Validation(_)), "{err}");
    }

    #[test]
    fn not_registered_warning_is_stable() {
        assert!(NOT_REGISTERED_WARNING.contains("not registered in any agent app"));
    }

    #[test]
    fn skill_outcome_refreshes_only_registered_apps() {
        let n = std::sync::atomic::AtomicUsize::new(0);
        let id = n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("poet-upskill-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (xdg, home) = (base.join("xdg"), base.join("home"));
        std::fs::create_dir_all(&xdg).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        // nothing registered → single not_registered outcome
        let out = skill_outcome_at(&xdg, &home);
        assert_eq!(out["reason"], json!("not_registered"));
        assert_eq!(out["warning"], json!(NOT_REGISTERED_WARNING));
        // claude-code only → exactly one refresh, for claude-code
        skill::register(App::ClaudeCode, &xdg, &home).unwrap();
        let out = skill_outcome_at(&xdg, &home);
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["app"], json!("claude-code"));
        assert_eq!(arr[0]["refreshed"], json!(true));
        // both registered → two refreshes, order = App::all()
        skill::register(App::Opencode, &xdg, &home).unwrap();
        let out = skill_outcome_at(&xdg, &home);
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["app"], json!("opencode"));
        assert_eq!(arr[1]["app"], json!("claude-code"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
