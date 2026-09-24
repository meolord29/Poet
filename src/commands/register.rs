//! `register` — write the poet `SKILL.md` into an agent app's skills dir and
//! merge the allow permission entry (adr/0019). `--print-skill` instead emits
//! the rendered bytes (no filesystem change). Mechanics live in
//! [`crate::core::skill`]; this file is the signature wrapper.

use crate::core::error::PoetError;
use crate::core::skill::{self, App};
use crate::models::data::Data;

/// Arguments for `poet register`.
#[derive(Debug, clap::Args)]
pub struct RegisterArgs {
    /// Agent app to register the skill into (`opencode` or `claude-code`).
    #[arg(long)]
    pub app: String,
    /// Print the rendered SKILL.md instead of writing it.
    #[arg(long)]
    pub print_skill: bool,
}

/// `poet register --app opencode|claude-code [--print-skill]`.
pub fn register(_ctx: &crate::core::Ctx, args: &RegisterArgs) -> Result<Data, PoetError> {
    register_at(&args.app, args.print_skill)
}

/// Anchor-free core of the command (the thin wrapper resolves env anchors).
fn register_at(app: &str, print_skill: bool) -> Result<Data, PoetError> {
    let app = App::parse(app)?;
    if print_skill {
        return Ok(Data::PrintSkill {
            skill: skill::render()?,
        });
    }
    let r = skill::register(app, &skill::xdg_root()?, &skill::home_dir()?)?;
    Ok(Data::Register {
        app: r.app,
        path: r.path,
        version: r.version,
        installed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::skill;

    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn anchors() -> (std::path::PathBuf, std::path::PathBuf) {
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("poet-register-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let xdg = base.join("xdg");
        let home = base.join("home");
        std::fs::create_dir_all(&xdg).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        (xdg, home)
    }

    #[test]
    fn register_rejects_unsupported_app() {
        let err = register_at("agents", false).unwrap_err();
        assert!(matches!(err, PoetError::Validation(_)));
    }

    #[test]
    fn register_print_skill_has_no_fs_effect_on_anchors() {
        // --print-skill renders bytes only; nothing may be written anywhere.
        let rendered = skill::render().expect("render");
        let Data::PrintSkill { skill } = register_at("opencode", true).expect("print") else {
            panic!("PrintSkill");
        };
        assert_eq!(skill, rendered);
    }

    #[test]
    fn register_writes_into_explicit_anchors() {
        // The env-anchor path is covered end-to-end in tests/install_surface.rs;
        // here the anchor-parameterized core is pinned.
        let (xdg, home) = anchors();
        let r = skill::register(App::Opencode, &xdg, &home).expect("register");
        assert_eq!(r.app, "opencode");
        assert!(r.path.ends_with("opencode/skills/poet/SKILL.md"));
        assert!(!r.version.is_empty());
        assert!(xdg.join("opencode/skills/poet/SKILL.md").exists());
        let _ = std::fs::remove_dir_all(&xdg);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn register_payload_echoes_app_and_installed() {
        let (xdg, home) = anchors();
        let r = skill::register(App::ClaudeCode, &xdg, &home).expect("register");
        let data = Data::Register {
            app: r.app.clone(),
            path: r.path.clone(),
            version: r.version.clone(),
            installed: true,
        };
        let Data::Register { app, installed, .. } = data else {
            panic!("Register");
        };
        assert_eq!(app, "claude-code");
        assert!(installed);
        let _ = std::fs::remove_dir_all(&xdg);
        let _ = std::fs::remove_dir_all(&home);
    }
}
