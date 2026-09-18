**example:**

```sh
poet image add .sandbox/logo.png --width 2.0
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "img1",
    "path": ".sandbox/logo.png",
    "width": 2.0,
    "height": null,
    "message": "Image added"
  }
}
```

Embeds an image file inline in a new bookmarked paragraph; width/height are inches, and the engine records EMU dimensions (adr/0008).
