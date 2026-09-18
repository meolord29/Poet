**example:**

```sh
poet batch run .sandbox/report-script.json
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "script": ".sandbox/report-script.json",
    "commands_executed": 6,
    "results": [
      {
        "status": "ok",
        "message": "Document created",
        "data": {
          "path": "report.docx",
          "message": "Document created"
        }
      },
      {
        "status": "ok",
        "message": "",
        "data": {
          "id": "h1",
          "text": "Quarterly Report",
          "level": 1,
          "message": "Heading 1 added"
        }
      },
      {
        "status": "ok",
        "message": "",
        "data": {
          "id": "h2",
          "text": "Summary",
          "level": 2,
          "message": "Heading 2 added"
        }
      },
      {
        "status": "ok",
        "message": "",
        "data": {
          "id": "p1",
          "text": "This report summarizes Q4 performance.",
          "style": null,
          "page_break": false,
          "message": "Paragraph added"
        }
      },
      {
        "status": "ok",
        "message": "",
        "data": {
          "id": "p2",
          "text": "Revenue grew 15% year over year.",
          "style": null,
          "page_break": false,
          "message": "Paragraph added"
        }
      },
      {
        "status": "ok",
        "message": "Document saved",
        "data": {
          "path": "report.docx",
          "message": "Document saved"
        }
      }
    ],
    "message": "Batch script executed"
  }
}
```

Executes a flat JSON script of commands in one session — the recommended multi-command pattern; execution stops at the first error and `commands_executed` counts the failed attempt.
