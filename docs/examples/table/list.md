**example:**

```sh
poet table list
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "tables": [
      {
        "index": 0,
        "id": "sales",
        "rows": 4,
        "cols": 4,
        "style": "Table Grid"
      }
    ]
  }
}
```

Lists body tables with dimensions and resolved style.
