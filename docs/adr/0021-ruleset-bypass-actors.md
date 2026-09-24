# ADR-0021: ruleset branch protection with explicit bypass actors

Date: 2026-09-24 · Status: Accepted

## Context

adr/0002 documented the branch topology but said nothing about enforcement.
Classic branch protection is a second-class citizen on GitHub now — rulesets
are the maintained mechanism — and protection that forgets the bots breaks
the adr/0016 ladder on the first merge: three automation pushes *must* keep
landing on `nightly` directly (the patch ladder's `chore(release): bump`
commits, the promotion minor bump, and the post-promotion recut
fast-forward), while a `pull_request` rule blocks every direct push unless
the pusher is a bypass actor. A further wrinkle: `GITHUB_TOKEN` pushes
neither re-fire workflows nor bypass rulesets on user-owned repos (the
Actions integration can't grant itself bypass) — carpenter hit exactly this
and moved ladder pushes to a GitHub App (its adr/023).

## Decision

Rulesets only, one per trunk, with explicit bypass actors:

- **`trunk protection (main)`** (targets `~DEFAULT_BRANCH`): PR rule with 1
  code-owner review (`CODEOWNERS` is `* @meolord29` — owner-only approval),
  required checks — the ci `gates` lanes, the release `smoke` lanes, and
  `guard (only nightly merges into main)`, which makes the channel rule
  merge-blocking — strict up-to-date branches, no force-pushes, no
  deletions. Owner bypass: `pull_request` only (merges via PR).
- **`trunk protection (nightly)`** (targets `nightly`): PR rule with the
  `gates` + `smoke` checks required, strict; no force-pushes, no deletions.
  Bypass actors: the **owner** (drift-fix direct pushes) and the
  **`poet-release-bot` GitHub App** (all ladder pushes: bump, promote-bump,
  recut). Nobody else can push or merge either trunk.

The App (setup in [docs/README.md](../README.md#releases-nightly--stable-main)
and adr/0016) holds Contents:read+write and Pull requests:read+write, and is
installed on the repository; its token is created per-run via
`actions/create-github-app-token` from the `RELEASE_BOT_APP_ID` /
`RELEASE_BOT_PRIVATE_KEY` secrets.

## Consequences

+ `main` is mechanically frozen: not even the owner pushes to it — a
  promotion merge is the only write, and the required `guard` check enforces
  that it comes from `nightly` carrying exactly main's minor+1 (adr/0016).
+ The ladder works under protection: the App bypasses the ruleset *and* its
  pushes re-fire workflows (required checks re-run on the bump head).
− Ops prerequisite: without the App + secrets, promotions fail loudly (the
  promote-bump/recut pushes are rejected) — deliberate.
− Bypass actors are trusted with the trunk; keep the App's permission scope
  minimal (two repos scopes, nothing else).

## Rejected

- **Classic branch protection** — deprecated direction on GitHub, cannot
  express bypass actors as cleanly, and carpenter's live drill showed the
  two layers drifting (classic said review-0, ruleset said review-1).
- **Unprotected `nightly`** — required checks on PRs would stay advisory;
  the smoke lanes only mean something when they block.
- **GITHUB_TOKEN ladder pushes** — no workflow re-fire, no ruleset bypass:
  the bump commit lands but the PR's checks go stale (unmergeable).
