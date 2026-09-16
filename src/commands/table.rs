//! Table commands (bodies land in phase 2).

use clap::Args;

use crate::commands::stub_actions;

/// Shared `--id`/`--index` table addressing.
#[derive(Debug, Args)]
pub struct AddressArgs {
    /// Table bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Table positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Arguments for `table add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub cols: usize,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Table style name.
    #[arg(long, default_value = "Table Grid")]
    pub style: String,
}

/// Arguments for `table list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `table get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `table set-cell`.
#[derive(Debug, Args)]
pub struct SetCellArgs {
    /// Cell row.
    pub row: usize,
    /// Cell column.
    pub col: usize,
    /// New cell value.
    pub value: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `table set-range`.
#[derive(Debug, Args)]
pub struct SetRangeArgs {
    /// JSON 2D array.
    pub values: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Treat the first row as a header.
    #[arg(short = 'H', long)]
    pub header: bool,
}

/// Arguments for `table add-row` / `table add-column`.
#[derive(Debug, Args)]
pub struct AppendArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// JSON array of cell values.
    #[arg(long)]
    pub values: Option<String>,
}

/// Arguments for `table delete-row`.
#[derive(Debug, Args)]
pub struct DeleteRowArgs {
    /// Row index to delete.
    pub row: usize,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `table delete-column`.
#[derive(Debug, Args)]
pub struct DeleteColumnArgs {
    /// Column index to delete.
    pub col: usize,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// All `table` actions.
#[derive(Debug, clap::Subcommand)]
pub enum TableAction {
    /// Add a table.
    Add(AddArgs),
    /// List tables.
    List(ListArgs),
    /// Get table details.
    Get(GetArgs),
    /// Set one cell's value.
    SetCell(SetCellArgs),
    /// Fill a 2D range of cells.
    SetRange(SetRangeArgs),
    /// Append a row.
    AddRow(AppendArgs),
    /// Append a column.
    AddColumn(AppendArgs),
    /// Delete a row.
    DeleteRow(DeleteRowArgs),
    /// Delete a column.
    DeleteColumn(DeleteColumnArgs),
}

stub_actions! {
    add => AddArgs,
    list => ListArgs,
    get => GetArgs,
    set_cell => SetCellArgs,
    set_range => SetRangeArgs,
    add_row => AppendArgs,
    add_column => AppendArgs,
    delete_row => DeleteRowArgs,
    delete_column => DeleteColumnArgs,
}
