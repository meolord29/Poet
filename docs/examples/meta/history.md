**example:**

```sh
poet meta history --limit 5
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "history": [
      {
        "timestamp": "2026-09-18T16:34:54.725959",
        "command": "run add",
        "target": "rev",
        "summary": "Added run 'strong quarter'",
        "details": {}
      },
      {
        "timestamp": "2026-09-18T16:34:54.768985",
        "command": "run add",
        "target": "rev",
        "summary": "Added run 'emphasize 'quarter''",
        "details": {}
      },
      {
        "timestamp": "2026-09-18T16:34:54.785797",
        "command": "paragraph add",
        "target": "tmp",
        "summary": "Added paragraph 'Temporary text.'",
        "details": {}
      },
      {
        "timestamp": "2026-09-18T16:34:54.896305",
        "command": "table add",
        "target": "sales",
        "summary": "Added 3x3 table",
        "details": {}
      },
      {
        "timestamp": "2026-09-18T16:34:54.927013",
        "command": "table data",
        "target": "sales",
        "summary": "Wrote 2 data rows",
        "details": {}
      }
    ]
  }
}
```

Returns the annotated-operation history (newest entries, chronological), capped by `--limit`.
