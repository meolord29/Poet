//! Metadata commands (bodies land in phase 4 — `docs/plans/phase-4-misc.md`).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `meta describe`.
#[derive(Debug, Args)]
pub struct DescribeArgs {}

/// Arguments for `meta get-document`.
#[derive(Debug, Args)]
pub struct GetDocumentArgs {}

/// Arguments for `meta set-document`.
#[derive(Debug, Args)]
pub struct SetDocumentArgs {
    /// JSON document metadata.
    pub metadata: String,
}

/// Arguments for `meta get-section`.
#[derive(Debug, Args)]
pub struct GetSectionArgs {
    /// Section name.
    pub name: String,
}

/// Arguments for `meta set-section`.
#[derive(Debug, Args)]
pub struct SetSectionArgs {
    /// Section name.
    pub name: String,
    /// JSON section metadata.
    pub metadata: String,
}

/// Arguments for `meta get-table`.
#[derive(Debug, Args)]
pub struct GetTableArgs {
    /// Table bookmark id.
    pub id: String,
}

/// Arguments for `meta set-table`.
#[derive(Debug, Args)]
pub struct SetTableArgs {
    /// Table bookmark id.
    pub id: String,
    /// JSON table schema.
    pub schema: String,
}

/// Arguments for `meta history`.
#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Maximum entries to return.
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}

/// All `meta` actions.
#[derive(Debug, clap::Subcommand)]
pub enum MetaAction {
    /// Summarize stored metadata.
    Describe(DescribeArgs),
    /// Get document metadata.
    GetDocument(GetDocumentArgs),
    /// Set document metadata.
    SetDocument(SetDocumentArgs),
    /// Get section metadata.
    GetSection(GetSectionArgs),
    /// Set section metadata.
    SetSection(SetSectionArgs),
    /// Get a table schema.
    GetTable(GetTableArgs),
    /// Set a table schema.
    SetTable(SetTableArgs),
    /// Show the operation history.
    History(HistoryArgs),
}

stub_actions! {
    describe => DescribeArgs,
    get_document => GetDocumentArgs,
    set_document => SetDocumentArgs,
    get_section => GetSectionArgs,
    set_section => SetSectionArgs,
    get_table => GetTableArgs,
    set_table => SetTableArgs,
    history => HistoryArgs,
}
