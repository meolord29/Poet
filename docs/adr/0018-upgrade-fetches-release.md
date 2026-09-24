# ADR-0018: `upgrade` fetches the published release

Date: 2026-09-24 · Status: Accepted

## Context

The installer (plan_v3 PR3) installs a tarball binary; without an in-CLI
upgrade path, binary-installed users would have "re-run install.sh" as their
only update story — and that never refreshes the registered skill. Fetching a
release does not reintroduce git: it fetches **tarballs over HTTPS with
checksum verification**, not a clone.

## Decision

`poet upgrade [--channel stable|nightly] [--bin-dir <p>] [--no-skill]` —
release mode only (carpenter adr/018 as updated; no `--source` rebuild mode
because Poet has no generated surfaces to regenerate — the dev loop is plain
cargo):

- **Pipeline** (`core/release.rs`): map the running platform to an asset
  (`x86_64-unknown-linux-musl`, `aarch64-apple-darwin`; anything else →
  `validation_error`) → `curl` tarball + `SHA256SUMS` → verify via
  `sha256sum`/`shasum -a 256` → `tar -xzf` → probe the new binary
  (`--version` — it must execute before anything is replaced) → atomic
  tmp+rename into the bin dir (default `~/.local/bin`). Subprocess tools
  (`curl`, `tar`, checksum), no HTTP/sha2 crates — the pipeline mirrors
  `scripts/install.sh` line for line, so the two paths cannot drift.
  `POET_DOWNLOAD_BASE` overrides the URL (tests/mirrors).
- **Channels** (adr/0002 ladder): default `stable` follows GitHub **Latest**
  (`releases/latest/download`, published by the nightly → main promotion);
  `--channel nightly` follows the rolling prerelease.
- **Skill:** refresh only **registered** apps (skill file present), never
  register a new one — parity with the installer, which registers only what
  the user confirmed. Per-app outcomes in `data.skill`; `--no-skill` yields
  `skill: null`; nothing registered → a single `{reason:"not_registered"}`
  object carrying the stable warning.

## Consequences

+ Binary-installed users self-update; the skill ships the new embedded howto
  with it.
+ A failed download/verify/probe aborts **before** the binary is touched; a
  failed skill write never rolls back a successful binary swap.

− `upgrade` needs `curl` + `tar` + a checksum tool on PATH (Linux/macOS ship
  them; a missing tool is a clear `file_error`).
− The running process replaces its own image: on Linux the kernel marks
  `/proc/self/exe` `" (deleted)"` — the skill renderer strips the suffix
  (adr/0017).

## Rejected

- **In-process HTTP (`ureq` + `sha2`)** — two new supply-chain deps to avoid
  subprocess tools every target platform already ships; also risks drifting
  from `install.sh`.
- **`--source` rebuild mode** — carpenter needed it for generated surfaces;
  Poet's build is single-stage cargo. Source users rebuild manually.
