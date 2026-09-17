# ADR 0010 — style registry: styles part + builtin catalog

Date: 2026-09-17 · Status: accepted

`style list` / `style apply` (and the retrofitted validation on
`paragraph add/insert --style`) need a style registry. Words reads
python-docx's default style table (~180 builtins); docx-rs' `Styles::default()`
is **empty** and a fresh Poet document ships a styles part containing only
`Normal` (verified by packing a blank document and inspecting
`word/styles.xml`). A zip-level side-read cannot conjure styles that are not
in the package, so:

## Registry = styles part ∪ builtin catalog

1. **Styles part** (`docx.styles.styles`, parsed by the docx-rs reader — no
   zip side-read needed): entries in document order, name resolved via the
   existing `style_display_name` machinery.
2. **Builtin catalog**: a curated table (~40 common Word styles with their
   types: Normal, Heading 1–9, Title, Subtitle, Quote, Intense Quote, Caption,
   List Paragraph, No Spacing, TOC 1–9, TOC Heading, character styles like
   Strong/Emphasis, table styles like Table Grid, list style No List, …).
   This approximates python-docx's default table for the styles an agent can
   meaningfully apply to a Poet document; ids follow the existing
   `style_id_from_name` compaction (`Heading 1` → `Heading1`).

`style list` output order: styles-part entries first, then catalog entries not
shadowed by the part. `--type` filters by Words' four tokens
(`paragraph|character|table|list`); an unknown token is a `validation_error`
(deviation — see adr/0011's policy). `style apply` resolves
display-name-part → id → catalog before rejecting with
`no style with name '<name>'` (`not_found`, Words' KeyError wording).

## Recorded deviations

- Word renders undefined-but-referenced styles leniently, so applying a
  catalog builtin whose definition is absent from the styles part behaves like
  Words (the `w:pStyle` lands; Word falls back to its local builtin).
- `builtin` is reported as `true` for every entry: docx-rs has no
  `customStyle` attribute model, and Poet offers no custom-style creation, so
  the flag carries no information (Words derived it from
  `w:customStyle` absence).
- List counts differ from Words on the same command: Poet's catalog is
  intentionally not the python-docx template (adr/0001 — CLI parity of
  behavior, not of document internals). Entries that exist in both use
  identical names and types.
- `type` strings keep Words' enum rendering: `PARAGRAPH (1)`,
  `CHARACTER (2)`, `TABLE (3)`, `LIST (4)` (docx-rs `StyleType::Numbering`
  maps to `LIST (4)`). docx-rs `StyleType::Unsupported` entries are skipped.
