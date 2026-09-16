//! Calculation and analysis commands (bodies land in phase 4).

use clap::Args;

use crate::commands::stub_actions;

/// Shared table addressing for calc commands.
#[derive(Debug, Args)]
pub struct AddressArgs {
    /// Table bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Table positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Arguments for `calc read`.
#[derive(Debug, Args)]
pub struct ReadArgs {
    /// Path to a .docx file.
    pub path: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `calc stats`.
#[derive(Debug, Args)]
pub struct StatsArgs {
    /// Path to a .docx file.
    pub path: String,
    /// Restrict stats to one column.
    #[arg(long)]
    pub column: Option<String>,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `calc aggregate`.
#[derive(Debug, Args)]
pub struct AggregateArgs {
    /// Path to a .docx file.
    pub path: String,
    /// Column to group by.
    #[arg(long)]
    pub group_by: String,
    /// Column to aggregate.
    #[arg(long)]
    pub agg_column: String,
    /// sum|mean|min|max|count|median.
    #[arg(long, default_value = "sum")]
    pub agg_func: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `calc filter`.
#[derive(Debug, Args)]
pub struct FilterArgs {
    /// Path to a .docx file.
    pub path: String,
    /// Column to filter on.
    #[arg(long)]
    pub column: String,
    /// ==|!=|>|<|>=|<=|contains|startswith|endswith.
    #[arg(long)]
    pub operator: String,
    /// Comparison value.
    #[arg(long)]
    pub value: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `calc transform`.
#[derive(Debug, Args)]
pub struct TransformArgs {
    /// Path to a .docx file.
    pub path: String,
    /// JSON array of operations.
    #[arg(long)]
    pub operations: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// All `calc` actions.
#[derive(Debug, clap::Subcommand)]
pub enum CalcAction {
    /// Read a table as rows of typed values.
    Read(ReadArgs),
    /// Column statistics.
    Stats(StatsArgs),
    /// Group-wise aggregation.
    Aggregate(AggregateArgs),
    /// Filter rows.
    Filter(FilterArgs),
    /// Apply a chain of transforms.
    Transform(TransformArgs),
}

stub_actions! {
    read => ReadArgs,
    stats => StatsArgs,
    aggregate => AggregateArgs,
    filter => FilterArgs,
    transform => TransformArgs,
}
