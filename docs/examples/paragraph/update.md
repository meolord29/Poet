**example:**

```sh
poet paragraph update 'Revenue grew 13% quarter over quarter.' --id rev
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "text": "Revenue grew 13% quarter over quarter.",
    "message": "Paragraph updated"
  }
}
```

Replaces the paragraph's text in place, preserving its id and style.
