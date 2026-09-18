# Poet - AI-First Word Document Automation CLI

You are an expert at using Poet, a command-line tool for creating, modifying, reading, and
analyzing **.docx Word documents**. Poet outputs structured JSON for programmatic consumption.
It has 70+ commands across 14 categories: document, section, paragraph, run, style, heading,
list, table, image, toc, page, meta, batch, calc.

## Installation

```bash
git clone <repo> && cd poet
cargo install --path .
poet --help
```

Requires a stable Rust toolchain to build and nothing at runtime. Poet uses **docx-rs** for
document operations, **polars** for table analysis, and **rhai** for calc expressions.

## Shell Completions

```bash
poet completions bash >> ~/.bashrc      # or zsh | fish | powershell | elvish
```

## Core Concepts

### Session Lifecycle

**CRITICAL: Each `poet` CLI call is a fresh process. Session state does NOT persist between
separate shell invocations.** You CANNOT chain separate CLI commands across multiple shell calls:

```bash
# WRONG — session is lost between calls:
poet document new report.docx
poet paragraph add "Hello"     # ERROR: "No document is open"
```

There are only two ways to execute multiple commands in a single session:

**Option 1 — Chain with `&&` in one shell call:**
```bash
poet document new report.docx && \
  poet heading add "Report" --level 1 && \
  poet paragraph add "Hello world." && \
  poet document save
```

**Option 2 — Use a batch script (recommended):**
```bash
poet batch run script.json
```

Whenever you need to run more than one command, always use batch scripts or `&&` chaining.

### Addressing: Bookmark IDs

Every element Poet creates (paragraph, heading, table, list item, image, TOC) is wrapped in an
OOXML **bookmark** and given a stable id. Commands address elements by `--id`:

```bash
poet paragraph add "Intro" --id intro          # create with explicit id
poet paragraph get --id intro
poet run add "more" --id intro --bold          # append a bold run to it
poet paragraph update "New text" --id intro
```

If you omit `--id`, Poet auto-assigns one (`p1`, `p2`, `h1`, `t1`, `l1`, `img1`, ...). The JSON
response always echoes the assigned `id`. Capture it to reference the element later.

For documents created **outside** Poet (no bookmarks), use `--index` (0-based position) as a
read fallback:

```bash
poet paragraph get --index 3
poet table set-cell --index 0 --row 1 --col 2 --value "x"
```

### JSON Output Format

Every command returns structured JSON:

```json
{"status": "ok", "data": {...}, "message": "..."}      // success
{"status": "error", "message": "...", "code": "..."}   // error
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success (or raw-text output: `howto`, `completions`) |
| `1` | The command returned an error envelope |
| `2` | Clap usage error (unknown command, bad flags — plain text, not JSON) |

### Error Codes

Branch on `code` in error envelopes:

| Code | Meaning |
|------|---------|
| `session_error` | Reading/writing `session.json` failed |
| `validation_error` | Bad input (bad level, bad range, invalid JSON argument, ...) |
| `metadata_error` | Metadata operation failed |
| `calculation_error` | Calc/analysis failed (unknown operator, type mismatch, missing column) |
| `file_error` | File I/O failed (read/pack) |
| `not_found` | File, bookmark id, or table does not exist |
| `conflict` | Name already taken or state conflict |
| `document_state` | Operation requires an open document (or none to be open) |
| `unsupported` | Engine capability not supported (e.g. pdf export) |
| `not_implemented` | Reserved for phased rebuilds; unused in release builds |
| `internal` | Unexpected internal condition (please report) |

### Multi-Phase Processing Strategy

Since sessions do not persist between CLI calls, split complex document generation into phases.
Each phase uses a separate batch script or chained CLI call.

- **Phase 1 — Content:** create the document, add headings, paragraphs, tables, lists, images,
  TOC. Save.
- **Phase 2 — Formatting & Layout:** re-open the saved file, apply run styling, page margins,
  headers/footers. Save.

Always co-locate batch JSON files with the output `.docx` (not in `/tmp`).

---

## FILE OPERATIONS (document)

```bash
poet document new ./report.docx
poet document open ./report.docx
poet document save
poet document save --path ./backup.docx
poet document close
poet document info
poet document export ./report.docx md --out report.md     # md | txt (pdf needs converter)
```

## SECTIONS (section)

```bash
poet section list
poet section info --index 0
poet section add --start-type new_page      # new_page|new_column|even_page|odd_page|continuous
poet section page-break                      # insert an explicit page break
```

## PARAGRAPHS (paragraph)

```bash
poet paragraph add "Some text."              # -> echoes id (e.g. p1)
poet paragraph add "Intro" --id intro
poet paragraph add "Quote" --style "Intense Quote"
poet paragraph insert 0 "Title" --id title
poet paragraph get --id intro
poet paragraph get --index 3
poet paragraph update "New text" --id intro
poet paragraph delete --id intro
poet paragraph clear --id intro              # remove text, keep paragraph
poet paragraph move up --id intro            # up | down
poet paragraph list
poet paragraph count
poet paragraph find "Total"
poet paragraph replace "Draft" "Final"
poet paragraph border --id intro --position bottom --color FF0000
```

`paragraph border` draws a box edge around a paragraph: `--position top|bottom|left|right`,
`--color RRGGBB`, `--size` (eighths of a point, default 4), `--space` (gap to text in
points, default 1), `--style` (e.g. single, double, dashed). Requires `--id` or `--index`.

## RUNS (run) — inline text spans with formatting

```bash
poet run add "important" --id intro --bold --color FF0000
poet run add " text" --id intro --italic --underline
poet run add "big" --id intro --font Arial --size 18
poet run get --id intro                       # list runs + their formatting
poet run format --id intro --bold             # restyle existing runs
poet run clear --id intro
```

Formatting flags: `--bold`, `--italic`, `--underline`, `--font NAME`, `--size N`,
`--color RRGGBB` (hex, optional leading #). All optional; combine freely.

### Emphasize a substring (inline) + table-cell addressing

`run emphasize` applies inline formatting (e.g. bold) to a **substring** inside a
paragraph by splitting the surrounding runs, so existing formatting is preserved.

```bash
poet run emphasize "22%" --index 5 --bold                       # body paragraph
poet run emphasize "6 million HKD" --table 1 --row 0 --col 0 --bold   # cell paragraph
poet run emphasize "TODO" --id note --bold --color FF0000 --all # every occurrence
```

Table-cell paragraphs are NOT in the body `--index` range. Reach them with
`--table/--row/--col` (and optional `--para` to target one paragraph; omit to
search every paragraph in the cell). The same cell addressing also works on:

```bash
poet paragraph get --table 1 --row 0 --col 0 --para 3
poet paragraph delete --table 1 --row 0 --col 0 --para 6
poet run get --table 1 --row 0 --col 0 --para 3
```

## HEADINGS (heading)

```bash
poet heading add "Chapter 1" --level 1 --id ch1
poet heading add "Subsection" --level 2
poet heading set-level 3 --id ch1
poet heading list
```

Levels 1–9. (Level 0 / "Title" also supported.)

## LISTS (list)

```bash
poet list add "First bullet"                    # bullet list
poet list add "First item" --ordered            # numbered list
poet list add "Nested" --level 2
poet list convert --ordered --id p1             # turn a paragraph into a list item
poet list set-level 2 --id l1
```

## TABLES (table)

```bash
poet table add 4 3 --id sales
poet table list
poet table get --id sales
poet table set-cell --id sales --row 0 --col 0 --value "Product"
poet table add-row --id sales --values '["X", 1, 2]'
poet table add-column --id sales
poet table delete-row --id sales --row 1
poet table delete-column --id sales --col 1
```

Bulk-fill from a 2D array (first row = header with `--header`, which auto-infers column types):

```bash
poet table set-range --id sales --header '[
  ["Product", "Q1", "Q2"],
  ["Widget A", 15000, 18000],
  ["Widget B", 12000, 14500]
]'
```

## STYLES (style)

```bash
poet style list                     # all styles
poet style list --type paragraph
poet style apply "Intense Quote" --id p1
```

## IMAGES (image)

```bash
poet image add ./logo.png --width 2.0 --height 1.5      # inches
poet image list
poet image get --index 0
poet image resize --index 0 --width 3.0
poet image delete --index 0
```

## TABLE OF CONTENTS (toc)

```bash
poet toc add --levels 1-3
poet toc update        # Word refreshes the TOC field automatically on open
```

## PAGE LAYOUT (page)

```bash
poet page margins --top 1.0 --bottom 1.0 --left 0.75 --right 0.75
poet page orientation landscape
poet page size --width 8.5 --height 11
poet page header "Company Confidential"
poet page footer "Page ?"            # literal text
poet page page-numbers --align center
poet page columns --count 2
```

All page commands default to section 0; use `--section N` to target another.

## METADATA (meta)

Poet tracks operations and stores rich metadata in a custom XML part inside the .docx.

```bash
poet meta describe
poet meta set-document '{"title": "Q4 Report", "author": "Finance", "tags": ["q4","2024"]}'
poet meta get-document
poet meta set-section "Body" '{"purpose": "Main content"}'
poet meta get-section "Body"
poet meta set-table sales '{"name": "Sales", "description": "Quarterly sales"}'
poet meta get-table sales
poet meta history --limit 20
```

Content mutations (paragraph/heading/list/table) are auto-annotated: every operation appends a
timestamped history entry, and `table set-range --header` infers column types (boolean, number,
currency, percentage, date, string) into the table's schema.

## BATCH PROCESSING (batch)

```bash
poet batch run script.json
poet batch template basic basic.json
poet batch template report report.json
poet batch template data_table data.json
```

### Script format — JSON array of command objects

```json
[
  {"cmd": "document", "action": "new", "path": "report.docx"},
  {"cmd": "heading", "action": "add", "text": "Q4 Report", "level": 1},
  {"cmd": "paragraph", "action": "add", "text": "Executive summary.", "id": "summary"},
  {"cmd": "table", "action": "add", "rows": 4, "cols": 3, "id": "sales"},
  {"cmd": "table", "action": "set-range", "id": "sales", "header": true, "values": [
    ["Product", "Q1", "Q2"],
    ["Widget A", 15000, 18000],
    ["Widget B", 12000, 14500],
    ["Total", 27000, 32500]
  ]},
  {"cmd": "document", "action": "save", "path": "report.docx"}
]
```

### Parameter mapping (JSON key -> CLI)

| Key | Used in | Maps to |
|-----|---------|---------|
| `cmd` | all | category |
| `action` | all | subcommand |
| `path` | document | file path |
| `id` | paragraph/run/heading/list/table/image/toc | `--id` (bookmark) |
| `index` | paragraph/run/table/image | `--index` (positional fallback) |
| `text` | paragraph/heading/list/page | text content |
| `level` | heading/list | level |
| `ordered` | list | bullet vs numbered |
| `style` | paragraph/table/style | style name |
| `rows`,`cols` | table | dimensions |
| `row`,`col`,`value` | table | cell coordinates |
| `values` | table set-range/add-row/add-column | 2D / 1D array |
| `header` | table set-range | infer schema |
| `bold`,`italic`,`underline`,`font`,`size`,`color` | run | formatting |
| `run_index` | run format | target one run |
| `width`,`height` | image | inches |
| `levels` | toc | heading levels |
| `top/bottom/left/right`,`unit` | page margins | margins |
| `orientation`,`count`,`align`,`section` | page | layout |
| `start_type` | section | break type |
| `metadata`/`schema` | meta | JSON object |

### Batch caveats

1. **Session does NOT persist between separate shell invocations.** Use `batch run` or `&&`.
2. **`document save` uses `path`**, falling back to the path from `new`/`open`.
3. **Reference elements by `id`**: create with `"id": "intro"` then use `"id": "intro"` later.
   Without an explicit id, capture the auto-assigned id from the JSON result.
4. **Batch stops on first error** (check `results` for which command failed).
5. **Co-locate batch JSON with the .docx output** for reproducibility.

## DATA ANALYSIS (calc) — analyze document tables with Polars

Works directly on .docx files WITHOUT an active session.

```bash
poet calc read data.docx --id sales                      # or --index 0
poet calc stats data.docx --id sales --column "Q1"
poet calc aggregate data.docx --id sales --group-by Product --agg-column Q1 --agg-func sum
poet calc filter data.docx --index 0 --column Q1 --operator ">" --value 14000
poet calc transform data.docx --index 0 --operations '[{"type":"sort","by":"Q1","descending":true}]'
```

All calc commands accept `--range "start:end"` (1-based, inclusive, header always kept).

Filter operators: `==`, `!=`, `>`, `<`, `>=`, `<=`, `contains`, `startswith`, `endswith`.
Transform operations: `sort`, `rename`, `drop`, `select`, `add_column`, `fill_null`.

`add_column` evaluates a **rhai** expression per row:

```bash
poet calc transform data.docx --id sales --operations \
  '[{"type":"add_column","name":"Q1x2","expression":"col(\"Q1\") * 2"}]'
```

Expression grammar: `col("Name")` reads the current row's cell; numeric/string/bool literals;
operators `+ - * / %` and comparisons; `if cond { a } else { b }`. A missing column or a type
mismatch (e.g. `"a" * 2`) is a `calculation_error`.

## COMMON WORKFLOWS

### Build a report (two-phase)

Phase 1 — content (`report.phase1.json`):
```json
[
  {"cmd": "document", "action": "new", "path": "report.docx"},
  {"cmd": "heading", "action": "add", "text": "Annual Report", "level": 1},
  {"cmd": "heading", "action": "add", "text": "Summary", "level": 2},
  {"cmd": "paragraph", "action": "add", "text": "Revenue grew 15% YoY.", "id": "summary"},
  {"cmd": "table", "action": "add", "rows": 4, "cols": 3, "id": "sales"},
  {"cmd": "table", "action": "set-range", "id": "sales", "header": true, "values": [
    ["Region", "Q1", "Q2"], ["North", 50000, 62000], ["South", 48000, 55000], ["Total", 98000, 117000]
  ]},
  {"cmd": "toc", "action": "add", "levels": "1-3"},
  {"cmd": "document", "action": "save", "path": "report.docx"}
]
```

Phase 2 — formatting (`report.phase2.json`):
```json
[
  {"cmd": "document", "action": "open", "path": "report.docx"},
  {"cmd": "run", "action": "add", "id": "summary", "text": " Strong quarter.", "bold": true},
  {"cmd": "page", "action": "footer", "text": "Confidential"},
  {"cmd": "page", "action": "page-numbers", "align": "center"},
  {"cmd": "document", "action": "save", "path": "report.docx"}
]
```

```bash
poet batch run report.phase1.json
poet batch run report.phase2.json
```

## TROUBLESHOOTING

- **"No document is open"** — run `poet document new` / `open` first (or use `batch run`).
- **"No paragraph with id 'X'"** — check `poet paragraph list` for valid ids.
- **TOC is empty** — Word populates it on open (right-click → Update Field).
- **PDF export** — requires docx2pdf or LibreOffice; otherwise export to `md`/`txt`.

## COMMAND CATEGORIES SUMMARY

| Category | Description |
|----------|-------------|
| document | new, open, save, close, info, export |
| section | list, info, add, page-break |
| paragraph | add, insert, get, update, delete, list, move, clear, border, find, replace, count |
| run | add, get, clear, format, emphasize |
| style | list, apply |
| heading | add, set-level, list |
| list | add, add-item, convert, set-level |
| table | add, list, get, set-cell, set-range, add-row, add-column, delete-row, delete-column |
| image | add, list, get, resize, delete |
| toc | add, update |
| page | margins, orientation, size, header, footer, page-numbers, columns |
| meta | describe, get/set-document, get/set-section, get/set-table, history |
| batch | run, template |
| calc | read, stats, aggregate, filter, transform |

## BEST PRACTICES

1. Create/open a document before other commands (or use `batch run`).
2. Give important elements explicit `--id` so you can reference them later.
3. Build content first, then apply formatting in a second phase.
4. Always `poet document save` when done.
5. Use batch scripts for multi-step workflows; co-locate them with the output.
6. Use `calc` to analyze tables in any .docx without a session.
7. Set document metadata to make files self-describing for AI agents and humans.
