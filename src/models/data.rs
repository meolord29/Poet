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
use serde_json::Value;

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
    /// `run format`.
    RunFormatted {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Run targeted (or `null` = every run).
        run_index: Option<usize>,
        /// The options actually applied (Words keys; absent = untouched).
        applied: AppliedFormat,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `run emphasize`.
    RunEmphasized {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Table index argument (or `null`).
        table: Option<usize>,
        /// Cell row argument (or `null`).
        row: Option<usize>,
        /// Cell column argument (or `null`).
        col: Option<usize>,
        /// Cell paragraph index argument (or `null`).
        para: Option<usize>,
        /// The emphasized substring.
        find: String,
        /// Number of occurrences formatted (0 is a successful no-match).
        replacements: usize,
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `style list`.
    StylesListed {
        /// All registry styles matching the type filter.
        styles: Vec<StyleInfo>,
        /// Style count.
        count: usize,
    },
    /// `style apply`.
    StyleApplied {
        /// Bookmark id argument (or `null`).
        id: Option<String>,
        /// Index argument (or `null`).
        index: Option<usize>,
        /// Style name as requested.
        style: String,
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

    /// `page margins`.
    MarginsSet {
        /// Section index.
        section: usize,
        /// Top margin value as given (or `null`).
        top: Option<f64>,
        /// Bottom margin value as given (or `null`).
        bottom: Option<f64>,
        /// Left margin value as given (or `null`).
        left: Option<f64>,
        /// Right margin value as given (or `null`).
        right: Option<f64>,
        /// Unit argument.
        unit: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `page orientation`.
    OrientationSet {
        /// Section index.
        section: usize,
        /// Orientation as requested.
        orientation: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `page size`.
    PageSizeSet {
        /// Section index.
        section: usize,
        /// Width value as given (or `null`).
        width: Option<f64>,
        /// Height value as given (or `null`).
        height: Option<f64>,
        /// Unit argument.
        unit: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `page header`.
    HeaderSet {
        /// Section index.
        section: usize,
        /// Header text.
        header: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `page footer`.
    FooterSet {
        /// Section index.
        section: usize,
        /// Footer text.
        footer: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `page page-numbers`.
    PageNumbersAdded {
        /// Section index.
        section: usize,
        /// Alignment argument.
        align: String,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `page columns`.
    ColumnsSet {
        /// Section index.
        section: usize,
        /// Column count as requested.
        columns: usize,
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

    /// `meta describe`.
    MetaDescribe(Value),
    /// `meta get-document`.
    MetaDocumentGot {
        /// Document metadata (`{}` when unset).
        metadata: Value,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `meta set-document`.
    MetaDocumentSet {
        /// The stored document metadata.
        metadata: Value,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `meta get-section`.
    MetaSectionGot {
        /// Section name as requested.
        name: String,
        /// Section metadata (`{}` when unset).
        metadata: Value,
    },
    /// `meta set-section`.
    MetaSectionSet {
        /// Section name as requested.
        name: String,
        /// The stored section metadata.
        metadata: Value,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `meta get-table`.
    MetaTableGot {
        /// Table bookmark id as requested.
        id: String,
        /// Table schema (`{}` when unset).
        schema: Value,
    },
    /// `meta set-table`.
    MetaTableSet {
        /// Table bookmark id as requested.
        id: String,
        /// The stored table schema.
        schema: Value,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `meta history`.
    MetaHistory {
        /// The newest entries, chronological.
        history: Vec<crate::core::meta::HistoryEntry>,
    },

    /// `calc read`.
    CalcRead {
        /// Path the table was read from.
        path: String,
        /// Table bookmark id as requested (or `null`).
        table_id: Option<String>,
        /// Table positional index as requested (or `null`).
        table_index: Option<usize>,
        /// Data-row count (header excluded).
        rows: usize,
        /// Column names.
        columns: Vec<String>,
        /// Rows as objects in column order.
        data: Vec<Value>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `calc stats`.
    CalcStats {
        /// Path the table was read from.
        path: String,
        /// Per-column statistics (Words' key set per column).
        statistics: Value,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `calc aggregate`.
    CalcAggregate {
        /// Path the table was read from.
        path: String,
        /// Group column.
        group_by: String,
        /// Aggregated column.
        agg_column: String,
        /// Aggregation function.
        agg_func: String,
        /// One object per group: group key + aggregated value.
        result: Vec<Value>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `calc filter`.
    CalcFilter {
        /// Path the table was read from.
        path: String,
        /// Filtered column.
        column: String,
        /// Operator.
        operator: String,
        /// Comparison value as given.
        value: String,
        /// Matching rows as objects.
        result: Vec<Value>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `calc transform`.
    CalcTransform {
        /// Path the table was read from.
        path: String,
        /// The parsed operations (echo).
        operations: Value,
        /// Resulting rows as objects.
        result: Vec<Value>,
        /// Human-readable summary (inside data only).
        message: String,
    },

    /// `batch run`.
    BatchRun {
        /// Script path as given.
        script: String,
        /// Attempted command count (including the failed one).
        commands_executed: usize,
        /// Per-command envelopes, in execution order.
        results: Vec<Value>,
        /// Human-readable summary (inside data only).
        message: String,
    },
    /// `batch template`.
    BatchTemplate {
        /// Template name.
        template: String,
        /// Output path written.
        output: String,
        /// Command count in the template.
        commands: usize,
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

/// The `applied` map of `run format` — Words' manager-side key names, and
/// only the options actually provided (absent keys are not serialized).
#[derive(Debug, Clone, Default, Serialize)]
pub struct AppliedFormat {
    /// Bold applied (`--no-bold` = `false`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    /// Italic applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Underline applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    /// Font name applied (Words' `font_name` key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    /// Font size applied in points (Words' `font_size` key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    /// Hex color applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl AppliedFormat {
    /// Mirror a [`crate::core::design::FormatSpec`] onto Words' key names.
    pub fn from_spec(spec: &crate::core::design::FormatSpec) -> Self {
        AppliedFormat {
            bold: spec.bold,
            italic: spec.italic,
            underline: spec.underline,
            font_name: spec.font.clone(),
            font_size: spec.size,
            color: spec.color.clone(),
        }
    }
}

/// One entry of `style list` (Words' `list_styles` shape; `type` keeps the
/// `str(WD_STYLE_TYPE.X)` rendering, adr/0010).
#[derive(Debug, Clone, Serialize)]
pub struct StyleInfo {
    /// Style display name.
    pub name: String,
    /// Type rendering, e.g. `PARAGRAPH (1)`.
    pub r#type: String,
    /// docx-rs carries no `customStyle` model, so this is always `true`
    /// (adr/0010).
    pub builtin: bool,
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
