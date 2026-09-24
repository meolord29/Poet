# ADR-0016: Automated version ladder

Date: 2026-09-24 · Status: Accepted

## Context

adr/0002 fixed the branch topology (feature PRs → `nightly`; promotion PRs
`nightly → main`) but left versioning manual. "Bump `version` in the promotion
PR" would be a checklist item; nightly would carry whatever version the last
promotion left; version-bearing example atoms would drift (the failure mode
carpenter lived through before its adr/022). Merges should encode readiness,
not clocks — but the *version* should follow the merges mechanically. Settled
ladder (carpenter adr/022 port):

- every merge into **nightly** → patch (`X.Y.Z → X.Y.Z+1`);
- every promotion (**nightly → main**) → minor (`X.Y.Z → X.(Y+1).0`),
  which is the released stable;
- **major** → reserved for critical/official changes, manual (not wired into
  CI; revisit when the first one is wanted).

Three hard constraints shape the mechanics:

1. **`main` is frozen** (ruleset protection, adr/0018): nothing may push to
   it, so the version a promotion releases must already be inside the PR's
   head (`nightly`).
2. **`GITHUB_TOKEN` pushes never trigger workflows** (recursion guard), and
   ruleset-protected branches decline them (user-owned repos cannot bypass
   the Actions integration) — so ladder pushes go through the **release-bot
   GitHub App**, whose pushes both bypass the ruleset and re-fire checks.
3. **`nightly` stays bot-pushable** (the ladder lands `chore(release): bump`
   commits directly), so its ruleset lists the owner and the App as bypass
   actors (adr/0018).

## Decision

One mechanical step — `cargo xtask bump patch|minor|major|--to X.Y.Z` — is
the single implementation of "bump the version everywhere": both manifests
(`Cargo.toml`, `xtask/Cargo.toml`), every whitelisted version-bearing example
atom (none yet; `register`/`upgrade` atoms are the expected first entries),
and the shared `Cargo.lock` via `cargo update -w` (workspace-member versions
only; release builds are `--locked`, so the bump commit must carry the lock).
Carpenter's port also re-invoked `cargo xtask build` to regenerate generated
surfaces; Poet needs no equivalent step — its gates live in `build.rs` and
its howto is static. Git stays with the caller. Three automation points in
`release.yml`/`ci.yml`:

- **patch ladder** (`bump` job, first on every nightly push): bump → commit
  `chore(release): bump to X.Y.Z` → push via the release-bot App token →
  *the same run* checks out the bumped sha and rolls the nightly prerelease,
  so the canary artifact reports the new version. The App-pushed bump head
  fires a cascade run whose `bump` job self-suppresses (its `if` recognizes
  bot-pushed bump heads), so nothing builds twice.
  **Pause window:** while a nightly→main PR is open, the ladder skips (a late
  patch bump would move nightly past the release minor and fail the guard).
- **promotion bump** (`promote-bump` job, on PR open/reopen): the
  **release-bot GitHub App** pushes `--to <main's minor+1>.0.0` onto nightly.
  An App token is required (constraint 2): its push fires `synchronize`, so
  the PR's required checks re-run. `ci.yml`'s guard enforces the exact
  version — the bot is the convenience, the guard is the rule.
- **recut** (`recut` job, after a main push publishes stable): fast-forward
  `nightly` to the promotion merge, so the trunk absorbs main's merge commit
  and the ladder resumes at `X.(Y+1).1`. FF-only — if nightly moved in
  between, the job fails loudly and the owner reconciles via a PR.

Version-bearing atoms are rewritten by whitelisted `d+.d+.d+` replacement —
a full `--capture-example` re-run would clobber hand-authored behavioral
notes. Other atoms may embed *package* versions and are never touched.

## Consequences

+ One command, one commit type, zero human version bookkeeping below major.
+ Every surface that shows the version moves together (compile-time `env!`,
  atoms, lock, release tags).
+ The nightly canary always reports a version strictly newer than stable.
− Ops prerequisite: the release-bot App + `RELEASE_BOT_APP_ID` /
  `RELEASE_BOT_PRIVATE_KEY` secrets must exist or promotions fail loudly.
− A promotion open while feature PRs land holds the patch ladder (they
  resume on merge); merges in the promote→recut window make recut red until
  reconciled (deliberate: fail loudly, never force).

## Rejected

- **Version stamps at build time (no commits)** — the repo would lie about
  its version between merges.
- **GITHUB_TOKEN bot for the promotion bump** — its pushes neither re-run PR
  checks nor bypass rulesets (constraint 2), leaving the PR unmergeable on
  stale checks.
- **Bumping inside feature PRs** — every PR would conflict on `Cargo.toml`.
- **Force-pushing the recut** — would silently discard nightly-only commits;
  FF-or-fail keeps the "promotion absorbs drift" rule of adr/0002 honest.

Extends [adr/0002](0002-branch-topology.md); branch protection mechanics in
[adr/0018](0018-ruleset-bypass-actors.md).
