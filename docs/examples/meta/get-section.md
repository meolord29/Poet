**example:**

```sh
poet meta get-section Body
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
    }
  }
}
```

Reads section metadata (`{}` when unset).
