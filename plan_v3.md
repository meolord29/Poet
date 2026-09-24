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

- [ ] Root `Cargo.toml`: `[workspace] members = ["xtask"]`.
- [ ] `xtask/` crate (clap + anyhow only, edition 2024):
  - `cargo xtask bump patch|minor|major|--to X.Y.Z` — port of carpenter
    `xtask/src/bump.rs`. Differences: no `gen-howto`/`gen-specs` (Poet's howto is
    static, gates live in `build.rs`); derived-surface step = `cargo update -w`
    (refreshes only workspace-member versions in `Cargo.lock` — release builds are
    `--locked`, so the bump commit must carry the lock).
  - `VERSION_ATOMS` starts empty; PR2's register/upgrade atoms (which embed the
    version in captured envelopes) are added there.
  - hooks drift test: `.githooks/pre-commit` present + executable (carpenter `hooks.rs`).
  - `paths.rs` port (workspace_root via CARGO_MANIFEST_DIR).
- [ ] `.cargo/config.toml` (`xtask` alias); `.githooks/pre-commit` (fmt gate on staged
  `.rs`, carpenter port with poet wording); enable locally:
  `git config core.hooksPath .githooks`.
- [ ] `.github/workflows/ci.yml`: port carpenter's shape — concurrency group,
  ubuntu+macos-14 gates matrix, dtolnay/rust-toolchain + Swatinem/rust-cache,
  shellcheck lane (Linux), `--workspace` everywhere, `RUSTDOCFLAGS=-D warnings` doc
  lane; keep Poet's extra `--features dev` clippy/test lanes; guard job gains the
  promotion version assertion (exactly main's minor+1.0).
- [ ] AGENTS.md § Build & verify: `--workspace` on test/clippy lines + hooksPath note.
- [ ] ADR `docs/adr/0016-automated-version-ladder.md` (port of carpenter adr/022).
- [ ] Gates: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D
  warnings && cargo clippy --all-targets --features dev -- -D warnings && cargo fmt
  --all --check && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`.

## PR2 — CLI surface: `register`, `upgrade`, `uninstall` (surface PR; dev-validate report required)

Branch: `ivan/infra-2-install-surface` off `nightly`.

- [ ] `src/core/` helpers + `src/commands/{register,upgrade,uninstall}.rs` (one
  signature everywhere: `pub fn <action>(ctx: &Ctx, args: <Args>) -> Result<Data,
  PoetError>`).
  - `register --app opencode|claude-code`: writes `skills/poet/SKILL.md`
    (opencode: `$XDG_CONFIG_HOME/opencode` or `~/Library/Application
    Support/opencode`; claude: `~/.claude/skills/poet/SKILL.md`), idempotent, ok
    envelope echoes the path.
  - `upgrade [--channel stable|nightly]`: fetch tarball + SHA256SUMS via curl
    subprocess (no new deps), verify checksum, swap binary, refresh skill.
  - `uninstall [--bin-dir]`: remove binary + registered skills.
- [ ] Skill content: frontmatter (name/description) + pointer to `poet howto`.
- [ ] adr/0015 gates auto-enroll: example atoms
  `docs/examples/<cat>/<action>.md` (captured envelopes), paired tests, integration
  invocations, `Data` variants in `src/models/examples.rs`. Register/upgrade atoms
  are version-bearing → add to xtask `VERSION_ATOMS`.
- [ ] ADRs: `0019-skill-integration.md` (carpenter 006+009),
  `0020-upgrade-fetches-release.md` (carpenter 018), `0021-uninstall-semantics.md`
  (carpenter 019). Numbering gap 0017/0018 reserved by PR3/PR4 — file ADRs in
  numeric order at merge time (renumber if needed).

## PR3 — installer + repo plumbing + docs

Branch: `ivan/infra-3-installer-docs` off `nightly`.

- [ ] `scripts/install.sh`: carpenter port with poet branding (`POET` wordmark,
  `poet:` prefix, amber-family palette), `POET_DOWNLOAD_BASE` / `POET_INSTALL_DIR`
  / `POET_INSTALL_YES` overrides, `skill.poet` permission hint, `TAG="nightly"`
  line byte-identical (release.yml seds it for stable); shellcheck-clean.
- [ ] `.github/CODEOWNERS` (`* @meolord29`); `.github/workflows/assign.yml`
  (auto-assign PRs to meolord29).
- [ ] PR template: "no version bumps — ladder automated" ground rule + Promotion PR
  section (carpenter port).
- [ ] README: install one-liner
  (`curl -LsSf https://github.com/meolord29/Poet/releases/latest/download/install.sh | sh`),
  nightly channel line, Contributing → docs/README.md.
- [ ] New `docs/README.md`: docs map + contributing (quickstart, dev loop, channel
  diagram, PR ground rules, promotion checklist, release-bot prerequisite, ruleset
  protection, rollback). `docs/adr/README.md` index.
- [ ] ADR `0017-install-consent-and-banner.md` (carpenter adr/024 port).

## PR4 — release pipeline + GitHub-side setup

Branch: `ivan/infra-4-release` off `nightly`.

- [ ] `.github/workflows/release.yml` — port all six jobs, poet-adapted:
  `bump` (patch ladder via App token, promotion-pause, cascade suppression,
  `refs/heads/nightly` refspec), `build` (matrix `x86_64-unknown-linux-musl` +
  `aarch64-apple-darwin`, strip + tar), `promote-bump` (main's minor+1.0.0 via App
  on PR open), `release` (roll `nightly` prerelease / publish stable `vX.Y.Z` +
  SHA256SUMS + tag-patched installer), `recut` (FF nightly via App), `smoke`
  (real opencode via script + brew lanes, PR `file://` mode, asserts `poet howto`,
  `poet register --app opencode`, SKILL.md presence, `poet uninstall` reversal).
- [ ] ADR `0018-ruleset-bypass-actors.md` (carpenter adr/023 port).
- [ ] **Manual setup checklist** (documented in docs/README.md; needs GitHub admin):
  create `poet-release-bot` GitHub App (contents:write, PRs:write), install on the
  repo, add `RELEASE_BOT_APP_ID`/`RELEASE_BOT_PRIVATE_KEY` secrets; rulesets:
  `main` (PR + codeowner review + required checks incl.
  `guard (only nightly merges into main)`, strict, owner bypass) and `nightly`
  (required gates+smoke checks; bypass actors: owner + App).
- [ ] Early risk check: `cargo build --release --target x86_64-unknown-linux-musl`
  locally (polars/image static-link risk). Fallback if it fails: gnu target for the
  linux lane.

## Execution protocol

Per PR: branch off `nightly` → implement → full gate battery → Conventional Commit
with ADR citations → merge into `nightly` locally (no pushes without owner OK; all
branches + nightly pushed in one batch at the end).

## Notes / review later

_(appended as work progresses — anything that did not go as planned or needs a
second look lands here)._

## Status

- [ ] PR1 — version ladder + hooks + CI
- [ ] PR2 — register / upgrade / uninstall
- [ ] PR3 — installer + plumbing + docs
- [ ] PR4 — release pipeline (+ manual GitHub checklist handed to owner)
- [ ] Final: push all branches + nightly to origin (owner OK), end-to-end drill
  (merge → nightly release → promotion PR → stable → recut → smoke)
