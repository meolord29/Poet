//! Install-surface integration (adr/0019/0020/0021): the real binary spawned
//! with fully isolated anchors (`XDG_CONFIG_HOME`/`HOME`/`POET_HOME` all
//! pointed into a temp dir), so no test can touch the developer's real
//! `~/.config`, `~/.claude`, or `~/.poet`.

use std::process::{Command, Output};

struct Anchors(std::path::PathBuf);

impl Anchors {
    fn new() -> Self {
        Anchors(std::env::temp_dir().join(format!(
            "poet-install-it-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        )))
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_poet"))
            .args(args)
            .current_dir(&self.0)
            .env("XDG_CONFIG_HOME", self.0.join("xdg"))
            .env("HOME", self.0.join("home"))
            .env("POET_HOME", self.0.join("home"))
            .output()
            .expect("spawn poet binary")
    }

    fn json(&self, out: &Output) -> serde_json::Value {
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("stdout must be one JSON document")
    }
}

impl Drop for Anchors {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn register_then_uninstall_round_trip() {
    let a = Anchors::new();
    std::fs::create_dir_all(&a.0).expect("anchor dir");
    let out = a.run(&["register", "--app", "opencode"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let env = a.json(&out);
    assert_eq!(env["status"], "ok");
    assert_eq!(env["message"], "skill registered");
    assert_eq!(env["data"]["app"], "opencode");
    assert_eq!(env["data"]["installed"], true);
    assert_eq!(env["data"]["version"], env!("CARGO_PKG_VERSION"));
    let skill = a.0.join("xdg/opencode/skills/poet/SKILL.md");
    assert!(skill.exists(), "SKILL.md must exist");
    let perm: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(a.0.join("xdg/opencode/opencode.json")).expect("perm file"),
    )
    .expect("perm json");
    assert_eq!(perm["permission"]["skill"]["poet"], "allow");

    // fake installed binary, then the full reversal
    let bin_dir = a.0.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir");
    std::fs::write(bin_dir.join("poet"), b"fake").expect("fake binary");
    let dir_arg = bin_dir.to_str().expect("utf8 path");
    let out = a.run(&["uninstall", "--bin-dir", dir_arg]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let env = a.json(&out);
    assert_eq!(env["status"], "ok");
    assert_eq!(env["message"], "uninstalled");
    assert_eq!(env["data"]["bin"], json_path(&bin_dir.join("poet")));
    assert_eq!(env["data"]["skill"][0]["removed"], true);
    assert_eq!(env["data"]["skill"][0]["app"], "opencode");
    assert!(!skill.exists(), "skill must be gone");
    assert!(!bin_dir.join("poet").exists(), "binary must be gone");
    let perm: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(a.0.join("xdg/opencode/opencode.json")).expect("perm file"),
    )
    .expect("perm json");
    assert!(perm["permission"]["skill"].get("poet").is_none());
}

#[test]
fn print_skill_emits_bytes_and_touches_nothing() {
    let a = Anchors::new();
    std::fs::create_dir_all(&a.0).expect("anchor dir");
    let out = a.run(&["register", "--app", "opencode", "--print-skill"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let env = a.json(&out);
    assert_eq!(env["status"], "ok");
    let skill_text = env["data"]["skill"].as_str().expect("skill bytes");
    assert!(skill_text.starts_with("---\nname: poet\n"), "{skill_text}");
    assert!(
        !a.0.join("xdg/opencode/skills").exists(),
        "no filesystem effect"
    );
}

#[test]
fn register_rejects_unknown_app() {
    let a = Anchors::new();
    std::fs::create_dir_all(&a.0).expect("anchor dir");
    let out = a.run(&["register", "--app", "agents"]);
    assert_eq!(out.status.code(), Some(1));
    let env = a.json(&out);
    assert_eq!(env["status"], "error");
    assert_eq!(env["code"], "validation_error");
}

#[test]
fn upgrade_rejects_unknown_channel() {
    let a = Anchors::new();
    std::fs::create_dir_all(&a.0).expect("anchor dir");
    let out = a.run(&["upgrade", "--channel", "edge"]);
    assert_eq!(out.status.code(), Some(1));
    let env = a.json(&out);
    assert_eq!(env["status"], "error");
    assert_eq!(env["code"], "validation_error");
}

#[test]
fn upgrade_release_mode_fails_cleanly_when_unreachable() {
    // Unsupported platforms never reach the download; where an asset exists,
    // an unreachable/empty base must fail as `file_error` — never panic, and
    // never touch the real anchors. (Probe of the POET_DOWNLOAD_BASE mirror
    // hook at process level: env is set on the child only, no unsafe/races.)
    let supported = matches!(
        (std::env::consts::OS, std::env::consts::ARCH),
        ("linux", "x86_64") | ("macos", "aarch64")
    );
    if !supported {
        return;
    }
    let empty = Anchors::new();
    std::fs::create_dir_all(empty.0.join("empty-release")).expect("empty base");
    let base = format!("file://{}", empty.0.join("empty-release").display());
    let out = Command::new(env!("CARGO_BIN_EXE_poet"))
        .args(["upgrade", "--no-skill"])
        .current_dir(&empty.0)
        .env("XDG_CONFIG_HOME", empty.0.join("xdg"))
        .env("HOME", empty.0.join("home"))
        .env("POET_HOME", empty.0.join("home"))
        .env("POET_DOWNLOAD_BASE", &base)
        .output()
        .expect("spawn poet binary");
    let env = empty.json(&out);
    assert_eq!(out.status.code(), Some(1), "{env}");
    assert_eq!(env["status"], "error");
    assert_eq!(env["code"], "file_error");
}

#[test]
fn uninstall_reports_not_found_when_nothing_installed() {
    let a = Anchors::new();
    std::fs::create_dir_all(&a.0).expect("anchor dir");
    let bin_dir = a.0.join("empty-bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir");
    let dir_arg = bin_dir.to_str().expect("utf8 path");
    let out = a.run(&["uninstall", "--bin-dir", dir_arg]);
    assert_eq!(out.status.code(), Some(1));
    let env = a.json(&out);
    assert_eq!(env["status"], "error");
    assert_eq!(env["code"], "not_found");
}

/// The payload echoes absolute paths; compare as strings portably.
fn json_path(p: &std::path::Path) -> serde_json::Value {
    serde_json::Value::String(p.display().to_string())
}
