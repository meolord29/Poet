**example:**

```sh
poet document info
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "paragraph_count": 8,
    "table_count": 1,
    "section_count": 2,
    "image_count": 0,
    "bookmark_count": 9,
    "core_properties": {
      "title": "",
      "author": "unknown",
      "subject": "",
      "keywords": "",
      "category": "",
      "comments": "",
      "created": "2026-09-18T08:34:54Z",
      "modified": "2026-09-18T08:34:54Z"
    }
  }
}
```

Structural summary counts. Content commands keep their message inside `data`, so the envelope-level `message` is empty (Words parity).
