//! The poet `SKILL.md` renderer + register/deregister (agent-app skill
//! integration, adr/0019). Ported from carpenter's `core/skill.rs`.
//!
//! Authored consts are the only hand-written atoms; the command surface is the
//! `poet howto` manual ([`crate::howto::HOWTO`]), inlined into the body at
//! render time (DRY — same shipped artifact, never hand-duplicated). Anchors
//! (`root` = XDG root, `home` = user home) are always explicit parameters, so
//! tests never touch the real `~/.config` / `~/.claude`.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::core::error::PoetError;
use crate::howto::HOWTO;

/// The skill name (also the skills-dir leaf). Always `poet`.
pub const NAME: &str = "poet";

/// Frontmatter description (authored once; opencode matches on it).
const DESCRIPTION: &str = "Activate when the user wants to create, edit, inspect, or analyze Word (.docx) documents from the command line. Drives the poet CLI (bookmark addressing, semantic metadata, batch scripts, Polars-powered table analysis).";

/// Authored prose — exists nowhere else in code to derive from.
const WHAT_THIS_IS: &str = "poet is an agent-driven CLI for Word (.docx) automation. Every element poet creates is wrapped in a bookmark with a stable id (`--id`), every command emits exactly one JSON envelope on stdout, and rich semantic metadata is stored inside the document. You (the agent) plan the document; poet does deterministic read/write/analysis.";

/// Authored workflow — the document-building loop a registering agent should
/// follow before consulting the inlined manual below.
const WORKFLOW: &str = "0. **Agree on the outline first** — audience, sections, tables, formatting. Do not create files before the user approves the plan.\n1. `poet document new <path>.docx` starts a document (the session auto-opens it, so chained invocations keep working).\n2. Build content: `heading add`, `paragraph add`, `table add` + `table set-range`, `list add`, `image add`, `toc add`. Pass `--id` to keep stable handles you can address later.\n3. Reference existing elements with `--id`; use `--index` only for documents created outside poet (no bookmarks).\n4. Format with `run add`/`run format`/`run emphasize` and `style apply`; lay out with the `page` commands.\n5. Analyze tables with `calc stats`/`calc aggregate`/`calc filter`/`calc transform` (Polars-backed).\n6. `poet document save` persists; chain multi-step work in a `poet batch run script.json` script instead of many shell calls.\n\nRun `poet howto` for the full command manual — it is inlined below for offline use.";

/// Validate a skill name against opencode's `^[a-z0-9]+(-[a-z0-9]+)*$`.
pub fn name_is_valid(s: &str) -> bool {
    let mut prev_dash = false;
    let mut alnum = 0;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() && !ch.is_ascii_uppercase() {
            prev_dash = false;
            alnum += 1;
        } else if ch == '-' {
            if prev_dash || alnum == 0 {
                return false;
            }
            prev_dash = true;
        } else {
            return false;
        }
    }
    !prev_dash && alnum > 0
}

/// Escape `s` for a YAML double-quoted scalar.
fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The howto with its leading H1 line stripped, so inlining it under the
/// skill's own structure doesn't produce a second `#` heading. The manual is
/// the single source of the command surface (adr/0019).
fn manual_body() -> &'static str {
    match HOWTO.find('\n') {
        Some(i) => HOWTO[i..].trim_start_matches('\n'),
        None => HOWTO,
    }
}

/// Strip Linux's `" (deleted)"` suffix. The kernel appends it to
/// `/proc/self/exe` (read by [`std::env::current_exe`]) when the running
/// binary's file has been replaced in place — which happens during `upgrade`
/// (it copies the new binary over its own path, then refreshes the skill).
fn strip_deleted(s: &str) -> &str {
    s.strip_suffix(" (deleted)").unwrap_or(s)
}

/// Render the full `SKILL.md` (frontmatter + body). Embeds the running
/// version, binary path, and the full howto manual. Deterministic within a
/// process (same `current_exe`).
pub fn render() -> Result<String, PoetError> {
    let exe =
        std::env::current_exe().map_err(|e| PoetError::File(format!("current_exe failed: {e}")))?;
    let exe = exe.display().to_string();
    let bin = strip_deleted(&exe);
    let version = env!("CARGO_PKG_VERSION");
    let frontmatter = format!(
        "---\nname: {NAME}\ndescription: \"{}\"\n---\n\n",
        yaml_escape(DESCRIPTION)
    );
    let body = format!(
        "# {NAME}\n\n\
         {WHAT_THIS_IS}\n\n\
         ## Workflow\n\n{WORKFLOW}\n\n\
         ## Command manual\n\n\
         Sourced from `{NAME} howto` at render time — always matches the installed \
         binary. Installed version: {version} (`{bin}`).\n\n\
         {manual}",
        manual = manual_body(),
    );
    Ok(format!("{frontmatter}{body}"))
}

/// Which agent app to integrate with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    /// opencode (`$XDG_CONFIG_HOME/opencode/`).
    Opencode,
    /// claude code (`~/.claude/skills/`).
    ClaudeCode,
}

impl App {
    /// Every supported app (iteration order = registration order).
    pub fn all() -> [App; 2] {
        [App::Opencode, App::ClaudeCode]
    }

    /// Parse + validate an `--app` value.
    pub fn parse(s: &str) -> Result<App, PoetError> {
        match s {
            "opencode" => Ok(App::Opencode),
            "claude-code" => Ok(App::ClaudeCode),
            other => Err(PoetError::Validation(format!(
                "unknown app {other:?} (opencode|claude-code)"
            ))),
        }
    }

    /// Human label.
    pub fn name(self) -> &'static str {
        match self {
            App::Opencode => "opencode",
            App::ClaudeCode => "claude-code",
        }
    }

    /// The skill file path. `root` is the XDG root (e.g. `~/.config` —
    /// opencode is a sibling of `poet/`'s own config); `home` is the user
    /// home dir (claude code anchors at `~/.claude/`, not the XDG root).
    pub fn skill_path(self, root: &Path, home: &Path) -> PathBuf {
        let base = match self {
            App::Opencode => root.join("opencode"),
            App::ClaudeCode => home.join(".claude"),
        };
        base.join("skills").join(NAME).join("SKILL.md")
    }

    /// The permission file path, if the app needs one. opencode loads skills
    /// behind a permission entry; claude code auto-discovers `~/.claude/skills/`
    /// — no permission store to merge.
    pub fn permission_path(self, root: &Path) -> Option<PathBuf> {
        match self {
            App::Opencode => Some(root.join("opencode").join("opencode.json")),
            App::ClaudeCode => None,
        }
    }
}

/// The XDG root (`$XDG_CONFIG_HOME` or `$HOME/.config`).
pub fn xdg_root() -> Result<PathBuf, PoetError> {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => Ok(home_dir()?.join(".config")),
    }
}

/// The user home dir (`$HOME`).
pub fn home_dir() -> Result<PathBuf, PoetError> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| PoetError::File("HOME is not set — cannot resolve the skills anchor".into()))
}

/// A successful registration.
#[derive(Debug)]
pub struct Registered {
    /// app name.
    pub app: String,
    /// skill file path.
    pub path: String,
    /// embedded version.
    pub version: String,
}

/// A successful deregistration.
#[derive(Debug)]
pub struct Deregistered {
    /// app name.
    pub app: String,
    /// skill file path that was removed.
    pub path: String,
}

fn read_json(path: &Path) -> Result<Option<Value>, PoetError> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| PoetError::File(format!("corrupt {}: {e}", path.display()))),
        Err(_) => Ok(None),
    }
}

fn write_json(path: &Path, v: &Value) -> Result<(), PoetError> {
    let mut bytes = serde_json::to_string_pretty(v)
        .unwrap_or_else(|_| v.to_string())
        .into_bytes();
    bytes.push(b'\n');
    atomic_write(path, &bytes)
}

/// Write bytes via a temp file + rename, so a crash never leaves a truncated
/// skill or permission file behind.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), PoetError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| PoetError::File(format!("cannot create {}: {e}", parent.display())))?;
    }
    let tmp = path.with_extension("poet-tmp");
    std::fs::write(&tmp, bytes)
        .map_err(|e| PoetError::File(format!("cannot write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| PoetError::File(format!("cannot finalize {}: {e}", path.display())))
}

/// Ensure each `path` segment is a JSON object, descend, and set the leaf to `value`.
fn set_nested(root: &mut Value, path: &[&str], value: Value) -> Result<(), PoetError> {
    let mut cur = root;
    for (i, key) in path.iter().enumerate() {
        if !cur.is_object() {
            *cur = Value::Object(serde_json::Map::new());
        }
        let obj = cur.as_object_mut().ok_or_else(|| {
            PoetError::File(format!("config segment `{}` not an object", path[i]))
        })?;
        cur = obj
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
    }
    *cur = value;
    Ok(())
}

/// Register: render + write `SKILL.md` (idempotent) and merge the allow entry
/// (apps that have one). `root` is the XDG root containing `opencode/`;
/// `home` is the user home dir.
pub fn register(app: App, root: &Path, home: &Path) -> Result<Registered, PoetError> {
    let content = render()?;
    let skill_path = app.skill_path(root, home);
    atomic_write(&skill_path, content.as_bytes())?;
    if let Some(perm_path) = app.permission_path(root) {
        let mut perm_root =
            read_json(&perm_path)?.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        set_nested(
            &mut perm_root,
            &["permission", "skill", NAME],
            Value::String(String::from("allow")),
        )?;
        write_json(&perm_path, &perm_root)?;
    }
    Ok(Registered {
        app: app.name().into(),
        path: skill_path.display().to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// Deregister: remove `SKILL.md` (+ dir if empty) and the allow key (apps
/// that have one). `NotFound` if the skill file is absent. `root` is the XDG
/// root; `home` is the user home dir.
pub fn deregister(app: App, root: &Path, home: &Path) -> Result<Deregistered, PoetError> {
    let skill_path = app.skill_path(root, home);
    if !skill_path.exists() {
        return Err(PoetError::NotFound(format!("skill for app {}", app.name())));
    }
    std::fs::remove_file(&skill_path)
        .map_err(|e| PoetError::File(format!("cannot remove {}: {e}", skill_path.display())))?;
    if let Some(dir) = skill_path.parent() {
        let empty = std::fs::read_dir(dir)
            .map_err(|e| PoetError::File(format!("cannot read {}: {e}", dir.display())))?
            .next()
            .is_none();
        if empty {
            let _ = std::fs::remove_dir(dir);
        }
    }
    if let Some(perm_path) = app.permission_path(root)
        && let Some(mut perm_root) = read_json(&perm_path)?
    {
        if let Some(skill) = perm_root
            .get_mut("permission")
            .and_then(|p| p.get_mut("skill"))
            && let Some(obj) = skill.as_object_mut()
        {
            obj.remove(NAME);
        }
        let _ = write_json(&perm_path, &perm_root);
    }
    Ok(Deregistered {
        app: app.name().into(),
        path: skill_path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn render_is_deterministic() {
        let a = render().expect("render");
        let b = render().expect("render");
        assert_eq!(a, b, "re-render must be byte-equal (adr/0019)");
    }

    #[test]
    fn strip_deleted_handles_upgrade_artifact() {
        assert_eq!(
            strip_deleted("/home/u/.local/bin/poet"),
            "/home/u/.local/bin/poet"
        );
        assert_eq!(
            strip_deleted("/home/u/.local/bin/poet (deleted)"),
            "/home/u/.local/bin/poet"
        );
        assert_eq!(strip_deleted("/x (deleted) y"), "/x (deleted) y");
        assert_eq!(strip_deleted(""), "");
    }

    #[test]
    fn frontmatter_validates_and_body_carries_the_manual() {
        let s = render().expect("render");
        assert!(s.starts_with("---\n"), "missing opening fence:\n{s}");
        let close = s.find("\n---\n").expect("frontmatter close fence");
        let fm = &s[4..close];
        let name = fm
            .lines()
            .find(|l| l.starts_with("name:"))
            .expect("name line");
        assert_eq!(name.trim_start_matches("name:").trim(), NAME);
        assert!(name_is_valid(NAME), "name must match the regex");
        assert!(fm.contains("description:"), "missing description");
        assert!(s.contains("poet howto"), "body must reference howto");
        assert!(s.contains("## Workflow"), "body must carry the workflow");
        assert!(
            s.contains("Do not create files before the user approves"),
            "workflow must gate document creation on an agreed outline"
        );
        assert!(s.contains("## Command manual"), "manual section missing");
        assert!(
            s.contains("## Command Categories") || s.contains("### Error Codes"),
            "the howto must be inlined, not summarized"
        );
        assert!(
            s.contains("bookmark"),
            "body must carry the addressing discipline"
        );
    }

    #[test]
    fn manual_body_strips_the_leading_h1() {
        assert!(!manual_body().starts_with('#'), "H1 must be stripped");
        assert!(
            !manual_body().contains("# Poet - AI-First"),
            "H1 must be gone"
        );
    }

    #[test]
    fn name_regex_branches() {
        assert!(name_is_valid("poet"));
        assert!(name_is_valid("a"));
        assert!(name_is_valid("a-b-2"));
        assert!(!name_is_valid(""));
        assert!(!name_is_valid("-a"));
        assert!(!name_is_valid("a-"));
        assert!(!name_is_valid("a--b"));
        assert!(!name_is_valid("A"));
        assert!(!name_is_valid("a_b"));
    }

    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn anchors() -> (PathBuf, PathBuf) {
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("poet-skill-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let xdg = base.join("xdg");
        let home = base.join("home");
        std::fs::create_dir_all(&xdg).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        (xdg, home)
    }

    #[test]
    fn register_writes_skill_and_merges_permission() {
        let (d, home) = anchors();
        let Registered { app, path, version } =
            register(App::Opencode, &d, &home).expect("register");
        assert_eq!(app, "opencode");
        assert!(path.ends_with("opencode/skills/poet/SKILL.md"), "{path}");
        assert!(!version.is_empty());
        assert!(d.join("opencode/skills/poet/SKILL.md").exists());
        let perm: Value = serde_json::from_str(
            &std::fs::read_to_string(d.join("opencode/opencode.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(perm["permission"]["skill"]["poet"], "allow");
        // re-register is idempotent (no error, no duplicate)
        register(App::Opencode, &d, &home).expect("idempotent");
        let perm2: Value = serde_json::from_str(
            &std::fs::read_to_string(d.join("opencode/opencode.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(perm2["permission"]["skill"]["poet"], "allow");
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn register_claude_code_writes_skill_without_permission_store() {
        let (d, home) = anchors();
        let Registered { app, path, .. } = register(App::ClaudeCode, &d, &home).expect("register");
        assert_eq!(app, "claude-code");
        assert!(path.ends_with(".claude/skills/poet/SKILL.md"), "{path}");
        assert!(home.join(".claude/skills/poet/SKILL.md").exists());
        assert!(!d.join("opencode").exists() && !d.join("opencode.json").exists());
        deregister(App::ClaudeCode, &d, &home).expect("deregister");
        assert!(!home.join(".claude/skills/poet").exists());
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn register_preserves_other_permission_keys() {
        let (d, home) = anchors();
        let perm_path = d.join("opencode/opencode.json");
        std::fs::create_dir_all(perm_path.parent().unwrap()).unwrap();
        std::fs::write(
            &perm_path,
            br#"{"permission":{"skill":{"other-tool":"allow"},"bash":{"*":"allow"}},"theme":"dark"}"#,
        )
        .unwrap();
        register(App::Opencode, &d, &home).expect("register");
        let perm: Value =
            serde_json::from_str(&std::fs::read_to_string(&perm_path).unwrap()).unwrap();
        assert_eq!(perm["permission"]["skill"]["poet"], "allow");
        assert_eq!(perm["permission"]["skill"]["other-tool"], "allow");
        assert_eq!(perm["permission"]["bash"]["*"], "allow");
        assert_eq!(perm["theme"], "dark");
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn deregister_removes_skill_and_key_then_not_found() {
        let (d, home) = anchors();
        register(App::Opencode, &d, &home).expect("register");
        let Deregistered { app, path } = deregister(App::Opencode, &d, &home).expect("deregister");
        assert_eq!(app, "opencode");
        assert!(path.ends_with("opencode/skills/poet/SKILL.md"));
        assert!(!d.join("opencode/skills/poet/SKILL.md").exists());
        assert!(
            !d.join("opencode/skills/poet").exists(),
            "empty dir removed"
        );
        let perm: Value = serde_json::from_str(
            &std::fs::read_to_string(d.join("opencode/opencode.json")).unwrap(),
        )
        .unwrap();
        assert!(perm["permission"]["skill"].get("poet").is_none());
        let err = deregister(App::Opencode, &d, &home).unwrap_err();
        assert!(matches!(err, PoetError::NotFound(_)));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn app_parse_branches() {
        assert_eq!(App::parse("opencode").unwrap(), App::Opencode);
        assert_eq!(App::parse("claude-code").unwrap(), App::ClaudeCode);
        assert!(App::parse("agents").is_err());
        assert!(App::parse("nope").is_err());
    }

    #[test]
    fn skill_paths_anchor_per_app() {
        let root = Path::new("/r");
        let home = Path::new("/h");
        assert_eq!(
            App::Opencode.skill_path(root, home),
            PathBuf::from("/r/opencode/skills/poet/SKILL.md")
        );
        assert_eq!(
            App::ClaudeCode.skill_path(root, home),
            PathBuf::from("/h/.claude/skills/poet/SKILL.md")
        );
        assert_eq!(
            App::Opencode.permission_path(root),
            Some(PathBuf::from("/r/opencode/opencode.json"))
        );
        assert_eq!(App::ClaudeCode.permission_path(root), None);
    }

    #[test]
    fn json_outcome_helpers() {
        // register envelope outcome shape used by upgrade/uninstall payloads
        let ok = json!({"refreshed": true, "app": "opencode", "path": "/x"});
        assert_eq!(ok["refreshed"], json!(true));
    }
}
