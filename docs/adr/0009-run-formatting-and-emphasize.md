# ADR 0009 — run formatting, emphasize run-splitting, paragraph borders

Date: 2026-09-17 · Status: accepted

Phase-3 design-layer behaviors ported from Words
(`src/words/core/document_manager.py`: `apply_run_format` ~308–327,
`emphasize_substring`/`_emphasize_in_paragraph` ~343–482, `set_paragraph_border`
~238–261). Engine mapping: docx-rs `RunProperty` derives `Clone` with pub
fields, so Words' deepcopy-rPr snapshot maps to a plain `.clone()`.

## Emphasize algorithm (faithful port)

1. Snapshot direct runs left-to-right as `(start, end, cloned RunProperty)`
   spans over the joined run text (byte offsets; every boundary lands on a
   char boundary because each segment comes from a valid string).
2. Match locations: sequential case-sensitive `find`, non-overlapping
   (`start` advances past each match end); first match only unless
   `--all`.
3. Cut points = `{0, len}` ∪ run-span boundaries ∪ match boundaries, sorted —
   every segment lies inside one source run and inside/outside exactly one
   match.
4. Rebuild: all direct runs are removed, then one run per non-empty segment is
   appended in order with the **source run's cloned property**; matched
   segments additionally get the provided `FormatSpec` overlaid (bold, italic,
   underline, color, font, size — in that application order, like Words'
   dm.py:433-444). Zero-width segments are skipped. Non-run children
   (hyperlinks, …) keep their positions; rebuilt runs land after them — the
   same observable quirk as Words' remove-and-re-append.
5. Returns the match count; zero matches mutate nothing and report
   `replacements: 0` **successfully** (Words dm.py:394-395).

## Deviations from Words (all recorded once here)

- **Invalid color pre-validates.** Words calls `_hex_to_rgb` mid-rebuild, so an
  invalid `--color` leaves the paragraph half-rebuilt. Poet validates the whole
  `FormatSpec` (including hex color) before touching the document.
- **`--run-index` out of range** raises Words' bare
  `IndexError("list index out of range")`; Poet reports
  `Run index {i} out of range (0..{n-1})` (`not_found`), consistent with every
  other Poet range error.
- **Border `--style` is not validated** (matching Words' verbatim write):
  docx-rs models borders as the closed `BorderType` enum whose `FromStr`
  catch-all coerces unknown strings to `single`. Words wrote the string
  verbatim and let Word render whatever it could; both behaviors leave a
  visible border and never error. Unknown values therefore produce a single
  border rather than a rejection.
- **Negative border `--size`/`--space`** are rejected (docx-rs fields are
  `usize`; Python would have serialized the negative verbatim).

## Formatting application

One shared helper applies a `FormatSpec` to a `RunProperty` for `run add`,
`run format` (all runs, or `--run-index`), and emphasize rebuilds. Tri-state
bools map to present/disabled elements (`w:b` vs `w:b w:val="0"`); underline
maps to `single`/`none`; size is stored in half-points (`sz` + `szCs`); font
sets **both** `w:rFonts/@w:ascii` and `@w:hAnsi` (python-docx sets both; the
phase-2 `add_run` path sets only ascii — the shared helper closes that gap).
