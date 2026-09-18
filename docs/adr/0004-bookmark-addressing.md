# ADR 0004 — bookmark addressing over docx-rs document children

Date: 2026-09-16 · Status: accepted

## Decision

`BookmarkManager` operates on `docx_rs::Document.children` — the body-level
`Vec<DocumentChild>` — by **sibling-wrapping**: `DocumentChild::BookmarkStart` is
inserted immediately before the target element and `BookmarkEnd` immediately after
(`add`), exactly mirroring Words' `bookmark.py` mechanism. Names, prefixes
(`p`/`h`/`l`/`t`/`img`/`toc`), `{prefix}{max+1}` allocation, `find`, `name_around`,
`rename`, `remove_around`, `list`, and max-id+1 `next_id` semantics are ported 1:1.

## Index contract (Rust-specific)

Python passes element identity; Rust works with indices into the children vec, so:

- `ensure(index, ...)` addresses the element's position *before* wrapping; after
  wrapping the element sits at `index + 1`.
- Re-calling `ensure` with either the original or the shifted index is idempotent
  and returns the existing name.
- Creation flows (phase 2+) insert a new block element and then call `add(index,
  name)` — the marker lands where the element was, so no shift bookkeeping is
  needed by callers.

## Verified

Fidelity probe `bookmark_survives_save_reopen_round_trip` pins that docx-rs's
reader restores body-level `BookmarkStart`/`BookmarkEnd` children, so `--id`
addressing works across processes. `next_id` scans into paragraph, cell, and
nested-table children so inline bookmarks never collide with body-level ids.
