# ADR 0003 — clap derive API for the CLI tree

Date: 2026-09-16 · Status: accepted

## Decision

Poet's CLI uses `clap` 4 with the **derive API**: each command module defines its
action `Subcommand` enum and arg structs next to the command functions; `app.rs`
holds the root `Commands` enum via a generic `CategoryArgs<A>` wrapper and the
dispatch match.

## Context

carpenter builds its clap tree with the builder API plus small arg-helper functions.
Poet has ~74 commands across 14 categories — more than triple carpenter's surface —
and the flags are stable, ported from the Words typer definitions.

## Consequences

- Action enums live beside their command functions; `app.rs` stays wiring-only.
- Usage errors and `--help`/`--version` print clap's plain text with clap's exit
  codes (0/2), matching the Typer reference behavior (only command *results* use
  the JSON envelope).
- Tri-state flags (`--bold/--no-bold`) are carried as two `bool` fields; the
  phase-3 formatting engine resolves them (`--no-bold` wins when both given).
