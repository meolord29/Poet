**example:**

```sh
poet calc transform .sandbox/q3.docx --id sales --operations '[{"type":"sort","by":"Q1","descending":true}]'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "path": ".sandbox/q3.docx",
    "operations": [
      {
        "type": "sort",
        "by": "Q1",
        "descending": true
      }
    ],
    "result": [
      {
        "Region": "EMEA",
        "Q1": 12000,
        "Q2": 13500
      },
      {
        "Region": "APAC",
        "Q1": 9000,
        "Q2": 11000
      }
    ],
    "message": "Data transformed successfully"
  }
}
```

Applies an ordered list of operations (`sort`, `filter`, `add_column` with a rhai expression, ...) and echoes the parsed operations.
