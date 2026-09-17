# ADR 0007 — table model mapping

Date: 2026-09-16 · Status: accepted

## Decision

docx-rs' `Table { rows: Vec<TableChild>, grid: Vec<usize>, … }` maps to the
Words/python-docx grid as follows:

- `table add rows cols` builds `rows × cols` cells manually (docx-rs has no
  such constructor); each cell holds one empty paragraph. `grid` is `cols`
  entries of 1440 twips (1"); new columns append 1440.
- Style is stored as the id `TableGrid`; display name resolution (adr/0008)
  reports "Table Grid" like Words.
- `set-cell` ports `cell.text = value`: the cell's *entire* content is replaced
  by a single paragraph containing one run (python-docx clears existing
  paragraphs first).
- `add-row` appends a row with as many cells as `grid.len()`; `--values` fills
  cells left-to-right, extras ignored (Words: `i < len(row.cells)`).
- `add-column` appends a cell to every row and extends `grid`.
- `delete-row` removes the row; `delete-column` removes the column's cell from
  every row plus the `grid` entry (Words does the same via raw XML).
- `get`/`list`/`calc` read cell text as `"\n"`-joined paragraph texts
  (python-docx `_Cell.text`).

## `set-range`

Values are parsed from the JSON argument in the command layer
(`Invalid JSON: {e}` on parse failure; non-2D arrays are a validation error).
Cells are written row-major from (0,0) by repeated `set_cell`, so out-of-range
ranges fail mid-way after writing the cells that fit — Words behaves
identically (index error on the first out-of-range cell). `--header` does not
change what is written (Words only uses it for its annotator).

## Deviations from Words

- Scalar cells are stringified with JSON conventions (`true`, `1.5`), not
  Python `str()` (`True`) — rust-native representation, chosen deliberately.
- Out-of-range `row`/`col` errors use the crafted range-message pattern Words
  uses elsewhere instead of Python's bare "list index out of range".

Ported per `Words/src/words/core/document_manager.py` (`add_table`, `set_cell`,
`add_row`, `add_column`, `delete_row`, `delete_column`, `get_table_data`) and
`Words/src/words/commands/table.py`.
