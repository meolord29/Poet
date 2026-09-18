**example:**

```sh
poet document open .sandbox/q3.docx
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "Document opened",
  "data": {
    "path": ".sandbox/q3.docx",
    "message": "Document opened"
  }
}
```

Opens an existing .docx and starts a session for it (chained invocations auto-open). Reader limitations of the engine are recorded in adr/0008.
