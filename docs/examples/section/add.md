**example:**

```sh
poet section add --start-type new_page
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "start_type": "new_page",
    "message": "Section added"
  }
}
```

Starts a new section at the given start type; inputs are validated in Poet where Words silently tolerated bad values (adr/0011).
