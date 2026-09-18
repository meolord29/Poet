**example:**

```sh
poet list convert --ordered --id notes
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "notes",
    "index": null,
    "list_type": "ordered",
    "message": "Paragraph converted to list"
  }
}
```

Turns an existing paragraph into a list item by attaching a numbering definition to it.
