# plan_v3 — replicate carpenter's infrastructure in Poet

Carpenter (`/home/mashkini/Workspace/carpenter`) is the infrastructure spec: CI/CD,
branch governance, release ladder, installer, agent integration. Words→Poet behavior
parity does not apply to this plan; where carpenter's code and this document disagree
on infra *behavior*, carpenter wins.

Scope decisions (owner-approved):
- **Full parity** — including `register` / `upgrade` / `uninstall` CLI commands and the
  `poet` skill integration (installer + smoke lanes depend on them).
- **Release-bot GitHub App** — bump commits are pushed via an App token because
  ruleset-protected branches reject GITHUB_TOKEN pushes (carpenter adr/022 + 023).
- License stays MIT (Poet's existing LICENSE) — infra replication, not carpenter's
  BSD-3 text.
- Repo slug: `meolord29/Poet`; binary `poet`; bot actor `poet-release-bot[bot]`;
  secrets `RELEASE_BOT_APP_ID` / `RELEASE_BOT_PRIVATE_KEY`.

## PR1 — version ladder + git hooks + CI hardening (infra; dev-validate: N/A)

Branch: `ivan/infra-1-ladder` off `nightly`.

- [x] Root `Cargo.toml`: `[workspace] members = ["xtask"]`.
- [x] `xtask/` crate (clap + anyhow only, edition 2024):
  - `cargo xtask bump patch|minor|major|--to X.Y.Z` — port of carpenter
    `xtask/src/bump.rs`. Differences: no `gen-howto`/`gen-specs` (Poet's howto is
    static, gates live in `build.rs`); derived-surface step = `cargo update -w`
    (refreshes only workspace-member versions in `Cargo.lock` — release builds are
    `--locked`, so the bump commit must carry the lock).
  - `VERSION_ATOMS` starts empty; PR2's register/upgrade atoms (which embed the
    version in captured envelopes) are added there. _(done in PR2, drift test
    included)_
  - hooks drift test: `.githooks/pre-commit` present + executable (carpenter `hooks.rs`).
  - `paths.rs` port (workspace_root via CARGO_MANIFEST_DIR).
- [x] `.cargo/config.toml` (`xtask` alias); `.githooks/pre-commit` (fmt gate on staged
  `.rs`, carpenter port with poet wording); enable locally:
  `git config core.hooksPath .githooks`.
- [x] `.github/workflows/ci.yml`: port carpenter's shape — concurrency group,
  ubuntu+macos-14 gates matrix, dtolnay/rust-toolchain + Swatinem/rust-cache,
  `--workspace` everywhere, `RUSTDOCFLAGS=-D warnings` doc
  lane; keep Poet's extra `--features dev` clippy/test lanes; guard job gains the
  promotion version assertion (exactly main's minor+1.0). _(the shellcheck lane
  was deferred to PR3, when `scripts/install.sh` first exists — see Notes)_
- [x] AGENTS.md § Build & verify: `--workspace` on test/clippy lines + hooksPath note.
- [x] ADR `docs/adr/0016-automated-version-ladder.md` (port of carpenter adr/022).
- [x] Gates: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D
  warnings && cargo clippy --all-targets --features dev -- -D warnings && cargo fmt
  --all --check && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`.

## PR2 — CLI surface: `register`, `upgrade`, `uninstall` (surface PR; dev-validate report required)

Branch: `ivan/infra-2-install-surface` off `nightly`.

- [x] `src/core/` helpers + `src/commands/{register,upgrade,uninstall}.rs` (one
  signature everywhere: `pub fn <action>(ctx: &Ctx, args: <Args>) -> Result<Data,
  PoetError>`).
  - `register --app opencode|claude-code [--print-skill]`: writes
    `skills/poet/SKILL.md` (opencode: `$XDG_CONFIG_HOME/opencode`; claude:
    `~/.claude/skills/poet/SKILL.md`), idempotent, merges
    `permission.skill.poet:"allow"` into `opencode.json`, ok envelope echoes the path.
  - `upgrade [--channel stable|nightly] [--bin-dir] [--no-skill]`: fetch tarball +
    SHA256SUMS via curl subprocess, verify, probe, atomic swap, refresh registered
    skills.
  - `uninstall [--bin-dir]`: remove skills first, binary last; `not_found` when
    neither exists.
- [x] Skill content: frontmatter + authored prose/workflow + inlined `poet howto`
  (H1 stripped) + version + binary path.
- [x] adr/0015 gates: atoms `docs/examples/{register,upgrade,uninstall}/*.md`
  (register/upgrade are version-bearing → added to xtask `VERSION_ATOMS` with a
  drift test), paired tests, `tests/install_surface.rs` integration coverage,
  `Data::{Register,PrintSkill,Upgrade,Uninstall}` examples in `models/examples.rs`.
- [x] ADRs `0017-skill-integration.md`, `0018-upgrade-fetches-release.md`,
  `0019-uninstall-semantics.md` (renumbered from the original 0019-0021 so ADR
  numbers land in merge order; PR3 → 0020, PR4 → 0021).

## PR3 — installer + repo plumbing + docs

Branch: `ivan/infra-3-installer-docs` off `nightly`.

- [x] `scripts/install.sh`: carpenter port with poet branding (`POET` wordmark,
  `poet:` prefix, amber-family palette), `POET_DOWNLOAD_BASE` / `POET_INSTALL_DIR`
  / `POET_INSTALL_YES` overrides, `skill.poet` permission hint, `TAG="nightly"`
  line byte-identical (release.yml seds it for stable); shellcheck-clean
  _(authoritatively by the CI lane — locally `sh -n` + the `tests/install_sh.rs`
  end-to-end fixture run, see Notes)_.
- [x] `.github/CODEOWNERS` (`* @meolord29`); `.github/workflows/assign.yml`
  (auto-assign PRs to meolord29).
- [x] PR template: "no version bumps — ladder automated" ground rule + Promotion PR
  section (carpenter port).
- [x] README: install one-liner
  (`curl -LsSf https://github.com/meolord29/Poet/releases/latest/download/install.sh | sh`),
  nightly channel line, Contributing → docs/README.md.
- [x] New `docs/README.md`: docs map + contributing (quickstart, dev loop, channel
  diagram, PR ground rules, promotion checklist, release-bot prerequisite, ruleset
  protection, rollback). `docs/adr/README.md` index.
- [x] ADR installer consent + banner — landed as
  `0020-install-consent-and-banner.md` (carpenter adr/024 port; renumbered from
  0017 so ADR numbers land in merge order).

## PR4 — release pipeline + GitHub-side setup

Branch: `ivan/infra-4-release` off `nightly`.

- [x] `.github/workflows/release.yml` — port all six jobs, poet-adapted:
  `bump` (patch ladder via App token, promotion-pause, cascade suppression,
  `refs/heads/nightly` refspec), `build` (matrix `x86_64-unknown-linux-musl` +
  `aarch64-apple-darwin`, strip + tar), `promote-bump` (main's minor+1.0.0 via App
  on PR open), `release` (roll `nightly` prerelease / publish stable `vX.Y.Z` +
  SHA256SUMS + tag-patched installer), `recut` (FF nightly via App), `smoke`
  (real opencode via script + brew lanes, PR `file://` mode, asserts `poet howto`,
  `poet register --app opencode`, SKILL.md presence, `poet uninstall` reversal).
  _(workflow YAML validated; the ladder itself is only proven live by the first
  nightly push — see the musl note)_
- [x] ADR `0018-ruleset-bypass-actors.md` → landed as
  `0021-ruleset-bypass-actors.md` (carpenter adr/023 port; renumbered so ADR
  numbers land in merge order).
- [ ] **Manual setup checklist** (documented in docs/README.md; needs GitHub admin):
  create `poet-release-bot` GitHub App (contents:write, PRs:write), install on the
  repo, add `RELEASE_BOT_APP_ID`/`RELEASE_BOT_PRIVATE_KEY` secrets; rulesets:
  `main` (PR + codeowner review + required checks incl.
  `guard (only nightly merges into main)`, strict, owner bypass) and `nightly`
  (required gates+smoke checks; bypass actors: owner + App).
- [x] Early risk check: musl build — **partially resolved**. The original
  blocker (`zstd-sys` C toolchain, pulled by `zip` default features) was
  eliminated by switching the zip dep to `default-features = false, features
  = ["deflate-flate2"]` (.docx needs deflate only; all remaining zip backends
  are pure Rust). A second blocker remains: `psm` (via polars → stacker)
  needs `musl-gcc`, which the release lane installs on the runner
  (`musl-tools`, carpenter-identical). **Local verification is impossible
  (no root to install musl-tools)** — see Notes.

## Execution protocol

Per PR: branch off `nightly` → implement → full gate battery → Conventional Commit
with ADR citations → merge into `nightly` locally (no pushes without owner OK; all
branches + nightly pushed in one batch at the end).

## Notes / review later

- **PR1:** the CI `shellcheck` lane was moved out of PR1 into PR3 (it lints
  `scripts/install.sh`, which only exists from PR3 on — landing it early would
  keep CI red between merges). _(done: the lane is in ci.yml since PR3.)_
- **PR1:** `cargo xtask bump` drill left `xtask/Cargo.toml` untracked at
  0.1.1 briefly (git checkout refused a mix of tracked/untracked paths) —
  fixed by hand before commit; the committed tree is consistent at 0.1.0.
- **PR2:** `Data::message()` returns `&str`, so the install commands use
  static envelope messages (`"skill registered"`, `"upgraded"`,
  `"uninstalled"`) where carpenter interpolates app/version — those values
  are already in `data`; revisit only if envelope-message parity matters.
- **PR2:** Poet is edition 2024, where `std::env::set_var` is `unsafe` — the
  `POET_DOWNLOAD_BASE` unreachable-download test moved from a unit test (env
  mutation) to a process-level integration test (`tests/install_surface.rs`,
  env set on the spawned child only). Carpenter's in-process version is not
  portable to this edition.
- **PR2 (incident, resolved):** the live `register` drill was first run
  without `XDG_CONFIG_HOME`/`HOME` overrides and wrote the real
  `~/.config/opencode/skills/poet/SKILL.md` + merged `"poet": "allow"` into
  the real `opencode.json` (unrelated keys preserved — the merge logic is
  now real-file-validated). Both reverted by hand. Lesson recorded: always
  export the isolated env **before** the first drill invocation.
- **PR2 (deviation, accepted):** carpenter's `Paths` config layering
  (`config.bin_dir`/`config.source_dir`) is not ported — Poet has no config
  feature yet; `--bin-dir` defaults to `~/.local/bin` directly. `--source`
  rebuild mode dropped for the same reason (no generated surfaces; dev loop
  is plain cargo). Add both with the config feature if it ever lands.
- **PR2:** no `poet deregister` command (carpenter has one) — the reversal
  path is `uninstall`; core `skill::deregister` exists for it. Minimal
  surface; reconsider if single-app deregistration is ever needed.
- **PR4 (review at first nightly run):** the musl release build could not be
  verified locally (no root to install `musl-tools`; `psm` via polars →
  stacker needs `musl-gcc`). `zstd-sys` was trimmed from the graph (zip
  feature change, PR4 commit), so `psm` is the only remaining C consumer.
  If the first release-lane build goes red on musl, the fallback is: switch
  the linux target to `x86_64-unknown-linux-gnu` in `release.yml` + the
  installer's platform table + `core::release::target_for` + the
  `tests/install_sh.rs` fixture — four places, one commit.
- **PR4:** `shellcheck` is not installed locally (no root); the installer
  was checked with `sh -n` + driven end-to-end by `tests/install_sh.rs`
  against a `file://` fixture. The CI `shellcheck` lane is the authoritative
  lint — expect it green (the script is a close carpenter port) but verify
  on the first CI run.
- **PR4:** release.yml/smoke reference `meolord29/Poet` casing from the git
  remote; if the GitHub repo slug ever changes casing, update
  `scripts/install.sh` REPO, `core/release.rs` REPO (pinned by test), the
  workflow smoke URLs, and the README one-liners.

## Status

- [x] PR1 — version ladder + hooks + CI (merged to nightly locally)
- [x] PR2 — register / upgrade / uninstall (merged to nightly locally)
- [x] PR3 — installer + plumbing + docs (merged to nightly locally)
- [x] PR4 — release pipeline (+ ADR-0021; merged to nightly locally)
- [ ] Final: push all branches + nightly to origin (owner OK), end-to-end drill
  (merge → nightly release → promotion PR → stable → recut → smoke)

## Owner's manual follow-ups (after push)

1. Create the `poet-release-bot` GitHub App + secrets (steps in
   [docs/README.md](docs/README.md#releases-nightly--stable-main)).
2. Create the two rulesets (main/nightly) per adr/0021 with the bypass actors.
3. Required-check names must match exactly: the ci `gates` lanes, the release
   `smoke` lanes, and `guard (only nightly merges into main)`.
4. Run the `poet-dev-validate` QA agent against the pushed nightly once —
   PR2 was a surface PR and its report requirement was satisfied by live
   drills + `tests/install_surface.rs`, not by the agent itself (noted in
   Notes); a one-pass agent run closes that gap.
5. The first push to `nightly` publishes the rolling `nightly` prerelease and
   proves the musl lane (see the PR4 note before pushing).
