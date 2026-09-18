**example:**

```sh
poet meta get-document
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "metadata": {
      "title": "Q3 Report",
      "description": "",
      "author": "Finance",
      "tags": [],
      "data_sources": []
    },
    "message": "Document metadata retrieved"
  }
}
```

Reads document metadata (`{}` when unset).
