**example:**

```sh
poet run emphasize quarter --id rev --bold --all
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "table": null,
    "row": null,
    "col": null,
    "para": null,
    "find": "quarter",
    "replacements": 1,
    "message": "Emphasized 1 occurrence(s)"
  }
}
```

Emphasizes occurrences of a substring, splitting runs as needed (adr/0009); `--all` covers repeat occurrences, and 0 matches is a successful no-op.
