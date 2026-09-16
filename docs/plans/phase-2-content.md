# Phase 2 — Core Functionality: Text & Structure ("the HTML")

You are executing phase 2 of the Poet rebuild. This brief is self-contained. Read
`AGENTS.md` first; skim `PLAN.md` and the phase-1 code you are extending.

## Global context (restated)

- **Poet** is an AI-first .docx automation CLI rebuilt in Rust from **Words**
  (`/home/mashkini/Workspace/Words`, read-only reference — the behavioral spec).
- Binary `poet`; engine `docx-rs` 0.4.x; envelope contract per `AGENTS.md` §2; single
  command signature per `AGENTS.md` §1; no unwrap; docs mandatory.
- Phase 1 delivered: scaffold, `core/{error,output,session,document,bookmark}.rs`,
  `Ctx` DI, clap tree with all 14 categories (stubs return `NotImplemented`),
  `document` lifecycle, autosave, session auto-open, CI.
- This phase turns the content stubs into working commands. No formatting commands yet —
  those are phase 3.

**Port behavior from these Python sources (read them; do not guess shapes):**
- `/home/mashkini/Workspace/Words/src/words/commands/{paragraph,run,heading,list,table,image,section,toc,document}.py`
- `/home/mashkini/Workspace/Words/src/words/core/document_manager.py` (all content
  methods: paragraphs, runs, headings, lists, tables, images, sections, TOC, find,
  cell addressing, `_resolve_paragraph`, `_resolve_targets`, export md/txt)
- `/home/mashkini/Workspace/Words/src/words/core/bookmark.py` (already ported in phase 1)
- `/home/mashkini/Workspace/Words/src/words/app.py` (exact flag names/defaults per command)

## Scope — commands to implement (flags/JSON shapes = Words)

| Category | Actions |
|---|---|
| `paragraph` (12) | add, insert, get, update, delete, list, move (up\|down), clear, find, replace, count, border* |
| `run` (5) | add, get, clear (+ format/emphasize* — stubs until phase 3) |
| `heading` (3) | add (level 1–9), set-level, list |
| `list` (4) | add, add-item, convert, set-level |
| `table` (9) | add, list, get, set-cell, set-range (JSON 2D array, `--header`), add-row, add-column, delete-row, delete-column |
| `section` (4) | list, info, add (start types), page-break |
| `toc` (2) | add (`--levels "1-3"`), update (hint no-op) |
| `image` (5) | add, list, get, resize, delete |
| `document` | `export <md\|txt>` (pdf stays unsupported-error; port `_export_text`/`_export_markdown`) |

\* `paragraph border` is a phase-3 deliverable — define the CLI arg now, stub the body.
`run format`/`run emphasize` likewise (phase 3).

## Design decisions to record in ADRs

- **ADR 0005 — lists via real numbering**: docx-rs models numbering with
  `AbstractNumbering`/`Numbering`/`NumberingId`. Build bullet and ordered numbering
  definitions on demand and reference them from paragraphs (Words relied on built-in
  `List Bullet`/`List Number` styles). Level handling: ` {level}` variants.
- **ADR 0006 — id allocation + addressing**: every created paragraph/heading/list item/
  table/image/toc is bookmark-wrapped and auto-named `{prefix}{max+1}`; `--id` wins over
  `--index`; cell-scoped addressing (`--table/--row/--col/--para`) resolves cell
  paragraphs *outside* the body index range (port this subtlety faithfully).
- **ADR 0007 — table model mapping**: how rows/cols/cells map to docx-rs
  `Table`/`TableRow`/`TableCell` including add/delete row/column and `set-cell`
  (port Words' `cell.text = value` semantics: replace all cell paragraphs with one).

## Implementation notes

- Extend `DocumentManager` (in `core/document.rs`) with content methods; keep command
  fns thin. Port the *observable* behavior: same defaults, same messages, same `data`
  keys, same error codes for the same mistakes (unknown id → not-found-style error,
  out-of-range index, malformed JSON `set-range` value, etc.).
- `emphasize`-style formatting is out of scope; `run add` in this phase carries plain
  text (formatting flags accepted if Words' CLI has them, applied in phase 3 — or apply
  trivially if docx-rs makes it free, your call, but document it).
- Images: docx-rs `Image`/`Pic`/drawing APIs; port `--width/--height` (inches) and the
  list/get/resize/delete semantics (hosting paragraph removal on delete).
- TOC: port the field-code construction (`TOC \o "levels" \h \z \u`) with docx-rs's
  `TableOfContents`/field support if usable, else build runs with `FieldChar`/instr text.
- Export: markdown/txt walkers over body content (headings → `#`, paragraphs, tables →
  pipe tables, list items → `-`/`1.`) — port from Words' `_export_*`.
- Update the howto placeholder only if trivially needed; full howto text is phase 4.

## Verification (all must pass before merge)

1. Full gate: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt
   --check && RUSTDOCFLAGS="-D warnings" cargo doc`.
2. Unit tests per command module (`<fn>_<expected_behavior>`, shared `testutil::setup()`):
   - paragraph add/insert/get/update/delete/move/clear/find/replace/count + id/index
     addressing + cell addressing isolation from body indices
   - heading levels, list convert/set-level, table set-range + add/delete row/col,
     image add/resize/delete, section add/list, toc add, export md/txt
3. Integration (in-process `run([...])`): build a full report document via chained
   commands (heading + paragraphs + 4×3 table via set-range with `--header` + toc +
   image), save, **reopen from disk**, and verify structure survives the read→pack→read
   round-trip (counts, ids resolvable, table contents).
4. Update `PLAN.md` tracker; ADRs written; merge `ivan/phase-2-content` → `nightly`
   (no-ff), suggested message `feat(phase2): content — text & structure commands`.
