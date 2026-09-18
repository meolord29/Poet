**example:**

```sh
poet table add-row --id sales --values '["LATAM", 7000, 8200]'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "index": null,
    "message": "Row added"
  }
}
```

Appends a row; `--values` fills it cell by cell (JSON array).
