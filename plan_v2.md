# Poet QA System — Implementation Plan (v2)

Port of carpenter's five-layer QA enforcement stack to Poet (`.docx` CLI), so the
CLI is validated by construction: compile-time gates, contract tests, CI guards,
and a manual black-box QA agent whose report gates PRs.

- **Reference implementation:** `/home/mashkini/Workspace/carpenter`
  - `build.rs` (gates), `src/core/dev.rs` (dev feature + capture), `src/models/examples.rs` (data examples + envelope smoke)
  - `.opencode/agents/carpenter-dev-validate.md` (the QA agent runbook)
  - `.github/PULL_REQUEST_TEMPLATE.md`; ADRs 007 (example gate), 008 (examples from types), 013 (scenario gate), 016 (dev feature), 021 (manual QA gate)
- **Design record:** `docs/adr/0015-qa-enforcement-layers.md` (lands in Phase 1; every artifact cites it)
- **Key Poet adaptations:** dev-only `--dev-home` flag (session isolation — Poet's session lives at `POET_HOME`/`$HOME/.poet`, carpenter has `--root`); integration-coverage gate C (Poet addition); scenarios at top-level `examples/` (carpenter layout; `docs/examples/quarterly-report/` migrates).

## Progress tracker

Status legend: ☐ pending · ◐ in progress · ☑ done

| # | Phase | Deliverable | Status |
|---|-------|-------------|--------|
| 1 | ADR 0015 + process docs | adr, PR template, AGENTS.md section, PLAN.md index | ☑ |
| 2 | `dev` feature + authoring loop | `dev setup/clean`, `--capture-example`, `--dev-home`, gates-relaxed builds | ☑ |
| 3 | Contract tests | data examples + envelope smoke, error pins, autosave pin, `tests/bin.rs`, panic battery | ☑ |
| 4 | Example corpus | 76 atoms `docs/examples/<category>/<action>.md` + 2 scenarios in `examples/` | ☑ |
| 5 | `build.rs` gates A–E | signature-derived gates, accumulate-then-fail, dev/release split | ☑ |
| 6 | QA agent | `.opencode/agents/poet-dev-validate.md` | ☑ |
| 7 | CI + wiring | dev-feature CI guards, PR-template wiring, docs sync | ☑ |

Verification at every phase: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo doc` (warning-free).

---

## Phase 1 — ADR + process docs

- [ ] `docs/adr/0015-qa-enforcement-layers.md`: five layers; why gates are signature-derived (auto-enrollment); why the example atom is a checked-in file; why the QA agent stays manual (carpenter adr/021 rationale); Poet adaptations.
- [ ] `.github/PULL_REQUEST_TEMPLATE.md`: checklist + mandatory `poet-dev-validate` report section for surface-touching PRs (no version ladder — Poet has no release bot).
- [ ] `AGENTS.md`: QA section + PR ground rules.
- [ ] `PLAN.md`: ADR index — add 0015, fix missing 0014 row.

## Phase 2 — `dev` feature + authoring loop

- [x] `Cargo.toml`: `[features] dev = []`.
- [x] `build.rs`: reject `dev` + `PROFILE=release`; under `dev`, gates skip + `#![deny(missing_docs)]` relaxed via `cfg_attr` (verified: release+dev exits 101).
- [x] `src/core/dev.rs` (mechanics) + `src/commands/dev.rs` (signature wrappers): `dev setup` → `Data::DevSetup {path, home, created}`, `dev clean` → `Data::DevClean {removed, path}`, both idempotent.
- [x] `Data::DevSetup` / `Data::DevClean` variants (cfg-gated) in `src/models/data.rs`; `Category::Dev` **not** in `AUTOSAVE_CATEGORIES`.
- [x] `--capture-example <PATH>` global flag (dev only): atom = `**example:**` + invocation (both dev flags stripped, `=` and space forms) + real envelope + TODO note; best-effort write; verified e2e incl. error envelopes.
- [x] `--dev-home <DIR>` global flag (dev only): `ctx_from_argv` redirects the session repo before auto-open; verified e2e (`session.json` inside `.sandbox/.poet`); release rejects all three (`dev`, `--dev-home`, `--capture-example`) with exit 2 and no `--help` leak.
- [x] Tests: idempotency (core, temp-root), surface pins (`app.rs` tests: dev present in dev build / absent in release), `howto_never_documents_dev_surface`. `POET_HOME`/bin-spawn coverage lands in `tests/bin.rs` (Phase 3).

## Phase 3 — Contract tests

- [x] `src/models/examples.rs`: `envelope_examples()` — one canonical example per `Data` variant (incl. cfg-gated dev variants), keyed by command; smoke test pins the exact envelope schema (key order, message mirror, trailing newline) + `data` round-trip; variant coverage enforced by the phase-5 build gate.
- [x] Error envelope pin: all 11 `PoetError` variants → exact keys `{status, message, code, details}`, `details == {}`, code stable.
- [x] `AUTOSAVE_CATEGORIES` pin (`src/commands/mod.rs` tests): exhaustive match on `Category` (new variant = compile error) vs the const.
- [x] `tests/bin.rs` via `env!("CARGO_BIN_EXE_poet")`: `--version`, no-args howto, unknown action/flag exit 2 (clap stderr, empty stdout), error command exit 1 one-JSON, `POET_HOME` chaining across processes, dev sandbox e2e + capture-atom e2e (dev builds), release-binary dev-surface rejection (strict builds).
- [x] `tests/contract.rs`: 7-test panic battery (empty/whitespace, hostile indices, unicode/control text, missing/duplicate args, 100k-char input, hostile paths incl. NUL, chained sequence) — never panics, always clap-or-envelope.

## Phase 4 — Example corpus (content)

- [x] 76 atoms captured via `--capture-example` (real envelopes) + hand-written behavioral notes (`/tmp/opencode/capture-atoms.sh` + `author-notes.py` loops); atoms document the envelope-message quirk.
- [x] Capture improved: shell-quote reconstruction in `core/dev.rs` (`shell_quote`) so atom invocations re-run exactly; dev commands captured via the real `dev clean`/`dev setup` flow.
- [x] **Howto drift found & fixed:** `table set-cell` positionals, `delete-row`/`delete-column` positionals (howto showed flag-style; the howto tests don't pin flag shapes — the QA agent would have caught this black-box).
- [x] Scenarios: `examples/quarterly-report.md` (migrated, self-contained heredoc script) + `examples/report-lifecycle.md`; each well past the ≥3-distinct-fn floor; `docs/examples/quarterly-report/` removed.

## Phase 5 — `build.rs` compile gates

- [x] syn-scan `src/commands/*.rs`: command = `pub fn` returning `Result` (peel parens; `r#move` → `move`); collect `#[test]` fns incl. nested mods.
- [x] Gate A (example): `docs/examples/<category>/<action>.md`, keyed by the **clap action name** (kebab — `set_level` fn → `set-level.md`).
- [x] Gate B (unit test): `#[test] fn <action>_…` in same module — surfaced ~50 gaps; wrote `per_command_tests` mods in 13 command files (55 focused tests; three tests corrected assumptions to match real behavior — close idempotency, columns echo, section start_type echo).
- [x] Gate C (integration coverage, Poet addition): `"<category>"`+`"<action>"` (kebab) token pair in ≥1 `tests/*.rs` — surfaced 34 gaps; `tests/command-coverage.rs` chains them all incl. image PNG flow.
- [x] Gate D (scenarios): ≥1 `examples/*.md`, each ≥3 distinct command fns in `sh`/`bash` fences resolved against the signature set (kebab→snake normalized); unknown invocations reported by name.
- [x] Gate E: every `Data` variant must appear in `src/models/examples.rs` (smoke-test registry).
- [x] Accumulate **all** violations → print → fail; `rerun-if-changed` on `src/commands`, `src/models/{data,examples}.rs`, `docs/examples`, `examples`, `tests`, `Cargo.toml`. First strict run: 108 violations → 0.

## Phase 6 — QA agent (`poet-dev-validate`)

- [x] `.opencode/agents/poet-dev-validate.md` written per the design: full permission denial except `.sandbox` + `cargo build*` + `./target/{debug,release}/poet *`; `poet howto` + `--help` as the contract; phases A–E; catalog mapped to real codes (`not_found` for table/image range errors — verified against the unit tests); dev-vs-release parity probe; prescribe-don't-author; report format + STOP.

## Phase 7 — CI + wiring

- [x] `ci.yml`: clippy job lints `--features dev`; test job runs `cargo test` (strict gates) + `cargo test --features dev`.
- [x] PR template + AGENTS.md QA section landed in Phase 1; PLAN.md ADR index fixed (0014 row restored, 0015 added).
- [ ] PLAN.md phase tracker + merge to `nightly` — pending owner review/commit.

---

## QA agent design (Phase 6 spec)

**File:** `.opencode/agents/poet-dev-validate.md` — port of carpenter's 312-line runbook.

### Frontmatter (permissions = the black-box discipline, mechanically enforced)

```yaml
---
description: >-
  Black-box QA agent. Drives the poet CLI with the user's real document
  request to actively hunt CLI interaction failures, missing --help
  explanations, and missing worked examples/scenarios. Prescribes missing
  examples (never authors them); reports every failure + code-level gap.
  Strict sandbox; no source access. Builds one document at a time,
  sequentially — never runs concurrent mutations.
mode: primary
permission:
  read:    { "*": "deny", ".sandbox": "allow", ".sandbox/**": "allow" }
  glob:    { "*": "deny", ".sandbox/**": "allow" }
  list:    { "*": "deny", ".sandbox/**": "allow" }
  grep:    { "*": "deny" }
  edit:    { "*": "deny", ".sandbox/**": "allow" }
  bash:
    "*": "deny"
    "cargo build": "allow"
    "cargo build *": "allow"
    "./target/debug/poet *": "allow"
    "./target/release/poet *": "allow"
  task:               { "*": "deny" }
  external_directory: { "*": "deny" }
  webfetch: deny
  websearch: deny
  lsp: deny
  question: allow
---
```

(No `skill: allow` — Poet has no skill; `poet howto` **is** the manual.)

### Runbook sections

1. **Persona.** Black-box QA fault-hunter. The user's document request is the *test corpus*, not the deliverable; the deliverable is a **failure + gap report**. Fixes nothing outside `.sandbox`; for doc gaps *prescribes*, never authors.
2. **Black-box rule.** Behavior learned **only** from `poet --help`, `poet <category> --help`, `poet <category> <action> --help`, `poet howto`. Documented behavior **is the contract**; `observed ≠ documented` is a failure. Never read/grep/glob `src/**` or `docs/**` (permissions enforce). Unknowns get probed empirically in `.sandbox` — the envelope is the test.
3. **Only `.sandbox`.** All documents, batch scripts, and session state under `.sandbox/`; `docs/examples/**` gaps are prescribed.
4. **Dynamic clarification.** One intake prompt: (a) document corpus (report type, sections, tables + sample data, styling, image, export format); (b) optional focus list of `<category>::<action>` (else: whole 66-command surface). Never ask about fixed contract facts (envelope shape, autosave, session-per-process). Propose the outline; sign-off **before `document new`**.
5. **Hard prerequisites & boundaries.** No filesystem permissions — sandbox lifecycle only via `poet dev {setup,clean}`; never `rm`/`mkdir`. No code authoring; bugs/missing docs/missing tests → human. Release/install binary lacks `dev` — bootstrap via `cargo build --features dev`, run through `./target/debug/poet`. **Session isolation:** every stateful call uses the prefix `./target/debug/poet --dev-home .sandbox/.poet …`; never env-var prefixes.
6. **Phase A — Prereq + recon.** `cargo build --features dev` → `dev clean` → `dev setup` → `--help` tree (14 categories) → `poet howto` (the contract). Enumerate the surface; note focus commands vs howto.
7. **Phase B — Sandbox.** Every iteration starts clean: `dev clean` unconditionally before `dev setup` (both are idempotency probes). Build only `.sandbox/<name>.docx`.
8. **Phase C — Active failure-hunt.** Model the real user: one document at a time, **sequential**, following the howto's two sanctioned patterns — `&&` chains in one shell call and `batch run` (session-per-process is *documented*, not a bug). Per-section loop fully finished before the next: `document new` → `heading add` → `paragraph add/insert` → `run add/format/emphasize` → `list add/add-item/convert` → `table add/set-cell/set-range/add-row…` → `image add/resize` → `toc add` → `page header/footer/margins` → `style apply` → `meta set_document/set_table` → `document save` → `close`; longer flows via `batch run`. Then reopen and verify: `document info/open`, `paragraph list/get`, `table get`, `meta get_*`, `calc read/stats/aggregate/filter/transform` on the saved table, `document export md/txt`.

   Failure-hunting catalog (ported; mapped to Poet's codes):

   | category | probes |
   |---|---|
   | error paths | no document open → `document_state` ("No document is open"); open nonexistent path → `not_found`; paragraph index out of range → `validation_error` (Words' exact message); unknown style → `validation_error`; heading/list level 0 or 7 → `validation_error`; set-cell out of bounds → `validation_error`; image add on missing file → `file_error`; calc on missing table → `not_found`/`calculation_error`; malformed batch script → `validation_error`; corrupt `session.json` → silent recovery, no crash |
   | edge cases | empty-string text, unicode/CJK content, huge/boundary indices, zero-row tables, duplicate operations, weird filenames, first-element index convention (0 vs 1 as documented) |
   | chaining | bookmark `--id` from A resolves in B; id after `delete`/`move`/`clear`; table id into `calc --table`; broken `&&` chain leaves valid state + session |
   | idempotency | `save` twice; `close` twice (2nd → `document_state`); `export` twice; `dev setup`/`clean` idempotent; re-running the same batch |
   | doc-vs-behavior | envelope matches `--help`/`howto` shape — incl. content commands carrying `message` inside `data` with envelope `message: ""` |
   | dev-vs-release | `dev` group, `--capture-example`, `--dev-home` absent from release `--help`/`howto` — any leak is a **bug**; strict `cargo build` gate failure names a gap → doc-gap (Phase D), not a crash |
   | panics/hangs | any panic, bare crash, or timeout is **critical** — never silent |

   Sequential rule: no `&`, no parallel/backgrounded mutations, no concurrent `batch run` — any corruption/lost-update under this discipline is a bug.
9. **Phase D — Doc gaps (prescribe, don't author).** Using only CLI output: (1) missing/placeholder `--help` text; (2) missing worked example — strict `cargo build` fails naming `docs/examples/<category>/<action>.md`, or absent from howto; (3) missing scenario — command not featured in any `examples/*.md`. For each: run the command in `.sandbox`, capture a **real envelope** (`--capture-example` may scaffold), prescribe location + invocation + envelope + note content + which contract it pins. Scenarios: ≥3 distinct commands featuring the gap chained with existing ones.
10. **Phase E — Report, STOP, teardown.** Present the report and **wait** — no edits/builds/cleanup until sign-off. Human owns fixes + final authoring. Then `./target/debug/poet --dev-home .sandbox/.poet dev clean` — runs even on failure; nothing kept.
11. **Report format.** Failures: `| command | probe | expected (from --help/howto) | observed | verdict |` — verdict ∈ **PASS** · **expected-error** · **bug**. Doc gaps: `| location | what's missing | prescription | why needed |`. Tallies: `commands audited: N · examples missing: M · scenarios prescribed: K · failures: F` + needs-human list.
12. **Never list.** Never read/grep/glob outside `.sandbox/**`; never write outside it; never `rm`/`mkdir` directly; never build documents at repo root; never run concurrent mutations; never skip the `--dev-home` prefix on stateful calls; never proceed past the report without sign-off; never hand-author examples/scenarios (prescribe).

### Lifecycle in the dev loop

Manual by design (carpenter adr/021): run per PR on the PR branch; enforcement = the PR template's mandatory report ("failure tally must show zero bugs"). CI never runs it — LLM-driven, slow, non-deterministic, interactive. The agent's checklist is updated in the same PR that changes CLI surface.

---

## Sequencing

Phases 1→7 in order; Phase 2 precedes 4 (corpus needs `--capture-example`); Phase 5 lands **with** the compliant Phase 4 corpus (strict build must go green in the same merge). Conventional Commits with ADR citations, e.g. `feat(phase5): dev sandbox lifecycle (adr/0015)`. Work on `ivan/phase-5-qa` cut from `nightly`, merged back per phase; PLAN.md tracker updated in the same merge that completes the phase.
