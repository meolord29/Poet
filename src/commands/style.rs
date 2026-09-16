//! Style commands (bodies land in phase 3).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `style list`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// paragraph|character|table|list.
    #[arg(long)]
    pub r#type: Option<String>,
}

/// Arguments for `style apply`.
#[derive(Debug, Args)]
pub struct ApplyArgs {
    /// Style name.
    pub style: String,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// All `style` actions.
#[derive(Debug, clap::Subcommand)]
pub enum StyleAction {
    /// List styles.
    List(ListArgs),
    /// Apply a style to a paragraph.
    Apply(ApplyArgs),
}

stub_actions! {
    list => ListArgs,
    apply => ApplyArgs,
}
