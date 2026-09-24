**example:**

```sh
poet upgrade
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "upgraded",
  "data": {
    "upgraded": true,
    "version": "0.1.0",
    "bin": "/home/u/.local/bin/poet",
    "source": "https://github.com/meolord29/Poet/releases/latest/download/poet-x86_64-unknown-linux-musl.tar.gz",
    "skill": [
      {
        "refreshed": true,
        "app": "opencode",
        "path": "/home/u/.config/opencode/skills/poet/SKILL.md"
      }
    ]
  }
}
```

Downloads the published release tarball for this platform (stable channel via GitHub Latest by default; `--channel nightly` follows the rolling prerelease), verifies it against `SHA256SUMS`, probes it, and atomically replaces the binary at `--bin-dir` (default `~/.local/bin`). Every already-registered app's skill is refreshed best-effort — nothing is newly registered; `--no-skill` skips the refresh (`"skill": null`).
