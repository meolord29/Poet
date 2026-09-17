# ADR 0012 — metadata in a custom-XML part, side-read at zip level

Date: 2026-09-17 · Status: accepted

`meta` commands and the annotator need per-document metadata persisted inside
the .docx (Words: `MetaEngine` + `AutoAnnotator`, stored via python-docx's
OPC custom-XML support at `/customXml/words_meta.xml` with a `<meta>` wrapper
around compact JSON, version `words_meta_v1`).

## Part format = byte-identical to Words

- Part name `customXml/words_meta.xml`, content type `application/xml`
  (covered by the `<Default Extension="xml"/>` entry docx-rs always emits).
- Payload bytes: `<?xml version="1.0" encoding="UTF-8"?>\n<meta
  xmlns="https://words.local/meta">{json}</meta>` — compact, non-ASCII
  preserved, no trailing newline. Reading is string surgery (find `<meta`,
  find `>`, `rfind </meta>`); malformed wrappers degrade to `{}` and
  therefore to the default payload, like Words' `_unwrap`/`_load`.
- The `words.local` namespace, `words_meta_v1` version string, and `words_meta`
  part name are **format constants, not branding**: keeping them byte-identical
  means files written by either tool carry metadata the other can read. The
  phase 4 "grep clean" rebrand check applies to user-facing docs, not this
  constant.

## Storage mechanism

docx-rs 0.4.22 **writes** custom-XML items (`customXml/item1.xml` +
`itemProps` + rels) but its **reader drops them entirely** (verified: no
custom-item path under `src/reader/`), so in-memory round-tripping through
`docx_rs::Docx` is impossible — the payload would vanish on open→save.
Chosen mechanism, following the established core-properties escape hatch
(adr/0001):

1. The parsed payload lives on `DocumentManager` (`meta: Option<MetaPayload>`;
   `None` = document has no metadata part, like Words' `has_meta() == false`).
   `document new` starts with `None`; `open` side-reads the part from the zip.
2. `save` clones-packs as usual, then one extra repack
   (`write_meta_parts`) replaces-or-appends `customXml/words_meta.xml` and
   rewrites `word/_rels/document.xml.rels`, repointing docx-rs' always-emitted
   (dangling, `item1.xml`) `customXml` relationship at the metadata part —
   fixing a pre-existing engine quirk and making the part OPC-discoverable in
   the same pass. Mutations only reach disk through save/autosave, exactly
   like Words (where `part._blob` flushes on `Document.save()`).

## Engine semantics (ported verbatim from Words)

- `set_document` = whole-object replace; sections upsert by the CLI `name`
  (the stored entry keeps its own JSON-provided `name` — mirroring Words'
  `setdefault("name", ...)` quirk); tables upsert by `schema.id` (the CLI id
  overrides any JSON id, Words' forced `data["id"] = table_id`).
- History entries cap at 1000 (newest kept); `get_history(limit)` returns the
  newest `limit` chronologically; `limit == 0` returns everything (the
  Python `-0` slice artifact, mirrored deliberately).
- `describe()` renames `paragraphs` → `paragraph_annotations` and exposes
  `history_count`; `history` and `version` are not exposed.
- Corrupt payloads (bad JSON, non-dict, mangled wrapper) reset to defaults on
  next mutation, like Words. One tightening: a structurally wrong typed value
  (e.g. `sections` as an object) also resets to defaults instead of surviving
  until a later crash — Poet validates what Words silently tolerated
  (adr/0011 policy).

## Annotator

Words hooks `AutoAnnotator` explicitly inside command methods; Poet mirrors
that placement (hooks in `src/commands/*.rs`, helpers in `core::annotate`):
paragraph add/insert/update, heading add, list add/add-item, run add +
emphasize (history only), table add, and table set-range with `--header`
(full schema rebuild via type inference, merged onto an existing schema so a
prior `meta set-table` name/description survives). History targets use
bookmark ids (`p1`, `t1`, ...) or Words' `index:N` rendering; `run emphasize`
with neither id nor index records `index:None` (Words' dead-fallback artifact,
mirrored).
