//! Page layout commands (bodies land in phase 3).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `page margins`.
#[derive(Debug, Args)]
pub struct MarginsArgs {
    /// Top margin.
    #[arg(long)]
    pub top: Option<f64>,
    /// Bottom margin.
    #[arg(long)]
    pub bottom: Option<f64>,
    /// Left margin.
    #[arg(long)]
    pub left: Option<f64>,
    /// Right margin.
    #[arg(long)]
    pub right: Option<f64>,
    /// inches|cm|points.
    #[arg(long, default_value = "inches")]
    pub unit: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page orientation`.
#[derive(Debug, Args)]
pub struct OrientationArgs {
    /// portrait | landscape.
    pub orientation: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page size`.
#[derive(Debug, Args)]
pub struct SizeArgs {
    /// Page width.
    #[arg(long)]
    pub width: Option<f64>,
    /// Page height.
    #[arg(long)]
    pub height: Option<f64>,
    /// inches|cm|points.
    #[arg(long, default_value = "inches")]
    pub unit: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page header`.
#[derive(Debug, Args)]
pub struct HeaderArgs {
    /// Header text.
    pub text: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page footer`.
#[derive(Debug, Args)]
pub struct FooterArgs {
    /// Footer text.
    pub text: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page page-numbers`.
#[derive(Debug, Args)]
pub struct PageNumbersArgs {
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
    /// left|center|right.
    #[arg(long, default_value = "center")]
    pub align: String,
}

/// Arguments for `page columns`.
#[derive(Debug, Args)]
pub struct ColumnsArgs {
    /// Column count.
    pub count: usize,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// All `page` actions.
#[derive(Debug, clap::Subcommand)]
pub enum PageAction {
    /// Set page margins.
    Margins(MarginsArgs),
    /// Set page orientation.
    Orientation(OrientationArgs),
    /// Set page size.
    Size(SizeArgs),
    /// Set the header text.
    Header(HeaderArgs),
    /// Set the footer text.
    Footer(FooterArgs),
    /// Insert page-number field.
    PageNumbers(PageNumbersArgs),
    /// Set column count.
    Columns(ColumnsArgs),
}

stub_actions! {
    margins => MarginsArgs,
    orientation => OrientationArgs,
    size => SizeArgs,
    header => HeaderArgs,
    footer => FooterArgs,
    page_numbers => PageNumbersArgs,
    columns => ColumnsArgs,
}
