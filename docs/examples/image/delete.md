**example:**

```sh
poet image delete 0
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "index": 0,
    "message": "Image deleted"
  }
}
```

Removes the image's drawing by positional index.
