**example:**

```sh
poet document new .sandbox/q3.docx
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "Document created",
  "data": {
    "path": ".sandbox/q3.docx",
    "message": "Document created"
  }
}
```

Creates an empty document in memory and saves it to the path immediately; the session is started so chained `&&` invocations auto-open it. Content mutations auto-save to this path from here on.
