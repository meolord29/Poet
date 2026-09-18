**example:**

```sh
poet table set-range --id sales --header '[["Region","Q1","Q2"],["EMEA",12000,13500],["APAC",9000,11000]]'
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "index": null,
    "rows_written": 3,
    "header": true,
    "message": "Table range set"
  }
}
```

Writes a 2D JSON array into the table starting at (0,0), growing rows/cols as needed; `--header` marks the first row as a header and infers column types for the table schema (adr/0007).
