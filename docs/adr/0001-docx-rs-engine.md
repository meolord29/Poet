# ADR 0001 — docx-rs as the .docx engine

Date: 2026-09-16 · Status: accepted

## Decision

Poet uses the [`docx-rs`](https://github.com/bokuweb/docx-rs) crate (0.4.x) as its only
.docx engine — no backward compatibility with documents produced by the Python original
is required, and the CLI behavior (not the file internals) is what must match Words.

## Context

Words is built on python-docx, which wraps a generic XML tree and supports surgical OOXML
edits (sibling-wrapped bookmarks, `w:pBdr`, `fldChar` TOC fields, custom XML parts).
Rust has no python-docx equivalent. Candidates:

1. **Custom zip+quick-xml layer** (~500 LOC we own): full control, but the most code.
2. **docx-rs**: mature writer; since 0.4.x it also parses packages back into the *same*
   model the writer uses (`read_docx`), including paragraphs, runs, tables, numbering,
   styles, bookmarks, sections, headers/footers, TOC fields, images, and custom XML
   parts. Children are public `Vec`s, so model-level surgery (insert/move/wrap/delete)
   is straightforward.

## Consequences

- Read-modify-write round-trips go through one `Docx` model; tests must verify
  read→pack→read fidelity for every feature Poet writes (fidelity probes).
- If a needed capability is missing from docx-rs's model, the escape hatch is
  `Docx::build()` (renders the uncompressed OPC package parts), which lets us patch or
  inject parts at the zip level before writing.
- The custom XML part used for metadata storage must be probed early (phase 4) with the
  zip-level fallback kept ready (see phase-4 brief).

## Deviations from Words

None at the CLI contract level. File internals will differ from python-docx output
(element ordering, default namespaces); this is accepted — Poet documents are not
required to be byte-compatible with Words documents.
