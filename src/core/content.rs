//! Content operations on the open document: paragraphs, runs, headings,
//! lists, tables, sections, images, TOC — the phase-2 port of Words'
//! `document_manager.py` (content methods).
//!
//! Free functions over `docx_rs::Docx`; `DocumentManager` exposes thin
//! delegates. Addressing follows adr/0006: body elements by bookmark id
//! (wins) or positional index over body paragraphs/tables only; table cells
//! by `--table/--row/--col/--para`, outside the body index space. Table
//! mapping per adr/0007; lists per adr/0005; field/section/image/style
//! workarounds per adr/0008.

use std::collections::HashMap;

use docx_rs::{
    AbstractNumbering, DocumentChild, DrawingData, IndentLevel, Level, LevelJc, LevelText,
    NumberFormat, Numbering, NumberingId, NumberingProperty, Paragraph, ParagraphChild,
    ParagraphStyle, Pic, Run, RunChild, SectionType, SpecialIndentType, Start, TableCell,
    TableCellContent, TableChild, TableRow, TableRowChild,
};

use crate::core::bookmark::BookmarkManager;
use crate::core::error::PoetError;
use crate::models::data::{
    CellText, FindResult, ImageInfo, ParagraphInfo, RunInfo, SectionDetail, SectionInfo, TableInfo,
};

/// Twips per inch.
const TWIPS_PER_INCH: f64 = 1440.0;
/// EMU per inch.
const EMUS_PER_INCH: f64 = 914400.0;

/// Style-id prefixes whose CLI names differ only by spaces
/// ("Heading 1" → `Heading1`, …) — adr/0008.
const BUILTIN_NAME_PREFIXES: [&str; 10] = [
    "Heading", "List", "Table", "Title", "Subtitle", "Quote", "Intense", "Caption", "TOC", "Normal",
];

// ---------------------------------------------------------------------
// Body-level helpers
// ---------------------------------------------------------------------

fn is_paragraph(child: &DocumentChild) -> bool {
    matches!(child, DocumentChild::Paragraph(_))
}

fn is_block(child: &DocumentChild) -> bool {
    matches!(child, DocumentChild::Paragraph(_) | DocumentChild::Table(_))
}

/// Child indices of body-level paragraphs, in order.
fn paragraph_children(children: &[DocumentChild]) -> Vec<usize> {
    children
        .iter()
        .enumerate()
        .filter(|(_, c)| is_paragraph(c))
        .map(|(i, _)| i)
        .collect()
}

/// Child indices of body-level tables, in order.
fn table_children(children: &[DocumentChild]) -> Vec<usize> {
    children
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, DocumentChild::Table(_)))
        .map(|(i, _)| i)
        .collect()
}

/// Words' range-message format; Python renders `0..-1` for empty sequences.
pub(crate) fn range_error(what: &str, index: usize, len: usize) -> PoetError {
    PoetError::NotFound(format!(
        "{what} {index} out of range (0..{})",
        len as i64 - 1
    ))
}

/// Child index of the block element wrapped by bookmark `name`.
pub(crate) fn bookmark_block(children: &[DocumentChild], name: &str) -> Option<usize> {
    let start = children
        .iter()
        .position(|c| matches!(c, DocumentChild::BookmarkStart(b) if b.name == name))?;
    children[start + 1..]
        .iter()
        .position(is_block)
        .map(|offset| start + 1 + offset)
}

/// Bookmark name immediately wrapping the element at child `index`, if any.
pub(crate) fn name_around(children: &[DocumentChild], index: usize) -> Option<String> {
    match index.checked_sub(1).and_then(|i| children.get(i)) {
        Some(DocumentChild::BookmarkStart(b)) => Some(b.name.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Text / run extraction (python-docx semantics)
// ---------------------------------------------------------------------

/// Concatenated text of a paragraph's direct runs: `w:t` verbatim, tab →
/// `\t`, break/carriage return → `\n` (python-docx `Paragraph.text`).
pub fn paragraph_text(paragraph: &Paragraph) -> String {
    let mut out = String::new();
    for child in &paragraph.children {
        if let ParagraphChild::Run(run) = child {
            push_run_text(run, &mut out);
        }
    }
    out
}

fn push_run_text(run: &Run, out: &mut String) {
    for child in &run.children {
        match child {
            RunChild::Text(t) => out.push_str(&t.text),
            RunChild::Tab(_) => out.push('\t'),
            RunChild::Break(_) | RunChild::CarriageReturn(_) => out.push('\n'),
            _ => {}
        }
    }
}

pub(crate) fn run_text(run: &Run) -> String {
    let mut out = String::new();
    push_run_text(run, &mut out);
    out
}

/// One run's observable formatting (Words' `get_runs` item shape). Property
/// values have private fields, so read through the serde view (adr/0008).
fn run_info(run: &Run) -> RunInfo {
    let value = serde_json::to_value(&run.run_property).unwrap_or_default();
    let underline = value
        .get("underline")
        .and_then(serde_json::Value::as_str)
        .map(|val| val != "none");
    RunInfo {
        text: run_text(run),
        bold: value.get("bold").and_then(serde_json::Value::as_bool),
        italic: value.get("italic").and_then(serde_json::Value::as_bool),
        underline,
        font: value
            .get("fonts")
            .and_then(|fonts| fonts.get("ascii"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        size: value
            .get("sz")
            .and_then(serde_json::Value::as_f64)
            .map(|half_points| half_points / 2.0),
        color: value
            .get("color")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    }
}

fn runs_of(paragraph: &Paragraph) -> Vec<RunInfo> {
    paragraph
        .children
        .iter()
        .filter_map(|child| match child {
            ParagraphChild::Run(run) => Some(run_info(run)),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------
// Styles (adr/0008)
// ---------------------------------------------------------------------

/// id → display-name map from the document's styles part.
pub(crate) fn style_name_map(docx: &docx_rs::Docx) -> HashMap<String, String> {
    docx.styles
        .styles
        .iter()
        .filter_map(|style| {
            let name = serde_json::to_value(&style.name).ok()?;
            let name = name.as_str()?.to_string();
            Some((style.style_id.clone(), name))
        })
        .collect()
}

/// Resolve a style id to its display name (styles part, then built-ins,
/// then the raw id).
pub fn style_display_name(docx: &docx_rs::Docx, id: Option<&str>) -> Option<String> {
    let id = id?;
    if let Some(name) = style_name_map(docx).get(id) {
        return Some(name.clone());
    }
    if let Some(name) = builtin_style_name(id) {
        return Some(name);
    }
    Some(id.to_string())
}

/// Built-in id → name pairs docx-rs documents rely on.
pub(crate) fn builtin_style_name(id: &str) -> Option<String> {
    let named: Option<&str> = match id {
        "TableGrid" => Some("Table Grid"),
        "ListParagraph" => Some("List Paragraph"),
        "NormalTable" => Some("Normal Table"),
        _ => {
            for (stem, label) in [
                ("Heading", "Heading"),
                ("ListBullet", "List Bullet"),
                ("ListNumber", "List Number"),
            ] {
                if let Some(rest) = id.strip_prefix(stem) {
                    if rest.is_empty() {
                        return Some(label.to_string());
                    }
                    if rest.parse::<u8>().is_ok() {
                        return Some(format!("{label} {rest}"));
                    }
                }
            }
            None
        }
    };
    named.map(str::to_string)
}

/// CLI style argument → style id (built-in names collapse their spaces).
pub fn style_id_from_name(name: &str) -> String {
    let compact: String = name.split_whitespace().collect();
    if BUILTIN_NAME_PREFIXES
        .iter()
        .any(|prefix| compact.starts_with(prefix))
    {
        compact
    } else {
        name.to_string()
    }
}

// ---------------------------------------------------------------------
// Paragraph resolution (adr/0006)
// ---------------------------------------------------------------------

/// Resolve a body paragraph to its child index.
pub(crate) fn resolve_paragraph(
    docx: &docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<usize, PoetError> {
    if let Some(id) = id {
        let children = &docx.document.children;
        return bookmark_block(children, id)
            .filter(|&at| is_paragraph(&children[at]))
            .ok_or_else(|| PoetError::NotFound(format!("No paragraph with id '{id}'")));
    }
    if let Some(index) = index {
        let paras = paragraph_children(&docx.document.children);
        return paras
            .get(index)
            .copied()
            .ok_or_else(|| range_error("Paragraph index", index, paras.len()));
    }
    Err(PoetError::Validation(
        "Either id or index is required".into(),
    ))
}

pub(crate) fn require_paragraph_mut<'a>(
    docx: &'a mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<&'a mut Paragraph, PoetError> {
    let at = resolve_paragraph(docx, id, index)?;
    match &mut docx.document.children[at] {
        DocumentChild::Paragraph(p) => Ok(p),
        _ => Err(PoetError::Internal(
            "resolved child is not a paragraph".into(),
        )),
    }
}

// ---------------------------------------------------------------------
// Paragraph operations
// ---------------------------------------------------------------------

fn build_paragraph(text: &str, style_id: Option<&str>, page_break: bool) -> Paragraph {
    let mut paragraph = Paragraph::new();
    if !text.is_empty() {
        paragraph = paragraph.add_run(Run::new().add_text(text));
    }
    if let Some(style_id) = style_id {
        paragraph = paragraph.style(style_id);
    }
    if page_break {
        paragraph = paragraph.page_break_before(true);
    }
    paragraph
}

/// Validate a CLI style name against the registry and produce the style id
/// (adr/0010 — closes the adr/0008 deviation for `--style` arguments).
fn resolve_cli_style(
    docx: &docx_rs::Docx,
    style: Option<&str>,
) -> Result<Option<String>, PoetError> {
    style
        .map(|name| crate::core::design::resolve_style_id(docx, name))
        .transpose()
}

/// Bookmark-wrap a freshly inserted paragraph at child position `at`
/// (adr/0004: the marker lands where the element was).
fn wrap_new_paragraph(
    docx: &mut docx_rs::Docx,
    paragraph: Paragraph,
    at: usize,
    id: Option<&str>,
    prefix: &str,
) -> Result<String, PoetError> {
    let children = &mut docx.document.children;
    children.insert(at, DocumentChild::Paragraph(Box::new(paragraph)));
    BookmarkManager::new(children).ensure(at, id, prefix)
}

/// `add_paragraph` — append at the body end, bookmark-wrap (prefix `p`).
pub fn add_paragraph(
    docx: &mut docx_rs::Docx,
    text: &str,
    style: Option<&str>,
    id: Option<&str>,
    page_break: bool,
) -> Result<String, PoetError> {
    let style_id = resolve_cli_style(docx, style)?;
    let paragraph = build_paragraph(text, style_id.as_deref(), page_break);
    let at = docx.document.children.len();
    wrap_new_paragraph(docx, paragraph, at, id, "p")
}

/// `insert_paragraph` — before the paragraph at `index`; appends when the
/// index is past the end (Words). The new paragraph is moved before a
/// bookmark start marker so it is not mistaken for the wrapped element.
pub fn insert_paragraph(
    docx: &mut docx_rs::Docx,
    index: usize,
    text: &str,
    style: Option<&str>,
    id: Option<&str>,
    page_break: bool,
) -> Result<String, PoetError> {
    let paras = paragraph_children(&docx.document.children);
    if index >= paras.len() {
        return add_paragraph(docx, text, style, id, page_break);
    }
    let mut at = paras[index];
    let style_id = resolve_cli_style(docx, style)?;
    let paragraph = build_paragraph(text, style_id.as_deref(), page_break);
    let children = &mut docx.document.children;
    children.insert(at, DocumentChild::Paragraph(Box::new(paragraph)));
    if at > 0 && matches!(children.get(at - 1), Some(DocumentChild::BookmarkStart(_))) {
        let moved = children.remove(at);
        at -= 1;
        children.insert(at, moved);
    }
    BookmarkManager::new(children).ensure(at, id, "p")
}

/// Remove every direct run of a paragraph (keeps bookmarks, etc.).
fn clear_paragraph_runs(paragraph: &mut Paragraph) {
    paragraph
        .children
        .retain(|child| !matches!(child, ParagraphChild::Run(_)));
}

/// `update_paragraph` — replace all runs with one run of `text` (empty text
/// leaves the paragraph without runs).
pub fn update_paragraph(
    docx: &mut docx_rs::Docx,
    text: &str,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let paragraph = require_paragraph_mut(docx, id, index)?;
    clear_paragraph_runs(paragraph);
    if !text.is_empty() {
        paragraph
            .children
            .push(ParagraphChild::Run(Box::new(Run::new().add_text(text))));
    }
    Ok(())
}

/// `delete_paragraph` — drop the bookmark pair and the element.
pub fn delete_paragraph(
    docx: &mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let at = resolve_paragraph(docx, id, index)?;
    let children = &mut docx.document.children;
    let wrapped = name_around(children, at).is_some();
    BookmarkManager::new(children).remove_around(at);
    // remove_around deletes the start marker *before* the element, so the
    // element shifts down one slot.
    children.remove(if wrapped { at - 1 } else { at });
    Ok(())
}

/// `clear_paragraph` — remove runs, keep the paragraph.
pub fn clear_paragraph(
    docx: &mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let paragraph = require_paragraph_mut(docx, id, index)?;
    clear_paragraph_runs(paragraph);
    Ok(())
}

/// `move_paragraph` — place the paragraph before/after the previous/next
/// body block, skipping bookmark markers (Words' `w:p`/`w:tbl` sibling
/// walk). Returns the paragraph's new positional index.
pub fn move_paragraph(
    docx: &mut docx_rs::Docx,
    direction: &str,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<usize, PoetError> {
    // Words resolves the paragraph first, so state errors win over the
    // direction validation.
    let at = resolve_paragraph(docx, id, index)?;
    if direction != "up" && direction != "down" {
        return Err(PoetError::Validation(format!(
            "direction must be 'up' or 'down', got '{direction}'"
        )));
    }
    let children = &mut docx.document.children;
    let neighbor = if direction == "up" {
        (0..at).rev().find(|&i| is_block(&children[i]))
    } else {
        (at + 1..children.len()).find(|&i| is_block(&children[i]))
    };
    let Some(neighbor) = neighbor else {
        return Err(PoetError::Validation(
            if direction == "up" {
                "Already at the top"
            } else {
                "Already at the bottom"
            }
            .into(),
        ));
    };
    // After removing the element, inserting at `neighbor` lands it directly
    // before (up) or after (down) the neighbor block — `addprevious`/
    // `addnext` semantics.
    let element = children.remove(at);
    children.insert(neighbor, element);
    let paras = paragraph_children(children);
    paras
        .iter()
        .position(|&p| p == neighbor)
        .ok_or_else(|| PoetError::Internal("moved paragraph lost from body".into()))
}

/// `list_paragraphs` — body paragraphs with id, resolved style, text.
pub fn list_paragraphs(docx: &docx_rs::Docx) -> Vec<ParagraphInfo> {
    let children = &docx.document.children;
    paragraph_children(children)
        .into_iter()
        .enumerate()
        .filter_map(|(index, at)| {
            let DocumentChild::Paragraph(p) = &children[at] else {
                return None;
            };
            let style_id = p
                .property
                .style
                .as_ref()
                .map(|style| style.val.clone())
                .unwrap_or_else(|| "Normal".into());
            Some(ParagraphInfo {
                index,
                id: name_around(children, at),
                style: style_display_name(docx, Some(&style_id)),
                text: paragraph_text(p),
            })
        })
        .collect()
}

/// `get_paragraph_text`.
pub fn get_paragraph_text(
    docx: &docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<String, PoetError> {
    let at = resolve_paragraph(docx, id, index)?;
    match &docx.document.children[at] {
        DocumentChild::Paragraph(p) => Ok(paragraph_text(p)),
        _ => Err(PoetError::Internal(
            "resolved child is not a paragraph".into(),
        )),
    }
}

/// `paragraph count`.
pub fn paragraph_count(docx: &docx_rs::Docx) -> usize {
    paragraph_children(&docx.document.children).len()
}

fn body_paragraph_texts(docx: &docx_rs::Docx) -> Vec<(usize, String)> {
    paragraph_children(&docx.document.children)
        .into_iter()
        .enumerate()
        .filter_map(|(index, at)| match &docx.document.children[at] {
            DocumentChild::Paragraph(p) => Some((index, paragraph_text(p))),
            _ => None,
        })
        .collect()
}

/// `find_text` — case-insensitive substring over body paragraphs, then
/// table cells row-major.
pub fn find_text(docx: &docx_rs::Docx, text: &str) -> Vec<FindResult> {
    let needle = text.to_lowercase();
    let mut results = Vec::new();
    for (index, text) in body_paragraph_texts(docx) {
        if text.to_lowercase().contains(&needle) {
            results.push(FindResult::Paragraph {
                kind: "paragraph".into(),
                index,
                text,
            });
        }
    }
    for (table_index, row, col, text) in cell_texts(docx) {
        if text.to_lowercase().contains(&needle) {
            results.push(FindResult::TableCell {
                kind: "table_cell".into(),
                table_index,
                row,
                col,
                text,
            });
        }
    }
    results
}

/// `replace_text` — body paragraphs, then every cell paragraph; Words
/// collapses the joined text into the first run (its formatting wins).
pub fn replace_text(docx: &mut docx_rs::Docx, find: &str, replace: &str) -> usize {
    let mut count = 0;
    let children = &mut docx.document.children;
    for at in paragraph_children(children) {
        if let DocumentChild::Paragraph(p) = &mut children[at] {
            count += replace_in_paragraph(p, find, replace);
        }
    }
    for at in table_children(children) {
        if let DocumentChild::Table(table) = &mut children[at] {
            for row in &mut table.rows {
                let TableChild::TableRow(row) = row;
                for cell in &mut row.cells {
                    let TableRowChild::TableCell(cell) = cell;
                    for content in &mut cell.children {
                        if let TableCellContent::Paragraph(p) = content {
                            count += replace_in_paragraph(p, find, replace);
                        }
                    }
                }
            }
        }
    }
    count
}

fn replace_in_paragraph(paragraph: &mut Paragraph, find: &str, replace: &str) -> usize {
    let mut runs: Vec<&mut Run> = paragraph
        .children
        .iter_mut()
        .filter_map(|child| match child {
            ParagraphChild::Run(run) => Some(run.as_mut()),
            _ => None,
        })
        .collect();
    if runs.is_empty() {
        return 0;
    }
    let full: String = runs.iter().map(|run| run_text(run)).collect();
    let new_full = full.replace(find, replace);
    if new_full == full {
        return 0;
    }
    for (i, run) in runs.iter_mut().enumerate() {
        run.children
            .retain(|child| !matches!(child, RunChild::Text(_)));
        if i == 0 {
            run.children
                .push(RunChild::Text(docx_rs::Text::new(new_full.clone())));
        }
    }
    full.matches(find).count()
}

// ---------------------------------------------------------------------
// Cell addressing (adr/0006: outside the body index space)
// ---------------------------------------------------------------------

/// (table, row, col, text) for every cell, row-major.
fn cell_texts(docx: &docx_rs::Docx) -> Vec<(usize, usize, usize, String)> {
    let mut out = Vec::new();
    for (table_index, at) in table_children(&docx.document.children)
        .into_iter()
        .enumerate()
    {
        let DocumentChild::Table(table) = &docx.document.children[at] else {
            continue;
        };
        for (row, table_row) in table.rows.iter().enumerate() {
            let TableChild::TableRow(table_row) = table_row;
            for (col, cell) in table_row.cells.iter().enumerate() {
                let TableRowChild::TableCell(cell) = cell;
                out.push((table_index, row, col, cell_text(cell)));
            }
        }
    }
    out
}

/// `\n`-joined texts of a cell's paragraphs (python-docx `_Cell.text`).
pub fn cell_text(cell: &TableCell) -> String {
    cell.children
        .iter()
        .filter_map(|content| match content {
            TableCellContent::Paragraph(p) => Some(paragraph_text(p)),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Cell paragraph texts of one cell.
fn cell_paragraph_texts(cell: &TableCell) -> Vec<String> {
    cell.children
        .iter()
        .filter_map(|content| match content {
            TableCellContent::Paragraph(p) => Some(paragraph_text(p)),
            _ => None,
        })
        .collect()
}

/// Resolve a cell by table index + row + col (0-based, adr/0006).
pub(crate) fn resolve_cell(
    docx: &mut docx_rs::Docx,
    table_index: Option<usize>,
    row: Option<usize>,
    col: Option<usize>,
) -> Result<&mut TableCell, PoetError> {
    let (table_index, row, col) = match (table_index, row, col) {
        (Some(t), Some(r), Some(c)) => (t, r, c),
        _ => {
            return Err(PoetError::Validation(
                "table index, row and col are required for cell addressing".into(),
            ));
        }
    };
    let tables = table_children(&docx.document.children);
    let at = tables
        .get(table_index)
        .copied()
        .ok_or_else(|| range_error("Table index", table_index, tables.len()))?;
    let DocumentChild::Table(table) = &mut docx.document.children[at] else {
        return Err(PoetError::Internal("resolved child is not a table".into()));
    };
    let rows_len = table.rows.len();
    let table_row = table.rows.get_mut(row).map(|child| match child {
        TableChild::TableRow(row) => row,
    });
    let Some(table_row) = table_row else {
        return Err(range_error("Cell row", row, rows_len));
    };
    let cols_len = table_row.cells.len();
    let cell = table_row.cells.get_mut(col);
    let Some(TableRowChild::TableCell(cell)) = cell else {
        return Err(range_error("Cell column", col, cols_len));
    };
    Ok(cell)
}

/// Cell paragraph texts — one (`--para`) or all (adr/0006).
pub fn get_cell_paragraph_text(
    docx: &mut docx_rs::Docx,
    table: Option<usize>,
    row: Option<usize>,
    col: Option<usize>,
    para: Option<usize>,
) -> Result<CellText, PoetError> {
    let cell = resolve_cell(docx, table, row, col)?;
    let texts = cell_paragraph_texts(cell);
    match para {
        None => Ok(CellText::Many(texts)),
        Some(para) => {
            let text = texts
                .get(para)
                .ok_or_else(|| range_error("Cell paragraph index", para, texts.len()))?;
            Ok(CellText::One(text.clone()))
        }
    }
}

/// `delete_cell_paragraph` — remove one paragraph from a cell.
pub fn delete_cell_paragraph(
    docx: &mut docx_rs::Docx,
    table: Option<usize>,
    row: Option<usize>,
    col: Option<usize>,
    para: Option<usize>,
) -> Result<(), PoetError> {
    let para = para.ok_or_else(|| PoetError::Validation("para index is required".into()))?;
    let cell = resolve_cell(docx, table, row, col)?;
    let count = cell
        .children
        .iter()
        .filter(|c| matches!(c, TableCellContent::Paragraph(_)))
        .count();
    if para >= count {
        return Err(range_error("Cell paragraph index", para, count));
    }
    let mut seen = 0usize;
    let at = cell
        .children
        .iter()
        .position(|content| match content {
            TableCellContent::Paragraph(_) => {
                let hit = seen == para;
                seen += 1;
                hit
            }
            _ => false,
        })
        .ok_or_else(|| PoetError::Internal("cell paragraph vanished".into()))?;
    cell.children.remove(at);
    Ok(())
}

// ---------------------------------------------------------------------
// Headings
// ---------------------------------------------------------------------

fn validate_heading_level(level: u8) -> Result<(), PoetError> {
    if !(1..=9).contains(&level) {
        return Err(PoetError::Validation(format!(
            "Heading level must be between 1 and 9, got {level}"
        )));
    }
    Ok(())
}

/// `add_heading` — Heading{level} style, bookmark prefix `h`.
pub fn add_heading(
    docx: &mut docx_rs::Docx,
    text: &str,
    level: u8,
    id: Option<&str>,
) -> Result<String, PoetError> {
    validate_heading_level(level)?;
    let paragraph = build_paragraph(text, Some(&format!("Heading{level}")), false);
    let at = docx.document.children.len();
    wrap_new_paragraph(docx, paragraph, at, id, "h")
}

/// `set_heading_level` — restyle an existing paragraph.
pub fn set_heading_level(
    docx: &mut docx_rs::Docx,
    level: u8,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    validate_heading_level(level)?;
    let paragraph = require_paragraph_mut(docx, id, index)?;
    paragraph.property.style = Some(ParagraphStyle::new(Some(format!("Heading{level}"))));
    Ok(())
}

// ---------------------------------------------------------------------
// Lists (adr/0005)
// ---------------------------------------------------------------------

fn list_style_id(ordered: bool, level: u8) -> String {
    let stem = if ordered { "ListNumber" } else { "ListBullet" };
    if level <= 1 {
        stem.to_string()
    } else {
        format!("{stem}{level}")
    }
}

/// Reuse or create the shared bullet/ordered numbering; returns the numId.
fn ensure_numbering(docx: &mut docx_rs::Docx, ordered: bool) -> NumberingId {
    let wanted = if ordered { "decimal" } else { "bullet" };
    let abstract_id = docx
        .numberings
        .abstract_nums
        .iter()
        .find(|abs| {
            abs.levels
                .first()
                .is_some_and(|level| level.format.val == wanted)
        })
        .map(|abs| abs.id);
    if let Some(abstract_id) = abstract_id
        && let Some(num) = docx
            .numberings
            .numberings
            .iter()
            .find(|num| num.abstract_num_id == abstract_id)
    {
        return NumberingId::new(num.id);
    }
    let next = docx
        .numberings
        .abstract_nums
        .iter()
        .map(|abs| abs.id)
        .chain(docx.numberings.numberings.iter().map(|num| num.id))
        .max()
        .map_or(0, |max| max + 1);
    let mut abstract_num = AbstractNumbering::new(next);
    for lvl in 0..9usize {
        let level = Level::new(
            lvl,
            Start::new(1),
            NumberFormat::new(wanted),
            LevelText::new(if ordered {
                format!("%{}.", lvl + 1)
            } else {
                "•".to_string()
            }),
            LevelJc::new("left"),
        )
        .indent(
            Some(720 * (lvl as i32 + 1)),
            Some(SpecialIndentType::Hanging(360)),
            None,
            None,
        );
        abstract_num = abstract_num.add_level(level);
    }
    docx.numberings.abstract_nums.push(abstract_num);
    docx.numberings.numberings.push(Numbering::new(next, next));
    docx.document_rels.has_numberings = true;
    NumberingId::new(next)
}

fn validate_list_level(level: u8) -> Result<(), PoetError> {
    if !(1..=9).contains(&level) {
        return Err(PoetError::Validation(format!(
            "List level must be between 1 and 9, got {level}"
        )));
    }
    Ok(())
}

fn apply_list(paragraph: &mut Paragraph, numbering: NumberingId, ordered: bool, level: u8) {
    paragraph.property.style = Some(ParagraphStyle::new(Some(list_style_id(ordered, level))));
    paragraph.property.numbering_property =
        Some(NumberingProperty::new().add_num(numbering, IndentLevel::new(usize::from(level) - 1)));
    paragraph.has_numbering = true;
}

/// `add_list_item` — real numbering + Words-named style, prefix `l`.
pub fn add_list_item(
    docx: &mut docx_rs::Docx,
    text: &str,
    ordered: bool,
    level: u8,
    id: Option<&str>,
) -> Result<String, PoetError> {
    validate_list_level(level)?;
    let numbering = ensure_numbering(docx, ordered);
    let style = list_style_id(ordered, level);
    let mut paragraph = build_paragraph(text, Some(&style), false);
    apply_list(&mut paragraph, numbering, ordered, level);
    let at = docx.document.children.len();
    wrap_new_paragraph(docx, paragraph, at, id, "l")
}

/// Whether a paragraph is styled as an ordered list item (Words detects
/// this from the style name prefix).
fn paragraph_is_ordered(paragraph: &Paragraph) -> bool {
    paragraph
        .property
        .style
        .as_ref()
        .is_some_and(|style| style.val.starts_with("ListNumber"))
}

/// `convert_to_list`.
pub fn convert_to_list(
    docx: &mut docx_rs::Docx,
    ordered: bool,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let numbering = ensure_numbering(docx, ordered);
    let paragraph = require_paragraph_mut(docx, id, index)?;
    apply_list(paragraph, numbering, ordered, 1);
    Ok(())
}

/// `set_list_level` — keeps the bullet/ordered flavor, changes `ilvl`.
pub fn set_list_level(
    docx: &mut docx_rs::Docx,
    level: u8,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    validate_list_level(level)?;
    let ordered = {
        let paragraph = require_paragraph_mut(docx, id, index)?;
        paragraph_is_ordered(paragraph)
    };
    let numbering = ensure_numbering(docx, ordered);
    let paragraph = require_paragraph_mut(docx, id, index)?;
    apply_list(paragraph, numbering, ordered, level);
    Ok(())
}

// ---------------------------------------------------------------------
// Runs
// ---------------------------------------------------------------------

/// `add_run` — append a run (with any given formatting) to a paragraph.
/// Formatting flows through the shared phase-3 helper so `add`, `format`
/// and `emphasize` share one mapping (adr/0009).
#[allow(clippy::too_many_arguments)]
pub fn add_run(
    docx: &mut docx_rs::Docx,
    text: &str,
    id: Option<&str>,
    index: Option<usize>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    font: Option<&str>,
    size: Option<f64>,
    color: Option<&str>,
) -> Result<(), PoetError> {
    let paragraph = require_paragraph_mut(docx, id, index)?;
    let mut run = Run::new().add_text(text);
    let spec = crate::core::design::FormatSpec {
        bold,
        italic,
        underline,
        font: font.map(str::to_string),
        size,
        color: color.map(str::to_string),
    };
    crate::core::design::apply_to_property(&spec, &mut run.run_property)?;
    paragraph.children.push(ParagraphChild::Run(Box::new(run)));
    Ok(())
}

/// `get_runs` of a body paragraph.
pub fn get_runs(
    docx: &docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<Vec<RunInfo>, PoetError> {
    let at = resolve_paragraph(docx, id, index)?;
    match &docx.document.children[at] {
        DocumentChild::Paragraph(p) => Ok(runs_of(p)),
        _ => Err(PoetError::Internal(
            "resolved child is not a paragraph".into(),
        )),
    }
}

/// `get_cell_runs` — cell addressing with a required `--para`.
pub fn get_cell_runs(
    docx: &mut docx_rs::Docx,
    table: Option<usize>,
    row: Option<usize>,
    col: Option<usize>,
    para: Option<usize>,
) -> Result<Vec<RunInfo>, PoetError> {
    let para = para.ok_or_else(|| PoetError::Validation("para index is required".into()))?;
    let cell = resolve_cell(docx, table, row, col)?;
    let texts = cell_paragraph_texts(cell);
    let count = texts.len();
    if para >= count {
        return Err(range_error("Cell paragraph index", para, count));
    }
    let cell = resolve_cell(docx, table, row, col)?;
    let mut seen = 0usize;
    for content in &cell.children {
        if let TableCellContent::Paragraph(p) = content {
            if seen == para {
                return Ok(runs_of(p));
            }
            seen += 1;
        }
    }
    Err(PoetError::Internal("cell paragraph vanished".into()))
}

/// `clear_runs` of a body paragraph.
pub fn clear_runs(
    docx: &mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let paragraph = require_paragraph_mut(docx, id, index)?;
    clear_paragraph_runs(paragraph);
    Ok(())
}

/// Words' `_hex_to_rgb`: strip `#`, expand 3-digit form, uppercase (the
/// python-docx `RGBColor` rendering). Non-hex input is rejected.
pub(crate) fn hex_color(color: &str) -> Result<String, PoetError> {
    let stripped = color.strip_prefix('#').unwrap_or(color);
    let expanded: String = if stripped.len() == 3 {
        stripped.chars().flat_map(|ch| [ch, ch]).collect()
    } else {
        stripped.to_string()
    };
    if expanded.len() == 6 && expanded.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(expanded.to_uppercase())
    } else {
        Err(PoetError::Validation(format!("invalid color: '{color}'")))
    }
}

// ---------------------------------------------------------------------
// Tables (adr/0007)
// ---------------------------------------------------------------------

fn grid_cell() -> TableCell {
    TableCell::new().add_paragraph(Paragraph::new())
}

/// `add_table` — rows × cols grid, 1"-wide columns, `TableGrid` style.
pub fn add_table(
    docx: &mut docx_rs::Docx,
    rows: usize,
    cols: usize,
    id: Option<&str>,
    style: &str,
) -> Result<String, PoetError> {
    let table = docx_rs::Table::new(
        (0..rows)
            .map(|_| TableRow::new((0..cols).map(|_| grid_cell()).collect()))
            .collect(),
    )
    .set_grid(vec![1440; cols])
    .style(style_id_from_name(style));
    let at = docx.document.children.len();
    let children = &mut docx.document.children;
    children.push(DocumentChild::Table(Box::new(table)));
    BookmarkManager::new(children).ensure(at, id, "t")
}

/// Resolve a body table to its child index.
pub(crate) fn resolve_table(
    docx: &docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<usize, PoetError> {
    if let Some(id) = id {
        let children = &docx.document.children;
        let at = bookmark_block(children, id)
            .ok_or_else(|| PoetError::NotFound(format!("No table with id '{id}'")))?;
        return if matches!(children[at], DocumentChild::Table(_)) {
            Ok(at)
        } else {
            Err(PoetError::NotFound(format!(
                "Bookmark '{id}' does not wrap a table"
            )))
        };
    }
    if let Some(index) = index {
        let tables = table_children(&docx.document.children);
        return tables
            .get(index)
            .copied()
            .ok_or_else(|| range_error("Table index", index, tables.len()));
    }
    Err(PoetError::Validation(
        "Either id or index is required".into(),
    ))
}

fn table_at_mut<'a>(
    docx: &'a mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<&'a mut docx_rs::Table, PoetError> {
    let at = resolve_table(docx, id, index)?;
    match &mut docx.document.children[at] {
        DocumentChild::Table(table) => Ok(table),
        _ => Err(PoetError::Internal("resolved child is not a table".into())),
    }
}

/// `table list` rows (Words' `list_tables`).
pub fn list_tables(docx: &docx_rs::Docx) -> Vec<TableInfo> {
    let children = &docx.document.children;
    table_children(children)
        .into_iter()
        .enumerate()
        .filter_map(|(index, at)| {
            let DocumentChild::Table(table) = &children[at] else {
                return None;
            };
            let style_id = serde_json::to_value(&table.property)
                .ok()
                .and_then(|value| {
                    value
                        .get("style")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                });
            let style = style_id
                .and_then(|id| style_display_name(docx, Some(&id)))
                .or_else(|| Some("Normal Table".into()));
            Some(TableInfo {
                index,
                id: name_around(children, at),
                rows: table.rows.len(),
                cols: table.grid.len(),
                style,
            })
        })
        .collect()
}

/// `table get` — (rows, cols, data).
pub fn table_data(
    docx: &docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(usize, usize, Vec<Vec<String>>), PoetError> {
    let at = resolve_table(docx, id, index)?;
    let DocumentChild::Table(table) = &docx.document.children[at] else {
        return Err(PoetError::Internal("resolved child is not a table".into()));
    };
    let data: Vec<Vec<String>> = table
        .rows
        .iter()
        .map(|row| {
            let TableChild::TableRow(row) = row;
            row.cells
                .iter()
                .map(|cell| {
                    let TableRowChild::TableCell(cell) = cell;
                    cell_text(cell)
                })
                .collect()
        })
        .collect();
    let cols = data.first().map_or(0, Vec::len);
    Ok((data.len(), cols, data))
}

/// Locate a cell of a resolved table for `set_cell`.
fn cell_of_table(
    table: &mut docx_rs::Table,
    row: usize,
    col: usize,
) -> Result<&mut TableCell, PoetError> {
    let rows_len = table.rows.len();
    let table_row = table.rows.get_mut(row).map(|child| match child {
        TableChild::TableRow(row) => row,
    });
    let Some(table_row) = table_row else {
        return Err(range_error("Cell row", row, rows_len));
    };
    let cols_len = table_row.cells.len();
    let Some(TableRowChild::TableCell(cell)) = table_row.cells.get_mut(col) else {
        return Err(range_error("Cell column", col, cols_len));
    };
    Ok(cell)
}

/// `set_cell` — replace the cell's whole content with one paragraph
/// (python-docx `cell.text = value`).
pub fn set_cell(
    docx: &mut docx_rs::Docx,
    row: usize,
    col: usize,
    value: &str,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let table = table_at_mut(docx, id, index)?;
    let cell = cell_of_table(table, row, col)?;
    cell.children = vec![TableCellContent::Paragraph(Box::new(build_paragraph(
        value, None, false,
    )))];
    Ok(())
}

/// JSON-scalar → cell text with rust/JSON conventions (adr/0007).
pub fn scalar_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn fill_cell(cell: &mut TableCell, value: &serde_json::Value) {
    cell.children = vec![TableCellContent::Paragraph(Box::new(build_paragraph(
        &scalar_text(value),
        None,
        false,
    )))];
}

/// `add-row` — append a row sized to the grid, fill `--values`.
pub fn add_row(
    docx: &mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
    values: Option<&[serde_json::Value]>,
) -> Result<(), PoetError> {
    let table = table_at_mut(docx, id, index)?;
    let cols = table.grid.len();
    let mut row = TableRow::new((0..cols).map(|_| grid_cell()).collect());
    if let Some(values) = values {
        for (i, value) in values.iter().enumerate().take(cols) {
            let TableRowChild::TableCell(cell) = &mut row.cells[i];
            fill_cell(cell, value);
        }
    }
    table.rows.push(TableChild::TableRow(row));
    Ok(())
}

/// `add-column` — append a cell to every row and extend the grid.
pub fn add_column(
    docx: &mut docx_rs::Docx,
    id: Option<&str>,
    index: Option<usize>,
    values: Option<&[serde_json::Value]>,
) -> Result<(), PoetError> {
    let table = table_at_mut(docx, id, index)?;
    for (row_index, row) in table.rows.iter_mut().enumerate() {
        let TableChild::TableRow(row) = row;
        let mut cell = grid_cell();
        if let Some(value) = values.and_then(|values| values.get(row_index)) {
            fill_cell(&mut cell, value);
        }
        row.cells.push(TableRowChild::TableCell(cell));
    }
    table.grid.push(1440);
    Ok(())
}

/// `delete-row`.
pub fn delete_row(
    docx: &mut docx_rs::Docx,
    row: usize,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let table = table_at_mut(docx, id, index)?;
    if row >= table.rows.len() {
        return Err(range_error("Row index", row, table.rows.len()));
    }
    table.rows.remove(row);
    Ok(())
}

/// `delete-column` — remove the cell from every row plus the grid entry.
pub fn delete_column(
    docx: &mut docx_rs::Docx,
    col: usize,
    id: Option<&str>,
    index: Option<usize>,
) -> Result<(), PoetError> {
    let table = table_at_mut(docx, id, index)?;
    if col >= table.grid.len() {
        return Err(range_error("Column index", col, table.grid.len()));
    }
    for row in &mut table.rows {
        let TableChild::TableRow(row) = row;
        if col < row.cells.len() {
            row.cells.remove(col);
        }
    }
    table.grid.remove(col);
    Ok(())
}

// ---------------------------------------------------------------------
// Sections (adr/0008: paragraph-embedded sectPr)
// ---------------------------------------------------------------------

/// Section properties in document order: embedded sectPr paragraphs first,
/// body-final property last. Read through serde because page/margin fields
/// have private accessors only (adr/0008).
fn section_properties(docx: &docx_rs::Docx) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for child in &docx.document.children {
        if let DocumentChild::Paragraph(p) = child
            && let Some(property) = &p.property.section_property
            && let Ok(value) = serde_json::to_value(property)
        {
            out.push(value);
        }
    }
    if let Ok(value) = serde_json::to_value(&docx.document.section_property) {
        out.push(value);
    }
    out
}

fn orientation_of(property: &serde_json::Value) -> &'static str {
    let w = property
        .pointer("/pageSize/w")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let h = property
        .pointer("/pageSize/h")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    if w > h { "landscape" } else { "portrait" }
}

fn start_type_name(property: &serde_json::Value) -> String {
    let raw = property
        .get("sectionType")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("none");
    match raw {
        "nextPage" => "new_page",
        "nextColumn" => "new_column",
        "continuous" => "continuous",
        "evenPage" => "even_page",
        "oddPage" => "odd_page",
        _ => "none",
    }
    .to_string()
}

fn twips_inches(property: &serde_json::Value, pointer: &str) -> Option<f64> {
    property
        .pointer(pointer)
        .and_then(serde_json::Value::as_f64)
        .map(|twips| twips / TWIPS_PER_INCH)
}

/// `section list` rows.
pub fn list_sections(docx: &docx_rs::Docx) -> Vec<SectionInfo> {
    section_properties(docx)
        .into_iter()
        .enumerate()
        .map(|(index, property)| SectionInfo {
            index,
            orientation: orientation_of(&property).to_string(),
            start_type: start_type_name(&property),
        })
        .collect()
}

/// `section info`.
pub fn section_detail(docx: &docx_rs::Docx, index: usize) -> Result<SectionDetail, PoetError> {
    let properties = section_properties(docx);
    let property = properties
        .get(index)
        .ok_or_else(|| range_error("Section index", index, properties.len()))?;
    Ok(SectionDetail {
        index,
        orientation: orientation_of(property).to_string(),
        page_width: twips_inches(property, "/pageSize/w"),
        page_height: twips_inches(property, "/pageSize/h"),
        top_margin: twips_inches(property, "/pageMargin/top"),
        bottom_margin: twips_inches(property, "/pageMargin/bottom"),
        left_margin: twips_inches(property, "/pageMargin/left"),
        right_margin: twips_inches(property, "/pageMargin/right"),
    })
}

/// `section add` — insert a paragraph whose sectPr clones the body-final
/// property (header/footer refs stripped), with the requested start type
/// (unknown types default to new_page, like Words' `start_map.get`).
pub fn add_section(docx: &mut docx_rs::Docx, start_type: &str) -> Result<(), PoetError> {
    let section_type = match start_type {
        "new_column" => SectionType::NextColumn,
        "even_page" => SectionType::EvenPage,
        "odd_page" => SectionType::OddPage,
        "continuous" => SectionType::Continuous,
        _ => SectionType::NextPage,
    };
    let mut property = docx.document.section_property.clone();
    property.header_reference = None;
    property.header = None;
    property.first_header_reference = None;
    property.first_header = None;
    property.even_header_reference = None;
    property.even_header = None;
    property.footer_reference = None;
    property.footer = None;
    property.first_footer_reference = None;
    property.first_footer = None;
    property.even_footer_reference = None;
    property.even_footer = None;
    property.section_type = Some(section_type);
    let mut paragraph = Paragraph::new();
    paragraph.property.section_property = Some(property);
    docx.document
        .children
        .push(DocumentChild::Paragraph(Box::new(paragraph)));
    Ok(())
}

/// `section page-break` — empty bookmarked paragraph holding a page-break
/// run (Words reuses `add_paragraph` + a break run).
pub fn add_page_break(docx: &mut docx_rs::Docx, id: Option<&str>) -> Result<String, PoetError> {
    let paragraph = Paragraph::new().add_run(Run::new().add_break(docx_rs::BreakType::Page));
    let at = docx.document.children.len();
    wrap_new_paragraph(docx, paragraph, at, id, "p")
}

// ---------------------------------------------------------------------
// Images (adr/0008: panic-free decode; paths for recursive access)
// ---------------------------------------------------------------------

/// Location of one inline drawing: the hosting paragraph's path from the
/// body children (`[child]`, or `[child, row, col, content]`, nested…).
type Path = Vec<usize>;

fn collect_image_paths(children: &[DocumentChild]) -> Vec<(Path, u32, u32)> {
    let mut out = Vec::new();
    collect_image_paths_in(children, &mut Vec::new(), &mut out);
    out
}

fn collect_image_paths_in(
    children: &[DocumentChild],
    prefix: &mut Path,
    out: &mut Vec<(Path, u32, u32)>,
) {
    for (i, child) in children.iter().enumerate() {
        match child {
            DocumentChild::Paragraph(p) => {
                let mut prefix = prefix.clone();
                prefix.push(i);
                for run in &p.children {
                    if let ParagraphChild::Run(run) = run
                        && let Some((w, h)) = pic_size(run)
                    {
                        out.push((prefix.clone(), w, h));
                    }
                }
            }
            DocumentChild::Table(t) => {
                let mut prefix = prefix.clone();
                prefix.push(i);
                collect_in_table(&t.rows, &mut prefix, out);
            }
            _ => {}
        }
    }
}

fn collect_in_table(rows: &[TableChild], prefix: &mut Path, out: &mut Vec<(Path, u32, u32)>) {
    for (r, row) in rows.iter().enumerate() {
        let TableChild::TableRow(row) = row;
        for (c, cell) in row.cells.iter().enumerate() {
            let TableRowChild::TableCell(cell) = cell;
            for (k, content) in cell.children.iter().enumerate() {
                match content {
                    TableCellContent::Paragraph(p) => {
                        for run in &p.children {
                            if let ParagraphChild::Run(run) = run
                                && let Some((w, h)) = pic_size(run)
                            {
                                let mut path = prefix.clone();
                                path.extend([r, c, k]);
                                out.push((path, w, h));
                            }
                        }
                    }
                    TableCellContent::Table(nested) => {
                        let mut nested_prefix = prefix.clone();
                        nested_prefix.extend([r, c, k]);
                        collect_in_table(&nested.rows, &mut nested_prefix, out);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn pic_size(run: &Run) -> Option<(u32, u32)> {
    run.children.iter().find_map(|child| match child {
        RunChild::Drawing(drawing) => match &drawing.data {
            Some(DrawingData::Pic(pic)) => Some(pic.size),
            _ => None,
        },
        _ => None,
    })
}

/// Mutable paragraph navigation along a collected path.
fn paragraph_at_mut<'a>(
    children: &'a mut [DocumentChild],
    path: &[usize],
) -> Option<&'a mut Paragraph> {
    let (head, rest) = path.split_first()?;
    match children.get_mut(*head)? {
        DocumentChild::Paragraph(p) if rest.is_empty() => Some(p),
        DocumentChild::Table(t) => paragraph_in_table_mut(&mut t.rows, rest),
        _ => None,
    }
}

fn paragraph_in_table_mut<'a>(
    rows: &'a mut [TableChild],
    path: &[usize],
) -> Option<&'a mut Paragraph> {
    let (r, rest) = path.split_first()?;
    let TableChild::TableRow(row) = rows.get_mut(*r)?;
    let (c, rest) = rest.split_first()?;
    let TableRowChild::TableCell(cell) = row.cells.get_mut(*c)?;
    let (k, rest) = rest.split_first()?;
    if rest.is_empty() {
        return match cell.children.get_mut(*k)? {
            TableCellContent::Paragraph(p) => Some(p),
            _ => None,
        };
    }
    match cell.children.get_mut(*k)? {
        TableCellContent::Table(nested) => paragraph_in_table_mut(&mut nested.rows, rest),
        _ => None,
    }
}

fn set_pic_size(paragraph: &mut Paragraph, width: Option<u32>, height: Option<u32>) {
    for child in &mut paragraph.children {
        let ParagraphChild::Run(run) = child else {
            continue;
        };
        for rc in &mut run.children {
            if let RunChild::Drawing(drawing) = rc
                && let Some(DrawingData::Pic(pic)) = &mut drawing.data
            {
                let (w0, h0) = pic.size;
                pic.size = (width.unwrap_or(w0), height.unwrap_or(h0));
            }
        }
    }
}

/// `image add` — read, decode (PNG passthrough, else re-encode), host in a
/// new bookmarked paragraph (prefix `img`). A missing dimension is scaled
/// from the native aspect ratio (python-docx `add_picture`).
pub fn add_image(
    docx: &mut docx_rs::Docx,
    path: &str,
    width: Option<f64>,
    height: Option<f64>,
    id: Option<&str>,
) -> Result<String, PoetError> {
    let bytes = std::fs::read(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PoetError::NotFound(format!("File not found: {path}")),
        _ => PoetError::File(format!("cannot read {path}: {e}")),
    })?;
    let (pic, (native_w, native_h)) = decode_image(&bytes, path)?;
    // With no explicit dimensions the Pic keeps its native px-derived EMU
    // size; otherwise the missing side is scaled from the aspect ratio
    // (python-docx `add_picture`).
    let pic = match (width, height) {
        (None, None) => pic,
        (width, height) => {
            let emu_w = width.map(|w| w * EMUS_PER_INCH);
            let emu_h = height.map(|h| h * EMUS_PER_INCH);
            let final_w = emu_w.unwrap_or_else(|| {
                emu_h
                    .map(|h| h * f64::from(native_w) / f64::from(native_h))
                    .unwrap_or_else(|| f64::from(native_w))
            });
            let final_h = emu_h.unwrap_or_else(|| {
                emu_w
                    .map(|w| w * f64::from(native_h) / f64::from(native_w))
                    .unwrap_or_else(|| f64::from(native_h))
            });
            pic.size(final_w.max(0.0) as u32, final_h.max(0.0) as u32)
        }
    };
    let paragraph = Paragraph::new().add_run(Run::new().add_image(pic));
    let at = docx.document.children.len();
    wrap_new_paragraph(docx, paragraph, at, id, "img")
}

/// PNG passthrough / decode-to-PNG, mirroring `Pic::new` without its panics.
fn decode_image(bytes: &[u8], path: &str) -> Result<(Pic, (u32, u32)), PoetError> {
    let bad = || {
        PoetError::Validation(format!(
            "cannot read image '{path}': unrecognized or corrupt image data"
        ))
    };
    const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.starts_with(&PNG_MAGIC) {
        let img = image::load_from_memory(bytes).map_err(|_| bad())?;
        let (w, h) = (img.width(), img.height());
        return Ok((Pic::new_with_dimensions(bytes.to_vec(), w, h), (w, h)));
    }
    let img = image::load_from_memory(bytes).map_err(|_| bad())?;
    let (w, h) = (img.width(), img.height());
    let mut png = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| bad())?;
    Ok((Pic::new_with_dimensions(png.into_inner(), w, h), (w, h)))
}

/// `image list` / `image get` items.
pub fn images(docx: &docx_rs::Docx) -> Vec<ImageInfo> {
    collect_image_paths(&docx.document.children)
        .into_iter()
        .enumerate()
        .map(|(index, (_, w, h))| ImageInfo {
            index,
            width: w,
            height: h,
            width_inches: Some(f64::from(w) / EMUS_PER_INCH),
            height_inches: Some(f64::from(h) / EMUS_PER_INCH),
        })
        .collect()
}

/// `image resize` — set extent in EMU.
pub fn resize_image(
    docx: &mut docx_rs::Docx,
    index: usize,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), PoetError> {
    let paths = collect_image_paths(&docx.document.children);
    let Some((path, _, _)) = paths.get(index) else {
        return Err(PoetError::NotFound(format!(
            "Image index {index} out of range"
        )));
    };
    let path = path.clone();
    let paragraph = paragraph_at_mut(&mut docx.document.children, &path)
        .ok_or_else(|| PoetError::Internal("image host paragraph vanished".into()))?;
    let width = width.map(|w| (w * EMUS_PER_INCH) as u32);
    let height = height.map(|h| (h * EMUS_PER_INCH) as u32);
    set_pic_size(paragraph, width, height);
    Ok(())
}

/// `image delete` — remove the hosting paragraph entirely (Words walks to
/// the `w:p` ancestor and drops it).
pub fn delete_image(docx: &mut docx_rs::Docx, index: usize) -> Result<(), PoetError> {
    let paths = collect_image_paths(&docx.document.children);
    let Some((path, _, _)) = paths.get(index) else {
        return Err(PoetError::NotFound(format!(
            "Image index {index} out of range"
        )));
    };
    let path = path.clone();
    if remove_paragraph_at(&mut docx.document.children, &path) {
        Ok(())
    } else {
        Err(PoetError::Internal("image host paragraph vanished".into()))
    }
}

fn remove_paragraph_at(children: &mut Vec<DocumentChild>, path: &[usize]) -> bool {
    let Some((head, rest)) = path.split_first() else {
        return false;
    };
    if rest.is_empty() {
        if matches!(children.get(*head), Some(DocumentChild::Paragraph(_))) {
            children.remove(*head);
            return true;
        }
        return false;
    }
    match children.get_mut(*head) {
        Some(DocumentChild::Table(t)) => remove_in_table(&mut t.rows, rest),
        _ => false,
    }
}

fn remove_in_table(rows: &mut [TableChild], path: &[usize]) -> bool {
    let Some((r, rest)) = path.split_first() else {
        return false;
    };
    let Some(TableChild::TableRow(row)) = rows.get_mut(*r) else {
        return false;
    };
    let Some((c, rest)) = rest.split_first() else {
        return false;
    };
    let Some(TableRowChild::TableCell(cell)) = row.cells.get_mut(*c) else {
        return false;
    };
    let Some((k, rest)) = rest.split_first() else {
        return false;
    };
    if rest.is_empty() && matches!(cell.children.get(*k), Some(TableCellContent::Paragraph(_))) {
        cell.children.remove(*k);
        return true;
    }
    match cell.children.get_mut(*k) {
        Some(TableCellContent::Table(nested)) => remove_in_table(&mut nested.rows, rest),
        _ => false,
    }
}

// ---------------------------------------------------------------------
// TOC (field-code construction, ported from Words' `_add_field`)
// ---------------------------------------------------------------------

/// `toc add` — one run: begin / instrText / separate / cached hint / end.
pub fn add_toc(
    docx: &mut docx_rs::Docx,
    levels: &str,
    id: Option<&str>,
) -> Result<String, PoetError> {
    let instruction = format!(r#"TOC \o "{levels}" \h \z \u"#);
    let run = Run::new()
        .add_field_char(docx_rs::FieldCharType::Begin, false)
        .add_instr_text(docx_rs::InstrText::Unsupported(instruction))
        .add_field_char(docx_rs::FieldCharType::Separate, false)
        .add_text("Update this field (Word: right-click → Update Field) to build the index.")
        .add_field_char(docx_rs::FieldCharType::End, false);
    let paragraph = Paragraph::new().add_run(run);
    let at = docx.document.children.len();
    wrap_new_paragraph(docx, paragraph, at, id, "toc")
}

// ---------------------------------------------------------------------
// Field-code normalization before save (adr/0008)
// ---------------------------------------------------------------------

/// Convert reader-restored `InstrTextString` children back into writable
/// `InstrText` values; docx-rs' writer maps the string variant to
/// `unreachable!()`, so a document with a field would otherwise be
/// unsaveable after one round trip.
pub fn normalize_field_texts(docx: &mut docx_rs::Docx) {
    normalize_in_children(&mut docx.document.children);
    // Header/footer parts carry fields too (`page page-numbers` puts a PAGE
    // field in the footer — adr/0011); the reader restores their instrText
    // as the writer-unsafe InstrTextString, so every section's parts must be
    // normalized as well.
    for child in &mut docx.document.children {
        if let DocumentChild::Paragraph(p) = child
            && let Some(property) = p.property.section_property.as_mut()
        {
            normalize_section_parts(property);
        }
    }
    normalize_section_parts(&mut docx.document.section_property);
}

fn normalize_section_parts(property: &mut docx_rs::SectionProperty) {
    for (_, part) in [
        property.header.as_mut(),
        property.first_header.as_mut(),
        property.even_header.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        normalize_in_header_part(&mut part.children);
    }
    for (_, part) in [
        property.footer.as_mut(),
        property.first_footer.as_mut(),
        property.even_footer.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        normalize_in_footer_part(&mut part.children);
    }
}

fn normalize_in_header_part(children: &mut [docx_rs::HeaderChild]) {
    for child in children {
        match child {
            docx_rs::HeaderChild::Paragraph(p) => normalize_in_paragraph(p),
            docx_rs::HeaderChild::Table(t) => normalize_in_table(&mut t.rows),
            docx_rs::HeaderChild::StructuredDataTag(_) => {}
        }
    }
}

fn normalize_in_footer_part(children: &mut [docx_rs::FooterChild]) {
    for child in children {
        match child {
            docx_rs::FooterChild::Paragraph(p) => normalize_in_paragraph(p),
            docx_rs::FooterChild::Table(t) => normalize_in_table(&mut t.rows),
            docx_rs::FooterChild::StructuredDataTag(_) => {}
        }
    }
}

fn normalize_in_children(children: &mut [DocumentChild]) {
    for child in children {
        match child {
            DocumentChild::Paragraph(p) => normalize_in_paragraph(p),
            DocumentChild::Table(t) => normalize_in_table(&mut t.rows),
            _ => {}
        }
    }
}

fn normalize_in_paragraph(paragraph: &mut Paragraph) {
    for child in &mut paragraph.children {
        let ParagraphChild::Run(run) = child else {
            continue;
        };
        for rc in &mut run.children {
            if let RunChild::InstrTextString(text) = rc {
                *rc = RunChild::InstrText(Box::new(docx_rs::InstrText::Unsupported(
                    std::mem::take(text),
                )));
            }
        }
    }
}

fn normalize_in_table(rows: &mut [TableChild]) {
    for row in rows {
        let TableChild::TableRow(row) = row;
        for cell in &mut row.cells {
            let TableRowChild::TableCell(cell) = cell;
            for content in &mut cell.children {
                match content {
                    TableCellContent::Paragraph(p) => normalize_in_paragraph(p),
                    TableCellContent::Table(nested) => normalize_in_table(&mut nested.rows),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::data::CellText;

    fn docx() -> docx_rs::Docx {
        docx_rs::Docx::new()
    }

    fn texts(docx: &docx_rs::Docx) -> Vec<String> {
        body_paragraph_texts(docx)
            .into_iter()
            .map(|(_, t)| t)
            .collect()
    }

    // ------------------------------------------------------------------
    // Paragraph add / insert / addressing
    // ------------------------------------------------------------------

    #[test]
    fn add_paragraph_allocates_bookmarks_and_appends() {
        let mut d = docx();
        let p1 = add_paragraph(&mut d, "one", None, None, false).expect("add");
        let p2 = add_paragraph(&mut d, "two", None, None, false).expect("add");
        assert_eq!((p1.as_str(), p2.as_str()), ("p1", "p2"));
        assert_eq!(texts(&d), ["one", "two"]);
        assert_eq!(paragraph_count(&d), 2);
    }

    #[test]
    fn add_paragraph_with_custom_id_and_style_and_page_break() {
        let mut d = docx();
        let name =
            add_paragraph(&mut d, "x", Some("Intense Quote"), Some("intro"), true).expect("add");
        assert_eq!(name, "intro");
        let paras = paragraph_children(&d.document.children);
        let DocumentChild::Paragraph(p) = &d.document.children[paras[0]] else {
            panic!("paragraph expected");
        };
        assert_eq!(
            p.property.style.as_ref().expect("style").val,
            "IntenseQuote"
        );
        assert_eq!(p.property.page_break_before, Some(true));
    }

    #[test]
    fn add_paragraph_duplicate_custom_id_conflicts() {
        let mut d = docx();
        add_paragraph(&mut d, "a", None, Some("dup"), false).expect("add");
        let err = add_paragraph(&mut d, "b", None, Some("dup"), false).expect_err("dup");
        assert!(matches!(err, PoetError::Conflict(_)));
    }

    #[test]
    fn insert_paragraph_positions_and_past_end_appends() {
        let mut d = docx();
        add_paragraph(&mut d, "one", None, None, false).expect("add");
        add_paragraph(&mut d, "three", None, None, false).expect("add");
        insert_paragraph(&mut d, 1, "two", None, None, false).expect("insert");
        assert_eq!(texts(&d), ["one", "two", "three"]);
        insert_paragraph(&mut d, 99, "four", None, None, false).expect("insert past end");
        assert_eq!(texts(&d), ["one", "two", "three", "four"]);
    }

    #[test]
    fn insert_paragraph_moves_before_wrapping_bookmark() {
        let mut d = docx();
        let p1 = add_paragraph(&mut d, "first", None, None, false).expect("add");
        add_paragraph(&mut d, "second", None, None, false).expect("add");
        insert_paragraph(&mut d, 0, "zeroth", None, None, false).expect("insert");
        // The new paragraph must sit *outside* p1's bookmark wrap and keep
        // p1 resolvable at its new position. Children after insertion:
        // [BS(new), new, BE(new), BS(p1), p1, BE(p1), BS(p2), p2, BE(p2)].
        assert_eq!(texts(&d), ["zeroth", "first", "second"]);
        assert_eq!(bookmark_block(&d.document.children, &p1), Some(4));
    }

    #[test]
    fn paragraph_addressing_errors_match_words_messages() {
        let mut d = docx();
        add_paragraph(&mut d, "one", None, None, false).expect("add");
        let err = get_paragraph_text(&d, None, None).expect_err("no addressing");
        assert!(
            matches!(err, PoetError::Validation(ref m) if m.contains("Either id or index is required"))
        );
        let err = get_paragraph_text(&d, None, Some(5)).expect_err("range");
        assert_eq!(
            err.to_string(),
            "not found: Paragraph index 5 out of range (0..0)"
        );
        let err = get_paragraph_text(&d, Some("ghost"), None).expect_err("unknown id");
        assert_eq!(err.to_string(), "not found: No paragraph with id 'ghost'");
    }

    #[test]
    fn update_delete_clear_paragraph_operate_on_runs() {
        let mut d = docx();
        let id = add_paragraph(&mut d, "hello", None, None, false).expect("add");
        update_paragraph(&mut d, "replaced", Some(&id), None).expect("update");
        assert_eq!(texts(&d), ["replaced"]);
        clear_paragraph(&mut d, Some(&id), None).expect("clear");
        assert_eq!(texts(&d), [""]);
        assert_eq!(paragraph_count(&d), 1, "clear keeps the paragraph");
        delete_paragraph(&mut d, Some(&id), None).expect("delete");
        assert_eq!(paragraph_count(&d), 0);
        let bm = BookmarkManager::new(&mut d.document.children);
        assert!(!bm.has(&id), "delete removes the bookmark pair");
    }

    #[test]
    fn move_paragraph_swaps_with_neighbors_and_reports_new_index() {
        let mut d = docx();
        add_paragraph(&mut d, "a", None, None, false).expect("add");
        add_paragraph(&mut d, "b", None, None, false).expect("add");
        add_paragraph(&mut d, "c", None, None, false).expect("add");
        assert_eq!(
            move_paragraph(&mut d, "down", None, Some(0)).expect("down"),
            1
        );
        assert_eq!(texts(&d), ["b", "a", "c"]);
        assert_eq!(move_paragraph(&mut d, "up", None, Some(1)).expect("up"), 0);
        assert_eq!(texts(&d), ["a", "b", "c"]);
        let err = move_paragraph(&mut d, "up", None, Some(0)).expect_err("top");
        assert_eq!(err.to_string(), "validation error: Already at the top");
        let err = move_paragraph(&mut d, "down", None, Some(2)).expect_err("bottom");
        assert_eq!(err.to_string(), "validation error: Already at the bottom");
        let err = move_paragraph(&mut d, "sideways", None, Some(0)).expect_err("direction");
        assert!(err.to_string().contains("direction must be 'up' or 'down'"));
    }

    // ------------------------------------------------------------------
    // Runs
    // ------------------------------------------------------------------

    #[test]
    fn run_add_applies_and_reports_formatting() {
        let mut d = docx();
        let id = add_paragraph(&mut d, "base", None, None, false).expect("add");
        add_run(
            &mut d,
            "loud",
            Some(&id),
            None,
            Some(true),
            Some(false),
            Some(true),
            Some("Arial"),
            Some(14.0),
            Some("#ff0000"),
        )
        .expect("run");
        let runs = get_runs(&d, Some(&id), None).expect("runs");
        assert_eq!(runs.len(), 2);
        let loud = &runs[1];
        assert_eq!(loud.text, "loud");
        assert_eq!(loud.bold, Some(true));
        assert_eq!(loud.italic, Some(false));
        assert_eq!(loud.underline, Some(true));
        assert_eq!(loud.font.as_deref(), Some("Arial"));
        assert_eq!(loud.size, Some(14.0));
        assert_eq!(loud.color.as_deref(), Some("FF0000"));
        clear_runs(&mut d, Some(&id), None).expect("clear");
        assert!(get_runs(&d, Some(&id), None).expect("runs").is_empty());
    }

    #[test]
    fn run_add_no_bold_flags_explicitly_disable() {
        let mut d = docx();
        let id = add_paragraph(&mut d, "base", None, None, false).expect("add");
        add_run(
            &mut d,
            "plain",
            Some(&id),
            None,
            Some(false),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("run");
        let runs = get_runs(&d, Some(&id), None).expect("runs");
        assert_eq!(runs[1].bold, Some(false));
    }

    #[test]
    fn run_add_invalid_color_is_a_validation_error() {
        let mut d = docx();
        let id = add_paragraph(&mut d, "base", None, None, false).expect("add");
        let err = add_run(
            &mut d,
            "x",
            Some(&id),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("zzz"),
        )
        .expect_err("bad color");
        assert!(matches!(err, PoetError::Validation(_)));
    }

    // ------------------------------------------------------------------
    // Headings / lists
    // ------------------------------------------------------------------

    #[test]
    fn heading_add_sets_style_and_bookmark_prefix() {
        let mut d = docx();
        let id = add_heading(&mut d, "Top", 2, None).expect("heading");
        assert!(id.starts_with('h'));
        let info = &list_paragraphs(&d)[0];
        assert_eq!(info.style.as_deref(), Some("Heading 2"));
        assert_eq!(info.id.as_deref(), Some(id.as_str()));
    }

    #[test]
    fn heading_levels_outside_1_9_are_rejected() {
        let mut d = docx();
        assert!(add_heading(&mut d, "x", 0, None).is_err());
        assert!(add_heading(&mut d, "x", 10, None).is_err());
        assert!(add_heading(&mut d, "x", 9, None).is_ok());
    }

    #[test]
    fn heading_set_level_restyles_paragraph() {
        let mut d = docx();
        let id = add_paragraph(&mut d, "text", None, None, false).expect("add");
        set_heading_level(&mut d, 3, Some(&id), None).expect("set");
        assert_eq!(list_paragraphs(&d)[0].style.as_deref(), Some("Heading 3"));
        assert!(set_heading_level(&mut d, 12, Some(&id), None).is_err());
    }

    #[test]
    fn list_add_creates_shared_numbering_and_words_style_names() {
        let mut d = docx();
        let l1 = add_list_item(&mut d, "first", false, 1, None).expect("item");
        let l2 = add_list_item(&mut d, "second", true, 2, None).expect("item");
        assert!(l1.starts_with('l'));
        assert!(l2.starts_with('l'));
        // Two definitions total: one bullet, one ordered (shared).
        assert_eq!(d.numberings.abstract_nums.len(), 2);
        assert_eq!(d.numberings.numberings.len(), 2);
        // Adding another item creates no new definitions.
        add_list_item(&mut d, "third", false, 1, None).expect("item");
        assert_eq!(d.numberings.abstract_nums.len(), 2);
        let items = list_paragraphs(&d);
        assert_eq!(items[0].style.as_deref(), Some("List Bullet"));
        assert_eq!(items[1].style.as_deref(), Some("List Number 2"));
        let DocumentChild::Paragraph(p) =
            &d.document.children[paragraph_children(&d.document.children)[1]]
        else {
            panic!("paragraph expected");
        };
        let numbering = p.property.numbering_property.as_ref().expect("numbering");
        assert_eq!(numbering.level.as_ref().expect("ilvl").val, 1);
        assert!(p.has_numbering);
    }

    #[test]
    fn list_convert_and_set_level_keep_flavor() {
        let mut d = docx();
        let id = add_paragraph(&mut d, "item", None, None, false).expect("add");
        convert_to_list(&mut d, true, Some(&id), None).expect("convert");
        assert_eq!(list_paragraphs(&d)[0].style.as_deref(), Some("List Number"));
        set_list_level(&mut d, 3, Some(&id), None).expect("level");
        assert_eq!(
            list_paragraphs(&d)[0].style.as_deref(),
            Some("List Number 3")
        );
        // Flavor detection follows the style id prefix.
        set_list_level(&mut d, 1, Some(&id), None).expect("level back");
        assert_eq!(list_paragraphs(&d)[0].style.as_deref(), Some("List Number"));
    }

    #[test]
    fn list_levels_outside_1_9_are_rejected() {
        let mut d = docx();
        assert!(add_list_item(&mut d, "x", false, 10, None).is_err());
        assert!(add_list_item(&mut d, "x", false, 0, None).is_err());
    }

    // ------------------------------------------------------------------
    // Tables
    // ------------------------------------------------------------------

    #[test]
    fn table_add_builds_grid_and_bookmark() {
        let mut d = docx();
        let id = add_table(&mut d, 2, 3, None, "Table Grid").expect("table");
        assert!(id.starts_with('t'));
        let tables = list_tables(&d);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].rows, 2);
        assert_eq!(tables[0].cols, 3);
        assert_eq!(tables[0].style.as_deref(), Some("Table Grid"));
        let (rows, cols, data) = table_data(&d, None, Some(0)).expect("data");
        assert_eq!((rows, cols), (2, 3));
        assert_eq!(data, vec![vec!["".to_string(); 3]; 2]);
    }

    #[test]
    fn table_set_cell_replaces_whole_cell_content() {
        let mut d = docx();
        let id = add_table(&mut d, 1, 1, None, "Table Grid").expect("table");
        // Seed a multi-paragraph cell by hand, then set-cell to one paragraph.
        set_cell(&mut d, 0, 0, "new", Some(&id), None).expect("set");
        set_cell(&mut d, 0, 0, "again", Some(&id), None).expect("set");
        let (_, _, data) = table_data(&d, None, Some(0)).expect("data");
        assert_eq!(data, vec![vec!["again".to_string()]]);
    }

    #[test]
    fn table_set_cell_out_of_range_messages() {
        let mut d = docx();
        let id = add_table(&mut d, 2, 2, None, "Table Grid").expect("table");
        let err = set_cell(&mut d, 5, 0, "x", Some(&id), None).expect_err("row");
        assert_eq!(err.to_string(), "not found: Cell row 5 out of range (0..1)");
        let err = set_cell(&mut d, 0, 5, "x", Some(&id), None).expect_err("col");
        assert_eq!(
            err.to_string(),
            "not found: Cell column 5 out of range (0..1)"
        );
    }

    #[test]
    fn table_add_delete_row_and_column_update_grid() {
        let mut d = docx();
        let id = add_table(&mut d, 1, 2, None, "Table Grid").expect("table");
        let values = vec![serde_json::Value::from("a"), serde_json::Value::from(true)];
        add_row(&mut d, Some(&id), None, Some(&values)).expect("row");
        let (_, _, data) = table_data(&d, None, Some(0)).expect("data");
        assert_eq!(data[1], ["a".to_string(), "true".to_string()]);
        // Column values fill top-down from row 0 (Words: col.cells[i]).
        let col_values = vec![serde_json::Value::from(""), serde_json::Value::from("z")];
        add_column(&mut d, Some(&id), None, Some(&col_values)).expect("column");
        let (rows, cols, data) = table_data(&d, None, Some(0)).expect("data");
        assert_eq!((rows, cols), (2, 3));
        assert_eq!(data[0][2], "");
        assert_eq!(data[1][2], "z");
        delete_row(&mut d, 0, Some(&id), None).expect("delete row");
        delete_column(&mut d, 1, Some(&id), None).expect("delete column");
        let (rows, cols, data) = table_data(&d, None, Some(0)).expect("data");
        assert_eq!((rows, cols), (1, 2));
        assert_eq!(data[0], ["a".to_string(), "z".to_string()]);
        assert!(delete_row(&mut d, 9, Some(&id), None).is_err());
        assert!(delete_column(&mut d, 9, Some(&id), None).is_err());
    }

    #[test]
    fn table_addressing_errors_match_words_messages() {
        let mut d = docx();
        let p = add_paragraph(&mut d, "para", None, None, false).expect("add");
        add_table(&mut d, 1, 1, None, "Table Grid").expect("table");
        let err = table_data(&d, Some("ghost"), None).expect_err("unknown");
        assert_eq!(err.to_string(), "not found: No table with id 'ghost'");
        let err = table_data(&d, Some(&p), None).expect_err("paragraph id");
        assert!(err.to_string().contains("does not wrap a table"));
        let err = table_data(&d, None, Some(3)).expect_err("range");
        assert_eq!(
            err.to_string(),
            "not found: Table index 3 out of range (0..0)"
        );
        let err = table_data(&d, None, None).expect_err("no addressing");
        assert!(err.to_string().contains("Either id or index is required"));
    }

    // ------------------------------------------------------------------
    // Cell addressing (outside the body index space)
    // ------------------------------------------------------------------

    #[test]
    fn cell_paragraph_text_returns_one_or_all() {
        let mut d = docx();
        let id = add_table(&mut d, 1, 1, None, "Table Grid").expect("table");
        set_cell(&mut d, 0, 0, "only", Some(&id), None).expect("set");
        let all = get_cell_paragraph_text(&mut d, Some(0), Some(0), Some(0), None).expect("all");
        assert!(matches!(all, CellText::Many(ref v) if v == &["only".to_string()]));
        let one = get_cell_paragraph_text(&mut d, Some(0), Some(0), Some(0), Some(0)).expect("one");
        assert!(matches!(one, CellText::One(ref t) if t == "only"));
        let err =
            get_cell_paragraph_text(&mut d, Some(0), Some(0), Some(0), Some(4)).expect_err("para");
        assert_eq!(
            err.to_string(),
            "not found: Cell paragraph index 4 out of range (0..0)"
        );
        let err =
            get_cell_paragraph_text(&mut d, Some(0), Some(0), None, None).expect_err("no col");
        assert!(err.to_string().contains("row and col are required"));
    }

    #[test]
    fn cell_paragraph_delete_removes_only_that_paragraph() {
        let mut d = docx();
        let id = add_table(&mut d, 1, 1, None, "Table Grid").expect("table");
        set_cell(&mut d, 0, 0, "one", Some(&id), None).expect("set");
        let cell = resolve_cell(&mut d, Some(0), Some(0), Some(0)).expect("cell");
        cell.children.push(TableCellContent::Paragraph(Box::new(
            Paragraph::new().add_run(Run::new().add_text("two")),
        )));
        let err = delete_cell_paragraph(&mut d, Some(0), Some(0), Some(0), None)
            .expect_err("para required");
        assert!(err.to_string().contains("para index is required"));
        delete_cell_paragraph(&mut d, Some(0), Some(0), Some(0), Some(0)).expect("delete");
        let all = get_cell_paragraph_text(&mut d, Some(0), Some(0), Some(0), None).expect("all");
        assert!(matches!(all, CellText::Many(ref v) if v == &["two".to_string()]));
    }

    #[test]
    fn cell_addressing_is_isolated_from_body_indices() {
        let mut d = docx();
        let id = add_table(&mut d, 1, 1, None, "Table Grid").expect("table");
        add_paragraph(&mut d, "body text", None, None, false).expect("add");
        set_cell(&mut d, 0, 0, "cell text", Some(&id), None).expect("set");
        // Body index 0 is the body paragraph, not the cell paragraph.
        assert_eq!(
            get_paragraph_text(&d, None, Some(0)).expect("body"),
            "body text"
        );
        // Cell addressing never sees body paragraphs.
        let cell =
            get_cell_paragraph_text(&mut d, Some(0), Some(0), Some(0), Some(0)).expect("cell");
        assert!(matches!(cell, CellText::One(ref t) if t == "cell text"));
    }

    // ------------------------------------------------------------------
    // Find / replace
    // ------------------------------------------------------------------

    #[test]
    fn find_text_matches_paragraphs_then_cells_case_insensitively() {
        let mut d = docx();
        add_paragraph(&mut d, "Revenue is up", None, None, false).expect("add");
        let id = add_table(&mut d, 1, 1, None, "Table Grid").expect("table");
        set_cell(&mut d, 0, 0, "revenue: 42", Some(&id), None).expect("set");
        add_paragraph(&mut d, "nothing here", None, None, false).expect("add");
        let results = find_text(&d, "REVENUE");
        assert_eq!(results.len(), 2);
        assert!(matches!(
            &results[0],
            FindResult::Paragraph { index: 0, .. }
        ));
        assert!(matches!(
            &results[1],
            FindResult::TableCell {
                table_index: 0,
                row: 0,
                col: 0,
                ..
            }
        ));
        assert!(find_text(&d, "absent").is_empty());
    }

    #[test]
    fn replace_text_collapses_into_first_run_and_counts() {
        let mut d = docx();
        let id = add_paragraph(&mut d, "a", None, None, false).expect("add");
        add_run(
            &mut d,
            "big",
            Some(&id),
            None,
            Some(true),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("run");
        add_run(
            &mut d,
            " whale",
            Some(&id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("run");
        let count = replace_text(&mut d, "big", "small");
        assert_eq!(count, 1);
        // full = "abig whale" → "asmall whale" (python str.replace).
        assert_eq!(texts(&d), ["asmall whale"]);
        // Words puts the joined text into the first run (its formatting —
        // here unformatted — wins).
        let runs = get_runs(&d, Some(&id), None).expect("runs");
        assert_eq!(runs[0].text, "asmall whale");
        assert_eq!(runs[0].bold, None);
        assert_eq!(runs[1].text, "");
    }

    // ------------------------------------------------------------------
    // Sections / page break
    // ------------------------------------------------------------------

    #[test]
    fn section_add_creates_embedded_break_and_list_reports_it() {
        let mut d = docx();
        add_paragraph(&mut d, "before", None, None, false).expect("add");
        add_section(&mut d, "continuous").expect("section");
        let sections = list_sections(&d);
        assert_eq!(sections.len(), 2);
        // Words sets the start type on the *clone* embedded in the break
        // paragraph (python-docx quirk, adr/0008); the body-final section
        // keeps its unset type.
        assert_eq!(sections[0].start_type, "continuous");
        assert_eq!(sections[1].start_type, "none");
        assert_eq!(sections[0].orientation, "portrait");
        add_section(&mut d, "bogus").expect("unknown defaults");
        let sections = list_sections(&d);
        assert_eq!(sections[1].start_type, "new_page");
        assert_eq!(sections[2].start_type, "none");
        let detail = section_detail(&d, 0).expect("detail");
        assert_eq!(detail.index, 0);
        assert!(detail.page_width.expect("width") > 0.0);
        assert!(section_detail(&d, 9).is_err());
        assert_eq!(
            section_detail(&d, 9).unwrap_err().to_string(),
            "not found: Section index 9 out of range (0..2)"
        );
    }

    #[test]
    fn page_break_creates_bookmarked_paragraph_with_break_run() {
        let mut d = docx();
        let id = add_page_break(&mut d, None).expect("break");
        assert!(id.starts_with('p'));
        let paras = paragraph_children(&d.document.children);
        let DocumentChild::Paragraph(p) = &d.document.children[paras[0]] else {
            panic!("paragraph expected");
        };
        assert!(p
            .children
            .iter()
            .any(|c| matches!(c, ParagraphChild::Run(r) if r.children.iter().any(|rc| matches!(rc, RunChild::Break(_))))));
    }

    // ------------------------------------------------------------------
    // Images
    // ------------------------------------------------------------------

    fn png_file(dir: &std::path::Path, w: u32, h: u32) -> String {
        let path = dir.join(format!("img{w}x{h}.png"));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(w, h)
            .write_to(&mut png, image::ImageFormat::Png)
            .expect("write png");
        std::fs::write(&path, png.into_inner()).expect("create");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn image_add_resize_delete_list_round_trip() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = png_file(tmp.path(), 4, 2);
        let mut d = docx();
        let id = add_image(&mut d, &path, None, None, None).expect("image");
        assert!(id.starts_with("img"));
        let listed = images(&d);
        assert_eq!(listed.len(), 1);
        // Native 4x2 px → 9525 EMU per px.
        assert_eq!(listed[0].width, 4 * 9525);
        assert_eq!(listed[0].height, 2 * 9525);
        resize_image(&mut d, 0, Some(2.0), None).expect("resize");
        let resized = images(&d);
        assert_eq!(resized[0].width, (2.0 * 914400.0) as u32);
        // Words' resize keeps the other dimension (no aspect scaling).
        assert_eq!(resized[0].height, 2 * 9525);
        delete_image(&mut d, 0).expect("delete");
        assert!(images(&d).is_empty());
        assert_eq!(paragraph_count(&d), 0, "host paragraph removed");
        assert!(delete_image(&mut d, 0).is_err(), "out of range");
    }

    #[test]
    fn image_add_missing_file_and_corrupt_data_error() {
        let mut d = docx();
        let err = add_image(&mut d, "/nonexistent/i.png", None, None, None).expect_err("missing");
        assert!(matches!(err, PoetError::NotFound(_)));
        let tmp = tempfile::tempdir().expect("tmp");
        let bad = tmp.path().join("bad.png");
        std::fs::write(&bad, b"not an image").expect("write");
        let err = add_image(&mut d, bad.to_string_lossy().as_ref(), None, None, None)
            .expect_err("corrupt");
        assert!(matches!(err, PoetError::Validation(_)));
    }

    #[test]
    fn image_add_scales_missing_dimension() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = png_file(tmp.path(), 4, 2);
        let mut d = docx();
        add_image(&mut d, &path, Some(1.0), None, None).expect("image");
        let images = images(&d);
        assert_eq!(images[0].width, 914400);
        assert_eq!(images[0].height, 914400 / 2);
    }

    // ------------------------------------------------------------------
    // TOC + field normalization
    // ------------------------------------------------------------------

    #[test]
    fn toc_add_bookmarks_and_normalizes_on_save() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("toc.docx");
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        let id = mgr.add_toc("1-3", None).expect("toc");
        assert!(id.starts_with("toc"));
        mgr.save("docx", &path).expect("save 1");
        mgr.close();
        mgr.open(&path).expect("reopen");
        // Reopen + resave would hit the engine's unreachable!() without
        // normalization (adr/0008).
        mgr.save("docx", &path).expect("save 2 survives");
        mgr.close();
        mgr.open(&path).expect("reopen 2");
        let docx = mgr.docx().expect("doc");
        let has_field = docx.document.children.iter().any(|child| match child {
            DocumentChild::Paragraph(p) => p.children.iter().any(|c| match c {
                ParagraphChild::Run(r) => r
                    .children
                    .iter()
                    .any(|rc| matches!(rc, RunChild::InstrTextString(t) if t.contains(r#"TOC \o "1-3""#))),
                _ => false,
            }),
            _ => false,
        });
        assert!(has_field, "TOC instruction survives the double round trip");
    }

    // ------------------------------------------------------------------
    // Styles
    // ------------------------------------------------------------------

    #[test]
    fn style_display_names_resolve_builtins_then_styles_part() {
        let d = docx();
        assert_eq!(
            style_display_name(&d, Some("Heading1")).as_deref(),
            Some("Heading 1")
        );
        assert_eq!(
            style_display_name(&d, Some("ListBullet3")).as_deref(),
            Some("List Bullet 3")
        );
        assert_eq!(
            style_display_name(&d, Some("TableGrid")).as_deref(),
            Some("Table Grid")
        );
        assert_eq!(
            style_display_name(&d, Some("Custom")).as_deref(),
            Some("Custom"),
            "unknown ids fall back to the raw id"
        );
        assert_eq!(style_display_name(&d, None), None);
    }

    #[test]
    fn style_id_from_name_compacts_builtins_only() {
        assert_eq!(style_id_from_name("Heading 1"), "Heading1");
        assert_eq!(style_id_from_name("List Bullet"), "ListBullet");
        assert_eq!(style_id_from_name("Table Grid"), "TableGrid");
        assert_eq!(style_id_from_name("My Custom Style"), "My Custom Style");
        assert_eq!(style_id_from_name("Custom"), "Custom");
    }

    #[test]
    fn list_paragraphs_defaults_style_to_normal() {
        let mut d = docx();
        add_paragraph(&mut d, "plain", None, None, false).expect("add");
        let info = list_paragraphs(&d);
        assert_eq!(info[0].style.as_deref(), Some("Normal"));
    }

    // ------------------------------------------------------------------
    // Numbering survival (adr/0005)
    // ------------------------------------------------------------------

    #[test]
    fn numbering_reuse_survives_save_reopen() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("lists.docx");
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.add_list_item("one", false, 1, None).expect("item");
        mgr.save("docx", &path).expect("save");
        mgr.close();
        mgr.open(&path).expect("reopen");
        // The reader also restores the writer's built-in default decimal
        // numbering, so 2 definitions exist after reopen. Adding another
        // bullet item must reuse ours — no growth.
        let before = mgr.docx().expect("doc").numberings.abstract_nums.len();
        mgr.add_list_item("two", false, 1, None).expect("item");
        let docx = mgr.docx().expect("doc");
        assert_eq!(
            docx.numberings.abstract_nums.len(),
            before,
            "bullet numbering reused after reopen"
        );
    }
}
