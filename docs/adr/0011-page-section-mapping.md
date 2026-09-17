# ADR 0011 — page/section mapping, validated inputs, header/footer gap

Date: 2026-09-17 · Status: accepted

Phase-3 `page` commands over docx-rs `SectionProperty` (Words
`document_manager.py` ~668–869). Section addressing follows adr/0008 §2:
embedded sectPr paragraphs in body order, then the body-final
`document.section_property` — the same order `section list` reports.

## Engine mapping

- **Mutations** target a resolved `&mut SectionProperty`. `PageMargin` /
  `PageSize` fields are private, so sides are swapped in via
  `std::mem::take` + builders (`Default` is derived on both).
- **Units**: Words converts CLI values to EMU first (`int(v * 914400)` for
  inches, `int(v / 2.54 * 914400)` for cm, `int(v * 12700)` for points —
  Python `int()` truncates toward zero, mirrored by Rust `as i64`), then
  python-docx serializes EMU to twips (`round(emu / 635)`). Poet replicates
  the full pipeline so `page margins --unit cm 2.54` lands on exactly 1440
  twips, and `page size`/`margins` values match Words' files bit-for-bit.
- **Orientation**: docx-rs writes `w:pgSz/@w:orient` plus `w`/`h`; the
  manager swaps width/height only when they disagree with the requested
  orientation (strict `<`/`>`, like Words — square pages never swap).
  Current w/h are read through the established serde view (adr/0008).
- **Columns**: `SectionProperty.columns` is a public `usize` written as
  `w:cols/@w:num`; set verbatim like Words (no range check).
- **Header/footer**: a section's own part is the `header`/`footer`
  `(rId, part)` tuple. Creating one allocates `rIdHeader{n}` /
  `rIdFooter{n}` (`create_header_rid`), bumps `document_rels.header_count` /
  `footer_count` (the packer writes one rel per count), and registers the
  content-type override (`content_type.add_header()`). Re-invoking replaces
  the text of the part's first paragraph (or appends one) — Words'
  `is_linked_to_previous = False` + `paragraphs[0].text = text` equivalent.
- **Page numbers**: footer first-paragraph gets `w:jc` left/center/right and a
  fresh `PAGE` field run (`fldChar begin` / `instrText PAGE` /
  `fldChar separate` / `fldChar end`, no cached result — Words' `_add_field`
  with `cached=False`). Repeat invocations append more field runs, exactly
  like Words.

## Deviation: validated inputs (Words was silent)

Words silently tolerates bad input here: unknown `--unit` → value
reinterpreted as raw EMU; unknown `--align` → center; an orientation typo →
portrait; unknown `style --type` → no filter. Poet rejects each with
`validation_error` instead (product decision, phase-3 review): silent
reinterpretation is the kind of behavior that corrupts documents invisibly.
Every rejected token is enumerated in its error message.

## Engine gaps: what the reader cannot see on reopen

1. **Header/footer parts on embedded sections.** The docx-rs reader
   re-assigns header/footer **content** only for the body-final section
   property (`reader/read_docx.rs` walks `document.section_property` alone);
   paragraph-embedded sectPrs keep their references but lose the part payload
   on reopen. Single-section documents — every document's section 0 until
   `section add` is used — are body-final and round-trip fully.
2. **`w:orient` attribute.** `reader/section_property.rs` reads only `w`/`h`
   from `w:pgSz`, so the orientation *attribute* is dropped on reopen. The
   swapped width/height survive — and those are what actually drive layout —
   so the rendered page is unchanged.
3. **`w:cols`.** The reader never parses `w:cols`, so `columns` reverts to
   the engine default on reopen. The written package is correct (pinned by a
   zip-level probe in the design tests); only Poet's own view of a reopened
   document loses the value.

Margins/size round-trip for all sections because plain `w:pgMar`/`w:pgSz`
attributes are parsed back.
