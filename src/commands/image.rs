//! Image commands (bodies land in phase 2).

use clap::Args;

use crate::commands::stub_actions;

/// Arguments for `image add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Path to the image file.
    pub path: String,
    /// Width in inches.
    #[arg(long)]
    pub width: Option<f64>,
    /// Height in inches.
    #[arg(long)]
    pub height: Option<f64>,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `image list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `image get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Image index.
    #[arg(default_value_t = 0)]
    pub index: usize,
}

/// Arguments for `image resize`.
#[derive(Debug, Args)]
pub struct ResizeArgs {
    /// Image index.
    pub index: usize,
    /// Width in inches.
    #[arg(long)]
    pub width: Option<f64>,
    /// Height in inches.
    #[arg(long)]
    pub height: Option<f64>,
}

/// Arguments for `image delete`.
#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Image index.
    pub index: usize,
}

/// All `image` actions.
#[derive(Debug, clap::Subcommand)]
pub enum ImageAction {
    /// Add an image.
    Add(AddArgs),
    /// List images.
    List(ListArgs),
    /// Get image details.
    Get(GetArgs),
    /// Resize an image.
    Resize(ResizeArgs),
    /// Delete an image.
    Delete(DeleteArgs),
}

stub_actions! {
    add => AddArgs,
    list => ListArgs,
    get => GetArgs,
    resize => ResizeArgs,
    delete => DeleteArgs,
}
