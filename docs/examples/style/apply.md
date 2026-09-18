**example:**

```sh
poet style apply 'Intense Quote' --id rev
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "style": "Intense Quote",
    "message": "Style 'Intense Quote' applied"
  }
}
```

Applies a style by display name; unknown names are a `validation_error` (Poet validates what Words tolerated).
