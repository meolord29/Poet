**example:**

```sh
poet uninstall --bin-dir ~/.local/bin
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "uninstalled",
  "data": {
    "uninstalled": true,
    "bin": "/home/u/.local/bin/poet",
    "skill": [
      {
        "removed": true,
        "app": "opencode",
        "path": "/home/u/.config/opencode/skills/poet/SKILL.md"
      }
    ]
  }
}
```

Removes the poet skill from every registered agent app first (recoverable via `poet register`), then deletes the binary at `--bin-dir` (default `~/.local/bin`; `"bin": null` when it was not present). `not_found` when neither a skill nor a binary exists. Poet session data (`~/.poet`) is never touched.
