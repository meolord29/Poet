# Phase 3 — Additional Functionality: Design & Layout ("the CSS")

You are executing phase 3 of the Poet rebuild. This brief is self-contained. Read
`AGENTS.md` first; skim `PLAN.md` and the phase-2 code you are extending.

## Global context (restated)

- **Poet** is an AI-first .docx automation CLI rebuilt in Rust from **Words**
  (`/home/mashkini/Workspace/Words`, read-only reference — the behavioral spec).
- Binary `poet`; engine `docx-rs` 0.4.x; envelope contract per `AGENTS.md` §2; single
  command signature; no unwrap; docs mandatory; tests inline; ADRs for decisions.
- Phase 2 delivered all content commands (text/structure) with bookmark addressing,
  cell addressing, export, and round-trip fidelity. `run format`, `run emphasize`, and
  `paragraph border` are CLI-defined stubs waiting for this phase.

**Port behavior from these Python sources:**
- `/home/mashkini/Workspace/Words/src/words/commands/{run,style,page,paragraph}.py`
- `/home/mashkini/Workspace/Words/src/words/core/document_manager.py` — especially
  `apply_run_format`, `emphasize_substring` (lines ~361–482: the run-span-splitting
  algorithm), `set_paragraph_border`, margins/orientation/size/header/footer/
  page-numbers/columns sections methods
- `/home/mashkini/Workspace/Words/src/words/app.py` (flags/defaults for these commands)

## Scope

| Category | Actions |
|---|---|
| `run` | `format` (all-runs or `--run-index`; bold/italic/underline/font/size/color flags), `emphasize <find>` (formatting flags + `--all`; id/index/cell addressing) |
| `paragraph` | `border --position top\|left\|bottom\|right\|between` with `--color --size --space --style` |
| `style` | `list [--type paragraph\|character\|table\|list]`, `apply <style>` |
| `page` (7) | margins (`--unit inches\|cm\|points`, `--section`), orientation, size, header, footer, page-numbers (`--align`), columns |

## Design decisions to record in ADRs

- **ADR 0008 — run-span emphasize algorithm**: port Words' `emphasize_substring`
  faithfully: snapshot each run's formatting; find match spans of `find` in the joined
  run text (first match or `--all`); cut points = run boundaries ∪ match boundaries;
  rebuild runs segment-by-segment re-attaching source formatting, applying the target
  format to matched segments. Must handle matches spanning multiple runs. If docx-rs
  requires cloning `RunProperty` rather than deep-copying, that's fine — document the
  mapping.
- **ADR 0009 — style reading**: `style list` needs the styles part. If docx-rs parses
  styles on read, use it; otherwise read `word/styles.xml` directly from the package
  (zip-level side-read via `Docx::build()` parts or a zip open) and record which path
  was taken. `apply` sets `pStyle`/`rStyle` on the target.
- **ADR 0010 — page/section mapping**: docx-rs `SectionProperty` fields for margins
  (twips/EMU conversions exactly as Words' `_emus`), orientation swap semantics
  (width↔height), header/footer creation + `is_linked_to_previous = false` equivalent,
  page-number field (`PAGE`), column count (`w:cols w:num`).

## Implementation notes

- Run formatting flags map: bold/italic/underline, `--font`, `--size` (points),
  `--color` (hex, no `#`). `run format` without `--run-index` formats all runs of the
  target paragraph (Words semantics).
- `emphasize` value handling: preserve surrounding text exactly; empty match text →
  validation error; `--all` → every non-overlapping occurrence.
- Page commands accept `--section` (index) and operate on the resolved section's
  properties; header/footer text creates the header/footer part and links it.
- Keep helpers in `core/` (constitution §1). Extend existing `DocumentManager` methods;
  do not fork a second document API.

## Verification (all must pass before merge)

1. Full gate: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt
   --check && RUSTDOCFLAGS="-D warnings" cargo doc`.
2. Unit tests: emphasize (single-run match, multi-run match, `--all`, no-match error,
   formatting preserved outside match), run format (all-runs vs run-index), paragraph
   border sides/styles, margins unit conversions, orientation swap, header/footer +
   page numbers field presence, columns.
3. Round-trip: every formatting/layout feature applied → save → reopen → assert the
   property survived `read_docx` (this is the phase's exit test; a formatting feature
   that does not survive is not done).
4. Integration: style an existing phase-2 report (bold emphasis on a phrase, footer,
   page numbers, landscape section) and verify on reopen.
5. Update `PLAN.md` tracker; merge `ivan/phase-3-design` → `nightly` (no-ff), suggested
   message `feat(phase3): design & layout — formatting, styles, page`.
