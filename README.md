# Poet

> AI-First Word Document Automation CLI — Create, modify, read, and analyze **.docx** files with rich semantic metadata, entirely from the command line.

## Features

- **70+ CLI Commands** organized into 14 categories (document, section, paragraph, run, style, heading, list, table, image, toc, page, meta, batch, calc)
- **AI-First Design**: Structured JSON output for programmatic consumption by AI agents
- **Bookmark Addressing**: Every element Poet creates gets a stable id (`--id`) you can reference later
- **Semantic Metadata**: Auto-annotated operations with rich context, stored in a custom XML part inside the .docx
- **Batch Processing**: Execute complex workflows from JSON scripts
- **Data Analysis**: Polars-powered statistics, filtering, aggregation, and transforms on document tables
- **Expression Engine**: rhai-powered calculated columns in `calc transform add_column`
- **Document Automation**: Create headings, paragraphs, tables, lists, images, TOC, headers/footers without opening Word
- **Single Static Binary**: Pure Rust; no runtime dependencies

## Installation

### Prerequisites

- A stable Rust toolchain (1.85+)

### Install from Source

```bash
git clone <repo-url>
cd poet
cargo install --path .
```

### Verify Installation

```bash
poet --version
poet --help
```

## Quick Start (5-Minute Tutorial)

Let's create a simple report with headings, a table, and formatting.

### Step 1: Create a Document

```bash
poet document new report.docx
```

Output:
```json
{
  "status": "ok",
  "message": "Document created",
  "data": {"path": "report.docx", "message": "Document created"}
}
```

### Step 2: Add Content

```bash
poet heading add "Quarterly Sales" --level 1
poet paragraph add "Revenue grew 15% year over year." --id summary
poet table add 4 3 --id sales
poet table set-range --id sales --header '[
  ["Product", "Q1", "Q2"],
  ["Widget A", 15000, 18000],
  ["Widget B", 12000, 14500],
  ["Total", 27000, 32500]
]'
```

### Step 3: Apply Formatting

```bash
poet run add " Strong quarter." --id summary --bold --color C00000
poet page footer "Confidential"
poet page page-numbers --align center
```

### Step 4: Save

```bash
poet document save --path report.docx
```

### Step 5: Analyze the Table

```bash
poet calc stats report.docx --id sales
```

## Core Concepts

### Bookmark IDs (Addressing)

Every element Poet creates (paragraph, heading, table, list item, image, TOC) is wrapped in an OOXML bookmark and given a stable id. Reference it with `--id`:

```bash
poet paragraph add "Intro" --id intro   # create with explicit id
poet paragraph get --id intro
poet run add "more" --id intro --bold   # append a bold run
```

If you omit `--id`, Poet auto-assigns one (`p1`, `h1`, `t1`, ...). The JSON response always echoes the assigned id.

For documents created outside Poet (no bookmarks), use `--index` as a positional fallback:

```bash
poet paragraph get --index 3
poet table set-cell --index 0 --row 1 --col 2 --value "x"
```

### Sessions

**Each `poet` CLI call is a fresh process.** Session state does NOT persist between separate shell invocations. To run multiple commands, either chain with `&&` or (recommended) use a batch script:

```bash
# chaining
poet document new report.docx && poet paragraph add "Hello" && poet document save

# batch (recommended)
poet batch run script.json
```

### JSON Output Format

Every command returns structured JSON:

**Success:**
```json
{"status": "ok", "message": "...", "data": { ... }}
```

**Error:**
```json
{"status": "error", "message": "...", "code": "...", "details": {}}
```

### Semantic Metadata

Poet stores rich metadata in a custom XML part inside the .docx (invisible to readers, survives save/reopen):

- **Document metadata**: title, description, author, tags
- **Section metadata**: purpose, description, orientation
- **Table schemas**: columns, inferred data types
- **Paragraph annotations**: kind, description
- **History**: timestamped log of all operations

## Command Categories

| Category | Description |
|----------|-------------|
| **document** | new, open, save, close, info, export (md/txt) |
| **section** | list, info, add, page-break |
| **paragraph** | add, insert, get, update, delete, list, move, clear, find, replace, count |
| **run** | add, get, clear, format, emphasize (bold/italic/underline/font/size/color) |
| **style** | list, apply |
| **heading** | add, set-level, list |
| **list** | add, add-item, convert, set-level |
| **table** | add, list, get, set-cell, set-range, add-row, add-column, delete-row, delete-column |
| **image** | add, list, get, resize, delete |
| **toc** | add, update |
| **page** | margins, orientation, size, header, footer, page-numbers, columns |
| **meta** | describe, get/set-document, get/set-section, get/set-table, history |
| **batch** | run, template |
| **calc** | read, stats, aggregate, filter, transform |

See `poet <category> --help` for detailed command lists, or run `poet howto` for the full AI-oriented reference.

## Common Workflows

### Build a Report (Two-Phase Batch)

**Phase 1 — content** (`report.phase1.json`):
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

**Phase 2 — formatting** (`report.phase2.json`):
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

### Analyze Tables in Any Document

```bash
poet calc read data.docx --id sales
poet calc stats data.docx --id sales --column Q1
poet calc filter data.docx --index 0 --column Q1 --operator ">" --value 14000
poet calc aggregate data.docx --id sales --group-by Region --agg-column Q1 --agg-func sum
```

### Metadata-Driven Documents

```bash
poet meta set-document '{"title": "Q4 Report", "author": "Finance", "tags": ["q4","2024"]}'
poet meta set-table sales '{"name": "Sales", "description": "Quarterly sales by region"}'
poet meta describe
```

## Configuration

Poet stores session/config in `~/.poet/` (override the home with `POET_HOME`):

```
~/.poet/
├── session.json       # Active session (current document)
└── config.json        # Future: user preferences
```

## Architecture

- **Core**: `DocumentManager` (docx-rs wrapper) + `BookmarkManager` (OOXML bookmark addressing)
- **Meta**: the meta engine persists metadata in a custom XML part; the annotator tracks operations
- **Calc**: the table reader loads docx tables into Polars DataFrames; strategies for stats/aggregate/filter/transform; rhai for calculated columns
- **CLI**: clap-based command tree with 14 categories
- **Models**: serde types for all payloads

## Testing

```bash
cargo test                              # all tests
cargo test --lib                        # unit tests only
```

## Howto (AI System Prompt)

```bash
poet howto
```

Prints a comprehensive, AI-oriented reference covering every command, addressing model, batch format, and gotchas. Use this when driving Poet from an agent.

## Built With

- [clap](https://docs.rs/clap) — CLI framework
- [docx-rs](https://docs.rs/docx-rs) — .docx read/write
- [Polars](https://pola.rs) — table analysis
- [rhai](https://rhai.rs) — expression engine
- [serde](https://serde.rs) — JSON envelopes & models

## License

MIT License - see LICENSE file for details.
