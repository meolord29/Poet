**example:**

```sh
poet paragraph clear --id notes
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "notes",
    "index": null,
    "message": "Paragraph cleared"
  }
}
```

Empties the paragraph's text but keeps the paragraph (and its id) — unlike `delete`.
