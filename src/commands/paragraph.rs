//! Paragraph commands (bodies land in phase 2; `border` in phase 3).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `paragraph add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Paragraph text.
    #[arg(default_value = "")]
    pub text: String,
    /// Paragraph style name.
    #[arg(long)]
    pub style: Option<String>,
    /// Bookmark id for this paragraph.
    #[arg(long)]
    pub id: Option<String>,
    /// Start the paragraph on a new page.
    #[arg(long)]
    pub page_break: bool,
}

/// Arguments for `paragraph insert`.
#[derive(Debug, Args)]
pub struct InsertArgs {
    /// 0-based position to insert at.
    pub index: usize,
    /// Paragraph text.
    #[arg(default_value = "")]
    pub text: String,
    /// Paragraph style name.
    #[arg(long)]
    pub style: Option<String>,
    /// Bookmark id for this paragraph.
    #[arg(long)]
    pub id: Option<String>,
    /// Start the inserted paragraph on a new page.
    #[arg(long)]
    pub page_break: bool,
}

/// Shared `--id`/`--index` addressing.
#[derive(Debug, Args)]
pub struct AddressArgs {
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Shared cell addressing (`--table/--row/--col/--para`).
#[derive(Debug, Args)]
pub struct CellArgs {
    /// Table index (cell addressing).
    #[arg(long)]
    pub table: Option<usize>,
    /// Cell row.
    #[arg(long)]
    pub row: Option<usize>,
    /// Cell column.
    #[arg(long)]
    pub col: Option<usize>,
    /// Paragraph index within the cell (omit for all).
    #[arg(long)]
    pub para: Option<usize>,
}

/// Arguments for `paragraph get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Cell addressing (`--table/--row/--col/--para`).
    #[command(flatten)]
    pub cell: CellArgs,
}

/// Arguments for `paragraph update`.
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// New text.
    pub text: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph delete`.
#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Cell addressing (`--table/--row/--col/--para`).
    #[command(flatten)]
    pub cell: CellArgs,
}

/// Arguments for `paragraph list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `paragraph move`.
#[derive(Debug, Args)]
pub struct MoveArgs {
    /// up | down.
    #[arg(default_value = "up")]
    pub direction: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph clear`.
#[derive(Debug, Args)]
pub struct ClearArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph border` (body lands in phase 3).
#[derive(Debug, Args)]
pub struct BorderArgs {
    /// top|left|bottom|right|between.
    #[arg(long, default_value = "bottom")]
    pub position: String,
    /// Hex color, e.g. 000000.
    #[arg(long, default_value = "000000")]
    pub color: String,
    /// Border thickness (eighths of a point).
    #[arg(long, default_value_t = 4)]
    pub size: i64,
    /// Gap between border and text (points).
    #[arg(long, default_value_t = 1)]
    pub space: i64,
    /// single|double|dashed|...
    #[arg(long, default_value = "single")]
    pub style: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph find`.
#[derive(Debug, Args)]
pub struct FindArgs {
    /// Text to search for.
    pub text: String,
}

/// Arguments for `paragraph replace`.
#[derive(Debug, Args)]
pub struct ReplaceArgs {
    /// Text to find.
    pub find: String,
    /// Replacement text.
    pub replace: String,
}

/// Arguments for `paragraph count`.
#[derive(Debug, Args)]
pub struct CountArgs {}

/// All `paragraph` actions.
#[derive(Debug, clap::Subcommand)]
pub enum ParagraphAction {
    /// Add a paragraph.
    Add(AddArgs),
    /// Insert a paragraph at a position.
    Insert(InsertArgs),
    /// Get paragraph details.
    Get(GetArgs),
    /// Update paragraph text.
    Update(UpdateArgs),
    /// Delete a paragraph.
    Delete(DeleteArgs),
    /// List paragraphs.
    List(ListArgs),
    /// Move a paragraph up or down.
    Move(MoveArgs),
    /// Clear paragraph text.
    Clear(ClearArgs),
    /// Add a paragraph border (default: a bottom underline).
    Border(BorderArgs),
    /// Find text in paragraphs.
    Find(FindArgs),
    /// Replace text in paragraphs.
    Replace(ReplaceArgs),
    /// Count paragraphs.
    Count(CountArgs),
}

stub_actions! {
    add => AddArgs,
    insert => InsertArgs,
    get => GetArgs,
    update => UpdateArgs,
    delete => DeleteArgs,
    list => ListArgs,
    r#move => MoveArgs,
    clear => ClearArgs,
    border => BorderArgs,
    find => FindArgs,
    replace => ReplaceArgs,
    count => CountArgs,
}
