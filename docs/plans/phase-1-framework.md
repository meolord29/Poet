# Phase 1 — Initial Framework

You are executing phase 1 of the Poet rebuild. This brief is self-contained: it restates
every global decision you need. Read `AGENTS.md` first — it is the constitution; this
brief is the scope.

## Global context (restated)

- **Poet** is an AI-first Word document (.docx) automation CLI, rebuilt in Rust from the
  Python original **Words** at `/home/mashkini/Workspace/Words` (read-only reference).
  Words is the behavioral spec: port observable behavior, not implementation.
- Binary name: `poet`. Session state: `~/.poet/session.json` — but an env override must
  exist so tests never touch the real path.
- Engine: `docx-rs` 0.4.x (`read_docx` parses into the same model the writer uses;
  children are public `Vec`s). No backward compatibility with python-docx internals.
- Output contract (identical to Words): every invocation prints exactly one pretty-printed
  JSON document to stdout — success `{"status":"ok","message":"...","data":{...}}` (exit 0),
  failure `{"status":"error","message":"...","code":"...","details":{}}` (exit 1).
- Conventions, signature rule, layout: see `AGENTS.md`. Branch: `ivan/phase-1-framework`
  cut from `nightly`; merge back when done.

**Port error codes and envelope details from the Python source — do not invent them:**
- `/home/mashkini/Workspace/Words/src/words/core/exceptions.py` (error semantics)
- `/home/mashkini/Workspace/Words/src/words/core/output.py` (envelope)
- `/home/mashkini/Workspace/Words/src/words/core/session.py` (session shape/behavior)
- `/home/mashkini/Workspace/Words/src/words/app.py` (`_emit`, `_AUTOSAVE`, `get_context`,
  sub-app/command inventory — the full list of CLI commands)
- `/home/mashkini/Workspace/Words/src/words/core/document_manager.py` (lifecycle: create/
  open/save/close, `get_info` shape — only the file-lifecycle parts in this phase)
- `/home/mashkini/Workspace/Words/src/words/core/bookmark.py` (addressing model)

## Deliverables

### 1. Scaffold
- `Cargo.toml`: package `poet`, `[[bin]] name = "poet"`. Deps: `docx-rs = "0.4"`,
  `clap = { version = "4", features = ["derive"] }`, `serde = { version = "1", features
  = ["derive"] }`, `serde_json = "1"`, `thiserror = "2"`, `chrono = "0.4"`.
  Dev-deps: `tempfile`. Do **not** add polars/rhai (phase 4).
- `rust-toolchain.toml`: stable with `clippy`, `rustfmt` components.
- `src/main.rs`: minimal — call `poet::app::run()`, return `ExitCode`.
- `src/lib.rs`: `#![deny(missing_docs)]`, public modules.
- `.gitignore`: `/target`.

### 2. Core (`src/core/`)
- `error.rs`: single `thiserror` enum `PoetError`; variants cover session, validation,
  metadata, calculation, file (not-found, unreadable, unsupported-format), not-implemented
  (phase-1 stubs), document-state, and internal errors. Every variant has `code()`; a
  unit test pins the full variant↔code mapping. Mirror Words' error semantics.
- `output.rs`: the envelope. `pub fn render(result: Result<Data, PoetError>) -> (String,
  bool)` → (pretty JSON, exit-1 flag). No other file may format envelopes. Never panics.
- `session.rs`: `Session { path, format, active_section }` (serde, matching Words'
  shape), `SessionRepository` get/save/delete against a session file path taken from the
  `Ctx` (resolved from `POET_HOME`/`POET_SESSION` env or `~/.poet/session.json`).
- `document.rs`: `DocumentManager` over `docx_rs::Docx` — file lifecycle only:
  `create` (new blank docx), `open` (read_docx), `save`/`save_as` (pack to path),
  `close`, `info` (port Words `get_info` JSON shape), plus `current_path` tracking.
- `bookmark.rs`: `BookmarkManager` implementing the Words addressing model over the
  docx-rs document model: sibling-wrapping `bookmarkStart`/`bookmarkEnd` around a target
  block element, unique names, id allocation (`{prefix}{max+1}` with prefixes
  `p`,`h`,`l`,`t`,`img`,`toc`), `find_element(name)`, `name_around(element)`,
  `ensure_bookmark`, `rename`, `remove_around`, `list_bookmarks`, `next_id` (max
  bookmark id in document + 1), duplicate-name rejection. If docx-rs's API cannot place
  block-level bookmarks cleanly, choose the minimal sound mechanism (e.g. paragraph-
  embedded bookmarks or an SDT wrapper) and **write `docs/adr/0004-bookmark-addressing.md`**
  recording the choice and the fallback.
- `mod.rs`, plus `Ctx` (or `Context`) struct: holds `SessionRepository`, optional open
  `DocumentManager`, resolved paths. Built once in `app::run`, passed by `&Ctx` to all
  command fns.

### 3. App wiring (`src/app.rs`)
- `pub fn run<I: IntoIterator<Item = String>>(args: I) -> ExitCode`: parse (clap derive),
  build `Ctx` (auto-open the session document like Words' `get_context`), dispatch, wrap
  `Result<Data, PoetError>` → `output::render`, print, autosave.
- Autosave: after a successful mutating command whose category is in
  `{section, paragraph, run, style, heading, list, table, image, toc, page, meta}`,
  save the open document to its current path (port `_AUTOSAVE`). `document` manages its
  own saves; `batch`/`calc` never autosave.
- Exit code 1 on error; never panic.
- Testability: dispatch must be reachable in-process (tests call `run([...])` or an
  internal `execute` and assert on the envelope — no spawned processes in unit tests).
- CLI tree: root `poet` with `--version` and 14 category subcommands — `document`,
  `section`, `paragraph`, `run`, `style`, `heading`, `list`, `table`, `image`, `toc`,
  `page`, `meta`, `batch`, `calc` — each with **every action from Words' app.py defined
  now**, including flags (derive enums; placeholder args structs fine). All actions
  return `PoetError::NotImplemented` except `document` (below). Invoking `poet` with no
  subcommand prints the howto text (phase 1: a short placeholder pointing to phase 4).
  `poet howto` exists as a command too.

### 4. Commands & models
- `src/commands/`: one module per category; only command fns (constitution §1). Phase 1
  implements `document`: `new`, `open`, `save`, `save-as`, `close`, `info` — port JSON
  `data` shapes from Words' `commands/document.py` (echo assigned id/path, counts, etc.).
  Every other action is a stub.
- `src/models/data.rs`: `#[serde(untagged)] enum Data` with variants for the document
  payloads (+ a unit placeholder variant if needed for stubs). `///` docs per field.
- `src/commands/testutil.rs`: `setup()` → `Ctx` with a unique temp dir per test
  (AtomicUsize + pid pattern), env override set.

### 5. CI (`.github/workflows/ci.yml`)
- Jobs: `fmt` (`cargo fmt --check`), `clippy` (`--all-targets -- -D warnings`),
  `test` (`cargo test`), `doc` (`RUSTDOCFLAGS="-D warnings" cargo doc`), and `guard`:
  any PR targeting `main` whose head branch is not `nightly` fails with a clear message.

## Verification (all must pass before merge)

1. `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check &&
   RUSTDOCFLAGS="-D warnings" cargo doc` — all green, no warnings.
2. Unit tests: envelope shapes + exit flags; error↔code mapping exhaustive; session
   get/save/delete round-trip in temp dir (never touching `~/.poet`); bookmark id
   allocation (max+1, per-prefix counters, duplicate rejection); document lifecycle.
3. Integration: `poet document new <tmp>/a.docx` → session written → `poet document save`
   → process-style reopen via `open` → `poet document info` shows the expected JSON
   (port Words' shape) → `close` deletes the session. Also: autosave path exercised by
   at least one stubbed mutating category (stub may still return NotImplemented — assert
   the error envelope + exit 1 instead).
4. Demo transcript in the merge commit message body (commands + JSON outputs).

## Done

- Update `PLAN.md` tracker (phase 1 → done, adr index if you added ADRs).
- Commit(s) on `ivan/phase-1-framework`, Conventional Commits with ADR cites, then merge
  to `nightly` (no-ff). Suggested final message: `feat(phase1): initial framework`.
