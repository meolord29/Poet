**example:**

```sh
poet run clear --id tmp
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "tmp",
    "index": null,
    "message": "Runs cleared"
  }
}
```

Removes all runs (and therefore all text) from the paragraph.
