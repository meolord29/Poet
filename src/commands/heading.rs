//! Heading commands (bodies land in phase 2).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `heading add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Heading text.
    pub text: String,
    /// Heading level 1-9.
    #[arg(short = 'l', long, default_value_t = 1)]
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `heading set-level`.
#[derive(Debug, Args)]
pub struct SetLevelArgs {
    /// New heading level.
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Arguments for `heading list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// All `heading` actions.
#[derive(Debug, clap::Subcommand)]
pub enum HeadingAction {
    /// Add a heading.
    Add(AddArgs),
    /// Change a heading's level.
    SetLevel(SetLevelArgs),
    /// List headings.
    List(ListArgs),
}

stub_actions! {
    add => AddArgs,
    set_level => SetLevelArgs,
    list => ListArgs,
}
