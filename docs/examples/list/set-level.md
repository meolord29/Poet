**example:**

```sh
poet list set-level 2 --id notes
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "notes",
    "index": null,
    "level": 2,
    "message": "List level set to 2"
  }
}
```

Re-nests a list item (1-9, validated); the real numbering definition updates indentation with the level.
