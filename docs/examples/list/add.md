**example:**

```sh
poet list add 'Ship dashboard' --ordered
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "l1",
    "text": "Ship dashboard",
    "list_type": "ordered",
    "level": 1,
    "message": "List item added"
  }
}
```

Starts a real numbering definition (adr/0005) — bullet by default, `--ordered` for numbered; `level` is 1-based nesting.
