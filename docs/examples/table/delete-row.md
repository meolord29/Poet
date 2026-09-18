**example:**

```sh
poet table delete-row --id sales 3
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "index": null,
    "row": 3,
    "message": "Row deleted"
  }
}
```

Deletes a row by positional index; out-of-range indices are a `validation_error`.
