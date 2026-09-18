**example:**

```sh
poet page margins --top 1.0 --bottom 1.0
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "section": 0,
    "top": 1.0,
    "bottom": 1.0,
    "left": null,
    "right": null,
    "unit": "inches",
    "message": "Margins set"
  }
}
```

Sets section margins; `--unit` accepts inches/cm/mm/pt and values are validated (adr/0011).
