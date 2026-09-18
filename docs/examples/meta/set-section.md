**example:**

```sh
poet meta set-section Body '{"purpose": "Main content"}'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "name": "Body",
    "metadata": {
      "name": "Body",
      "purpose": "Main content",
      "description": "",
      "orientation": "",
      "page_size": ""
    },
    "message": "Section metadata set"
  }
}
```

Stores metadata under a section name of your choosing (a label, not the section index).
