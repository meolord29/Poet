**example:**

```sh
poet image resize 0 --width 3.0
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "index": 0,
    "width": 3.0,
    "height": null,
    "message": "Image resized"
  }
}
```

Sets new dimensions in inches (the engine preserves aspect ratio when only one of width/height is given).
