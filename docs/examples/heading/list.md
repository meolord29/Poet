**example:**

```sh
poet heading list
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "headings": [
      {
        "index": 1,
        "id": "head-title",
        "style": "Heading 2",
        "text": "Revenue grew 15% quarter over quarter."
      },
      {
        "index": 2,
        "id": null,
        "style": "Heading 1",
        "text": "Q3 Report"
      }
    ]
  }
}
```

Lists paragraphs whose resolved style is a heading style, in document order.
