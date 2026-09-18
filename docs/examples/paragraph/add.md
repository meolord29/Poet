**example:**

```sh
poet paragraph add 'Temporary text.' --id tmp
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "tmp",
    "text": "Temporary text.",
    "style": null,
    "page_break": false,
    "message": "Paragraph added"
  }
}
```

Appends a paragraph and echoes its bookmark `id`, which every other command accepts as `--id`. Supply `--id` to pick a stable, human-readable id instead of the auto `p<N>`.
