**example:**

```sh
poet paragraph replace 13% 15%
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "find": "13%",
    "replace": "15%",
    "count": 1,
    "message": "Replace completed"
  }
}
```

Replaces text across every body paragraph and reports the replacement count; 0 matches is a successful no-op.
