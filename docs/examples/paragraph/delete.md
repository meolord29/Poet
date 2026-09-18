**example:**

```sh
poet paragraph delete --id tmp
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "",
  "data": {
    "id": "tmp",
    "index": null,
    "message": "Paragraph deleted"
  }
}
```

Deletes the paragraph (body or, with `--table/--row/--col`, a cell paragraph); the id dies with it.
