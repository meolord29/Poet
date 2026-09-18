**example:**

```sh
poet batch template report .sandbox/report-script.json
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "template": "report",
    "output": ".sandbox/report-script.json",
    "commands": 6,
    "message": "Template generated"
  }
}
```

Writes a ready-made script (`basic` | `report` | `data_table`) to the output path for editing and `batch run`.
