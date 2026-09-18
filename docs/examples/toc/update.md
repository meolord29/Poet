**example:**

```sh
poet toc update
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "message": "TOC fields refresh automatically when the document is opened in Word."
  }
}
```

No-op by design — Word refreshes TOC fields on open; the hint documents this instead of pretending to recompute (Words parity).
