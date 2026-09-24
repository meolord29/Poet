**example:**

```sh
poet register --app opencode
```

Result (one envelope on stdout):
```json
{
  "status": "ok",
  "message": "skill registered",
  "data": {
    "app": "opencode",
    "path": "/home/u/.config/opencode/skills/poet/SKILL.md",
    "version": "0.1.0",
    "installed": true
  }
}
```

Writes the rendered poet `SKILL.md` into the agent app's skills dir (idempotent) and merges the `permission.skill.poet: "allow"` entry into `opencode.json` (claude-code auto-discovers skills — no permission store). `--app claude-code` anchors at `~/.claude/skills/poet/SKILL.md`. `--print-skill` emits the rendered bytes on stdout and touches nothing.
