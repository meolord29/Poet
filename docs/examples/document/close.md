**example:**

```sh
poet document close
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "Document closed",
  "data": {
    "message": "Document closed"
  }
}
```

Closes the document and ends the session — a second `close` (or any content command) returns the `document_state` error shown in the error table.
