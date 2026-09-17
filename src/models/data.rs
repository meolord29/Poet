//! The success-payload enum: one variant per command's `data` shape.
//!
//! Serialized inside the `ok` envelope (`{"status":"ok","message":...,
//! "data":<variant>}`). Shapes ported from the Words commands; field order
//! matches Words' dict insertion order where observable.
//!
//! Envelope-message parity note: Words' `format_success(data)` leaves the
//! envelope `message` empty unless `message=` is passed explicitly — only the
//! document lifecycle commands do. Content commands carry their message
//! *inside* `data` only, so their [`Data::message`] returns `""`.

use serde::Serialize;

/// One success payload per command.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Data {
    /// `document new` — the created document's path.
    DocumentNew {
        /// Path the document will be saved to.
        path: String,
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document open` — the opened document's path.
    DocumentOpened {
        /// Path the document was opened from.
        path: String,
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document save` — the saved document's path.
    DocumentSaved {
        /// Path the document was written to.
        path: String,
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document close`.
    DocumentClosed {
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document info` — structure summary; envelope message is empty.
    DocumentInfo(DocumentInfo),
    /// `document export` — the written export file.
    DocumentExported {
        /// Path the export was written to.
        path: String,
        /// Export format (`md`/`txt`).
        format: String,
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `paragraph add`.
    ParagraphAdded {
        /// Bookmark id of the new paragraph.
        id: String,
        /// Paragraph text.
        text: String,
        /// Style name as requested (or `null`).
        style: Option<String>,
        /// Whether a page break was requested.
        page_break: bool,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph insert`.
    ParagraphInserted {
        /// Bookmark id of the new paragraph.
        id: String,
        /// Position the paragraph was inserted at (echo of the argument).
        index: usize,
        /// Paragraph text.
        text: String,
        /// Style name as requested (or `null`).
        style: Option<String>,
        /// Whether a page break was requested.
        page_break: bool,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph get` on a body paragraph.
    ParagraphGot {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Paragraph text.
        text: String,
    },
    /// `paragraph get` with cell addressing; text is one paragraph or all.
    ParagraphGotCell {
        /// Table index argument.
        table: usize,
        /// Cell row argument.
        row: usize,
        /// Cell column argument.
        col: usize,
        /// Cell paragraph index argument (or `null`).
        para: Option<usize>,
        /// Text of one paragraph or a list of all.
        text: CellText,
    },
    /// `paragraph update`.
    ParagraphUpdated {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// New paragraph text.
        text: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph delete` on a body paragraph.
    ParagraphDeleted {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph delete` with cell addressing.
    CellParagraphDeleted {
        /// Table index argument.
        table: usize,
        /// Cell row argument.
        row: usize,
        /// Cell column argument.
        col: usize,
        /// Cell paragraph index argument.
        para: usize,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph list`.
    ParagraphList {
        /// All body paragraphs.
        paragraphs: Vec<ParagraphInfo>,
    },
    /// `paragraph move`.
    ParagraphMoved {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// The paragraph's new positional index.
        index: usize,
        /// Direction applied (`up`/`down`).
        direction: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph clear`.
    ParagraphCleared {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph border` (body lands in phase 3 — stub keeps the shape).
    ParagraphBorder {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Border position.
        position: String,
        /// Hex color.
        color: String,
        /// Thickness (eighths of a point).
        size: i64,
        /// Gap to text (points).
        space: i64,
        /// Border style.
        style: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph find`.
    ParagraphFound {
        /// Searched text.
        text: String,
        /// Matches in document order (paragraphs first, then table cells).
        results: Vec<FindResult>,
        /// Number of matches.
        count: usize,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph replace`.
    ParagraphReplaced {
        /// Text that was searched for.
        find: String,
        /// Replacement text.
        replace: String,
        /// Number of replacements.
        count: usize,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `paragraph count`.
    ParagraphCount {
        /// Body paragraph count.
        count: usize,
    },

    /// `run add`.
    RunAdded {
        /// Paragraph bookmark id argument (or `null`).
        id: Option<String>,
        /// Paragraph index argument (or `null`).
        index: Option<usize>,
        /// Run text.
        text: String,
        /// Bold flag (or `null` when not given).
        bold: Option<bool>,
        /// Italic flag (or `null` when not given).
        italic: Option<bool>,
        /// Underline flag (or `null` when not given).
        underline: Option<bool>,
        /// Font name (or `null` when not given).
        font: Option<String>,
        /// Font size in points (or `null` when not given).
        size: Option<f64>,
        /// Hex color (or `null` when not given).
        color: Option<String>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `run get` on a body paragraph.
    RunsGot {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// The paragraph's runs.
        runs: Vec<RunInfo>,
    },
    /// `run get` with cell addressing.
    RunsGotCell {
        /// Table index argument.
        table: usize,
        /// Cell row argument.
        row: usize,
        /// Cell column argument.
        col: usize,
        /// Cell paragraph index argument (or `null`).
        para: Option<usize>,
        /// The cell paragraph's runs.
        runs: Vec<RunInfo>,
    },
    /// `run clear`.
    RunsCleared {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `heading add`.
    HeadingAdded {
        /// Bookmark id of the new heading.
        id: String,
        /// Heading text.
        text: String,
        /// Heading level.
        level: u8,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `heading set-level`.
    HeadingLevelSet {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// New heading level.
        level: u8,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `heading list`.
    HeadingList {
        /// Paragraphs whose style is a heading style.
        headings: Vec<ParagraphInfo>,
    },

    /// `list add` / `list add-item`.
    ListItemAdded {
        /// Bookmark id of the new item.
        id: String,
        /// Item text.
        text: String,
        /// `ordered` or `bullet`.
        list_type: String,
        /// Nesting level (1-based).
        level: u8,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `list convert`.
    ListConverted {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// `ordered` or `bullet`.
        list_type: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `list set-level`.
    ListLevelSet {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// New nesting level (1-based).
        level: u8,
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `table add`.
    TableAdded {
        /// Bookmark id of the new table.
        id: String,
        /// Row count.
        rows: usize,
        /// Column count.
        cols: usize,
        /// Style name as requested.
        style: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `table list`.
    TableList {
        /// All body tables.
        tables: Vec<TableInfo>,
    },
    /// `table get`.
    TableGot {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Row count.
        rows: usize,
        /// Column count (0 when empty).
        cols: usize,
        /// Cell texts.
        data: Vec<Vec<String>>,
    },
    /// `table set-cell`.
    CellSet {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Cell row.
        row: usize,
        /// Cell column.
        col: usize,
        /// Value written.
        value: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `table set-range`.
    TableRangeSet {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Rows written.
        rows_written: usize,
        /// Whether the first row was marked as a header.
        header: bool,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `table add-row`.
    TableRowAdded {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `table add-column`.
    TableColumnAdded {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `table delete-row`.
    TableRowDeleted {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Deleted row index.
        row: usize,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `table delete-column`.
    TableColumnDeleted {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Deleted column index.
        col: usize,
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `section list`.
    SectionList {
        /// All sections (body-final one last).
        sections: Vec<SectionInfo>,
        /// Section count.
        count: usize,
    },
    /// `section info`.
    SectionDetail(SectionDetail),
    /// `section add`.
    SectionAdded {
        /// Start type applied.
        start_type: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `section page-break`.
    PageBreakInserted {
        /// Bookmark id of the hosting paragraph.
        id: String,
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `toc add`.
    TocAdded {
        /// Bookmark id of the TOC paragraph.
        id: String,
        /// Heading levels included.
        levels: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `toc update` (no-op hint).
    TocUpdateHint {
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `image add`.
    ImageAdded {
        /// Bookmark id of the hosting paragraph.
        id: String,
        /// Path of the image file.
        path: String,
        /// Requested width in inches (or `null`).
        width: Option<f64>,
        /// Requested height in inches (or `null`).
        height: Option<f64>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `image list`.
    ImageList {
        /// All inline images.
        images: Vec<ImageInfo>,
        /// Image count.
        count: usize,
    },
    /// `image get`.
    ImageGot(ImageInfo),
    /// `image resize`.
    ImageResized {
        /// Image index.
        index: usize,
        /// Requested width in inches (or `null`).
        width: Option<f64>,
        /// Requested height in inches (or `null`).
        height: Option<f64>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `image delete`.
    ImageDeleted {
        /// Image index.
        index: usize,
        /// Human-readable summary (inside data only).
        message: String,
    },
}

impl Data {
    /// The envelope-level message for this payload. Words puts the message in
    /// both places only for the document lifecycle commands; content commands
    /// keep it inside `data` and leave the envelope message empty.
    pub fn message(&self) -> &str {
        match self {
            Data::DocumentNew { message, .. }
            | Data::DocumentOpened { message, .. }
            | Data::DocumentSaved { message, .. }
            | Data::DocumentClosed { message } => message,
            _ => "",
        }
    }
}

/// `document info` payload — ported from Words' `get_info`.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentInfo {
    /// Body-level paragraph count.
    pub paragraph_count: usize,
    /// Body-level table count.
    pub table_count: usize,
    /// Section count (body-final section included).
    pub section_count: usize,
    /// Inline drawing count.
    pub image_count: usize,
    /// Body-level bookmark count.
    pub bookmark_count: usize,
    /// Document core properties.
    pub core_properties: CoreProperties,
}

/// Core properties (Words' `get_core_properties` key set).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CoreProperties {
    /// Title.
    pub title: String,
    /// Author (engine `creator`).
    pub author: String,
    /// Subject.
    pub subject: String,
    /// Keywords (engine has no equivalent; always empty — adr/0001).
    pub keywords: String,
    /// Category (engine has no equivalent; always empty — adr/0001).
    pub category: String,
    /// Comments (engine `description`).
    pub comments: String,
    /// Creation timestamp (RFC 3339) or empty.
    pub created: String,
    /// Last-modified timestamp (RFC 3339) or empty.
    pub modified: String,
}

/// One entry of `paragraph list` / `heading list` (Words' `list_paragraphs`).
#[derive(Debug, Clone, Serialize)]
pub struct ParagraphInfo {
    /// Body paragraph index.
    pub index: usize,
    /// Bookmark id wrapping the paragraph (or `null`).
    pub id: Option<String>,
    /// Resolved style display name (or `null`) — adr/0008.
    pub style: Option<String>,
    /// Paragraph text.
    pub text: String,
}

/// One entry of `table list` (Words' `list_tables`).
#[derive(Debug, Clone, Serialize)]
pub struct TableInfo {
    /// Body table index.
    pub index: usize,
    /// Bookmark id wrapping the table (or `null`).
    pub id: Option<String>,
    /// Row count.
    pub rows: usize,
    /// Column count.
    pub cols: usize,
    /// Resolved style display name (or `null`) — adr/0008.
    pub style: Option<String>,
}

/// One entry of `run get` (Words' `get_runs` shape).
#[derive(Debug, Clone, Serialize)]
pub struct RunInfo {
    /// Run text.
    pub text: String,
    /// Bold state (`null` = inherited/unset).
    pub bold: Option<bool>,
    /// Italic state (`null` = inherited/unset).
    pub italic: Option<bool>,
    /// Underline state (`null` = inherited/unset).
    pub underline: Option<bool>,
    /// Font name (or `null`).
    pub font: Option<String>,
    /// Font size in points (or `null`).
    pub size: Option<f64>,
    /// Hex color without `#` (or `null`).
    pub color: Option<String>,
}

/// Text of one cell paragraph or all of them (`paragraph get` cell mode).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum CellText {
    /// One paragraph's text (`--para` given).
    One(String),
    /// All of the cell's paragraphs (`--para` omitted).
    Many(Vec<String>),
}

/// One `paragraph find` match — a body paragraph or a table cell.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum FindResult {
    /// A body paragraph match.
    Paragraph {
        /// Always `paragraph`.
        #[serde(rename = "type")]
        kind: String,
        /// Body paragraph index.
        index: usize,
        /// Paragraph text.
        text: String,
    },
    /// A table cell match.
    TableCell {
        /// Always `table_cell`.
        #[serde(rename = "type")]
        kind: String,
        /// Table index.
        table_index: usize,
        /// Cell row.
        row: usize,
        /// Cell column.
        col: usize,
        /// Cell text.
        text: String,
    },
}

/// One entry of `section list` (Words' `SectionCommand.list` items).
#[derive(Debug, Clone, Serialize)]
pub struct SectionInfo {
    /// Section index.
    pub index: usize,
    /// `landscape` or `portrait`.
    pub orientation: String,
    /// Stable start-type name — adr/0008 (Words leaked Python enum reprs).
    pub start_type: String,
}

/// `section info` payload (Words' `SectionCommand.info`).
#[derive(Debug, Clone, Serialize)]
pub struct SectionDetail {
    /// Section index.
    pub index: usize,
    /// `landscape` or `portrait`.
    pub orientation: String,
    /// Page width in inches.
    pub page_width: Option<f64>,
    /// Page height in inches.
    pub page_height: Option<f64>,
    /// Top margin in inches.
    pub top_margin: Option<f64>,
    /// Bottom margin in inches.
    pub bottom_margin: Option<f64>,
    /// Left margin in inches.
    pub left_margin: Option<f64>,
    /// Right margin in inches.
    pub right_margin: Option<f64>,
}

/// One entry of `image list` / payload of `image get` (Words' `get_images`).
#[derive(Debug, Clone, Serialize)]
pub struct ImageInfo {
    /// Image index.
    pub index: usize,
    /// Width in EMU.
    pub width: u32,
    /// Height in EMU.
    pub height: u32,
    /// Width in inches.
    pub width_inches: Option<f64>,
    /// Height in inches.
    pub height_inches: Option<f64>,
}
