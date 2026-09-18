# Changelog

All notable changes to Poet are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-09-18

Initial release: feature parity with the Words reference CLI, rebuilt in Rust.

### Added

- **Document lifecycle** (`document new/open/save/close/info/export`) with
  per-process session auto-open (`~/.poet/session.json`, `POET_HOME` override)
  so `&&` chains work like shell processes.
- **Content commands**: paragraphs (add/insert/get/update/delete/list/move/
  clear/border/find/replace/count), runs (add/get/clear/format/emphasize with
  run-splitting inline formatting), headings, real numbering-based lists,
  tables (add/get/set-cell/set-range/add-row/add-column/delete-row/
  delete-column), sections, TOC field, images, and md/txt export.
- **Design commands**: run formatting flags, style registry (styles part +
  builtin catalog), paragraph borders, and page layout ×7 (margins,
  orientation, size, header, footer, page numbers, columns).
- **Metadata** (`meta describe/get-document/set-document/get-section/
  set-section/get-table/set-table/history`): payload stored as a custom-XML
  part inside the .docx (adr/0012), with auto-annotation of content mutations
  and type-inferred table schemas (boolean/number/currency/percentage/date/
  string).
- **Calc** (`calc read/stats/aggregate/filter/transform`): fresh-from-disk
  table analysis on polars with Words' coercion and header-dedup semantics,
  `--range` row windows, and rhai expressions for calculated columns
  (adr/0013).
- **Batch** (`batch run/template`): flat JSON command scripts executed
  in-process with stop-on-first-error, plus the basic/report/data_table
  templates.
- **Agent ergonomics**: single JSON envelope per invocation, stable error
  codes, bookmark-id addressing for every element, and `poet howto` — the
  full AI-oriented reference (also printed when no subcommand is given).
- **CLI polish**: shell completions via `poet completions <shell>`
  (adr/0014), per-command `--help`, exit codes 0/1/2.

### Sources

- Behavioral spec: the Words CLI (Python) — observable behavior ported,
  implementation replaced.
- Decision records: `docs/adr/0001`–`0014`.
