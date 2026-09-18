**example:**

```sh
poet calc read .sandbox/q3.docx --id sales
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "path": ".sandbox/q3.docx",
    "table_id": "sales",
    "table_index": null,
    "rows": 2,
    "columns": [
      "Region",
      "Q1",
      "Q2"
    ],
    "data": [
      {
        "Region": "EMEA",
        "Q1": 12000,
        "Q2": 13500
      },
      {
        "Region": "APAC",
        "Q1": 9000,
        "Q2": 11000
      }
    ],
    "message": "Table read successfully"
  }
}
```

Reads a table from a saved .docx into a frame; column types come from the stored schema or inference (one dtype per column, adr/0013).
