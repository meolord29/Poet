**example:**

```sh
poet table delete-column --id sales 3
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "index": null,
    "col": 3,
    "message": "Column deleted"
  }
}
```

Deletes a column by positional index.
