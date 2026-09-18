<!-- Ground rules for merging into `nightly` (adr/0002). The repo owner
     reviews every PR and verifies each box. -->

## What this PR does

<!-- Explain what the feature/change does and why a document-building user or
     agent would care. One short paragraph beats a changelog dump. -->

## Ground rules (feature PRs into nightly)

- [ ] All tests pass (`cargo test` green in CI; gates from `cargo build` pass).
- [ ] New/changed behavior ships with unit tests (a new command fn lands its
      `#[test]` and its example atom by construction — the build gate enforces
      both; adr/0015).
- [ ] If the CLI surface or document workflow changed:
      `.opencode/agents/poet-dev-validate.md` (the QA agent's checklist/prompt)
      is updated to match in this PR.
- [ ] A **poet-dev-validate report** is attached below — **required when this
      PR changes user-facing surface** (commands, envelope shapes, workflow,
      howto contract): the document-building simulation ran smoothly end to
      end, verifying existing features **and** the new/changed ones.
      Infra/docs/CI-only PRs: tick with a one-line
      `N/A — no surface change` plus the targeted contract validation that ran
      instead.

### poet-dev-validate report

<!-- Required for surface-touching PRs: paste the report (or its summary) —
      commands audited, failure tally (must show zero bugs), and confirmation
      that building the requested document succeeded smoothly. Run the agent
      with the PR branch checked out. Otherwise: one line explaining the N/A
      + what contract validation covered the change. -->
