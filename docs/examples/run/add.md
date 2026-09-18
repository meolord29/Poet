**example:**

```sh
poet run add 'strong quarter' --id rev --bold
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "text": "strong quarter",
    "bold": true,
    "italic": null,
    "underline": null,
    "font": null,
    "size": null,
    "color": null,
    "message": "Run added"
  }
}
```

Appends a run to the addressed paragraph with optional inline formatting; `--id`/`--index` address the paragraph.
