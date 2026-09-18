**example:**

```sh
poet run format --id rev --italic
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "run_index": null,
    "applied": {
      "italic": true
    },
    "message": "Run formatting applied"
  }
}
```

Restyles existing runs; `applied` echoes exactly the options provided (absent keys were untouched) using Words' key names.
