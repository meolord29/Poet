**example:**

```sh
poet page header 'Company Confidential'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "section": 0,
    "header": "Company Confidential",
    "message": "Header set"
  }
}
```

Sets the section header text; headers live in a separate part the engine's reader does not round-trip (adr/0011 gap).
