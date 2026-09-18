**example:**

```sh
poet document save
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "Document saved",
  "data": {
    "path": ".sandbox/q3.docx",
    "message": "Document saved"
  }
}
```

Persists the open document; with `--path` it saves a copy and retargets the session there. Metadata round-trips through the custom XML part (adr/0012).
