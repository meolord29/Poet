**example:**

```sh
poet section info --index 0
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "index": 0,
    "orientation": "portrait",
    "page_width": 8.268055555555556,
    "page_height": 11.693055555555556,
    "top_margin": 1.3784722222222223,
    "bottom_margin": 1.18125,
    "left_margin": 1.18125,
    "right_margin": 1.18125
  }
}
```

Section geometry in inches; values come from the established serde view of the engine model (adr/0011).
