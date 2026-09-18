**example:**

```sh
poet meta set-table sales '{"name": "Sales", "description": "Quarterly sales"}'
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
    },
    "message": "Table schema set"
  }
}
```

Stores a table schema under the table's bookmark id; `calc` reads this schema for column typing (adr/0013).
