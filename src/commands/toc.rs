//! Table-of-contents commands (bodies land in phase 2).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `toc add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Heading levels to include, e.g. "1-3".
    #[arg(long, default_value = "1-3")]
    pub levels: String,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `toc update`.
#[derive(Debug, Args)]
pub struct UpdateArgs {}

/// All `toc` actions.
#[derive(Debug, clap::Subcommand)]
pub enum TocAction {
    /// Insert a TOC field.
    Add(AddArgs),
    /// Refresh hint (Word updates the field on open).
    Update(UpdateArgs),
}

stub_actions! {
    add => AddArgs,
    update => UpdateArgs,
}
