# ADR-0020: installer consent plan + branded banner

Date: 2026-09-24 · Status: Accepted

## Context

The `curl | sh` one-liner is the primary install path, and trust is the whole
game for a script piped straight into `sh`. A bare `downloading …` first line
gives the user no visibility into what would be touched, and the first
consent moment would arrive only *after* the binary is already installed (the
per-app skill-registration prompts). Carpenter's installer (its adr/024)
solved this with a consent plan + branded banner; Poet ports the design.

## Decision

`scripts/install.sh` shows what it will do, asks before doing it, and reports
what it did (adr-gated constraint: the `TAG="nightly"` line stays
byte-identical — `release.yml` seds it for stable releases):

- **Banner** — embedded figlet-style `POET` wordmark (static heredoc, no
  runtime figlet) plus a channel-correct tagline derived from `TAG`:
  `poet installer · nightly` on the canary script, plain `poet installer` on
  the stable release's tag-patched copy. One script, two channels, no drift.
- **Install plan** — before any network activity: download source, checksum,
  binary destination, per *detected* app the exact `SKILL.md` path plus the
  `opencode.json` permission merge, and PATH status. App detection moved
  ahead of consent (pure `command -v`, no side effects); register actions
  stay post-install with their per-app `[Y/n]` (Enter registers —
  registration is the encouraged default; only `n` skips).
- **Consent** — interactive (TTY + readable `/dev/tty`) →
  `proceed with the install plan? [Y/n]`; Enter or any non-`n` answer
  proceeds, only an explicit `n` declines — aborting exit 1 before anything
  is downloaded. A dead tty (EOF on the read) also aborts: Enter means "a
  human chose the default", EOF means "nobody is answering".
  `POET_INSTALL_YES=1` skips the prompt. Non-interactive (CI smoke,
  `curl | sh` automation) prints the plan and proceeds — unattended lanes
  never hang.
- **Palette** — amber (wordmark, prompts, plan heading), honey (channel
  mark, `!` warnings), cream (plan values), muted (keys, tagline), good
  (`✓` summary), bad (errors, abort). Truecolor when `COLORTERM` advertises
  it, xterm-256 approximations otherwise, empty strings when piped so CI
  logs stay plain.
- **Summary** — post-install: `✓ checksum verified`, `✓ installed vX → bin`,
  `! PATH` hint when off-PATH.
- **Verification** — `tests/install_sh.rs` runs the installer non-
  interactively against a `file://` fixture (tarball + SHA256SUMS), pinning
  plan output, install, and the release-time `TAG="nightly"` sed anchor;
  `ci.yml` runs `shellcheck scripts/install.sh` (Linux lane).

## Consequences

+ A user piping the script sees exactly what will be written where, and can
  stop it with one keypress before any network activity.
+ One script serves both channels; the tagline cannot disagree with the tag.

− The sed anchor is a string contract between `release.yml` and this file;
  the installer test pins it.
