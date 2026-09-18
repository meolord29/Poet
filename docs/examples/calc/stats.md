**example:**

```sh
poet calc stats .sandbox/q3.docx --id sales --column Q1
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "path": ".sandbox/q3.docx",
    "statistics": {
      "Q1": {
        "count": 2,
        "mean": 10500.0,
        "min": 9000.0,
        "max": 12000.0,
        "std": 2121.3203435596424,
        "median": 10500.0
      }
    },
    "message": "Statistics calculated"
  }
}
```

Per-column statistics for the addressed column.
