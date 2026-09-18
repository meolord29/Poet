**example:**

```sh
poet page footer 'Q3 2026'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "section": 0,
    "footer": "Q3 2026",
    "message": "Footer set"
  }
}
```

Sets the section footer text (same part-model caveat as `page header`).
