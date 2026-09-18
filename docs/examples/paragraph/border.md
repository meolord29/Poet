**example:**

```sh
poet paragraph border --id rev --position bottom --color FF0000
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "rev",
    "index": null,
    "position": "bottom",
    "color": "FF0000",
    "size": 4,
    "space": 1,
    "style": "single",
    "message": "Paragraph bottom border set"
  }
}
```

Applies a paragraph border; an unknown border `--style` is coerced to `single` by the engine (adr/0009), while position/size/space are validated.
