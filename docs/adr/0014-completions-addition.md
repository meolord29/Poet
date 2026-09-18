# ADR 0014 — additions over Words: shell completions

Date: 2026-09-18 · Status: accepted

Words (the behavioral spec) has no shell-completion support. `poet completions
<shell>` (bash, zsh, fish, powershell, elvish — clap's `Shell` enum) is a
deliberate, purely additive CLI surface beyond Words:

- It emits no document data and cannot diverge from ported behavior — the
  parity contract (command names, flags, JSON `data` shapes, exit semantics)
  is untouched.
- It prints the script **raw to stdout, exit 0, no JSON envelope** — the
  same raw-text precedent as `poet howto`.
- Generated from the live clap command tree (`clap_complete::generate`), so
  completions can never drift from the actual flags; new subcommands are
  picked up for free.

Rejected alternatives: hand-maintained completion scripts (drift), a
`--generate-completions` hidden flag on the root command (hidden flags are
invisible to the agents this CLI serves), and shelling out to a helper
binary (needless surface).
