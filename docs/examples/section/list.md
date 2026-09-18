**example:**

```sh
poet section list
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "sections": [
      {
        "index": 0,
        "orientation": "portrait",
        "start_type": "none"
      }
    ],
    "count": 1
  }
}
```

Lists every section including the body-final one; `start_type` uses stable names (Words leaked Python enum reprs — adr/0008).
