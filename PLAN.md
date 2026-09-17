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
| 3 | `ivan/phase-3-design` | [phase-3-design.md](docs/plans/phase-3-design.md) | Design & layout ("CSS"): run format/emphasize, paragraph border, style, page ×7 | ☐ pending | formatting survives save/reopen |
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

## Handover notes from phase 2 (for phases 3–4)

Engine facts discovered the hard way — do not rediscover them:

- **docx-rs' writer always injects a default decimal numbering** (abstract id 1,
  `%1.` levels) into `numbering.xml`. After any save→reopen it exists even if Poet
  never created a list; numbering reuse scans must tolerate it (see
  `ensure_numbering` in `core/content.rs`).
- **docx-rs serde keys are camelCase and several fields are private**
  (`sectionType`, `pageSize`, `pageMargin`, `sz`, …). The established read path is
  `serde_json::to_value` on the property (adr/0008) — see `section_properties`,
  `run_info`, `list_tables` in `core/content.rs`. Reuse, don't guess field names.
- **Field codes**: `RunChild::InstrTextString` is reader-only and the writer maps it
  to `unreachable!()`. `DocumentManager::save` normalizes it back to `InstrText`
  before packing — any new code that builds or mutates field runs goes through
  save, never packs directly.
- **Sections are paragraph-embedded `sectPr`** (`DocumentChild::Section` cannot
  express start types — its property is `pub(crate)`). Phase 3 (`page ×7`) must
  enumerate via `section_properties` in `core/content.rs` and mutate the
  body-final props through `document.section_property` (pub) or the embedding
  paragraph's `property.section_property` (pub).
- **Phase-3 stubs still return `NotImplemented`**: `paragraph border`,
  `run format`, `run emphasize` — their CLI args are final; only bodies are
  missing. `emphasize` needs Words' run-splitting algorithm
  (`_emphasize_in_paragraph`) and reuses `_resolve_targets`-style addressing,
  already present as `resolve_cell` / cell args.
- **Style names**: resolve ids via `style_display_name` / write via
  `style_id_from_name` (builtin table + styles part). `paragraph add --style`
  does not validate yet (adr/0008 deviation) — the phase-3 style commands are the
  place to add registry-based validation.
- **Defaults differ from Words' python-docx template**: Poet documents are A4 with
  docx-rs margins (not Letter/1″). Accepted under adr/0001; `section info` values
  reflect it.
- **Integration-test gotcha**: `app::run_with` takes `Ctx` by value and mutates
  its own clone — a shared caller `Ctx` does NOT carry an open document across
  invocations. Chain commands with a fresh `Ctx::at_dir(dir)` per invocation
  (session auto-open = Words' process chaining), as `tests/phase2_content.rs` does.
