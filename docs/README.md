# poet docs

Five areas. The **code is the source of truth**; these are the human-readable
mirrors plus the generated contracts. When code and docs disagree, the code
wins — update the docs.

| area | what lives here | start with |
|------|-----------------|------------|
| [adr/](adr/) | architecture decision records (append-only history) | [adr/README](adr/README.md) |
| [reference/](reference/) | per-surface contracts (error codes, batch scripts) | [error-codes](reference/error-codes.md) |
| [tutorials/](tutorials/) | step-by-step user walkthroughs | [getting-started](tutorials/01-getting-started.md) |
| [plans/](plans/) | the phased rebuild plan (Words → Poet) | [phase-1](plans/phase-1-framework.md) |
| [examples/](examples/) | one worked example per CLI command — keyed by `build.rs` (adr/0015) | any `<category>/<action>.md` |

## Read order (newcomer)

1. [../README](../README.md) — what poet is, install, quick start.
2. [tutorials/01-getting-started](tutorials/01-getting-started.md) — first document.
3. `poet howto` — the full command manual (the agent-facing contract).
4. [adr/README](adr/README.md) — why the decisions were made.

## Do not consolidate (build-coupled)

`docs/examples/` (one file per command fn, keyed by `build.rs` — adr/0015) is
coupled to the build; merging it into prose breaks the gates. `adr/` is
append-only history. Free prose lives in `reference/` and `tutorials/`.

## Contributing to poet

For contributors. Just want to use poet? Start at the
[README](../README.md). AI agents working in this repo read
[`AGENTS.md`](../AGENTS.md) — it is the authoritative contributor guide; this
section is the human on-ramp.

### Quickstart

```sh
git clone https://github.com/meolord29/Poet poet
cd poet
git config core.hooksPath .githooks   # pre-commit fmt gate
cargo test --workspace   # --workspace is required: bare cargo test skips xtask
```

Full command matrix (clippy, fmt, doc):
[AGENTS.md § Build & verify](../AGENTS.md#build--verify).

### The dev loop

Two build stages (adr/0015):

- **strict** — every build in CI: each command fn must be documented by a
  worked example, tested in-module, integration-covered, and scenario-featured
  (`build.rs` enforces; a miss aborts the build).
- **dev** — `cargo build --features dev`: relaxed gates, so a new command
  compiles before its worked-example and test exist. `--capture-example`
  writes the example atom from a real run; `--dev-home` isolates the session
  dir. `dev` + release is rejected — no relaxed binary ships.

### Releases (nightly + stable main)

Two channels, two long-lived branches
([adr/0002](adr/0002-branch-topology.md), [adr/0016](adr/0016-automated-version-ladder.md)):

```
ivan/<topic> ──PR──▶ nightly   integration trunk; ground-rules checklist + owner-only
                               approval; each merge bumps a patch (bot) and rolls the
                               `nightly` prerelease at the bumped version
nightly ──PR──▶ main           promotion; the release-bot lands main's minor+1 on PR
                               open (guard enforces it); merge publishes immutable
                               stable vX.Y.0 (Latest), then nightly is recut
                               (fast-forwarded) to the promotion merge
```

Version ladder ([adr/0016](adr/0016-automated-version-ladder.md)): **patch**
per nightly merge, **minor** per promotion (the released stable), **major**
manual — reserved for critical/official changes. `cargo xtask bump
patch|minor|major|--to X.Y.Z` is the one mechanical step behind all of it;
humans never edit versions in PRs.

- **`nightly` (unstable)** — the rolling prerelease `release.yml` re-creates
  on every push to the `nightly` branch. Canary users soak each build before
  promotion. Install / stay on it:

  ```sh
  curl -LsSf https://github.com/meolord29/Poet/releases/download/nightly/install.sh | sh
  poet upgrade --channel nightly
  ```

- **stable (`vX.Y.Z`)** — immutable, versioned from `Cargo.toml`, published
  when `nightly` merges into `main`. GitHub marks it **Latest**, so the README
  one-liner and `poet upgrade` (default `--channel stable`) follow it. Users
  never see nightly — the README is end-user-only (install + getting started;
  stable).

**PR ground rules** (feature branches → `nightly`;
[`.github/PULL_REQUEST_TEMPLATE.md`](../.github/PULL_REQUEST_TEMPLATE.md),
verified by the owner at review):

1. All tests pass (`cargo test --workspace` green in CI).
2. New features ship with tests (a new command fn lands its `#[test]` and its
   example atom by construction — the build gate enforces both).
3. If the CLI surface or document workflow changed, the QA agent's
   checklist/prompt (`.opencode/agents/poet-dev-validate.md`) is updated in
   the same PR.
4. The PR explains what the feature does.
5. A poet-dev-validate report is attached when the PR changes user-facing
   surface — the document-building simulation ran smoothly over existing
   features and any new ones. Infra/docs/release-process PRs record an
   `N/A — no surface change` with the targeted contract validation that ran
   instead.
6. No version bumps in the PR — the ladder is automated (adr/0016).

**Promotion checklist** (nightly → main PR):
1. Open the PR — the `promote-bump` bot commits main's minor+1 (`chore(release):
   bump to X.Y.0`) onto nightly; the `guard` check must be green.
2. Merge; CI tags `v<version>` and publishes stable; the smoke lanes verify the
   published artifact via the `/latest/` one-liner; then `recut` fast-forwards
   `nightly` to the promotion merge and the patch ladder resumes.

Prerequisite (one-time, owner): the **release-bot GitHub App** + the
`RELEASE_BOT_APP_ID`/`RELEASE_BOT_PRIVATE_KEY` repo secrets (adr/0016,
[adr/0021](adr/0021-ruleset-bypass-actors.md)):

1. GitHub → Settings → Developer settings → GitHub Apps → **New GitHub App**;
   name `poet-release-bot`, homepage the repo URL, webhook off.
2. Permissions: **Contents: Read and write**, **Pull requests: Read and
   write**. Create, note the **App ID**.
3. Generate a private key for the app.
4. Install the app on the `meolord29/Poet` repository (All repositories).
5. Repo secrets: `RELEASE_BOT_APP_ID` (the App ID) and
   `RELEASE_BOT_PRIVATE_KEY` (the downloaded `.pem` contents).

**Branch protection** ([adr/0021](adr/0021-ruleset-bypass-actors.md);
rulesets only, Settings → Rules → Rulesets):

- `trunk protection (main)`: target `~DEFAULT_BRANCH`; PR required, 1
  code-owner review (`CODEOWNERS` is `* @meolord29`), required checks — the ci
  `gates` lanes, the release `smoke` lanes, and `guard (only nightly merges
  into main)` — strict up-to-date, no force-pushes, no deletions. Bypass
  actor: the owner (`pull_request` only — merges via PR).
- `trunk protection (nightly)`: target `nightly`; PR required, required
  checks (`gates` + `smoke`), strict. Bypass actors: the owner and the
  `poet-release-bot` GitHub App (all ladder pushes: bump, promote-bump,
  recut) — nobody else can push or merge either trunk.

**Rollback**: stable tags are immutable — install any previous `vX.Y.Z` by
substituting its tag for `latest` in the install URL.

### Contributing flow

Short-lived `ivan/<topic>` branches off `nightly`, green CI, owner-approved
merge (ground rules above), delete after merge. See
[AGENTS.md § Git topology](../AGENTS.md#git-topology-carpenter-model).
