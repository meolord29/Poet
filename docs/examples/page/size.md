**example:**

```sh
poet page size --width 8.5 --height 11
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "section": 0,
    "width": 8.5,
    "height": 11.0,
    "unit": "inches",
    "message": "Page size set"
  }
}
```

Sets page width/height in the given unit.
