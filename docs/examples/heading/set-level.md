**example:**

```sh
poet heading set-level 2 --id head-title
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "head-title",
    "index": null,
    "level": 2,
    "message": "Heading level set to 2"
  }
}
```

Changes an existing heading's level; the style is remapped (`Heading <N>`).
