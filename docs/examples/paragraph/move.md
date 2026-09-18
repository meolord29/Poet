**example:**

```sh
poet paragraph move up --id rev
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": 1,
    "direction": "up",
    "message": "Paragraph moved up"
  }
}
```

Moves a paragraph up or down one slot; `index` echoes the new position. The direction is positional.
