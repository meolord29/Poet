# ADR 0006 — id allocation + element addressing

Date: 2026-09-16 · Status: accepted

## Decision

Every paragraph/heading/list item/table/image/TOC Poet creates is
bookmark-wrapped (adr/0004) and auto-named `{prefix}{max+1}` with the Words
prefixes: `p` (paragraph/page-break), `h` (heading), `l` (list item),
`t` (table), `img` (image), `toc` (TOC). An explicit `--id` always wins over
`--index`; when both are absent, addressing fails with the Words message
"Either id or index is required".

## Cell addressing is outside the body index range

Words' `_resolve_targets` distinguishes two addressing worlds:

- **Body paragraphs**: `--id`/`--index`, where `index` enumerates only body-level
  paragraphs (table contents are invisible to it).
- **Cell paragraphs**: `--table/--row/--col` (+ optional `--para`) resolve
  *directly inside* `tables[i].rows[r].cells[c]`, never through the body index
  range. `--para` omitted targets all paragraphs of the cell (read operations);
  operations that need one paragraph require `--para`.

This isolation is ported faithfully: cell indices and body indices are
independent namespaces, so cell addressing never shifts or consults body
positioning.

## Index space

`--index` addresses the paragraph's position among `DocumentChild::Paragraph`
children only (bookmark markers and section-break paragraphs with embedded
`sectPr` do not count as paragraphs for these operations; a section break is not
a deletable/movable paragraph). Table indices enumerate
`DocumentChild::Table` children in body order.

Ported per `Words/src/words/core/document_manager.py` (`_resolve_paragraph`,
`_resolve_targets`, `add_paragraph`, `insert_paragraph`, …) and the command
files' `id`/`index` echo semantics.
