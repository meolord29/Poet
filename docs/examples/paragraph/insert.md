**example:**

```sh
poet paragraph insert 0 'Executive Summary'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "p1",
    "index": 0,
    "text": "Executive Summary",
    "style": null,
    "page_break": false,
    "message": "Paragraph inserted"
  }
}
```

Inserts at a 0-based positional index (positional, not a flag); indices out of range are a `validation_error`.
