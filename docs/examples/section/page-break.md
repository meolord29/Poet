**example:**

```sh
poet section page-break
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "p2",
    "message": "Page break inserted"
  }
}
```

Inserts a real page-break run wrapped in its own paragraph; the returned `id` addresses the hosting paragraph.
