**example:**

```sh
poet dev setup
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "dev sandbox ready",
  "data": {
    "path": "/home/mashkini/Workspace/Poet/.sandbox",
    "home": "/home/mashkini/Workspace/Poet/.sandbox/.poet",
    "created": true
  }
}
```

Dev-builds only (adr/0015): creates `.sandbox/` with an isolated session home. The QA agent's only workspace — it holds no filesystem permissions of its own.
