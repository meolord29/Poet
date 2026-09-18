**example:**

```sh
poet meta set-document '{"title": "Q3 Report", "author": "Finance"}'
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
    "message": "Document metadata set"
  }
}
```

Stores document-level metadata in the custom XML part (adr/0012); the stored value is echoed back.
