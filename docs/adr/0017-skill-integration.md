# ADR-0017: skill integration (register into agent apps)

Date: 2026-09-24 · Status: Accepted

## Context

Poet's contract with its primary users — AI coding agents — is `poet howto`,
but an agent only finds Poet if the skill is discoverable by its app. The
binary is self-contained, so the skill can ship *inside* the binary and be
written out at registration time: no separate skill artifact to keep in sync
with the installed version. opencode loads skills from
`$XDG_CONFIG_HOME/opencode/skills/<name>/SKILL.md` behind a permission entry;
claude code auto-discovers `~/.claude/skills/` (no permission store).

## Decision

`poet register --app opencode|claude-code [--print-skill]`
(carpet of carpenter adr/006 + adr/009):

- **Renderer** (`core/skill.rs`): the `SKILL.md` is assembled from typed
  authored consts — frontmatter `name`/`description`, a "what this is" prose,
  the document-building workflow — plus the **running binary's howto manual
  inlined verbatim** (leading H1 stripped), the embedded version
  (`env!("CARGO_PKG_VERSION")`), and `current_exe` (with Linux's
  `" (deleted)"` suffix stripped — the artifact `upgrade`'s in-place replace
  leaves in `/proc/self/exe`). Rendered deterministic within a process.
- **Register** is idempotent: atomic-write `SKILL.md`, then merge
  `permission.skill.poet: "allow"` into `opencode.json` for opencode only,
  preserving every unrelated key (best-effort JSON surgery, corrupt file →
  `file_error`).
- **`--print-skill`** emits the rendered bytes and touches nothing (the
  authoring/inspection path).
- **Anchors** (`$XDG_CONFIG_HOME` or `$HOME/.config`; `$HOME`) resolve inside
  the command wrapper; core functions always take `(root, home)` explicitly,
  so unit tests never touch real anchors and integration tests set the env on
  the spawned process only.

## Consequences

+ One binary carries its own agent-facing documentation; register always
  matches the installed version.
+ `upgrade` can refresh skills in place (adr/0018); `uninstall` reverses the
  footprint (adr/0019).
− A hand-edited `SKILL.md` is clobbered by re-register — deliberate: the
  binary is the source of truth.

## Rejected

- **Shipping a static SKILL.md in the repo** — drifts from the binary the
  moment the surface moves; the howto-inline keeps one source.
- **`poet deregister` as a command** — carpenter keeps one, but Poet's
  reversal path is `uninstall` (adr/0019); a single app can be dropped by
  deleting the `SKILL.md` file. Surface stays minimal; the core
  `deregister` helper exists for `uninstall`.
