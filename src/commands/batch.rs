//! Batch processing commands (bodies land in phase 4).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `batch run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Path to a batch JSON script.
    pub script: String,
}

/// Arguments for `batch template`.
#[derive(Debug, Args)]
pub struct TemplateArgs {
    /// basic | report | data_table.
    pub name: String,
    /// Output path for the template script.
    pub output: String,
}

/// All `batch` actions.
#[derive(Debug, clap::Subcommand)]
pub enum BatchAction {
    /// Run a batch JSON script.
    Run(RunArgs),
    /// Write a built-in template script.
    Template(TemplateArgs),
}

stub_actions! {
    run => RunArgs,
    template => TemplateArgs,
}
