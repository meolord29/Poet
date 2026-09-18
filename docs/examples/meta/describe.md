**example:**

```sh
poet meta describe
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "document": {},
    "sections": [],
    "tables": [
      {
        "id": "sales",
        "name": "",
        "description": "3x3 table",
        "columns": [
          {
            "name": "Region",
            "data_type": "string",
            "unit": "",
            "description": ""
          },
          {
            "name": "Q1",
            "data_type": "number",
            "unit": "",
            "description": ""
          },
          {
            "name": "Q2",
            "data_type": "number",
            "unit": "",
            "description": ""
          }
        ],
        "header_row": true
      }
    ],
    "paragraph_annotations": [
      {
        "id": "head-title",
        "kind": "heading",
        "description": "Heading level 1: 'Q3 Report'",
        "notes": ""
      },
      {
        "id": "rev",
        "kind": "paragraph",
        "description": "Text updated to: 'Revenue grew 13% quarter over quarter.'",
        "notes": ""
      },
      {
        "id": "p1",
        "kind": "paragraph",
        "description": "Paragraph created with text: 'Executive Summary'",
        "notes": ""
      },
      {
        "id": "l1",
        "kind": "ordered list",
        "description": "ordered list (level 1) item: 'Ship dashboard'",
        "notes": ""
      },
      {
        "id": "l2",
        "kind": "bullet list",
        "description": "bullet list (level 1) item: 'Add export filters'",
        "notes": ""
      },
      {
        "id": "notes",
        "kind": "paragraph",
        "description": "Paragraph created with text: 'Notes and caveats.'",
        "notes": ""
      },
      {
        "id": "tmp",
        "kind": "paragraph",
        "description": "Paragraph created with text: 'Temporary text.'",
        "notes": ""
      }
    ],
    "history_count": 12
  }
}
```

Full metadata inventory: document properties, per-section annotations, table schemas (with inferred column types), and paragraph annotations — the machine-readable view an agent reads before mutating.
