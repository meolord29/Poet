**example:**

```sh
poet paragraph find Revenue
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "text": "Revenue",
    "results": [
      {
        "type": "paragraph",
        "index": 2,
        "text": "Revenue grew 13% quarter over quarter."
      }
    ],
    "count": 1,
    "message": "Search completed"
  }
}
```

Searches body paragraphs and table cells, returning typed results (`paragraph` / `table_cell`) in document order.
