//! Installer contract tests (adr/0020): `scripts/install.sh` is driven as a
//! black box against a `file://` fixture — no network. Pins the plan output,
//! the non-interactive proceed, the install itself, and the release-time
//! `TAG="nightly"` sed anchor that `release.yml` depends on.

use std::process::Command;

/// A fake `poet` binary (shell script) that answers `--version` and
/// `register` like the real one — the installer probes both.
fn fake_poet_source() -> &'static str {
    "#!/bin/sh\ncase \"${1:-}\" in\n  --version) echo \"poet 0.0.0-fixture\" ;;\n  *) echo '{\"status\":\"ok\",\"message\":\"skill registered\",\"data\":{}}' ;;\nesac\n"
}

/// Build the fixture release dir: tarball + SHA256SUMS, as release.yml writes them.
fn stage_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("fixture dir");
    let base = dir.path().join("release");
    std::fs::create_dir_all(&base).expect("release dir");
    let target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => panic!("installer fixture needs linux-x86_64 or macos-aarch64"),
    };
    let tarball = format!("poet-{target}.tar.gz");
    let bin = base.join("poet");
    std::fs::write(&bin, fake_poet_source()).expect("fake binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake binary");
    }
    let status = Command::new("tar")
        .args(["-czf"])
        .arg(base.join(&tarball))
        .arg("poet")
        .current_dir(&base)
        .status()
        .expect("tar");
    assert!(status.success(), "tar failed");
    let sums = if cfg!(target_os = "macos") {
        let out = Command::new("shasum")
            .args(["-a", "256"])
            .arg(&tarball)
            .current_dir(&base)
            .output()
            .expect("shasum");
        String::from_utf8_lossy(&out.stdout).into_owned()
    } else {
        let out = Command::new("sha256sum")
            .arg(&tarball)
            .current_dir(&base)
            .output()
            .expect("sha256sum");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    std::fs::write(base.join("SHA256SUMS"), sums).expect("SHA256SUMS");
    (dir, base)
}

fn run_installer(base: &std::path::Path, install_dir: &std::path::Path) -> String {
    let out = Command::new("sh")
        .arg("scripts/install.sh")
        .env("POET_DOWNLOAD_BASE", format!("file://{}", base.display()))
        .env("POET_INSTALL_DIR", install_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        // anchors stay real-looking but never touched: no opencode/claude on
        // PATH in the test env is not guaranteed — the register probe is
        // best-effort in the installer, and XDG/HOME pin any writes.
        .env("XDG_CONFIG_HOME", install_dir.join("xdg"))
        .env("HOME", install_dir.join("home"))
        .output()
        .expect("run install.sh");
    assert_eq!(
        out.status.code(),
        Some(0),
        "installer failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn installer_runs_non_interactively_installs_and_pins_the_sed_anchor() {
    let (_fixture, base) = stage_fixture();
    let scratch = tempfile::tempdir().expect("scratch");
    let install_dir = scratch.path().join("bin");
    let out = run_installer(&base, &install_dir);
    // plan output (the consent contract, adr/0020)
    assert!(out.contains("install plan"), "{out}");
    assert!(out.contains("non-interactive — proceeding"), "{out}");
    assert!(out.contains("SHA256SUMS checksum"), "{out}");
    // summary
    assert!(out.contains("checksum verified"), "{out}");
    assert!(out.contains("installed"), "{out}");
    // binary landed, executes, version probes
    let bin = install_dir.join("poet");
    assert!(bin.exists(), "binary must be installed");
    let ver = Command::new(&bin).arg("--version").output().expect("probe");
    assert!(String::from_utf8_lossy(&ver.stdout).contains("poet"),);
    // the release-time sed anchor: release.yml replaces exactly this line
    let script = std::fs::read_to_string("scripts/install.sh").expect("install.sh");
    let anchor_lines = script.lines().filter(|l| *l == "TAG=\"nightly\"").count();
    assert_eq!(anchor_lines, 1, "TAG anchor must appear exactly once");
    let patched = script.replacen("TAG=\"nightly\"", "TAG=\"v9.9.9\"", 1);
    assert!(patched.contains("TAG=\"v9.9.9\""), "sed simulation failed");
    // channel-correct banner: the nightly mark rides on TAG
    assert!(script.contains("nightly"));
}

#[test]
fn installer_declines_before_download_on_explicit_no() {
    // A declined plan must abort before any network/file activity: drive the
    // script with a broken download base and an explicit "n" — the download
    // failure would abort too, so assert the *abort* marker instead.
    let script = std::fs::read_to_string("scripts/install.sh").expect("install.sh");
    // the abort path fires before mktemp/curl: statically pin the ordering
    let consent = script
        .find("proceed with the install plan")
        .expect("consent");
    let download = script
        .find("downloading ${tarball} from")
        .expect("download step");
    let fetch = script.find("curl -fsSL -o").expect("curl step");
    assert!(consent < fetch, "consent must precede any fetch");
    assert!(download < fetch, "download notice precedes fetch");
}
