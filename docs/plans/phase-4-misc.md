# Phase 4 — Miscellaneous: Metadata, Calc, Batch, Docs

You are executing phase 4 (final) of the Poet rebuild. This brief is self-contained.
Read `AGENTS.md` first; skim `PLAN.md` and the phase 1–3 code you are extending.

## Global context (restated)

- **Poet** is an AI-first .docx automation CLI rebuilt in Rust from **Words**
  (`/home/mashkini/Workspace/Words`, read-only reference — the behavioral spec).
- Binary `poet`; engine `docx-rs` 0.4.x; envelope per `AGENTS.md` §2; single command
  signature; no unwrap; docs mandatory; ADRs for decisions; CI gate must stay green.
- Phases 1–3 delivered: lifecycle + bookmarks, all content commands, all formatting/
  layout commands. This phase adds the AI-first extras, then ports the docs/howto.

**Port behavior from these Python sources:**
- `/home/mashkini/Workspace/Words/src/words/meta/{engine,annotator,models,type_inference}.py`
- `/home/mashkini/Workspace/Words/src/words/calc/{reader,strategies,analyzer}.py`
- `/home/mashkini/Workspace/Words/src/words/commands/{meta_cmd,calc_cmd,batch,howto}.py`
- `/home/mashkini/Workspace/Words/src/words/commands/batch.py` (dispatch table + templates)
- `/home/mashkini/Workspace/Words/docs/reference/batch-scripts.md` (batch format spec)
- Tests to mirror: `/home/mashkini/Workspace/Words/tests/` (~179 tests across
  unit/core, unit/commands, unit/calc, unit/meta, integration)

## Scope

### 1. meta (8 commands)
`describe`, `get-document`, `set-document <JSON>`, `get-section <name>`, `set-section`,
`get-table <id>`, `set-table <id> <JSON schema>`, `history [--limit 50]`.
- **Storage**: custom XML part inside the .docx (Words uses `/customXml/words_meta.xml`
  with a `<meta>` wrapper around a JSON payload
  `{version:"words_meta_v1", document:{}, sections:[], tables:[], paragraphs:[], history:[]}`).
  Probe docx-rs's custom-XML/custom-item support; if read-preservation is unreliable,
  fall back to zip-level part injection via `Docx::build()` package parts. Record in
  **ADR 0011**. Payload shape/defaults/caps (history max 1000) = Words.
- **Annotator**: hook calls on paragraph/heading/list/run/table mutations that append
  history entries and (for tables) infer column schemas — port `AutoAnnotator` +
  `infer_column_type` (bool → number → currency → percentage → date → numeric-string →
  string; currency symbol → USD/EUR/GBP/JPY/INR).
- **Models**: serde structs for `DocumentMeta`, `SectionMeta`, `TableColumn`,
  `TableSchema`, `ParagraphAnnotation`, `HistoryEntry` — port fields exactly.

### 2. calc (5 commands)
`read`, `stats`, `aggregate`, `filter`, `transform` — each takes `<path>` and reads the
table fresh from disk (no session), resolving by `--id` or `--index` (default 0).
- Add deps: `polars` (features: lazy + needed dtypes), `rhai`.
- Reader: header = first row; coerce bool/number/string; dedup duplicate headers as
  `name_1`; optional `--range "r1:r2"` (header always kept).
- `stats` (per numeric column: count/mean/min/max/std/median), `aggregate`
  (sum/mean/min/max/count/median), `filter` ops (`==,!=,>,<,>=,<=,contains,startswith,
  endswith`), all results as row objects (`to_dicts` equivalent).
- `transform` ops: sort, rename, drop, select, fill_null, add_column — add_column
  evaluates a **rhai expression** per row (see ADR 0012).

**ADR 0012 — rhai expression grammar** (replaces Python `eval()`): supported:
`col("Name")` returning number/string/bool, numeric/string literals, `+ - * / %`,
comparisons, `if/else` if free. Provide `col` as a registered function; define
type-mismatch and missing-column errors; document the grammar in the `--help` and howto.
Test `col("Q1") * 2`, string concat, bool column.

### 3. batch (2 commands)
`run <script.json>`, `template <basic|report|data_table> <output>`.
- Script = JSON **array of flat command objects** `{"cmd": "...", "action": "...", ...}`
  (kebab-case actions, snake/kebab params exactly as Words' docs show). Dispatch covers
  the same 12 categories as Words; **stop on first error**; `document new/open` track
  the path used by later `save`; result envelope
  `{status:ok, data:{script, commands_executed, results:[], message}}`.
- Templates: port the three built-ins byte-for-byte (rename nothing).
- Note: Words' `models/batch.py` schema does NOT match the real flat format — the
  reference is `commands/batch.py` + `docs/reference/batch-scripts.md`.

### 4. Docs & howto (rebranded)
- `src/howto.rs` + `src/howto.md`: port Words' `HOWTO_PROMPT`
  (`/home/mashkini/Workspace/Words/src/words/commands/howto.py`) with `words`→`poet`,
  `~/.words`→`~/.poet` substitutions; keep structure/coverage identical (role framing,
  addressing model, per-category reference, batch format, calc operators, two-phase
  workflow, troubleshooting). Update command counts if they differ.
- `README.md`: port Words' README rebranded (installation via `cargo install --path .`,
  same feature list adjusted to reality, same quick-start).
- `docs/`: port `docs/reference/batch-scripts.md`, `docs/tutorials/01-getting-started.md`,
  `docs/examples/quarterly-report/` — rebranded.
- `poet howto` + no-subcommand invocation print the howto text.

### 5. Test-parity sweep
- Port the remaining Words test areas not yet mirrored (meta engine/annotator/type
  inference, calc reader/strategies/analyzer, batch, end-to-end integration: batch
  build→reopen→verify; batch table + `calc stats` asserting mean 20.0; session
  persistence across invocations; error-after-close).
- Fidelity probes: read→pack→read for every feature; meta part survives round-trip.

## Verification (all must pass before merge)

1. Full gate: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt
   --check && RUSTDOCFLAGS="-D warnings" cargo doc`.
2. Meta: set/get document/section/table round-trips through save→reopen; describe;
   history capped at 1000; type inference table (all seven types + currency units).
3. Calc: stats/aggregate/filter/transform golden-value tests incl. the Words
   integration case (mean 20.0) and rhai `add_column` cases.
4. Batch: template generation matches Words' three templates; a two-phase batch
   (content, then formatting) runs stop-on-first-error correctly end-to-end.
5. Howto/README/docs: rebranded, no stale `words`/`~/.words` references (grep clean).
6. Update `PLAN.md` tracker; merge `ivan/phase-4-misc` → `nightly` (no-ff), suggested
   message `feat(phase4): meta, calc, batch, docs — parity complete`.

## After merge (lead/reviewer, not the phase agent)

- Promotion PR `nightly → main`, tag `v0.1.0` (ADR 0002).
