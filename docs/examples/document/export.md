**example:**

```sh
poet document export .sandbox/q3.docx md --out .sandbox/q3.md
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "path": ".sandbox/q3.md",
    "format": "md",
    "message": "Exported"
  }
}
```

Writes a plain-text rendering (`md` or `txt`) of the saved document; the export walker covers paragraphs, headings, lists, and tables.
