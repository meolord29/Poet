//! List commands (bodies land in phase 2).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `list add` / `list add-item`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// List item text.
    pub text: String,
    /// Numbered list (default: bullet).
    #[arg(long)]
    pub ordered: bool,
    /// Nesting level.
    #[arg(long, default_value_t = 1)]
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `list convert`.
#[derive(Debug, Args)]
pub struct ConvertArgs {
    /// Convert to numbered (default: bullet).
    #[arg(long)]
    pub ordered: bool,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Arguments for `list set-level`.
#[derive(Debug, Args)]
pub struct SetLevelArgs {
    /// New nesting level.
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// All `list` actions.
#[derive(Debug, clap::Subcommand)]
pub enum ListAction {
    /// Add a list item.
    Add(AddArgs),
    /// Add an item to an existing list.
    AddItem(AddArgs),
    /// Convert a paragraph to a list item.
    Convert(ConvertArgs),
    /// Change a list item's level.
    SetLevel(SetLevelArgs),
}

stub_actions! {
    add => AddArgs,
    add_item => AddArgs,
    convert => ConvertArgs,
    set_level => SetLevelArgs,
}
