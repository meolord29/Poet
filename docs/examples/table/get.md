**example:**

```sh
poet table get --id sales
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "sales",
    "index": null,
    "rows": 4,
    "cols": 4,
    "data": [
      [
        "Region",
        "Q1",
        "Q2",
        ""
      ],
      [
        "EMEA",
        "12000",
        "13500",
        ""
      ],
      [
        "APAC",
        "9000",
        "11000",
        ""
      ],
      [
        "LATAM",
        "7000",
        "8200",
        ""
      ]
    ]
  }
}
```

Returns the table's cell texts as a 2D array — the shape `calc read` consumes.
