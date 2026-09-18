**example:**

```sh
poet calc aggregate .sandbox/q3.docx --id sales --group-by Region --agg-column Q1 --agg-func sum
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "path": ".sandbox/q3.docx",
    "group_by": "Region",
    "agg_column": "Q1",
    "agg_func": "sum",
    "result": [
      {
        "Region": "EMEA",
        "Q1": 12000
      },
      {
        "Region": "APAC",
        "Q1": 9000
      }
    ],
    "message": "Data aggregated successfully"
  }
}
```

Groups by one column and aggregates another; evaluation runs on polars, not Python (adr/0013).
