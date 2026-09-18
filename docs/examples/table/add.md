**example:**

```sh
poet table add 3 3 --id sales
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "rows": 3,
    "cols": 3,
    "style": "Table Grid",
    "message": "Table added"
  }
}
```

Creates a table wrapped in a bookmark; rows and cols are positional. The style is stored as the `TableGrid` id and resolved for display (adr/0007).
