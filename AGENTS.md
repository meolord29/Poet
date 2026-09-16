# AGENTS.md — Poet engineering constitution

Poet is an AI-first Word document (.docx) automation CLI. It is a Rust rebuild of
`/home/mashkini/Workspace/Words` (Python). **Words is the behavioral spec**: when this
document and the Words code disagree on *behavior*, Words wins unless an ADR records the
deviation. Design sense and git topology are inherited from
`/home/mashkini/Workspace/carpenter` (read its `AGENTS.md` for the original rationale).

## Non-negotiables

1. **One signature everywhere.** Command functions in `src/commands/` are
   `pub fn <action>(ctx: &Ctx, args: <Args>) -> Result<Data, PoetError>` — nothing else.
   Helpers live in `src/core/`, never in `src/commands/`.
2. **One envelope out.** Every invocation prints exactly one JSON document to stdout:
   - success: `{"status":"ok","message":"...","data":{...}}` — exit 0
   - failure: `{"status":"error","message":"...","code":"...","details":{}}` — exit 1
   The contract lives in `src/core/output.rs` and nowhere else. Pretty-printed (2-space
   indent), non-ASCII preserved. `run()` never panics.
3. **Errors map 1:1 to codes.** One `thiserror` enum (`src/core/error.rs`); every variant
   has a `code()` string; a unit test pins the variant↔code mapping. Error codes are
   ported from Words (`src/words/core/exceptions.py`).
4. **No `unwrap`/`expect` outside tests.** No panicking paths in command or core code.
5. **Docs are mandatory.** `#![deny(missing_docs)]` in `src/lib.rs`. Every file opens with
   a `//!` module doc (purpose + rationale + ADR pointer when non-obvious). Plain `//`
   comments are discouraged — code is self-documenting.
6. **No globals.** State flows through an explicit `Ctx` struct built once in `app::run`
   and passed by reference. No `OnceLock`, no `static mut`, no builder patterns.
7. **Parity-first.** Command names, flags, JSON `data` shapes, and exit semantics are
   ported from Words. Port the observable behavior, not the Python implementation.
8. **Record decisions.** Every non-obvious decision gets a numbered ADR
   (`docs/adr/0NN-slug.md`) and is cited from code. "If a human must keep two things in
   sync, that's a bug" — automate verifiable invariants as tests.

## The ponytail ladder

Before writing code, stop at the first rung that holds: (1) does it need to exist?
(2) already in the codebase? (3) stdlib/crate? (4) one line? (5) the minimum that works.
Lazy about the solution, never about reading.

## Layout conventions

- Top-level modules are single files (`app.rs`, `howto.rs`); subdirectories use `mod.rs`
  (`src/commands/mod.rs`, `src/core/mod.rs`, `src/models/mod.rs`).
- `src/app.rs` is wiring only — argument parsing, dispatch, emit, autosave, session
  auto-open. No business logic.
- `src/models/data.rs` holds `#[serde(untagged)] enum Data` — one variant per command's
  success payload, each with `///` field docs.
- Tests: inline `#[cfg(test)] mod tests` at the bottom of each module, named
  `<fn>_<expected_behavior>`; `tests/` only for whole-process integration. Shared test
  fixture: `src/commands/testutil.rs` (`setup()` → temp-dir `Ctx`).
- A session dir override (env var) must exist so tests never touch `~/.poet`.

## Git topology (carpenter model)

- `nightly` is the integration branch — all work lands here.
- `main` is stable and only ever accepts merges from `nightly` (CI guard enforces).
- Phase work: `ivan/phase-N-<slug>` cut from `nightly`, merged back per phase.
- Conventional Commits with scopes and ADR citations, e.g.
  `feat(phase2): table set-range with header inference (adr/005)`.
- PLAN.md's phase tracker is updated in the same merge that completes a phase.

## Build & verify

```bash
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
cargo doc # must be warning-free
```
