# Scenario: Quarterly report

A complete document built from a single batch script: headings, a summary
paragraph, a sales table with an inferred schema, a bulleted list, a table of
contents, and page footer/numbers — plus document-level metadata.

Write the script (a flat JSON list of `{cmd, action, ...}` objects, stop on
first error):

```sh
cat > quarterly-report.json <<'EOF'
[
  {"cmd": "document", "action": "new", "path": "quarterly_report.docx"},
  {"cmd": "meta", "action": "set-document",
   "metadata": {"title": "Q4 Quarterly Report", "author": "Finance Team", "tags": ["q4", "2024"]}},
  {"cmd": "heading", "action": "add", "text": "Quarterly Report", "level": 1},
  {"cmd": "heading", "action": "add", "text": "Executive Summary", "level": 2},
  {"cmd": "paragraph", "action": "add",
   "text": "Revenue grew 15% year over year, driven by strong performance in the North region.",
   "id": "summary"},
  {"cmd": "heading", "action": "add", "text": "Sales by Region", "level": 2},
  {"cmd": "table", "action": "add", "rows": 4, "cols": 3, "id": "sales"},
  {"cmd": "table", "action": "set-range", "id": "sales", "header": true,
   "values": [
     ["Region", "Q1", "Q2"],
     ["North", 50000, 62000],
     ["South", 38000, 41000],
     ["East", 45000, 47000]
   ]},
  {"cmd": "list", "action": "add", "text": "North region exceeded targets", "ordered": false},
  {"cmd": "list", "action": "add-item", "text": "South region recovery plan underway"},
  {"cmd": "toc", "action": "add", "levels": "1-3"},
  {"cmd": "page", "action": "footer", "text": "Confidential - Q4 2024"},
  {"cmd": "page", "action": "page-numbers", "align": "center"},
  {"cmd": "document", "action": "save"}
]
EOF
```

Build it (one JSON envelope per command; the batch stops at the first error):

```sh
poet batch run quarterly-report.json
```

This produces `quarterly_report.docx` in the current directory — mutations
auto-save, and `document save` at the end makes it explicit.

Analyze the table afterward (`calc` reads the saved file; column types come
from the schema `set-range --header` inferred):

```sh
poet calc stats quarterly_report.docx --id sales --column Q1
poet calc aggregate quarterly_report.docx --id sales --group-by Region --agg-column Q1 --agg-func sum
```

Inspect the metadata the run left behind:

```sh
poet document open quarterly_report.docx && poet meta describe
```
