# ADR 0008 — engine gaps: field round-trip, sections, images, style names

Date: 2026-09-16 · Status: accepted

Four docx-rs 0.4.22 gaps discovered while porting phase 2, each with a
contained workaround. All are engine-level; the CLI contract stays Words-shaped.

## 1. Field codes must be normalized before saving

docx-rs' reader restores `w:instrText` as `RunChild::InstrTextString`, but its
writer maps that variant to `unreachable!()` (`documents/elements/run.rs`) — a
document with a TOC field becomes **unsaveable** after one round trip.
`DocumentManager::save` therefore walks every run (body, tables, nested
tables, and — since phase 3 — header/footer parts of every section, where
`page page-numbers` parks a PAGE field) converting `InstrTextString` back into
`InstrText` before packing. Pinned by a save→reopen→save fidelity probe.

## 2. Section breaks are paragraphs with embedded `sectPr`

`docx_rs::Section`'s property is `pub(crate)` and has no `section_type`
builder, so `DocumentChild::Section` cannot express start types. But
`ParagraphProperty.section_property` is public, written (`w:p/w:pPr/w:sectPr`)
and read back symmetrically — exactly the on-disk shape python-docx produces.
`section add` inserts a paragraph whose `sectPr` is a clone of the current
body `section_property` (header/footer references stripped, start type set on
the clone — python-docx does the same). Section enumeration = embedded-sectPr
paragraphs in order + the body-final `document.section_property` last.
Word's built-in layout behavior is unaffected by the modeling choice.

## 3. Images are decoded without `Pic::new`

`Pic::new` panics (`expect`) on undecodable input and silently requires the
`image` crate. Poet validates and decodes itself: PNG magic → dimension probe;
anything else → `image::ImageReader` decode → PNG re-encode (mirroring
`Pic::new`'s pipeline; the package collector writes `media/*.png` regardless).
Failures map to `PoetError::Validation`, never a panic. `image` becomes a
direct dependency (it is already in the tree via docx-rs' default features).

## 4. Style ids vs style names

docx-rs stores `w:pStyle` ids (`Heading1`); python-docx surfaces names
(`Heading 1`). A `style_display_name` helper resolves ids through a built-in
table (`Heading1..9`, `ListBullet*`, `ListNumber*`, `TableGrid`, …), then the
document's `styles.xml` (`w:name`), falling back to the raw id. All read-side
payloads (`paragraph list`, `heading list`, table style, md export) report
resolved names so Words' consumers keep working. Writes use canonical ids.

## Observable deviations from Words (recorded once here)

- `section list`/`info` report `start_type` as stable names
  (`new_page|new_column|even_page|odd_page|continuous|none`), not Python enum
  reprs (`"NEW_PAGE (2)"`, `"None"`).
- `heading add/set-level` validate `1..=9`; Words passed level 0 through to
  python-docx's `Title` style. The CLI documents 1–9.
- `paragraph add/insert --style` originally applied the style id without
  validation; since phase 3 (adr/0010) all `--style` arguments validate
  against the registry via `resolve_style_id`.
