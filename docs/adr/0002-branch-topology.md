# ADR 0002 — carpenter-style branch topology

Date: 2026-09-16 · Status: accepted

## Decision

Poet adopts the branch topology of `/home/mashkini/Workspace/carpenter`:

- `nightly` is the integration branch — all work lands here.
- `main` is stable and only ever accepts merges from `nightly`; a CI `guard` job
  fails any PR to `main` whose head branch is not `nightly` (added in phase 1).
- Phase work happens on short-lived `ivan/phase-N-<slug>` branches cut from `nightly`
  and merged back per phase (one merge = one phase checkpoint).
- Conventional Commits with scopes and ADR citations.

## Consequences

- Release automation from carpenter (version ladder bot, rolling `nightly` prerelease,
  promote-bump on PR open) is **deferred** until after phase 4 — the repo has no
  releases during the rebuild.
- Promotion after phase 4 is a single PR `nightly → main` tagged `v0.1.0`.
- `main` does not exist until first promotion; bootstrap history lives on `nightly`
  only (same cold-start carpenter used).
