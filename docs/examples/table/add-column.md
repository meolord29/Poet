**example:**

```sh
poet table add-column --id sales
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "index": null,
    "message": "Column added"
  }
}
```

Appends an empty column.
