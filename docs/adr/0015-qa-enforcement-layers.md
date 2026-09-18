# ADR 0015 — QA enforcement layers

Date: 2026-09-18 · Status: accepted

## Decision

Poet adopts the five-layer QA enforcement stack of
`/home/mashkini/Workspace/carpenter` (its `build.rs`, `src/core/dev.rs`,
`src/models/examples.rs`, `.opencode/agents/carpenter-dev-validate.md`, ADRs
007/008/013/016/021). Each layer uses the cheapest mechanism that fails
fastest; together they make an undocumented, untested, or under-exercised
command impossible to ship:

1. **Compile-time gates** (`build.rs`): the command set is derived by
   signature-scanning `src/commands/` (`pub fn` returning `Result`), so adding
   a command auto-enrolls it in every gate — no registry to forget. Strict
   builds (no `dev` feature) require, per command: a worked-example atom
   `docs/examples/<category>/<action>.md`, a paired in-module
   `#[test] fn <action>_…`, an invocation in at least one `tests/*.rs`
   integration test, and scenario coverage (`examples/*.md`, each invoking
   ≥3 distinct command fns). All violations accumulate, then the build fails.
2. **Contract tests**: every `Data` variant has a co-located example in
   `src/models/examples.rs` (an exhaustive `match` — a new variant without an
   example refuses to compile) and a smoke test renders each through the
   envelope, pinning the exact schema. Error envelopes, error codes, and
   `AUTOSAVE_CATEGORIES` are pinned by exhaustive-match tests; the real binary
   is spawned for exit-code/stdout contract tests; a hostile-argv battery pins
   `run()`'s never-panic guarantee.
3. **CI**: fmt/clippy/test/doc plus `--features dev` compile-and-test guards so
   the dev feature cannot rot; the build gates ride every job.
4. **Black-box QA agent** (`.opencode/agents/poet-dev-validate.md`): an
   LLM-driven fault-hunter, run manually per PR. It drives the CLI with a real
   document corpus inside `.sandbox/`, treats `poet howto` + `--help` as the
   contract (`observed ≠ documented` = failure), is denied all source access by
   its permission block, hunts error paths / edge cases / chaining /
   idempotency / doc drift / dev-vs-release leaks / panics, *prescribes*
   missing examples (never authors them), and emits a tabular report whose
   "failure tally must show zero bugs" before merge.
5. **Dev feature** (`--features dev`): `poet dev setup|clean` (idempotent
   `.sandbox/` lifecycle — the agent's only filesystem handle), the
   `--capture-example` authoring loop (captures real envelopes for example
   atoms), and `--dev-home` for session isolation. Under `dev`, gates relax so
   a command can be run and captured before its atom/test exist; `dev` +
   release profile is rejected — a relaxed binary never ships.

## Rationale for the key choices

- **Gates are signature-derived, not listed.** "If a human must keep two
  things in sync, that's a bug" — a hand-written gate list would drift from the
  command surface; the syn scan cannot.
- **The example atom is a checked-in file** (not an inline doc fence) because
  it is the QA corpus: captured real envelopes double as regression
  documentation for agents and humans, and the build gate can verify existence
  mechanically.
- **The QA gate is manual.** Automating the agent in CI was rejected (as in
  carpenter adr/021): it needs an LLM key, is slow and non-deterministic, and
  is interactive by design — a CI rerun would test something other than what
  the report requires. Enforcement is the owner's review plus the mandatory
  report section in the PR template.
- **Byte-level goldens were rejected.** Poet's payloads embed ids and paths;
  instead of brittle goldens, the smoke tests pin schema exactly and round-trip
  values, and the QA agent supplies the behavioral "does the whole flow work"
  tier (carpenter's adr/019 stance).

## Poet-specific deviations from carpenter

- **`--dev-home <DIR>` (dev builds only).** Carpenter isolates state with a
  release `--root` flag; Poet's session lives at `POET_HOME`/`$HOME/.poet`
  (Words parity). Without a dev-only home override, agent runs would leak
  `session.json` into the real `$HOME/.poet`, and after teardown the user's
  next run would silently auto-open a dead session. The flag does not exist in
  release builds (unknown flag), which the agent probes as a surface-parity
  check — zero Words-parity risk.
- **Integration-coverage gate.** Beyond carpenter's example + unit-test +
  scenario gates, strict builds require each command to be invoked in at least
  one `tests/*.rs` file — the executable path, not just prose, must cover every
  command.
- **No per-command generated spec tables.** Carpenter generates spec docs from
  types (its adr/008); Poet's doc surface is `poet howto` (Words parity), so
  only the example-atom half of that design is ported.

## Consequences

- Strict `cargo build`/`cargo test` fails until every command has its atom,
  unit test, integration coverage, and scenario floor — the corpus and the
  gates land in the same merge (phase 5).
- The PR template requires a `poet-dev-validate` report for surface-touching
  PRs; the agent file must be updated in the same PR that changes the CLI
  surface, so the runbook and the contract cannot drift.
- `syn` joins `[build-dependencies]`; the `dev` feature is never enabled in
  release artifacts (enforced by `build.rs`, probed by the agent).
