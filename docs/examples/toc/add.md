**example:**

```sh
poet toc add --levels 1-3
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "toc1",
    "levels": "1-3",
    "message": "Table of contents inserted (update field in Word to populate)"
  }
}
```

Inserts a real TOC field over the given heading levels; the field text is what Word re-computes on open (engine field round-trip gap: adr/0008).
