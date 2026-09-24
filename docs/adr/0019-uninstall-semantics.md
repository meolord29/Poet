# ADR-0019: `uninstall` semantics

Date: 2026-09-24 · Status: Accepted

## Context

`register`/`upgrade` had no inverse: removing Poet meant manually deleting
`~/.local/bin/poet` plus every registered app's `SKILL.md` (and the opencode
permission key). Supported platforms are Linux and macOS only, which makes
self-deletion tractable.

## Decision

`poet uninstall [--bin-dir <p>]`:

- **Resolution:** `--bin-dir` → default `~/.local/bin` — identical to
  `upgrade` (one resolution path, no drift).
- **Ordering:** skill first, binary last. The skill is recoverable via
  `poet register`; the binary delete is the point of no return, so it goes
  last.
- **Skill removal is best-effort** (mirrors `upgrade`'s outcome): per-app
  `{removed:true,app,path}` or `{removed:false,reason:"not_registered"}`;
  it never fails the run.
- **Self-delete:** plain `remove_file` on the running binary — safe on
  Linux/macOS, where unlink keeps the inode alive for the running process
  (the same property `upgrade`'s copy-replace and `strip_deleted` rely on).
- **Nothing found** (no skill file and no binary) → `not_found` error
  envelope — mirrors `deregister`'s contract on an absent skill.
- **Session data is never touched:** `~/.poet` holds user documents' session
  state, not installation state.

## Consequences

+ One command reverses the full `install.sh`/`register` footprint; the
  envelope reports exactly what was (`bin: null` when no binary existed) and
  wasn't removed.

− Unlike carpenter there is no `--purge-config`: Poet has no config feature
  yet (`~/.poet/config.json` is a planned placeholder). Add the flag with the
  config feature, not before.

## Rejected

- **Purging `~/.poet` by default** — documents' session state is user data,
  not installation state.
