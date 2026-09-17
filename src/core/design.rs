//! Design & layout operations (phase 3): run formatting, substring
//! emphasize, paragraph borders, the style registry, and page/section
//! properties — the port of Words' `document_manager.py` design methods
//! (`apply_run_format`, `emphasize_substring`, `set_paragraph_border`,
//! `list_styles`, `apply_style`, margins/orientation/size/header/footer/
//! page-numbers/columns).
//!
//! Free functions over `docx_rs::Docx`, like [`crate::core::content`];
//! `DocumentManager` exposes thin delegates. Algorithms and deviations are
//! recorded in adr/0009 (formatting/emphasize/borders), adr/0010 (style
//! registry), adr/0011 (page/section mapping + validated inputs + the
//! header/footer engine gap).

use std::collections::BTreeSet;

use docx_rs::{
    AlignmentType, BorderType, Color, DocumentChild, FieldChar, FieldCharType, Footer, FooterChild,
    FooterReference, Header, HeaderChild, HeaderReference, InstrPAGE, InstrText, Paragraph,
    ParagraphBorder, ParagraphBorderPosition, ParagraphBorders, ParagraphChild, ParagraphStyle,
    Run, RunChild, RunFonts, RunProperty, SectionProperty, StyleType, TableCellContent, Underline,
};

use crate::core::content::{
    builtin_style_name, hex_color, range_error, require_paragraph_mut, resolve_cell,
    resolve_paragraph, run_text, style_display_name, style_id_from_name, style_name_map,
};
use crate::core::error::PoetError;
use crate::models::data::StyleInfo;

// ---------------------------------------------------------------------
// Formatting (adr/0009)
// ---------------------------------------------------------------------

/// The formatting options shared by `run add`, `run format` and
/// `run emphasize`. `None` fields are left untouched (Words tri-state:
/// `--no-bold` maps to `Some(false)` = explicit off, not inherit).
#[derive(Debug, Clone, Default)]
pub struct FormatSpec {
    /// Bold (explicit off = disabled element).
    pub bold: Option<bool>,
    /// Italic (explicit off = disabled element).
    pub italic: Option<bool>,
    /// Underline (`single` on, `none` off).
    pub underline: Option<bool>,
    /// Font name (sets ascii + hAnsi, like python-docx).
    pub font: Option<String>,
    /// Font size in points (stored as half-points).
    pub size: Option<f64>,
    /// Hex color, e.g. `FF0000` (validated before any mutation).
    pub color: Option<String>,
}

impl FormatSpec {
    /// Whether no option was provided at all.
    pub fn is_empty(&self) -> bool {
        self.bold.is_none()
            && self.italic.is_none()
            && self.underline.is_none()
            && self.font.is_none()
            && self.size.is_none()
            && self.color.is_none()
    }

    /// Pre-flight validation. Words validated the color mid-rebuild and left
    /// partially-formatted state behind; Poet validates before mutating
    /// (adr/0009).
    pub fn validate(&self) -> Result<(), PoetError> {
        if let Some(color) = &self.color {
            hex_color(color)?;
        }
        Ok(())
    }
}

/// Apply the provided (non-`None`) options to a run property. Used by
/// `run add`, `run format` and the emphasize rebuild, so all three share
/// one mapping (property fields are pub and typed on docx-rs).
pub(crate) fn apply_to_property(
    spec: &FormatSpec,
    property: &mut RunProperty,
) -> Result<(), PoetError> {
    if let Some(on) = spec.bold {
        property.bold = Some(if on {
            docx_rs::Bold::new()
        } else {
            docx_rs::Bold::new().disable()
        });
    }
    if let Some(on) = spec.italic {
        property.italic = Some(if on {
            docx_rs::Italic::new()
        } else {
            docx_rs::Italic::new().disable()
        });
    }
    if let Some(on) = spec.underline {
        property.underline = Some(Underline::new(if on { "single" } else { "none" }));
    }
    if let Some(font) = &spec.font {
        property.fonts = Some(RunFonts::new().ascii(font).hi_ansi(font));
    }
    if let Some(size) = spec.size {
        *property = std::mem::take(property).size((size * 2.0).round() as usize);
    }
    if let Some(color) = &spec.color {
        property.color = Some(Color::new(hex_color(color)?));
    }
    Ok(())
}

/// `run format` — all runs of the target paragraph, or only `--run-index`
/// (default = all runs, Words dm.py:314). Reuses the `run add` mapping.
pub fn run_format(
    docx: &mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
    spec: &FormatSpec,
    run_index: Option<usize>,
) -> Result<(), PoetError> {
    let paragraph = require_paragraph_mut(docx, id, index)?;
    let mut runs: Vec<&mut Run> = paragraph
        .children
        .iter_mut()
        .filter_map(|child| match child {
            ParagraphChild::Run(run) => Some(run.as_mut()),
            _ => None,
        })
        .collect();
    if runs.is_empty() {
        return Err(PoetError::Validation(
            "Paragraph has no runs to format".into(),
        ));
    }
    let count = runs.len();
    match run_index {
        Some(wanted) => {
            let run = runs
                .get_mut(wanted)
                .ok_or_else(|| range_error("Run index", wanted, count))?;
            apply_to_property(spec, &mut run.run_property)
        }
        None => {
            for run in runs.iter_mut() {
                apply_to_property(spec, &mut run.run_property)?;
            }
            Ok(())
        }
    }
}

/// `run emphasize` — apply `spec` to occurrences of `find` in one body
/// paragraph or in cell paragraph(s). Cell targeting wins over body
/// id/index (Words dm.py:351 checks the table path first); `--para`
/// omitted searches every paragraph of the cell (dm.py:343-359). Returns
/// the number of formatted occurrences; zero matches succeed.
#[allow(clippy::too_many_arguments)]
pub fn emphasize(
    docx: &mut docx_rs::Docx,
    find: &str,
    spec: &FormatSpec,
    all: bool,
    id: Option<&str>,
    index: Option<usize>,
    table: Option<usize>,
    row: Option<usize>,
    col: Option<usize>,
    para: Option<usize>,
) -> Result<usize, PoetError> {
    if let Some(table) = table {
        let cell = resolve_cell(docx, Some(table), row, col)?;
        let count = cell
            .children
            .iter()
            .filter(|content| matches!(content, TableCellContent::Paragraph(_)))
            .count();
        let targets: Vec<usize> = match para {
            Some(wanted) => {
                if wanted >= count {
                    return Err(range_error("Cell paragraph index", wanted, count));
                }
                vec![wanted]
            }
            None => (0..count).collect(),
        };
        let mut total = 0usize;
        let mut seen = 0usize;
        for content in cell.children.iter_mut() {
            if let TableCellContent::Paragraph(paragraph) = content {
                if targets.contains(&seen) {
                    total += emphasize_in_paragraph(paragraph, find, spec, all)?;
                }
                seen += 1;
            }
        }
        Ok(total)
    } else {
        let paragraph = require_paragraph_mut(docx, id, index)?;
        emphasize_in_paragraph(paragraph, find, spec, all)
    }
}

/// Words' `_emphasize_in_paragraph` (dm.py:361-445), the run-span-splitting
/// algorithm of adr/0009: snapshot cloned properties over the joined run
/// text, cut at run ∪ match boundaries, rebuild segment runs with source
/// formatting and overlay `spec` on matched segments.
fn emphasize_in_paragraph(
    paragraph: &mut Paragraph,
    find: &str,
    spec: &FormatSpec,
    all: bool,
) -> Result<usize, PoetError> {
    if find.is_empty() {
        return Ok(0);
    }
    let mut spans: Vec<(usize, usize, RunProperty)> = Vec::new();
    let mut full = String::new();
    for child in &paragraph.children {
        if let ParagraphChild::Run(run) = child {
            let text = run_text(run);
            spans.push((
                full.len(),
                full.len() + text.len(),
                run.run_property.clone(),
            ));
            full.push_str(&text);
        }
    }
    if spans.is_empty() {
        return Ok(0);
    }
    let mut matches: Vec<(usize, usize)> = Vec::new();
    let mut at = 0usize;
    while let Some(pos) = full[at..].find(find) {
        let start = at + pos;
        let end = start + find.len();
        matches.push((start, end));
        at = end;
        if !all {
            break;
        }
    }
    if matches.is_empty() {
        return Ok(0);
    }
    let mut cuts: BTreeSet<usize> = BTreeSet::from([0, full.len()]);
    for (start, end, _) in &spans {
        cuts.insert(*start);
        cuts.insert(*end);
    }
    for (start, end) in &matches {
        cuts.insert(*start);
        cuts.insert(*end);
    }
    let points: Vec<usize> = cuts.into_iter().collect();
    let source_property = |pos: usize| -> RunProperty {
        spans
            .iter()
            .find(|(start, end, _)| *start <= pos && pos < *end)
            .map(|(_, _, property)| property.clone())
            .unwrap_or_else(|| {
                spans
                    .last()
                    .map(|(_, _, property)| property.clone())
                    .unwrap_or_default()
            })
    };
    let in_match = |pos: usize| matches.iter().any(|(s, e)| *s <= pos && pos < *e);
    let mut rebuilt: Vec<Run> = Vec::new();
    for pair in points.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let segment = &full[start..end];
        if segment.is_empty() {
            continue;
        }
        let mut property = source_property(start);
        if in_match(start) {
            apply_to_property(spec, &mut property)?;
        }
        let mut run = Run::new().add_text(segment);
        run.run_property = property;
        rebuilt.push(run);
    }
    paragraph
        .children
        .retain(|child| !matches!(child, ParagraphChild::Run(_)));
    for run in rebuilt {
        paragraph.children.push(ParagraphChild::Run(Box::new(run)));
    }
    Ok(matches.len())
}

// ---------------------------------------------------------------------
// Paragraph borders (adr/0009)
// ---------------------------------------------------------------------

/// `paragraph border` — set one side of `w:pBdr` on a paragraph. Re-setting
/// the same position replaces it; other sides are preserved (Words'
/// remove-then-append per position). Border styles map through docx-rs'
/// closed `BorderType` enum, so unknown values are rejected (adr/0009).
#[allow(clippy::too_many_arguments)]
pub fn set_paragraph_border(
    docx: &mut docx_rs::Docx,
    position: &str,
    color: &str,
    size: i64,
    space: i64,
    style: &str,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let position = match position {
        "top" => ParagraphBorderPosition::Top,
        "left" => ParagraphBorderPosition::Left,
        "bottom" => ParagraphBorderPosition::Bottom,
        "right" => ParagraphBorderPosition::Right,
        "between" => ParagraphBorderPosition::Between,
        other => {
            return Err(PoetError::Validation(format!(
                "position must be one of ['between', 'bottom', 'left', 'right', 'top'], got '{other}'"
            )));
        }
    };
    let border_type: BorderType = style.parse().map_err(|_| {
        PoetError::Validation(format!(
            "border style must be a docx border value (single, double, dashed, ...), got '{style}'"
        ))
    })?;
    if size < 0 {
        return Err(PoetError::Validation(format!(
            "border size must not be negative, got {size}"
        )));
    }
    if space < 0 {
        return Err(PoetError::Validation(format!(
            "border space must not be negative, got {space}"
        )));
    }
    let border = ParagraphBorder::new(position)
        .val(border_type)
        .size(size as usize)
        .space(space as usize)
        .color(color.to_string());
    let paragraph = require_paragraph_mut(docx, id, index)?;
    let existing = paragraph
        .property
        .borders
        .take()
        .unwrap_or_else(ParagraphBorders::with_empty);
    paragraph.property.borders = Some(existing.set(border));
    Ok(())
}

// ---------------------------------------------------------------------
// Style registry (adr/0010)
// ---------------------------------------------------------------------

/// The four Words style-type tokens and their `str(WD_STYLE_TYPE.X)`
/// renderings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleKind {
    Paragraph,
    Character,
    Table,
    List,
}

impl StyleKind {
    fn label(self) -> &'static str {
        match self {
            StyleKind::Paragraph => "PARAGRAPH (1)",
            StyleKind::Character => "CHARACTER (2)",
            StyleKind::Table => "TABLE (3)",
            StyleKind::List => "LIST (4)",
        }
    }

    fn from_token(token: &str) -> Result<Self, PoetError> {
        match token {
            "paragraph" => Ok(StyleKind::Paragraph),
            "character" => Ok(StyleKind::Character),
            "table" => Ok(StyleKind::Table),
            "list" => Ok(StyleKind::List),
            other => Err(PoetError::Validation(format!(
                "type must be paragraph, character, table or list, got '{other}'"
            ))),
        }
    }

    fn from_docx(kind: &StyleType) -> Option<Self> {
        match kind {
            StyleType::Paragraph => Some(StyleKind::Paragraph),
            StyleType::Character => Some(StyleKind::Character),
            StyleType::Table => Some(StyleKind::Table),
            StyleType::Numbering => Some(StyleKind::List),
            StyleType::Unsupported => None,
        }
    }
}

/// Builtin catalog `(display name, style id, kind)` approximating
/// python-docx's default style table for the styles Poet can meaningfully
/// apply (adr/0010 — a fresh Poet document's styles part holds only
/// `Normal`). Ids follow the docx-rs compaction convention.
const BUILTIN_STYLES: &[(&str, &str, StyleKind)] = &[
    ("Normal", "Normal", StyleKind::Paragraph),
    ("No Spacing", "NoSpacing", StyleKind::Paragraph),
    ("Title", "Title", StyleKind::Paragraph),
    ("Subtitle", "Subtitle", StyleKind::Paragraph),
    ("List Paragraph", "ListParagraph", StyleKind::Paragraph),
    ("Caption", "Caption", StyleKind::Paragraph),
    ("Quote", "Quote", StyleKind::Paragraph),
    ("Intense Quote", "IntenseQuote", StyleKind::Paragraph),
    ("Header", "Header", StyleKind::Paragraph),
    ("Footer", "Footer", StyleKind::Paragraph),
    ("Heading 1", "Heading1", StyleKind::Paragraph),
    ("Heading 2", "Heading2", StyleKind::Paragraph),
    ("Heading 3", "Heading3", StyleKind::Paragraph),
    ("Heading 4", "Heading4", StyleKind::Paragraph),
    ("Heading 5", "Heading5", StyleKind::Paragraph),
    ("Heading 6", "Heading6", StyleKind::Paragraph),
    ("Heading 7", "Heading7", StyleKind::Paragraph),
    ("Heading 8", "Heading8", StyleKind::Paragraph),
    ("Heading 9", "Heading9", StyleKind::Paragraph),
    ("TOC Heading", "TOCHeading", StyleKind::Paragraph),
    ("TOC 1", "TOC1", StyleKind::Paragraph),
    ("TOC 2", "TOC2", StyleKind::Paragraph),
    ("TOC 3", "TOC3", StyleKind::Paragraph),
    ("TOC 4", "TOC4", StyleKind::Paragraph),
    ("TOC 5", "TOC5", StyleKind::Paragraph),
    ("TOC 6", "TOC6", StyleKind::Paragraph),
    ("TOC 7", "TOC7", StyleKind::Paragraph),
    ("TOC 8", "TOC8", StyleKind::Paragraph),
    ("TOC 9", "TOC9", StyleKind::Paragraph),
    (
        "Default Paragraph Font",
        "DefaultParagraphFont",
        StyleKind::Character,
    ),
    ("Emphasis", "Emphasis", StyleKind::Character),
    ("Strong", "Strong", StyleKind::Character),
    ("Book Title", "BookTitle", StyleKind::Character),
    ("Normal Table", "NormalTable", StyleKind::Table),
    ("Table Grid", "TableGrid", StyleKind::Table),
    ("Light Shading", "LightShading", StyleKind::Table),
    ("Light List", "LightList", StyleKind::Table),
    ("Light Grid", "LightGrid", StyleKind::Table),
    ("Medium Shading 1", "MediumShading1", StyleKind::Table),
    ("No List", "NoList", StyleKind::List),
];

/// CLI `--type` token → catalog kind (validated; adr/0011 policy).
fn style_kind_filter(token: Option<&str>) -> Result<Option<StyleKind>, PoetError> {
    token.map(StyleKind::from_token).transpose()
}

/// `style list` — styles-part entries in document order, then catalog
/// entries not shadowed by the part (adr/0010).
pub fn list_styles(
    docx: &docx_rs::Docx,
    type_filter: Option<&str>,
) -> Result<Vec<StyleInfo>, PoetError> {
    let wanted = style_kind_filter(type_filter)?;
    let keep = |kind: StyleKind| wanted.is_none_or(|w| kind == w);
    let mut rows: Vec<StyleInfo> = Vec::new();
    let mut seen_ids: BTreeSet<String> = BTreeSet::new();
    for style in &docx.styles.styles {
        let Some(kind) = StyleKind::from_docx(&style.style_type) else {
            continue;
        };
        if !keep(kind) {
            continue;
        }
        seen_ids.insert(style.style_id.clone());
        rows.push(StyleInfo {
            name: style_display_name(docx, Some(&style.style_id))
                .unwrap_or_else(|| style.style_id.clone()),
            r#type: kind.label().to_string(),
            builtin: true,
        });
    }
    for (name, id, kind) in BUILTIN_STYLES {
        if seen_ids.contains(*id) || !keep(*kind) {
            continue;
        }
        rows.push(StyleInfo {
            name: (*name).to_string(),
            r#type: kind.label().to_string(),
            builtin: true,
        });
    }
    Ok(rows)
}

/// Resolve a CLI style argument to a style id: styles-part display name
/// (exact, like Words' lookup), then a styles-part id (the compaction
/// convention), then the builtin catalog. Unknown names are a lookup miss.
pub fn resolve_style_id(docx: &docx_rs::Docx, name: &str) -> Result<String, PoetError> {
    let map = style_name_map(docx);
    if let Some((id, _)) = map.iter().find(|(_, display)| display.as_str() == name) {
        return Ok(id.clone());
    }
    let candidate = style_id_from_name(name);
    if map.contains_key(&candidate) || builtin_style_name(&candidate).is_some() {
        return Ok(candidate);
    }
    if let Some((_, id, _)) = BUILTIN_STYLES
        .iter()
        .find(|(display, _, _)| *display == name)
    {
        return Ok((*id).to_string());
    }
    Err(PoetError::NotFound(format!("no style with name '{name}'")))
}

/// `style apply` — resolve the paragraph first (Words dm.py:837 order),
/// then set `w:pStyle`. Only paragraph style ids are written, matching
/// Words' single pStyle path.
pub fn apply_style(
    docx: &mut docx_rs::Docx,
    style: &str,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let at = resolve_paragraph(docx, id, index)?;
    let style_id = resolve_style_id(docx, style)?;
    match &mut docx.document.children[at] {
        DocumentChild::Paragraph(paragraph) => {
            paragraph.property.style = Some(ParagraphStyle::new(Some(&style_id)));
            Ok(())
        }
        _ => Err(PoetError::Internal(
            "resolved child is not a paragraph".into(),
        )),
    }
}

// ---------------------------------------------------------------------
// Sections & page layout (adr/0011)
// ---------------------------------------------------------------------

/// Mutable section property by index: embedded sectPr paragraphs in body
/// order, then the body-final property (the `section list` order).
pub(crate) fn section_property_mut(
    docx: &mut docx_rs::Docx,
    index: usize,
) -> Result<&mut SectionProperty, PoetError> {
    let mut seen = 0usize;
    for child in &mut docx.document.children {
        if let DocumentChild::Paragraph(paragraph) = child
            && let Some(property) = paragraph.property.section_property.as_mut()
        {
            if seen == index {
                return Ok(property);
            }
            seen += 1;
        }
    }
    if seen == index {
        return Ok(&mut docx.document.section_property);
    }
    Err(range_error("Section index", index, seen + 1))
}

/// Words' unit tokens (validated — adr/0011 policy; Words reinterpreted
/// unknown units as raw EMU).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Inches,
    Cm,
    Points,
}

fn parse_unit(unit: &str) -> Result<Unit, PoetError> {
    match unit {
        "inches" => Ok(Unit::Inches),
        "cm" => Ok(Unit::Cm),
        "points" => Ok(Unit::Points),
        other => Err(PoetError::Validation(format!(
            "unit must be inches, cm or points, got '{other}'"
        ))),
    }
}

/// Words' `_emus` pipeline: value → EMU (Python `int()` truncation toward
/// zero, mirrored by `as i64`) → twips (python-docx serialization
/// `round(emu / 635)`). `page margins --unit cm 2.54` lands on exactly
/// 1440 twips, like Words.
fn twips(value: f64, unit: Unit) -> i32 {
    let emus = match unit {
        Unit::Inches => value * 914_400.0,
        Unit::Cm => value / 2.54 * 914_400.0,
        Unit::Points => value * 12_700.0,
    };
    (emus / 635.0).round() as i32
}

fn page_size_wh(property: &SectionProperty) -> (f64, f64) {
    let value = serde_json::to_value(&property.page_size).unwrap_or_default();
    let w = value
        .pointer("/w")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let h = value
        .pointer("/h")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    (w, h)
}

/// `page margins` — set only the provided sides on the resolved section.
pub fn set_margins(
    docx: &mut docx_rs::Docx,
    top: Option<f64>,
    bottom: Option<f64>,
    left: Option<f64>,
    right: Option<f64>,
    unit: &str,
    section: usize,
) -> Result<(), PoetError> {
    let property = section_property_mut(docx, section)?;
    let unit = parse_unit(unit)?;
    let mut margin = std::mem::take(&mut property.page_margin);
    if let Some(value) = top {
        margin = margin.top(twips(value, unit));
    }
    if let Some(value) = bottom {
        margin = margin.bottom(twips(value, unit));
    }
    if let Some(value) = left {
        margin = margin.left(twips(value, unit));
    }
    if let Some(value) = right {
        margin = margin.right(twips(value, unit));
    }
    property.page_margin = margin;
    Ok(())
}

/// `page orientation` — set `w:orient` and swap width/height only when they
/// disagree with the request (strict comparisons; square pages never swap —
/// Words dm.py:707-711).
pub fn set_orientation(
    docx: &mut docx_rs::Docx,
    orientation: &str,
    section: usize,
) -> Result<(), PoetError> {
    let landscape = match orientation {
        "landscape" => true,
        "portrait" => false,
        other => {
            return Err(PoetError::Validation(format!(
                "orientation must be portrait or landscape, got '{other}'"
            )));
        }
    };
    let property = section_property_mut(docx, section)?;
    let (w, h) = page_size_wh(property);
    let (w, h) = if (landscape && w < h) || (!landscape && w > h) {
        (h, w)
    } else {
        (w, h)
    };
    property.page_size = std::mem::take(&mut property.page_size)
        .size(w.max(0.0) as u32, h.max(0.0) as u32)
        .orient(if landscape {
            docx_rs::PageOrientationType::Landscape
        } else {
            docx_rs::PageOrientationType::Portrait
        });
    Ok(())
}

/// `page size` — set only the provided dimensions (both omitted is the
/// silent no-op Words performs).
pub fn set_page_size(
    docx: &mut docx_rs::Docx,
    width: Option<f64>,
    height: Option<f64>,
    unit: &str,
    section: usize,
) -> Result<(), PoetError> {
    let property = section_property_mut(docx, section)?;
    let unit = parse_unit(unit)?;
    let (current_w, current_h) = page_size_wh(property);
    let w = match width {
        Some(value) => twips(value, unit).max(0) as u32,
        None => current_w.max(0.0) as u32,
    };
    let h = match height {
        Some(value) => twips(value, unit).max(0) as u32,
        None => current_h.max(0.0) as u32,
    };
    property.page_size = std::mem::take(&mut property.page_size).size(w, h);
    Ok(())
}

/// Replace the text of a header part's first paragraph — python-docx
/// `paragraphs[0].text = text` semantics (runs cleared, other children and
/// properties kept); a paragraph is added when the part is empty.
fn set_first_header_paragraph(part: &mut Vec<HeaderChild>, text: &str) {
    if let Some(HeaderChild::Paragraph(paragraph)) = part.first_mut() {
        paragraph
            .children
            .retain(|child| !matches!(child, ParagraphChild::Run(_)));
        paragraph
            .children
            .push(ParagraphChild::Run(Box::new(Run::new().add_text(text))));
        return;
    }
    part.push(HeaderChild::Paragraph(Box::new(Paragraph::new())));
}

/// The footer twin of [`set_first_header_paragraph`] (`FooterChild` is a
/// distinct enum from `HeaderChild`).
fn set_first_footer_paragraph(part: &mut Vec<FooterChild>, text: &str) {
    if let Some(FooterChild::Paragraph(paragraph)) = part.first_mut() {
        paragraph
            .children
            .retain(|child| !matches!(child, ParagraphChild::Run(_)));
        paragraph
            .children
            .push(ParagraphChild::Run(Box::new(Run::new().add_text(text))));
        return;
    }
    part.push(FooterChild::Paragraph(Box::new(Paragraph::new())));
}

/// `page header` — give the resolved section its own header part (Words'
/// `is_linked_to_previous = False`) carrying `text`, or replace the text of
/// the existing part's first paragraph (adr/0011).
pub fn set_header(docx: &mut docx_rs::Docx, text: &str, section: usize) -> Result<(), PoetError> {
    let has_part = section_property_mut(docx, section)?.header.is_some();
    if has_part {
        let property = section_property_mut(docx, section)?;
        if let Some((_, header)) = property.header.as_mut() {
            set_first_header_paragraph(&mut header.children, text);
        }
        return Ok(());
    }
    let count = docx.document_rels.header_count + 1;
    let rid = docx_rs::create_header_rid(count);
    docx.document_rels.header_count = count;
    docx.content_type = std::mem::take(&mut docx.content_type).add_header();
    let header = Header::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)));
    let property = section_property_mut(docx, section)?;
    property.header_reference = Some(HeaderReference::new("default", &rid));
    property.header = Some((rid, header));
    Ok(())
}

/// `page footer` — the footer twin of [`set_header`].
pub fn set_footer(docx: &mut docx_rs::Docx, text: &str, section: usize) -> Result<(), PoetError> {
    let has_part = section_property_mut(docx, section)?.footer.is_some();
    if has_part {
        let property = section_property_mut(docx, section)?;
        if let Some((_, footer)) = property.footer.as_mut() {
            set_first_footer_paragraph(&mut footer.children, text);
        }
        return Ok(());
    }
    let count = docx.document_rels.footer_count + 1;
    let rid = docx_rs::create_footer_rid(count);
    docx.document_rels.footer_count = count;
    docx.content_type = std::mem::take(&mut docx.content_type).add_footer();
    let footer = Footer::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)));
    let property = section_property_mut(docx, section)?;
    property.footer_reference = Some(FooterReference::new("default", &rid));
    property.footer = Some((rid, footer));
    Ok(())
}

/// The `PAGE` field run: `fldChar begin` / `instrText PAGE` / `fldChar
/// separate` / `fldChar end`, no cached result (Words `_add_field` with
/// `cached=False`).
fn page_field_run() -> Run {
    let mut run = Run::new();
    run.children = vec![
        RunChild::FieldChar(FieldChar::new(FieldCharType::Begin)),
        RunChild::InstrText(Box::new(InstrText::PAGE(InstrPAGE::new()))),
        RunChild::FieldChar(FieldChar::new(FieldCharType::Separate)),
        RunChild::FieldChar(FieldChar::new(FieldCharType::End)),
    ];
    run
}

/// `page page-numbers` — give the section a footer part if missing, then
/// set the first footer paragraph's alignment and append a `PAGE` field run
/// (repeated invocations append more field runs, exactly like Words).
pub fn add_page_numbers(
    docx: &mut docx_rs::Docx,
    align: &str,
    section: usize,
) -> Result<(), PoetError> {
    let alignment = match align {
        "left" => AlignmentType::Left,
        "center" => AlignmentType::Center,
        "right" => AlignmentType::Right,
        other => {
            return Err(PoetError::Validation(format!(
                "align must be left, center or right, got '{other}'"
            )));
        }
    };
    let has_part = section_property_mut(docx, section)?.footer.is_some();
    if !has_part {
        let count = docx.document_rels.footer_count + 1;
        let rid = docx_rs::create_footer_rid(count);
        docx.document_rels.footer_count = count;
        docx.content_type = std::mem::take(&mut docx.content_type).add_footer();
        let footer = Footer::new().add_paragraph(Paragraph::new());
        let property = section_property_mut(docx, section)?;
        property.footer_reference = Some(FooterReference::new("default", &rid));
        property.footer = Some((rid, footer));
    }
    let property = section_property_mut(docx, section)?;
    let Some((_, footer)) = property.footer.as_mut() else {
        return Err(PoetError::Internal("footer part vanished".into()));
    };
    if !matches!(footer.children.first(), Some(FooterChild::Paragraph(_))) {
        footer
            .children
            .push(FooterChild::Paragraph(Box::new(Paragraph::new())));
    }
    let Some(FooterChild::Paragraph(paragraph)) = footer.children.first_mut() else {
        return Err(PoetError::Internal("footer paragraph vanished".into()));
    };
    paragraph.property = std::mem::take(&mut paragraph.property).align(alignment);
    paragraph
        .children
        .push(ParagraphChild::Run(Box::new(page_field_run())));
    Ok(())
}

/// `page columns` — set `w:cols/@w:num` verbatim (Words performs no range
/// check either).
pub fn set_columns(
    docx: &mut docx_rs::Docx,
    count: usize,
    section: usize,
) -> Result<(), PoetError> {
    let property = section_property_mut(docx, section)?;
    property.columns = count;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{add_paragraph, add_run, add_table, get_runs, set_cell};
    use crate::core::document::DocumentManager;

    fn doc() -> docx_rs::Docx {
        docx_rs::Docx::new()
    }

    /// The `n`th body paragraph child (bookmarks interleave paragraphs, so
    /// raw child indices don't line up).
    fn nth_paragraph(docx: &docx_rs::Docx, n: usize) -> &Paragraph {
        let mut seen = 0usize;
        for child in &docx.document.children {
            if let DocumentChild::Paragraph(p) = child {
                if seen == n {
                    return p;
                }
                seen += 1;
            }
        }
        panic!("test expected a paragraph at position {n}");
    }

    fn run_texts(paragraph: &Paragraph) -> Vec<String> {
        paragraph
            .children
            .iter()
            .filter_map(|child| match child {
                ParagraphChild::Run(run) => Some(run_text(run)),
                _ => None,
            })
            .collect()
    }

    fn seed(docx: &mut docx_rs::Docx, text: &str) {
        add_paragraph(docx, text, None, None, false).expect("seed paragraph");
    }

    // -----------------------------------------------------------------
    // run format
    // -----------------------------------------------------------------

    #[test]
    fn run_format_hits_all_runs_by_default_and_one_with_index() {
        let mut docx = doc();
        seed(&mut docx, "first");
        add_run(
            &mut docx,
            "second",
            None,
            Some(0),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("run");
        let spec = FormatSpec {
            bold: Some(true),
            ..Default::default()
        };
        run_format(&mut docx, None, Some(0), &spec, None).expect("format all");
        let runs = get_runs(&docx, None, Some(0)).expect("runs");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "first");
        assert_eq!(runs[0].bold, Some(true));
        assert_eq!(runs[1].bold, Some(true));

        let spec = FormatSpec {
            italic: Some(true),
            ..Default::default()
        };
        run_format(&mut docx, None, Some(0), &spec, Some(0)).expect("format run 0");
        let runs = get_runs(&docx, None, Some(0)).expect("runs");
        assert_eq!(runs[0].italic, Some(true), "targeted run formatted");
        assert_eq!(runs[1].italic, None, "other run untouched");
    }

    #[test]
    fn run_format_rejects_bad_index_and_empty_paragraph() {
        let mut docx = doc();
        seed(&mut docx, "text");
        let spec = FormatSpec {
            bold: Some(true),
            ..Default::default()
        };
        // A paragraph without runs.
        add_paragraph(&mut docx, "", None, Some("empty"), false).expect("empty para");
        let err = run_format(&mut docx, Some("empty"), None, &spec, None).expect_err("no runs");
        assert!(
            matches!(err, PoetError::Validation(ref m) if m == "Paragraph has no runs to format")
        );

        let err = run_format(&mut docx, None, Some(0), &spec, Some(7)).expect_err("index");
        assert!(
            matches!(err, PoetError::NotFound(ref m) if m == "Run index 7 out of range (0..0)")
        );
    }

    // -----------------------------------------------------------------
    // emphasize
    // -----------------------------------------------------------------

    #[test]
    fn emphasize_single_run_match_splits_and_formats() {
        let mut docx = doc();
        seed(&mut docx, "hello world");
        let count = emphasize(
            &mut docx,
            "world",
            &FormatSpec {
                bold: Some(true),
                ..Default::default()
            },
            false,
            None,
            Some(0),
            None,
            None,
            None,
            None,
        )
        .expect("emphasize");
        assert_eq!(count, 1);
        let runs = get_runs(&docx, None, Some(0)).expect("runs");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "hello ");
        assert_eq!(runs[0].bold, None, "unmatched segment unformatted");
        assert_eq!(runs[1].text, "world");
        assert_eq!(runs[1].bold, Some(true));
    }

    #[test]
    fn emphasize_match_spanning_runs_is_formatted_as_one() {
        let mut docx = doc();
        seed(&mut docx, "ab");
        add_run(
            &mut docx,
            "cd",
            None,
            Some(0),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("run");
        let count = emphasize(
            &mut docx,
            "bc",
            &FormatSpec {
                bold: Some(true),
                ..Default::default()
            },
            false,
            None,
            Some(0),
            None,
            None,
            None,
            None,
        )
        .expect("emphasize");
        assert_eq!(count, 1);
        let runs = get_runs(&docx, None, Some(0)).expect("runs");
        // Words' cut points include run boundaries, so a multi-run match
        // rebuilds per-run fragments (each formatted), not one merged run —
        // the observable formatting is what matters.
        assert_eq!(
            runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c", "d"]
        );
        assert_eq!(runs[1].bold, Some(true));
        assert_eq!(runs[2].bold, Some(true));
        assert_eq!(runs[0].bold, None);
        assert_eq!(runs[3].bold, None);
    }

    #[test]
    fn emphasize_all_vs_first_occurrence() {
        let mut docx = doc();
        seed(&mut docx, "xaxaxa");
        let count = emphasize(
            &mut docx,
            "xa",
            &FormatSpec {
                italic: Some(true),
                ..Default::default()
            },
            false,
            None,
            Some(0),
            None,
            None,
            None,
            None,
        )
        .expect("first only");
        assert_eq!(count, 1);
        let count = emphasize(
            &mut docx,
            "xa",
            &FormatSpec {
                italic: Some(true),
                ..Default::default()
            },
            true,
            None,
            Some(0),
            None,
            None,
            None,
            None,
        )
        .expect("all");
        assert_eq!(count, 3);
        assert_eq!(run_texts(nth_paragraph(&docx, 0)), vec!["xa", "xa", "xa"]);
    }

    #[test]
    fn emphasize_no_match_is_a_successful_zero() {
        let mut docx = doc();
        seed(&mut docx, "hello world");
        let count = emphasize(
            &mut docx,
            "zebra",
            &FormatSpec {
                bold: Some(true),
                ..Default::default()
            },
            false,
            None,
            Some(0),
            None,
            None,
            None,
            None,
        )
        .expect("no match succeeds");
        assert_eq!(count, 0);
        let runs = get_runs(&docx, None, Some(0)).expect("runs");
        assert_eq!(runs.len(), 1, "paragraph untouched on no match");
        assert_eq!(runs[0].text, "hello world");
    }

    #[test]
    fn emphasize_preserves_source_formatting_outside_and_inside_match() {
        let mut docx = doc();
        seed(&mut docx, "AA BB");
        let paragraph = require_paragraph_mut(&mut docx, None, Some(0)).expect("para");
        paragraph.children.clear();
        let mut bold = Run::new().add_text("AA ");
        bold.run_property.bold = Some(docx_rs::Bold::new());
        paragraph.children.push(ParagraphChild::Run(Box::new(bold)));
        paragraph
            .children
            .push(ParagraphChild::Run(Box::new(Run::new().add_text("BB"))));

        let count = emphasize(
            &mut docx,
            "B",
            &FormatSpec {
                italic: Some(true),
                ..Default::default()
            },
            true,
            None,
            Some(0),
            None,
            None,
            None,
            None,
        )
        .expect("emphasize");
        assert_eq!(count, 2);
        let runs = get_runs(&docx, None, Some(0)).expect("runs");
        assert_eq!(runs[0].text, "AA ");
        assert_eq!(runs[0].bold, Some(true), "source bold preserved");
        assert_eq!(runs[0].italic, None, "unmatched stays unitalicized");
        assert_eq!(runs[1].text, "B");
        assert_eq!(runs[1].italic, Some(true));
        assert_eq!(runs[1].bold, None, "match inherits plain source only");
    }

    #[test]
    fn emphasize_in_cell_targets_all_or_one_paragraph() {
        let mut docx = doc();
        add_table(&mut docx, 1, 1, None, "Table Grid").expect("table");
        set_cell(&mut docx, 0, 0, "alpha beta", None, Some(0)).expect("cell");
        let count = emphasize(
            &mut docx,
            "beta",
            &FormatSpec {
                bold: Some(true),
                ..Default::default()
            },
            false,
            None,
            None,
            Some(0),
            Some(0),
            Some(0),
            None,
        )
        .expect("cell emphasize");
        assert_eq!(count, 1);
        let err = emphasize(
            &mut docx,
            "beta",
            &FormatSpec {
                bold: Some(true),
                ..Default::default()
            },
            false,
            None,
            None,
            Some(0),
            Some(0),
            Some(0),
            Some(9),
        )
        .expect_err("para out of range");
        assert!(matches!(err, PoetError::NotFound(ref m) if m.contains("Cell paragraph index 9")));
    }

    // -----------------------------------------------------------------
    // paragraph border
    // -----------------------------------------------------------------

    #[test]
    fn border_sets_sides_and_replaces_only_the_same_side() {
        let mut docx = doc();
        seed(&mut docx, "boxed");
        set_paragraph_border(&mut docx, "bottom", "FF0000", 8, 2, "single", None, Some(0))
            .expect("bottom");
        set_paragraph_border(&mut docx, "top", "00FF00", 4, 1, "dashed", None, Some(0))
            .expect("top");
        set_paragraph_border(
            &mut docx,
            "bottom",
            "0000FF",
            12,
            3,
            "double",
            None,
            Some(0),
        )
        .expect("bottom again");

        let paragraph = nth_paragraph(&docx, 0);
        let value = serde_json::to_value(paragraph.property.borders.as_ref().expect("borders"))
            .expect("serde");
        assert_eq!(value["top"]["color"], "00FF00");
        assert_eq!(value["top"]["val"], "dashed");
        assert_eq!(
            value["bottom"]["color"], "0000FF",
            "same-side re-set replaces"
        );
        assert_eq!(value["bottom"]["val"], "double");
        assert_eq!(value["bottom"]["size"], 12);
        assert_eq!(value["bottom"]["space"], 3);
        assert!(value["left"].is_null(), "untouched side stays absent");
        assert!(value["right"].is_null(), "untouched side stays absent");
    }

    #[test]
    fn border_rejects_bad_position_and_negatives_but_coerces_unknown_style() {
        let mut docx = doc();
        seed(&mut docx, "boxed");
        let err =
            set_paragraph_border(&mut docx, "middle", "000000", 4, 1, "single", None, Some(0))
                .expect_err("position");
        assert!(matches!(err, PoetError::Validation(ref m)
                if m == "position must be one of ['between', 'bottom', 'left', 'right', 'top'], got 'middle'"));
        // Unknown style strings are coerced to `single` by the engine's
        // FromStr catch-all — the Words parity choice (no error, a visible
        // border) over validation (adr/0009).
        set_paragraph_border(&mut docx, "top", "000000", 4, 1, "blurple", None, Some(0))
            .expect("unknown style coerced");
        let value = serde_json::to_value(
            nth_paragraph(&docx, 0)
                .property
                .borders
                .as_ref()
                .expect("borders"),
        )
        .expect("serde");
        assert_eq!(value["top"]["val"], "single");
        let err = set_paragraph_border(&mut docx, "left", "000000", -1, 1, "single", None, Some(0))
            .expect_err("size");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("negative")));
    }

    // -----------------------------------------------------------------
    // styles
    // -----------------------------------------------------------------

    #[test]
    fn style_list_reports_catalog_with_words_type_labels() {
        let docx = doc();
        let all = list_styles(&docx, None).expect("list");
        assert!(!all.is_empty());
        let normal = all.iter().find(|s| s.name == "Normal").expect("Normal");
        assert_eq!(normal.r#type, "PARAGRAPH (1)");
        assert!(normal.builtin);
        let grid = all
            .iter()
            .find(|s| s.name == "Table Grid")
            .expect("TableGrid");
        assert_eq!(grid.r#type, "TABLE (3)");
        let no_list = all.iter().find(|s| s.name == "No List").expect("NoList");
        assert_eq!(no_list.r#type, "LIST (4)");

        let characters = list_styles(&docx, Some("character")).expect("characters");
        assert!(characters.iter().all(|s| s.r#type == "CHARACTER (2)"));
        assert_eq!(characters.len(), 4);

        let err = list_styles(&docx, Some("weird")).expect_err("bad token");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("weird")));
    }

    #[test]
    fn style_apply_resolves_names_and_rejects_unknown() {
        let mut docx = doc();
        seed(&mut docx, "styled");
        apply_style(&mut docx, "Heading 2", None, Some(0)).expect("apply");
        let paragraph = nth_paragraph(&docx, 0);
        let style = paragraph.property.style.as_ref().expect("style");
        assert_eq!(style.val, "Heading2");

        let err = apply_style(&mut docx, "Nope", None, Some(0)).expect_err("unknown");
        assert!(matches!(err, PoetError::NotFound(ref m) if m == "no style with name 'Nope'"));

        // Resolution order: paragraph errors precede style errors (Words).
        let err = apply_style(&mut docx, "Nope", Some("ghost"), None).expect_err("paragraph first");
        assert!(
            matches!(err, PoetError::NotFound(ref m) if m.contains("No paragraph with id 'ghost'"))
        );
    }

    #[test]
    fn paragraph_add_validates_style_against_registry() {
        let mut docx = doc();
        add_paragraph(&mut docx, "fine", Some("Quote"), None, false).expect("builtin");
        let err = add_paragraph(&mut docx, "bad", Some("Nonsense"), None, false)
            .expect_err("unknown style");
        assert!(matches!(err, PoetError::NotFound(ref m) if m == "no style with name 'Nonsense'"));
    }

    // -----------------------------------------------------------------
    // page / sections
    // -----------------------------------------------------------------

    #[test]
    fn margins_convert_units_exactly_like_words() {
        let mut docx = doc();
        set_margins(&mut docx, Some(1.0), None, None, None, "inches", 0).expect("inches");
        set_margins(&mut docx, None, Some(2.54), None, None, "cm", 0).expect("cm");
        set_margins(&mut docx, None, None, Some(72.0), None, "points", 0).expect("points");
        set_margins(&mut docx, None, None, None, Some(1.25), "inches", 0).expect("fractional");

        let property = &docx.document.section_property;
        let value = serde_json::to_value(&property.page_margin).expect("serde");
        assert_eq!(value["top"], 1440, "1 inch = 1440 twips");
        assert_eq!(value["bottom"], 1440, "2.54 cm = 1440 twips");
        assert_eq!(value["left"], 1440, "72 points = 1440 twips");
        assert_eq!(value["right"], 1800, "1.25 inch = 1800 twips");

        let err =
            set_margins(&mut docx, Some(1.0), None, None, None, "mm", 0).expect_err("bad unit");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("mm")));

        let err = set_margins(&mut docx, Some(1.0), None, None, None, "inches", 3)
            .expect_err("section first");
        assert!(
            matches!(err, PoetError::NotFound(ref m) if m == "Section index 3 out of range (0..0)")
        );
    }

    #[test]
    fn orientation_swaps_dimensions_and_rejects_typos() {
        let mut docx = doc();
        set_orientation(&mut docx, "landscape", 0).expect("landscape");
        let value = serde_json::to_value(&docx.document.section_property.page_size).expect("serde");
        assert_eq!(value["orient"], "landscape");
        let (w, h) = (
            value["w"].as_f64().unwrap_or(0.0),
            value["h"].as_f64().unwrap_or(0.0),
        );
        assert!(w > h, "landscape requires w > h (was {w}x{h})");
        set_orientation(&mut docx, "portrait", 0).expect("portrait");
        let value = serde_json::to_value(&docx.document.section_property.page_size).expect("serde");
        let (w, h) = (
            value["w"].as_f64().unwrap_or(0.0),
            value["h"].as_f64().unwrap_or(0.0),
        );
        assert!(w < h, "portrait requires w < h");
        assert_eq!(value["orient"], "portrait");

        let err = set_orientation(&mut docx, "lndscape", 0).expect_err("typo");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("lndscape")));
    }

    #[test]
    fn page_size_sets_only_given_dimensions() {
        let mut docx = doc();
        set_page_size(&mut docx, Some(8.5), Some(11.0), "inches", 0).expect("letter");
        let value = serde_json::to_value(&docx.document.section_property.page_size).expect("serde");
        assert_eq!(value["w"], 12240);
        assert_eq!(value["h"], 15840);
    }

    #[test]
    fn header_footer_and_page_numbers_build_parts_and_fields() {
        let mut docx = doc();
        set_header(&mut docx, "Company Confidential", 0).expect("header");
        set_footer(&mut docx, "Poet", 0).expect("footer");
        add_page_numbers(&mut docx, "right", 0).expect("page numbers");

        let property = &docx.document.section_property;
        let (_, header) = property.header.as_ref().expect("header part");
        assert_eq!(header.children.len(), 1);
        let (_, footer) = property.footer.as_ref().expect("footer part");
        assert_eq!(
            property.header_reference.as_ref().expect("ref").id,
            "rIdHeader1"
        );
        assert_eq!(
            property.footer_reference.as_ref().expect("ref").id,
            "rIdFooter1"
        );
        assert_eq!(docx.document_rels.header_count, 1);
        assert_eq!(docx.document_rels.footer_count, 1);

        let page_field_present = footer.children.iter().any(|child| match child {
            FooterChild::Paragraph(p) => p.children.iter().any(|c| match c {
                ParagraphChild::Run(r) => r.children.iter().any(|rc| {
                    matches!(rc, RunChild::InstrText(boxed) if matches!(boxed.as_ref(), InstrText::PAGE(_)))
                }),
                _ => false,
            }),
            _ => false,
        });
        assert!(page_field_present, "footer must carry a PAGE field run");

        // Re-invoking header replaces the first paragraph's text instead of
        // stacking paragraphs.
        set_header(&mut docx, "Replaced", 0).expect("header again");
        let (_, header) = docx
            .document
            .section_property
            .header
            .as_ref()
            .expect("header part");
        assert_eq!(header.children.len(), 1);
        let Some(HeaderChild::Paragraph(p)) = header.children.first() else {
            panic!("header paragraph");
        };
        assert_eq!(run_texts(p), vec!["Replaced"]);

        // Alignment lands on the footer's first paragraph (Justification
        // serializes as a plain string).
        let (_, footer) = docx
            .document
            .section_property
            .footer
            .as_ref()
            .expect("footer part");
        let Some(FooterChild::Paragraph(p)) = footer.children.first() else {
            panic!("footer paragraph");
        };
        let value = serde_json::to_value(&p.property).expect("serde");
        assert_eq!(value["alignment"], "right");

        let err = add_page_numbers(&mut docx, "middle", 0).expect_err("bad align");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("middle")));
    }

    #[test]
    fn columns_set_verbatim() {
        let mut docx = doc();
        set_columns(&mut docx, 3, 0).expect("columns");
        assert_eq!(docx.document.section_property.columns, 3);
    }

    #[test]
    fn section_property_mut_walks_embedded_then_body_final() {
        let mut docx = doc();
        crate::core::content::add_section(&mut docx, "continuous").expect("section");
        // Section 0 = the embedded one; section 1 = body-final.
        set_columns(&mut docx, 2, 0).expect("columns embedded");
        set_columns(&mut docx, 3, 1).expect("columns body");
        assert_eq!(docx.document.section_property.columns, 3);
        let embedded = docx
            .document
            .children
            .iter()
            .find_map(|child| match child {
                DocumentChild::Paragraph(p) => p.property.section_property.as_ref(),
                _ => None,
            })
            .expect("embedded sectPr");
        assert_eq!(embedded.columns, 2);
        let err = set_columns(&mut docx, 1, 2).expect_err("out of range");
        assert!(
            matches!(err, PoetError::NotFound(ref m) if m == "Section index 2 out of range (0..1)")
        );
    }

    // -----------------------------------------------------------------
    // Save→reopen round trips (the phase exit test, adr brief §3)
    // -----------------------------------------------------------------

    fn reopened(path: &std::path::Path) -> DocumentManager {
        let mut mgr = DocumentManager::new();
        mgr.open(path).expect("reopen");
        mgr
    }

    /// Side-read one package part as a string (probe for file-level
    /// survival when the docx-rs reader cannot see a property).
    fn read_part(path: &std::path::Path, name: &str) -> String {
        let file = std::fs::File::open(path).expect("open package");
        let mut archive = zip::ZipArchive::new(file).expect("zip");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut archive.by_name(name).expect("part"), &mut xml)
            .expect("utf8");
        xml
    }

    #[test]
    fn run_formatting_emphasize_and_border_survive_save_reopen() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("design.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.add_paragraph("hello brave world", None, None, false)
            .expect("para");
        mgr.emphasize(
            "brave",
            &FormatSpec {
                bold: Some(true),
                color: Some("FF0000".into()),
                ..Default::default()
            },
            false,
            None,
            Some(0),
            None,
            None,
            None,
            None,
        )
        .expect("emphasize");
        mgr.run_format(
            None,
            Some(0),
            &FormatSpec {
                font: Some("Arial".into()),
                size: Some(12.0),
                ..Default::default()
            },
            Some(0),
        )
        .expect("format first run");
        mgr.set_paragraph_border("bottom", "000000", 4, 1, "single", None, Some(0))
            .expect("border");
        mgr.save("docx", &path).expect("save");
        drop(mgr);

        let mgr = reopened(&path);
        let docx = mgr.docx().expect("doc");
        let runs = get_runs(docx, None, Some(0)).expect("runs");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "hello ");
        assert_eq!(runs[0].font.as_deref(), Some("Arial"));
        assert_eq!(runs[0].size, Some(12.0));
        assert_eq!(runs[0].bold, None);
        assert_eq!(runs[1].text, "brave");
        assert_eq!(runs[1].bold, Some(true));
        assert_eq!(runs[1].color.as_deref(), Some("FF0000"));
        let paragraph = nth_paragraph(docx, 0);
        let value =
            serde_json::to_value(paragraph.property.borders.as_ref().expect("borders")).expect("v");
        assert_eq!(value["bottom"]["val"], "single");
        assert_eq!(value["bottom"]["size"], 4);
    }

    #[test]
    fn page_layout_survives_save_reopen() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("page.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.set_margins(Some(0.5), None, None, None, "inches", 0)
            .expect("margins");
        mgr.set_orientation("landscape", 0).expect("orientation");
        mgr.set_header("H", 0).expect("header");
        mgr.set_footer("F", 0).expect("footer");
        mgr.add_page_numbers("center", 0).expect("numbers");
        mgr.set_columns(2, 0).expect("columns");
        mgr.save("docx", &path).expect("save");
        drop(mgr);

        let mgr = reopened(&path);
        let docx = mgr.docx().expect("doc");
        let property = &docx.document.section_property;
        let margin = serde_json::to_value(&property.page_margin).expect("serde");
        assert_eq!(margin["top"], 720, "0.5 inch survives");
        let size = serde_json::to_value(&property.page_size).expect("serde");
        // The reader drops the w:orient *attribute* (adr/0011 gap), but the
        // swapped w/h — what actually drives layout — survive.
        let (w, h) = (
            size["w"].as_f64().unwrap_or(0.0),
            size["h"].as_f64().unwrap_or(0.0),
        );
        assert!(w > h, "landscape swap survives");
        // The reader also drops w:cols (adr/0011 gap); the written file is
        // still correct, so assert at the package level.
        let document_xml = read_part(&path, "word/document.xml");
        assert!(
            document_xml.contains("w:cols"),
            "w:cols must be written into the package"
        );
        assert!(
            document_xml.contains("w:num=\"2\""),
            "column count must survive in the package"
        );
        let (_, header) = property.header.as_ref().expect("header survives");
        let Some(HeaderChild::Paragraph(p)) = header.children.first() else {
            panic!("header paragraph");
        };
        assert_eq!(run_texts(p), vec!["H"]);
        let (_, footer) = property.footer.as_ref().expect("footer survives");
        let has_page = footer.children.iter().any(|child| match child {
            FooterChild::Paragraph(p) => p.children.iter().any(|c| match c {
                ParagraphChild::Run(r) => r.children.iter().any(
                    |rc| matches!(rc, RunChild::InstrTextString(text) if text.contains("PAGE")),
                ),
                _ => false,
            }),
            _ => false,
        });
        assert!(has_page, "PAGE field survives as reader-side instr text");
    }

    #[test]
    fn style_apply_survives_save_reopen() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("style.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.add_paragraph("styled", None, None, false)
            .expect("para");
        mgr.apply_style("Intense Quote", None, Some(0))
            .expect("apply");
        mgr.save("docx", &path).expect("save");
        drop(mgr);

        let mgr = reopened(&path);
        let docx = mgr.docx().expect("doc");
        let paragraph = nth_paragraph(docx, 0);
        let style = paragraph.property.style.as_ref().expect("style survives");
        assert_eq!(style.val, "IntenseQuote");
    }
}
