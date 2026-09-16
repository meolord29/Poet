//! Section commands (bodies land in phase 2 — `docs/plans/phase-2-content.md`).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `section list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `section info`.
#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub index: usize,
}

/// Arguments for `section add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// new_page|new_column|even_page|odd_page|continuous.
    #[arg(long, default_value = "new_page")]
    pub start_type: String,
}

/// Arguments for `section page-break`.
#[derive(Debug, Args)]
pub struct PageBreakArgs {}

/// All `section` actions.
#[derive(Debug, clap::Subcommand)]
pub enum SectionAction {
    /// List sections.
    List(ListArgs),
    /// Show section details.
    Info(InfoArgs),
    /// Add a section break.
    Add(AddArgs),
    /// Insert an explicit page break.
    PageBreak(PageBreakArgs),
}

stub_actions! {
    list => ListArgs,
    info => InfoArgs,
    add => AddArgs,
    page_break => PageBreakArgs,
}
