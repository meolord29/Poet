//! Run (text span) commands — `add`/`get`/`clear` ported from Words'
//! `commands/run.py`; `format`/`emphasize` stay stubbed until phase 3.

use clap::Args;

use crate::commands::stub_actions;
use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;
use crate::models::data::RunInfo;

/// Shared formatting flags (`run add`/`format`/`emphasize`).
///
/// Words' `--bold/--no-bold` tri-state is carried as two flags; the effective
/// value is resolved when the formatting engine lands in phase 3
/// (`off` wins if both are given — documented there).
#[derive(Debug, Args)]
pub struct FormatFlags {
    /// Apply bold / `--no-bold` to clear.
    #[arg(long)]
    pub bold: bool,
    /// Explicitly clear bold (`--no-bold`).
    #[arg(long, overrides_with = "bold")]
    pub no_bold: bool,
    /// Apply italic / `--no-italic` to clear.
    #[arg(long)]
    pub italic: bool,
    /// Explicitly clear italic (`--no-italic`).
    #[arg(long, overrides_with = "italic")]
    pub no_italic: bool,
    /// Apply underline / `--no-underline` to clear.
    #[arg(long)]
    pub underline: bool,
    /// Explicitly clear underline (`--no-underline`).
    #[arg(long, overrides_with = "underline")]
    pub no_underline: bool,
    /// Font name.
    #[arg(long)]
    pub font: Option<String>,
    /// Font size in points.
    #[arg(long)]
    pub size: Option<f64>,
    /// Hex color, e.g. FF0000.
    #[arg(long)]
    pub color: Option<String>,
}

/// Arguments for `run add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Run text.
    pub text: String,
    /// Paragraph bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Paragraph positional index.
    #[arg(long)]
    pub index: Option<usize>,
    /// Addressing.
    #[command(flatten)]
    pub format: FormatFlags,
}

/// Shared `--id`/`--index` addressing.
#[derive(Debug, Args)]
pub struct AddressArgs {
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Shared cell addressing (`--table/--row/--col/--para`).
#[derive(Debug, Args)]
pub struct CellArgs {
    /// Table index (cell addressing).
    #[arg(long)]
    pub table: Option<usize>,
    /// Cell row.
    #[arg(long)]
    pub row: Option<usize>,
    /// Cell column.
    #[arg(long)]
    pub col: Option<usize>,
    /// Paragraph index within the cell.
    #[arg(long)]
    pub para: Option<usize>,
}

/// Arguments for `run get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Cell addressing (`--table/--row/--col/--para`).
    #[command(flatten)]
    pub cell: CellArgs,
}

/// Arguments for `run clear`.
#[derive(Debug, Args)]
pub struct ClearArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `run format`.
#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Addressing.
    #[command(flatten)]
    pub format: FormatFlags,
    /// Format only this run.
    #[arg(long)]
    pub run_index: Option<usize>,
}

/// Arguments for `run emphasize`.
#[derive(Debug, Args)]
pub struct EmphasizeArgs {
    /// Substring to emphasize (inline formatting).
    pub find: String,
    /// Body paragraph bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Body paragraph positional index.
    #[arg(long)]
    pub index: Option<usize>,
    /// Table index (cell addressing).
    #[arg(long)]
    pub table: Option<usize>,
    /// Cell row.
    #[arg(long)]
    pub row: Option<usize>,
    /// Cell column.
    #[arg(long)]
    pub col: Option<usize>,
    /// Paragraph index within the cell (omit to find across the cell).
    #[arg(long)]
    pub para: Option<usize>,
    /// Addressing.
    #[command(flatten)]
    pub format: FormatFlags,
    /// Emphasize every occurrence (default: first only).
    #[arg(long)]
    pub all: bool,
}

/// All `run` actions.
#[derive(Debug, clap::Subcommand)]
pub enum RunAction {
    /// Add a run to a paragraph.
    Add(AddArgs),
    /// Get runs of a paragraph.
    Get(GetArgs),
    /// Clear runs of a paragraph.
    Clear(ClearArgs),
    /// Format runs.
    Format(FormatArgs),
    /// Apply inline formatting to a substring.
    Emphasize(EmphasizeArgs),
}

stub_actions! {
    format => FormatArgs,
    emphasize => EmphasizeArgs,
}

/// Resolve the `--flag`/`--no-flag` pair into Words' tri-state: `None` when
/// neither given, `Some(false)` wins when both are given (documented in the
/// [`FormatFlags`] doc; clap `overrides_with` enforces the precedence).
fn tri(bold: bool, no_bold: bool) -> Option<bool> {
    if no_bold {
        Some(false)
    } else if bold {
        Some(true)
    } else {
        None
    }
}

/// `run add` — append a run with optional formatting to a paragraph.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    let bold = tri(args.format.bold, args.format.no_bold);
    let italic = tri(args.format.italic, args.format.no_italic);
    let underline = tri(args.format.underline, args.format.no_underline);
    crate::commands::with_doc(ctx, |mgr| {
        mgr.add_run(
            &args.text,
            args.id.as_deref(),
            args.index,
            bold,
            italic,
            underline,
            args.format.font.as_deref(),
            args.format.size,
            args.format.color.as_deref(),
        )
    })?;
    Ok(Data::RunAdded {
        id: args.id.clone(),
        index: args.index,
        text: args.text.clone(),
        bold,
        italic,
        underline,
        font: args.format.font.clone(),
        size: args.format.size,
        color: args.format.color.clone(),
        message: "Run added".into(),
    })
}

/// `run get` — runs of a body paragraph or a cell paragraph.
pub fn get(ctx: &Ctx, args: &GetArgs) -> Result<Data, PoetError> {
    if let Some(table) = args.cell.table {
        let runs: Vec<RunInfo> = crate::commands::with_doc(ctx, |mgr| {
            mgr.get_cell_runs(Some(table), args.cell.row, args.cell.col, args.cell.para)
        })?;
        return Ok(Data::RunsGotCell {
            table,
            row: args.cell.row.unwrap_or(0),
            col: args.cell.col.unwrap_or(0),
            para: args.cell.para,
            runs,
        });
    }
    let runs = crate::commands::with_doc(ctx, |mgr| {
        mgr.get_runs(args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::RunsGot {
        id: args.address.id.clone(),
        index: args.address.index,
        runs,
    })
}

/// `run clear` — remove all runs of a paragraph.
pub fn clear(ctx: &Ctx, args: &ClearArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.clear_runs(args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::RunsCleared {
        id: args.address.id.clone(),
        index: args.address.index,
        message: "Runs cleared".into(),
    })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::output::render;

    use super::*;

    fn open_doc(ctx: &Ctx) {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        *ctx.doc.borrow_mut() = Some(mgr);
    }

    fn seed_paragraph(ctx: &Ctx, text: &str) {
        crate::commands::paragraph::add(
            ctx,
            &crate::commands::paragraph::AddArgs {
                text: text.into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("para");
    }

    #[test]
    fn run_add_get_clear_round_trip() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        seed_paragraph(&ctx, "base");
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                text: "loud".into(),
                id: None,
                index: Some(0),
                format: FormatFlags {
                    bold: true,
                    no_bold: false,
                    italic: false,
                    no_italic: false,
                    underline: false,
                    no_underline: false,
                    font: Some("Arial".into()),
                    size: Some(12.0),
                    color: Some("#00ff00".into()),
                },
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"Run added\""));
        assert!(json.contains("\"bold\": true"));
        assert!(json.contains("\"color\": \"#00ff00\""));
        let (json, err) = render(&get(
            &ctx,
            &GetArgs {
                address: AddressArgs {
                    id: None,
                    index: Some(0),
                },
                cell: CellArgs {
                    table: None,
                    row: None,
                    col: None,
                    para: None,
                },
            },
        ));
        assert!(!err);
        assert!(json.contains("\"font\": \"Arial\""));
        assert!(json.contains("\"size\": 12.0"));
        assert!(json.contains("\"color\": \"00FF00\""));
        let (json, err) = render(&clear(
            &ctx,
            &ClearArgs {
                address: AddressArgs {
                    id: Some("p1".into()),
                    index: None,
                },
            },
        ));
        assert!(!err);
        assert!(json.contains("Runs cleared"));
        let (json, _) = render(&get(
            &ctx,
            &GetArgs {
                address: AddressArgs {
                    id: None,
                    index: Some(0),
                },
                cell: CellArgs {
                    table: None,
                    row: None,
                    col: None,
                    para: None,
                },
            },
        ));
        assert!(json.contains("\"runs\": []"));
    }

    #[test]
    fn run_get_cell_mode_requires_para() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        crate::commands::table::add(
            &ctx,
            &crate::commands::table::AddArgs {
                rows: 1,
                cols: 1,
                id: None,
                style: "Table Grid".into(),
            },
        )
        .expect("table");
        let (_, err) = render(&get(
            &ctx,
            &GetArgs {
                address: AddressArgs {
                    id: None,
                    index: None,
                },
                cell: CellArgs {
                    table: Some(0),
                    row: Some(0),
                    col: Some(0),
                    para: None,
                },
            },
        ));
        assert!(err, "--para is required for cell run addressing");
    }

    #[test]
    fn run_add_unknown_paragraph_is_not_found() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let err = add(
            &ctx,
            &AddArgs {
                text: "x".into(),
                id: Some("ghost".into()),
                index: None,
                format: FormatFlags {
                    bold: false,
                    no_bold: false,
                    italic: false,
                    no_italic: false,
                    underline: false,
                    no_underline: false,
                    font: None,
                    size: None,
                    color: None,
                },
            },
        )
        .expect_err("unknown id");
        assert!(matches!(err, PoetError::NotFound(_)));
    }
}
