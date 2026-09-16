//! Run (text span) commands (bodies land in phases 2–3).

use clap::Args;

use crate::commands::stub_actions;

/// Shared formatting flags (`run add`/`format`/`emphasize`).
///
/// Words' `--bold/--no-bold` tri-state is carried as two flags; the effective
/// value is resolved when the formatting engine lands in phase 3
/// (`off` wins if both are given — documented there).
#[derive(Debug, Args)]
pub struct FormatFlags {
    /// Apply bold / `--no-bold` to clear.
    #[arg(long)]
    pub bold: bool,
    /// Explicitly clear bold (`--no-bold`).
    #[arg(long, overrides_with = "bold")]
    pub no_bold: bool,
    /// Apply italic / `--no-italic` to clear.
    #[arg(long)]
    pub italic: bool,
    /// Explicitly clear italic (`--no-italic`).
    #[arg(long, overrides_with = "italic")]
    pub no_italic: bool,
    /// Apply underline / `--no-underline` to clear.
    #[arg(long)]
    pub underline: bool,
    /// Explicitly clear underline (`--no-underline`).
    #[arg(long, overrides_with = "underline")]
    pub no_underline: bool,
    /// Font name.
    #[arg(long)]
    pub font: Option<String>,
    /// Font size in points.
    #[arg(long)]
    pub size: Option<f64>,
    /// Hex color, e.g. FF0000.
    #[arg(long)]
    pub color: Option<String>,
}

/// Arguments for `run add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Run text.
    pub text: String,
    /// Paragraph bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Paragraph positional index.
    #[arg(long)]
    pub index: Option<usize>,
    /// Addressing.
    #[command(flatten)]
    pub format: FormatFlags,
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
    /// Paragraph index within the cell.
    #[arg(long)]
    pub para: Option<usize>,
}

/// Arguments for `run get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Cell addressing (`--table/--row/--col/--para`).
    #[command(flatten)]
    pub cell: CellArgs,
}

/// Arguments for `run clear`.
#[derive(Debug, Args)]
pub struct ClearArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `run format`.
#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Addressing.
    #[command(flatten)]
    pub format: FormatFlags,
    /// Format only this run.
    #[arg(long)]
    pub run_index: Option<usize>,
}

/// Arguments for `run emphasize`.
#[derive(Debug, Args)]
pub struct EmphasizeArgs {
    /// Substring to emphasize (inline formatting).
    pub find: String,
    /// Body paragraph bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Body paragraph positional index.
    #[arg(long)]
    pub index: Option<usize>,
    /// Table index (cell addressing).
    #[arg(long)]
    pub table: Option<usize>,
    /// Cell row.
    #[arg(long)]
    pub row: Option<usize>,
    /// Cell column.
    #[arg(long)]
    pub col: Option<usize>,
    /// Paragraph index within the cell (omit to find across the cell).
    #[arg(long)]
    pub para: Option<usize>,
    /// Addressing.
    #[command(flatten)]
    pub format: FormatFlags,
    /// Emphasize every occurrence (default: first only).
    #[arg(long)]
    pub all: bool,
}

/// All `run` actions.
#[derive(Debug, clap::Subcommand)]
pub enum RunAction {
    /// Add a run to a paragraph.
    Add(AddArgs),
    /// Get runs of a paragraph.
    Get(GetArgs),
    /// Clear runs of a paragraph.
    Clear(ClearArgs),
    /// Format runs.
    Format(FormatArgs),
    /// Apply inline formatting to a substring.
    Emphasize(EmphasizeArgs),
}

stub_actions! {
    add => AddArgs,
    get => GetArgs,
    clear => ClearArgs,
    format => FormatArgs,
    emphasize => EmphasizeArgs,
}
