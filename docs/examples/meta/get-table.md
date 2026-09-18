**example:**

```sh
poet meta get-table sales
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "schema": {
      "id": "sales",
      "name": "Sales",
      "description": "Quarterly sales",
      "columns": [],
      "header_row": true
    }
  }
}
```

Reads the stored table schema (`{}` when unset).
