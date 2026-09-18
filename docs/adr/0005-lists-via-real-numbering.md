# ADR 0005 — lists via real numbering definitions

Date: 2026-09-16 · Status: accepted

## Decision

`list add`/`add-item`/`convert`/`set-level` are implemented with **real OOXML
numbering** instead of Words' style-only approach: the document lazily gets two
`AbstractNumbering` definitions (bullet and ordered, 9 levels each) plus one
`Numbering` each; paragraphs reference them via `NumberingProperty`
(`numId` + `ilvl = level - 1`) and additionally carry a paragraph style id
`ListBullet{level}` / `ListNumber{level}` (un-suffixed id for level 1, matching
Words' style naming) so the observable `style` fields keep Words' values
("List Bullet", "List Number 2", …).

## Context

Words relies on python-docx's template styles (`List Bullet`, `List Number`,
`List Bullet {level}`, …), which carry the numbering indirectly. docx-rs documents
have no template; its numbering model is explicit
(`AbstractNumbering`/`Numbering`/`NumberingId`). Without real definitions, list
items would not render as lists in Word.

## Mechanics

- Definitions are ensured on demand by scanning `Docx.numberings`: an abstract
  numbering whose first level is `bullet` (text `•`) is reused for bullets;
  `decimal` (`%1.`) for ordered; otherwise one is created at `max id + 1`.
  This makes reuse survive save→reopen (the reader restores `numbering.xml`).
- Bullet levels: format `bullet`, text `•`, indent left `720·(level+1)` twips,
  hanging 360. Ordered levels: format `decimal`, text `%{n}.`, same indents.
- Because both lists share one `numId` (like Word's built-in list styles),
  ordered numbering continues across separate `list add` calls — exactly the
  observable behavior Words has.

## Deviations from Words

- Words' template only defines list styles for levels 1–3; `--level 4+` silently
  fell back to the level-1 style (python-docx `KeyError` fallback). Poet supports
  levels 1–9 (validated) — a deliberate superset.

Ported per `Words/src/words/core/document_manager.py` (`add_list_item`,
`convert_to_list`, `set_list_level`) and `Words/src/words/commands/list.py`.
