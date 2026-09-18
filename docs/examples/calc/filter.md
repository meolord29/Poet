**example:**

```sh
poet calc filter .sandbox/q3.docx --id sales --column Q1 --operator '>' --value 9000
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "path": ".sandbox/q3.docx",
    "column": "Q1",
    "operator": ">",
    "value": "9000",
    "result": [
      {
        "Region": "\"EMEA\"",
        "Q1": 12000,
        "Q2": 13500
      }
    ],
    "message": "Data filtered successfully"
  }
}
```

Returns rows matching `--column --operator --value`.
