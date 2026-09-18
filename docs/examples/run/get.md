**example:**

```sh
poet run get --id rev
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "runs": [
      {
        "text": "Ship dashboard",
        "bold": null,
        "italic": null,
        "underline": null,
        "font": null,
        "size": null,
        "color": null
      },
      {
        "text": "strong quarter",
        "bold": true,
        "italic": null,
        "underline": null,
        "font": null,
        "size": null,
        "color": null
      }
    ]
  }
}
```

Lists the paragraph's runs with resolved formatting; `null` fields mean inherited/unset rather than false.
