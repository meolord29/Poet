# PLAN.md — Poet rebuild tracker

Poet is a Rust rebuild of the Words CLI (`/home/mashkini/Workspace/Words`): an AI-first
.docx automation CLI. This file is the index and the live progress tracker. Each phase
has a **self-contained brief** in `docs/plans/` — a fresh agent with no prior context can
execute a phase from its brief alone. Conventions live in `AGENTS.md`; decisions in
`docs/adr/`.

## Global decisions

| Decision | Value | Rationale / ADR |
|---|---|---|
| Language / edition | Rust 2021, stable toolchain | adr/0001 |
| .docx engine | `docx-rs` 0.4.x (read + write, one model) | adr/0001 |
| CLI framework | `clap` 4, derive API (14 category subcommands) | adr/0003 |
| Binary name | `poet` | — |
| Session state | `~/.poet/session.json` (env-overridable for tests) | — |
| Output contract | JSON envelope, identical shape to Words | `AGENTS.md` §2 |
| Compatibility | None required with Python-generated .docx internals; CLI behavior parity is required | adr/0001 |
| Table analysis | `polars` (added in phase 4) | — |
| Expressions | `rhai` (replaces Python `eval()` in `calc transform add_column`) | adr/0006 |
| Behavioral spec | `/home/mashkini/Workspace/Words` — port behavior, cite source paths | `AGENTS.md` |

## Branch topology (carpenter model)

- `nightly` — integration; all phase branches merge here.
- `main` — stable; only `nightly` merges into it (CI guard, added phase 1).
- Phases: `ivan/phase-N-<slug>` → merge to `nightly` on completion.
- Promotion: after phase 4, PR `nightly → main` tagged `v0.1.0`.

## Phase tracker

| # | Branch | Brief | Scope | Status | Exit test |
|---|---|---|---|---|---|
| 1 | `ivan/phase-1-framework` | [phase-1-framework.md](docs/plans/phase-1-framework.md) | Scaffold, deps, errors/envelopes/session, clap skeleton (14 categories, stubs), DocumentManager lifecycle, BookmarkManager, Ctx DI, CI | ☑ done | `document new→save→open→info` round-trip |
| 2 | `ivan/phase-2-content` | [phase-2-content.md](docs/plans/phase-2-content.md) | Text & structure ("HTML"): paragraph, heading, run (plain), table, list, section, toc, image, export md/txt | ☑ done | create→mutate→save→reopen→verify |
| 3 | `ivan/phase-3-design` | [phase-3-design.md](docs/plans/phase-3-design.md) | Design & layout ("CSS"): run format/emphasize, paragraph border, style, page ×7 | ☑ done | formatting survives save/reopen |
| 4 | `ivan/phase-4-misc` | [phase-4-misc.md](docs/plans/phase-4-misc.md) | Misc: meta engine/annotator/type inference, calc (polars + rhai), batch + templates, howto/README/docs rebrand | ☐ pending | ~179 ported tests green; CV batch workflow e2e |

Statuses: ☐ pending → ◐ in progress → ☑ done. Update in the merge that completes a phase.

## ADR index

| ADR | Title |
|---|---|
| [0001](docs/adr/0001-docx-rs-engine.md) | docx-rs as the .docx engine |
| [0002](docs/adr/0002-branch-topology.md) | carpenter-style branch topology |
| [0003](docs/adr/0003-clap-derive-cli.md) | clap derive API for the CLI tree |
| [0004](docs/adr/0004-bookmark-addressing.md) | bookmark addressing over docx-rs document children |
| [0005](docs/adr/0005-lists-via-real-numbering.md) | lists via real numbering definitions |
| [0006](docs/adr/0006-id-allocation-and-addressing.md) | id allocation + element addressing |
| [0007](docs/adr/0007-table-model-mapping.md) | table model mapping |
| [0008](docs/adr/0008-engine-gaps.md) | engine gaps: field round-trip, sections, images, style names |
| [0009](docs/adr/0009-run-formatting-and-emphasize.md) | run formatting, emphasize run-splitting, paragraph borders |
| [0010](docs/adr/0010-style-registry.md) | style registry: styles part + builtin catalog |
| [0011](docs/adr/0011-page-section-mapping.md) | page/section mapping, validated inputs, reader gaps |

## Handover notes from phases 2–3 (for phase 4)

Engine facts discovered the hard way — do not rediscover them:

- **docx-rs' writer always injects a default decimal numbering** (abstract id 1,
  `%1.` levels) into `numbering.xml`. After any save→reopen it exists even if Poet
  never created a list; numbering reuse scans must tolerate it (see
  `ensure_numbering` in `core/content.rs`).
- **docx-rs serde keys are camelCase and several fields are private**
  (`sectionType`, `pageSize`, `pageMargin`, `sz`, …). The established read path is
  `serde_json::to_value` on the property (adr/0008) — see `section_properties`,
  `run_info`, `list_tables` in `core/content.rs` and `page_size_wh` in
  `core/design.rs`. Reuse, don't guess field names.
- **Field codes**: `RunChild::InstrTextString` is reader-only and the writer maps it
  to `unreachable!()`. `DocumentManager::save` normalizes it back to `InstrText`
  before packing — body, tables, **and header/footer parts of every section**
  (the PAGE field lives in a footer; phase 3 extended the walk). Any new code that
  builds or mutates field runs goes through save, never packs directly.
- **Sections are paragraph-embedded `sectPr`** (`DocumentChild::Section` cannot
  express start types). Enumerate via `section_properties` in `core/content.rs`;
  mutate through `section_property_mut` in `core/design.rs` (embedded-then-
  body-final order). Phase-3 page commands all flow through it.
- **Emphasize formatting** (adr/0009): `RunProperty` derives `Clone` with pub
  fields — snapshot/clone/rebuild works. Shared mapping: `FormatSpec` +
  `apply_to_property` in `core/design.rs` (used by `run add/format/emphasize`);
  do not fork a second mapping. Multi-run matches rebuild per-run fragments
  (cut points include run boundaries), never merged runs.
- **Style registry** (adr/0010): `docx.styles.styles` is parsed on read, but a
  fresh Poet document's styles part holds only `Normal`. Registry = part ∪
  `BUILTIN_STYLES` catalog in `core/design.rs`; `resolve_style_id` is the one
  validator (used by `style apply` and `paragraph add/insert --style`).
- **Reader blind spots on reopened docs** (adr/0011): `w:orient` attribute and
  `w:cols` are dropped by the docx-rs reader (the written file is correct);
  header/footer *content* is reassigned only for the body-final section, so
  parts on embedded sections don't survive reopen. Verify file-level survival
  with zip side-reads (`read_part` pattern in the design tests).
- **Words silent fallbacks**: Poet deliberately validates what Words silently
  tolerated (unknown `--unit`/`--align`/orientation/`style --type` →
  `validation_error`; adr/0011 policy). Unknown border `--style` is coerced to
  `single` by the engine's `FromStr` catch-all (adr/0009) — no error, like Words.
- **Integration-test gotcha**: `app::run_with` takes `Ctx` by value and mutates
  its own clone — a shared caller `Ctx` does NOT carry an open document across
  invocations. Chain commands with a fresh `Ctx::at_dir(dir)` per invocation
  (session auto-open = Words' process chaining), as `tests/phase2_content.rs`
  and `tests/phase3_design.rs` do. Note that read commands in autosaving
  categories still trigger autosave on success — a reopened document is
  re-saved (and must therefore be normalize-safe) even after read-only
  commands.
