//! Paragraph commands — ported from Words' `commands/paragraph.py` +
//! `document_manager.py` content methods (adr/0006 addressing).

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;
use crate::models::data::{FindResult, ParagraphInfo};

/// Arguments for `paragraph add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Paragraph text.
    #[arg(default_value = "")]
    pub text: String,
    /// Paragraph style name.
    #[arg(long)]
    pub style: Option<String>,
    /// Bookmark id for this paragraph.
    #[arg(long)]
    pub id: Option<String>,
    /// Start the paragraph on a new page.
    #[arg(long)]
    pub page_break: bool,
}

/// Arguments for `paragraph insert`.
#[derive(Debug, Args)]
pub struct InsertArgs {
    /// 0-based position to insert at.
    pub index: usize,
    /// Paragraph text.
    #[arg(default_value = "")]
    pub text: String,
    /// Paragraph style name.
    #[arg(long)]
    pub style: Option<String>,
    /// Bookmark id for this paragraph.
    #[arg(long)]
    pub id: Option<String>,
    /// Start the inserted paragraph on a new page.
    #[arg(long)]
    pub page_break: bool,
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
    /// Paragraph index within the cell (omit for all).
    #[arg(long)]
    pub para: Option<usize>,
}

/// Arguments for `paragraph get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Cell addressing (`--table/--row/--col/--para`).
    #[command(flatten)]
    pub cell: CellArgs,
}

/// Arguments for `paragraph update`.
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// New text.
    pub text: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph delete`.
#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Cell addressing (`--table/--row/--col/--para`).
    #[command(flatten)]
    pub cell: CellArgs,
}

/// Arguments for `paragraph list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `paragraph move`.
#[derive(Debug, Args)]
pub struct MoveArgs {
    /// up | down.
    #[arg(default_value = "up")]
    pub direction: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph clear`.
#[derive(Debug, Args)]
pub struct ClearArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph border` (body lands in phase 3).
#[derive(Debug, Args)]
pub struct BorderArgs {
    /// top|left|bottom|right|between.
    #[arg(long, default_value = "bottom")]
    pub position: String,
    /// Hex color, e.g. 000000.
    #[arg(long, default_value = "000000")]
    pub color: String,
    /// Border thickness (eighths of a point).
    #[arg(long, default_value_t = 4)]
    pub size: i64,
    /// Gap between border and text (points).
    #[arg(long, default_value_t = 1)]
    pub space: i64,
    /// single|double|dashed|...
    #[arg(long, default_value = "single")]
    pub style: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `paragraph find`.
#[derive(Debug, Args)]
pub struct FindArgs {
    /// Text to search for.
    pub text: String,
}

/// Arguments for `paragraph replace`.
#[derive(Debug, Args)]
pub struct ReplaceArgs {
    /// Text to find.
    pub find: String,
    /// Replacement text.
    pub replace: String,
}

/// Arguments for `paragraph count`.
#[derive(Debug, Args)]
pub struct CountArgs {}

/// All `paragraph` actions.
#[derive(Debug, clap::Subcommand)]
pub enum ParagraphAction {
    /// Add a paragraph.
    Add(AddArgs),
    /// Insert a paragraph at a position.
    Insert(InsertArgs),
    /// Get paragraph details.
    Get(GetArgs),
    /// Update paragraph text.
    Update(UpdateArgs),
    /// Delete a paragraph.
    Delete(DeleteArgs),
    /// List paragraphs.
    List(ListArgs),
    /// Move a paragraph up or down.
    Move(MoveArgs),
    /// Clear paragraph text.
    Clear(ClearArgs),
    /// Add a paragraph border (default: a bottom underline).
    Border(BorderArgs),
    /// Find text in paragraphs.
    Find(FindArgs),
    /// Replace text in paragraphs.
    Replace(ReplaceArgs),
    /// Count paragraphs.
    Count(CountArgs),
}

/// `paragraph border` — set one `w:pBdr` side of a paragraph (position
/// validated in the core layer before resolution, like Words).
pub fn border(ctx: &Ctx, args: &BorderArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.set_paragraph_border(
            &args.position,
            &args.color,
            args.size,
            args.space,
            &args.style,
            args.address.id.as_deref(),
            args.address.index,
        )
    })?;
    Ok(Data::ParagraphBorder {
        id: args.address.id.clone(),
        index: args.address.index,
        position: args.position.clone(),
        color: args.color.clone(),
        size: args.size,
        space: args.space,
        style: args.style.clone(),
        message: format!("Paragraph {} border set", args.position),
    })
}

/// `paragraph add` — append and bookmark a paragraph.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| {
        let id = mgr.add_paragraph(
            &args.text,
            args.style.as_deref(),
            args.id.as_deref(),
            args.page_break,
        )?;
        crate::core::annotate::on_paragraph_add(mgr, &id, &args.text, args.style.as_deref())?;
        Ok(id)
    })?;
    Ok(Data::ParagraphAdded {
        id,
        text: args.text.clone(),
        style: args.style.clone(),
        page_break: args.page_break,
        message: "Paragraph added".into(),
    })
}

/// `paragraph insert` — insert before the paragraph at `index`.
pub fn insert(ctx: &Ctx, args: &InsertArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| {
        let id = mgr.insert_paragraph(
            args.index,
            &args.text,
            args.style.as_deref(),
            args.id.as_deref(),
            args.page_break,
        )?;
        crate::core::annotate::on_paragraph_add(mgr, &id, &args.text, args.style.as_deref())?;
        Ok(id)
    })?;
    Ok(Data::ParagraphInserted {
        id,
        index: args.index,
        text: args.text.clone(),
        style: args.style.clone(),
        page_break: args.page_break,
        message: "Paragraph inserted".into(),
    })
}

/// `paragraph get` — body paragraph text, or cell paragraph text(s) with
/// `--table` addressing.
pub fn get(ctx: &Ctx, args: &GetArgs) -> Result<Data, PoetError> {
    if let Some(table) = args.cell.table {
        let text = crate::commands::with_doc(ctx, |mgr| {
            mgr.get_cell_paragraph_text(Some(table), args.cell.row, args.cell.col, args.cell.para)
        })?;
        return Ok(Data::ParagraphGotCell {
            table,
            row: args.cell.row.unwrap_or(0),
            col: args.cell.col.unwrap_or(0),
            para: args.cell.para,
            text,
        });
    }
    let text = crate::commands::with_doc(ctx, |mgr| {
        mgr.get_paragraph_text(args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::ParagraphGot {
        id: args.address.id.clone(),
        index: args.address.index,
        text,
    })
}

/// `paragraph update` — replace the paragraph's runs.
pub fn update(ctx: &Ctx, args: &UpdateArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.update_paragraph(&args.text, args.address.id.as_deref(), args.address.index)?;
        crate::core::annotate::on_paragraph_set(
            mgr,
            &crate::core::annotate::target_or_index(args.address.id.as_deref(), args.address.index),
            &args.text,
        )
    })?;
    Ok(Data::ParagraphUpdated {
        id: args.address.id.clone(),
        index: args.address.index,
        text: args.text.clone(),
        message: "Paragraph updated".into(),
    })
}

/// `paragraph delete` — body paragraph, or cell paragraph with `--table`.
pub fn delete(ctx: &Ctx, args: &DeleteArgs) -> Result<Data, PoetError> {
    if let Some(table) = args.cell.table {
        let para = args.cell.para.unwrap_or(0);
        crate::commands::with_doc(ctx, |mgr| {
            mgr.delete_cell_paragraph(Some(table), args.cell.row, args.cell.col, args.cell.para)
        })?;
        return Ok(Data::CellParagraphDeleted {
            table,
            row: args.cell.row.unwrap_or(0),
            col: args.cell.col.unwrap_or(0),
            para,
            message: "Cell paragraph deleted".into(),
        });
    }
    crate::commands::with_doc(ctx, |mgr| {
        mgr.delete_paragraph(args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::ParagraphDeleted {
        id: args.address.id.clone(),
        index: args.address.index,
        message: "Paragraph deleted".into(),
    })
}

/// `paragraph list`.
pub fn list(ctx: &Ctx, _args: &ListArgs) -> Result<Data, PoetError> {
    let paragraphs: Vec<ParagraphInfo> =
        crate::commands::with_doc(ctx, |mgr| mgr.list_paragraphs())?;
    Ok(Data::ParagraphList { paragraphs })
}

/// `paragraph move` — up (default) or down.
pub fn r#move(ctx: &Ctx, args: &MoveArgs) -> Result<Data, PoetError> {
    let index = crate::commands::with_doc(ctx, |mgr| {
        mgr.move_paragraph(
            &args.direction,
            args.address.id.as_deref(),
            args.address.index,
        )
    })?;
    Ok(Data::ParagraphMoved {
        id: args.address.id.clone(),
        index,
        direction: args.direction.clone(),
        message: format!("Paragraph moved {}", args.direction),
    })
}

/// `paragraph clear` — empty the paragraph's runs.
pub fn clear(ctx: &Ctx, args: &ClearArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.clear_paragraph(args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::ParagraphCleared {
        id: args.address.id.clone(),
        index: args.address.index,
        message: "Paragraph cleared".into(),
    })
}

/// `paragraph find` — case-insensitive across paragraphs and cells.
pub fn find(ctx: &Ctx, args: &FindArgs) -> Result<Data, PoetError> {
    let results: Vec<FindResult> = crate::commands::with_doc(ctx, |mgr| mgr.find_text(&args.text))?;
    let count = results.len();
    Ok(Data::ParagraphFound {
        text: args.text.clone(),
        results,
        count,
        message: "Search completed".into(),
    })
}

/// `paragraph replace` — across paragraphs and cells.
pub fn replace(ctx: &Ctx, args: &ReplaceArgs) -> Result<Data, PoetError> {
    let count = crate::commands::with_doc(ctx, |mgr| mgr.replace_text(&args.find, &args.replace))?;
    Ok(Data::ParagraphReplaced {
        find: args.find.clone(),
        replace: args.replace.clone(),
        count,
        message: "Replace completed".into(),
    })
}

/// `paragraph count`.
pub fn count(ctx: &Ctx, _args: &CountArgs) -> Result<Data, PoetError> {
    let count = crate::commands::with_doc(ctx, |mgr| mgr.paragraph_count())?;
    Ok(Data::ParagraphCount { count })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::output::render;

    use super::*;

    fn open_doc(ctx: &Ctx, _dir: &std::path::Path) {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        *ctx.doc.borrow_mut() = Some(mgr);
    }

    #[test]
    fn add_returns_words_payload_with_empty_envelope_message() {
        let (ctx, dir) = setup();
        open_doc(&ctx, &dir);
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                text: "hello".into(),
                style: None,
                id: Some("intro".into()),
                page_break: false,
            },
        ));
        assert!(!err);
        // Envelope message stays empty; the message lives inside data (Words).
        assert!(json.contains("\"status\": \"ok\""));
        assert!(json.contains("\"message\": \"\""));
        assert!(json.contains("\"id\": \"intro\""));
        assert!(json.contains("\"message\": \"Paragraph added\""));
        assert!(json.contains("\"page_break\": false"));
    }

    #[test]
    fn get_update_clear_delete_flow_by_id_and_index() {
        let (ctx, dir) = setup();
        open_doc(&ctx, &dir);
        add(
            &ctx,
            &AddArgs {
                text: "v1".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("add");
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
        assert!(json.contains("\"text\": \"v1\""));
        update(
            &ctx,
            &UpdateArgs {
                text: "v2".into(),
                address: AddressArgs {
                    id: Some("p1".into()),
                    index: None,
                },
            },
        )
        .expect("update");
        let (json, _) = render(&get(
            &ctx,
            &GetArgs {
                address: AddressArgs {
                    id: Some("p1".into()),
                    index: None,
                },
                cell: CellArgs {
                    table: None,
                    row: None,
                    col: None,
                    para: None,
                },
            },
        ));
        assert!(json.contains("v2"));
        clear(
            &ctx,
            &ClearArgs {
                address: AddressArgs {
                    id: None,
                    index: Some(0),
                },
            },
        )
        .expect("clear");
        let (_, err) = render(&delete(
            &ctx,
            &DeleteArgs {
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
        let (_, err) = render(&get(
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
        assert!(err, "deleted paragraph no longer resolves");
    }

    #[test]
    fn get_cell_mode_returns_cell_text() {
        let (ctx, dir) = setup();
        open_doc(&ctx, &dir);
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
        crate::commands::table::set_cell(
            &ctx,
            &crate::commands::table::SetCellArgs {
                row: 0,
                col: 0,
                value: "cell txt".into(),
                address: crate::commands::table::AddressArgs {
                    id: None,
                    index: Some(0),
                },
            },
        )
        .expect("cell");
        let (json, err) = render(&get(
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
                    para: Some(0),
                },
            },
        ));
        assert!(!err);
        assert!(json.contains("\"cell txt\""));
        assert!(json.contains("\"table\": 0"));
    }

    #[test]
    fn insert_move_find_replace_count_list() {
        let (ctx, dir) = setup();
        open_doc(&ctx, &dir);
        add(
            &ctx,
            &AddArgs {
                text: "alpha".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("add");
        add(
            &ctx,
            &AddArgs {
                text: "gamma".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("add");
        let data = insert(
            &ctx,
            &InsertArgs {
                index: 1,
                text: "beta".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("insert");
        let (json, _) = render(&Ok(data));
        assert!(json.contains("Paragraph inserted"));
        let data = r#move(
            &ctx,
            &MoveArgs {
                direction: "down".into(),
                address: AddressArgs {
                    id: None,
                    index: Some(0),
                },
            },
        )
        .expect("move");
        let (json, _) = render(&Ok(data));
        assert!(json.contains("Paragraph moved down"));
        assert!(json.contains("\"index\": 1"));
        let (json, _) = render(&find(
            &ctx,
            &FindArgs {
                text: "ALPHA".into(),
            },
        ));
        assert!(json.contains("Search completed"));
        assert!(json.contains("\"count\": 1"));
        let (json, _) = render(&replace(
            &ctx,
            &ReplaceArgs {
                find: "beta".into(),
                replace: String::new(),
            },
        ));
        assert!(json.contains("Replace completed"));
        let (json, _) = render(&count(&ctx, &CountArgs {}));
        assert!(json.contains("\"count\": 3"));
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"paragraphs\""));
        assert!(json.contains("\"style\": \"Normal\""));
    }

    #[test]
    fn commands_without_open_document_report_state_error() {
        let (ctx, _dir) = setup();
        let err = count(&ctx, &CountArgs {}).expect_err("no doc");
        assert!(matches!(err, PoetError::DocumentState(_)));
    }

    #[test]
    fn border_envelope_and_position_validation() {
        let (ctx, _dir) = setup();
        open_doc(&ctx, &_dir);
        add(
            &ctx,
            &AddArgs {
                text: "boxed".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("add");
        let (json, err) = render(&border(
            &ctx,
            &BorderArgs {
                position: "top".into(),
                color: "AA0011".into(),
                size: 8,
                space: 2,
                style: "double".into(),
                address: AddressArgs {
                    id: None,
                    index: Some(0),
                },
            },
        ));
        assert!(!err);
        assert!(json.contains("\"position\": \"top\""));
        assert!(json.contains("\"color\": \"AA0011\""));
        assert!(json.contains("\"style\": \"double\""));
        assert!(json.contains("\"message\": \"Paragraph top border set\""));

        let err = border(
            &ctx,
            &BorderArgs {
                position: "middle".into(),
                color: "000000".into(),
                size: 4,
                space: 1,
                style: "single".into(),
                address: AddressArgs {
                    id: None,
                    index: Some(0),
                },
            },
        )
        .expect_err("bad position");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("'middle'")));
    }
}
