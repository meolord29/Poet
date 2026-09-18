**example:**

```sh
poet page page-numbers --align center
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "section": 0,
    "align": "center",
    "message": "Page numbers added"
  }
}
```

Inserts a PAGE field into the footer with the given alignment.
