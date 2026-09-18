**example:**

```sh
poet heading add 'Q3 Report' --level 1 --id head-title
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "head-title",
    "text": "Q3 Report",
    "level": 1,
    "message": "Heading 1 added"
  }
}
```

Adds a heading paragraph (level 1-9, validated) wrapped in a bookmark; headings appear in `heading list` and `meta describe` annotations.
