**example:**

```sh
poet table set-cell --id sales 0 0 Region
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "index": null,
    "row": 0,
    "col": 0,
    "value": "Region",
    "message": "Cell set"
  }
}
```

Sets one cell by positional `<ROW> <COL> <VALUE>` (positionals, despite what older flag-style docs showed); out-of-bounds coordinates are a `validation_error`.
