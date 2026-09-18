# Example: Quarterly Report

A complete document built from a single batch script: headings, a summary paragraph, a sales
table with an inferred schema, a bulleted list, a table of contents, and page footer/numbers —
plus document-level metadata.

## Build it

```bash
poet batch run script.json
```

This produces `quarterly_report.docx` in the current directory.

## What the script does

1. Creates the document and sets metadata (title, author, tags).
2. Adds a title (`Heading 1`) and an executive summary section.
3. Adds a 4×3 sales table and bulk-fills it with `set-range` + `--header` (infers column types).
4. Adds a bulleted list of key points.
5. Inserts a Table of Contents (refresh the field in Word to populate it).
6. Sets a footer and centered page numbers.
7. Saves.

## Analyze the table afterward

```bash
poet calc stats quarterly_report.docx --id sales
poet calc aggregate quarterly_report.docx --id sales --group-by Region --agg-column Q1 --agg-func sum
```

## Inspect metadata

```bash
poet document open quarterly_report.docx
poet meta describe
```
