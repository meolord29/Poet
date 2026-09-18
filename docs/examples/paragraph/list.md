**example:**

```sh
poet paragraph list
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "paragraphs": [
      {
        "index": 0,
        "id": "p1",
        "style": "Normal",
        "text": "Executive Summary"
      },
      {
        "index": 1,
        "id": "head-title",
        "style": "Normal",
        "text": "Revenue grew 15% quarter over quarter."
      },
      {
        "index": 2,
        "id": null,
        "style": "Heading 1",
        "text": "Q3 Report"
      },
      {
        "index": 3,
        "id": "l1",
        "style": "List Number",
        "text": "Ship dashboard"
      },
      {
        "index": 4,
        "id": "l2",
        "style": "List Bullet 2",
        "text": "Add export filters"
      },
      {
        "index": 5,
        "id": "notes",
        "style": "List Number 2",
        "text": ""
      }
    ]
  }
}
```

Lists body paragraphs with resolved style names; style display-name resolution is adr/0008/0010.
