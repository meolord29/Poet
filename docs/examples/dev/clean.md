**example:**

```sh
poet dev clean
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "dev sandbox removed",
  "data": {
    "removed": true,
    "path": "/home/mashkini/Workspace/Poet/.sandbox"
  }
}
```

Dev-builds only (adr/0015): removes `.sandbox/` including session state; idempotent, and runs even after failed iterations.
