**example:**

```sh
poet paragraph get --id rev
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "text": "Revenue grew 12% quarter over quarter."
  }
}
```

Reads one paragraph by `--id` or `--index`; addressing by id is stable across edits, indices shift.
