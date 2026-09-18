**example:**

```sh
poet list add-item 'Add export filters'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "l2",
    "text": "Add export filters",
    "list_type": "bullet",
    "level": 1,
    "message": "List item added"
  }
}
```

Appends another item to the list flow, continuing the previous list's numbering.
