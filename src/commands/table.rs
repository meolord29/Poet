//! Table commands — ported from Words' `commands/table.py`; model mapping
//! per adr/0007. `set-range` parses its JSON in this layer, like Words'
//! typer wrapper.

use clap::Args;
use serde_json::Value;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;
use crate::models::data::TableInfo;

/// Shared `--id`/`--index` table addressing.
#[derive(Debug, Args)]
pub struct AddressArgs {
    /// Table bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Table positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Arguments for `table add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub cols: usize,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Table style name.
    #[arg(long, default_value = "Table Grid")]
    pub style: String,
}

/// Arguments for `table list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `table get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `table set-cell`.
#[derive(Debug, Args)]
pub struct SetCellArgs {
    /// Cell row.
    pub row: usize,
    /// Cell column.
    pub col: usize,
    /// New cell value.
    pub value: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `table set-range`.
#[derive(Debug, Args)]
pub struct SetRangeArgs {
    /// JSON 2D array.
    pub values: String,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// Treat the first row as a header.
    #[arg(short = 'H', long)]
    pub header: bool,
}

/// Arguments for `table add-row` / `table add-column`.
#[derive(Debug, Args)]
pub struct AppendArgs {
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
    /// JSON array of cell values.
    #[arg(long)]
    pub values: Option<String>,
}

/// Arguments for `table delete-row`.
#[derive(Debug, Args)]
pub struct DeleteRowArgs {
    /// Row index to delete.
    pub row: usize,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// Arguments for `table delete-column`.
#[derive(Debug, Args)]
pub struct DeleteColumnArgs {
    /// Column index to delete.
    pub col: usize,
    /// Addressing by `--id`/`--index`.
    #[command(flatten)]
    pub address: AddressArgs,
}

/// All `table` actions.
#[derive(Debug, clap::Subcommand)]
pub enum TableAction {
    /// Add a table.
    Add(AddArgs),
    /// List tables.
    List(ListArgs),
    /// Get table details.
    Get(GetArgs),
    /// Set one cell's value.
    SetCell(SetCellArgs),
    /// Fill a 2D range of cells.
    SetRange(SetRangeArgs),
    /// Append a row.
    AddRow(AppendArgs),
    /// Append a column.
    AddColumn(AppendArgs),
    /// Delete a row.
    DeleteRow(DeleteRowArgs),
    /// Delete a column.
    DeleteColumn(DeleteColumnArgs),
}

/// Parse the `--values` JSON array argument.
fn parse_values(raw: &Option<String>) -> Result<Option<Vec<Value>>, PoetError> {
    match raw {
        None => Ok(None),
        Some(raw) => serde_json::from_str(raw)
            .map(Some)
            .map_err(|e| PoetError::Validation(format!("Invalid JSON: {e}"))),
    }
}

/// `table add` — empty rows × cols grid, bookmarked.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| {
        let id = mgr.add_table(args.rows, args.cols, args.id.as_deref(), &args.style)?;
        crate::core::annotate::on_table_add(mgr, &id, args.rows, args.cols)?;
        Ok(id)
    })?;
    Ok(Data::TableAdded {
        id,
        rows: args.rows,
        cols: args.cols,
        style: args.style.clone(),
        message: "Table added".into(),
    })
}

/// `table list`.
pub fn list(ctx: &Ctx, _args: &ListArgs) -> Result<Data, PoetError> {
    let tables: Vec<TableInfo> = crate::commands::with_doc(ctx, |mgr| mgr.list_tables())?;
    Ok(Data::TableList { tables })
}

/// `table get` — the grid as a 2D array of cell texts.
pub fn get(ctx: &Ctx, args: &GetArgs) -> Result<Data, PoetError> {
    let (rows, cols, data) = crate::commands::with_doc(ctx, |mgr| {
        mgr.table_data(args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::TableGot {
        id: args.address.id.clone(),
        index: args.address.index,
        rows,
        cols,
        data,
    })
}

/// `table set-cell` — replace one cell's content.
pub fn set_cell(ctx: &Ctx, args: &SetCellArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.set_cell(
            args.row,
            args.col,
            &args.value,
            args.address.id.as_deref(),
            args.address.index,
        )
    })?;
    Ok(Data::CellSet {
        id: args.address.id.clone(),
        index: args.address.index,
        row: args.row,
        col: args.col,
        value: args.value.clone(),
        message: "Cell set".into(),
    })
}

/// `table set-range` — write a 2D array from (0,0), row-major. Out-of-range
/// ranges fail mid-way exactly like Words (repeated `set-cell`).
pub fn set_range(ctx: &Ctx, args: &SetRangeArgs) -> Result<Data, PoetError> {
    let values: Value = serde_json::from_str(&args.values)
        .map_err(|e| PoetError::Validation(format!("Invalid JSON: {e}")))?;
    let Value::Array(rows) = values else {
        return Err(PoetError::Validation(
            "values must be a JSON 2D array".into(),
        ));
    };
    crate::commands::with_doc(ctx, |mgr| {
        for (ri, row) in rows.iter().enumerate() {
            let Value::Array(cells) = row else {
                return Err(PoetError::Validation(
                    "values must be a JSON 2D array".into(),
                ));
            };
            for (ci, cell) in cells.iter().enumerate() {
                mgr.set_cell(
                    ri,
                    ci,
                    &crate::core::content::scalar_text(cell),
                    args.address.id.as_deref(),
                    args.address.index,
                )?;
            }
        }
        if args.header && !rows.is_empty() {
            // Every row was validated as an array by the write loop above.
            let typed: Vec<Vec<Value>> = rows
                .iter()
                .map(|row| row.as_array().cloned().unwrap_or_default())
                .collect();
            annotate_table_data(mgr, args, &typed)?;
        }
        Ok(())
    })?;
    Ok(Data::TableRangeSet {
        id: args.address.id.clone(),
        index: args.address.index,
        rows_written: rows.len(),
        header: args.header,
        message: "Table range set".into(),
    })
}

/// Resolve the `set-range` annotator target: the given id, else the bookmark
/// wrapping the resolved table, else Words' `index:N` rendering.
fn annotate_table_data(
    mgr: &mut crate::core::document::DocumentManager,
    args: &SetRangeArgs,
    rows: &[Vec<Value>],
) -> Result<(), PoetError> {
    let docx = mgr.docx()?;
    let at =
        crate::core::content::resolve_table(docx, args.address.id.as_deref(), args.address.index)?;
    let target = args
        .address
        .id
        .clone()
        .or_else(|| crate::core::content::name_around(&docx.document.children, at))
        .unwrap_or_else(|| crate::core::annotate::target_or_index(None, args.address.index));
    crate::core::annotate::on_table_data(mgr, &target, rows, true)
}

/// `table add-row` — optional `--values` JSON array.
pub fn add_row(ctx: &Ctx, args: &AppendArgs) -> Result<Data, PoetError> {
    let values = parse_values(&args.values)?;
    crate::commands::with_doc(ctx, |mgr| {
        mgr.add_row(
            args.address.id.as_deref(),
            args.address.index,
            values.as_deref(),
        )
    })?;
    Ok(Data::TableRowAdded {
        id: args.address.id.clone(),
        index: args.address.index,
        message: "Row added".into(),
    })
}

/// `table add-column` — optional `--values` JSON array.
pub fn add_column(ctx: &Ctx, args: &AppendArgs) -> Result<Data, PoetError> {
    let values = parse_values(&args.values)?;
    crate::commands::with_doc(ctx, |mgr| {
        mgr.add_column(
            args.address.id.as_deref(),
            args.address.index,
            values.as_deref(),
        )
    })?;
    Ok(Data::TableColumnAdded {
        id: args.address.id.clone(),
        index: args.address.index,
        message: "Column added".into(),
    })
}

/// `table delete-row`.
pub fn delete_row(ctx: &Ctx, args: &DeleteRowArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.delete_row(args.row, args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::TableRowDeleted {
        id: args.address.id.clone(),
        index: args.address.index,
        row: args.row,
        message: "Row deleted".into(),
    })
}

/// `table delete-column`.
pub fn delete_column(ctx: &Ctx, args: &DeleteColumnArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.delete_column(args.col, args.address.id.as_deref(), args.address.index)
    })?;
    Ok(Data::TableColumnDeleted {
        id: args.address.id.clone(),
        index: args.address.index,
        col: args.col,
        message: "Column deleted".into(),
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

    fn addr(id: Option<&str>, index: Option<usize>) -> AddressArgs {
        AddressArgs {
            id: id.map(str::to_string),
            index,
        }
    }

    #[test]
    fn table_add_get_set_cell_payload_flow() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                rows: 2,
                cols: 2,
                id: None,
                style: "Table Grid".into(),
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"Table added\""));
        assert!(json.contains("\"id\": \"t1\""));
        let (json, err) = render(&set_cell(
            &ctx,
            &SetCellArgs {
                row: 0,
                col: 1,
                value: "cell".into(),
                address: addr(None, Some(0)),
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"Cell set\""));
        let (json, _) = render(&get(
            &ctx,
            &GetArgs {
                address: addr(None, Some(0)),
            },
        ));
        assert!(json.contains("\"rows\": 2"));
        assert!(json.contains("\"cols\": 2"));
        assert!(json.contains("\"data\""));
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"style\": \"Table Grid\""));
    }

    #[test]
    fn set_range_writes_grid_and_reports_rows_written() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        add(
            &ctx,
            &AddArgs {
                rows: 2,
                cols: 2,
                id: None,
                style: "Table Grid".into(),
            },
        )
        .expect("table");
        let (json, err) = render(&set_range(
            &ctx,
            &SetRangeArgs {
                values: r#"[["h1","h2"],[1,true]]"#.into(),
                address: addr(None, Some(0)),
                header: true,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"Table range set\""));
        assert!(json.contains("\"rows_written\": 2"));
        assert!(json.contains("\"header\": true"));
        let (json, _) = render(&get(
            &ctx,
            &GetArgs {
                address: addr(None, Some(0)),
            },
        ));
        assert!(json.contains("\"h1\""));
        assert!(
            json.contains("\"true\""),
            "JSON conventions stringification"
        );
    }

    #[test]
    fn set_range_bad_json_is_validation_error() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let err = set_range(
            &ctx,
            &SetRangeArgs {
                values: "not json".into(),
                address: addr(None, Some(0)),
                header: false,
            },
        )
        .expect_err("bad json");
        assert!(matches!(err, PoetError::Validation(ref m) if m.starts_with("Invalid JSON: ")));
        let err = set_range(
            &ctx,
            &SetRangeArgs {
                values: "[1,2]".into(),
                address: addr(None, Some(0)),
                header: false,
            },
        )
        .expect_err("not 2d");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("2D array")));
    }

    #[test]
    fn add_row_column_parse_values_and_delete_flow() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        add(
            &ctx,
            &AddArgs {
                rows: 1,
                cols: 1,
                id: None,
                style: "Table Grid".into(),
            },
        )
        .expect("table");
        let err = add_row(
            &ctx,
            &AppendArgs {
                address: addr(None, Some(0)),
                values: Some("nope".into()),
            },
        )
        .expect_err("bad values");
        assert!(matches!(err, PoetError::Validation(ref m) if m.starts_with("Invalid JSON: ")));
        let (json, err) = render(&add_row(
            &ctx,
            &AppendArgs {
                address: addr(None, Some(0)),
                values: Some(r#"["r1c1"]"#.into()),
            },
        ));
        assert!(!err);
        assert!(json.contains("Row added"));
        let (json, _) = render(&add_column(
            &ctx,
            &AppendArgs {
                address: addr(None, Some(0)),
                values: Some(r#"["c1","c2"]"#.into()),
            },
        ));
        assert!(json.contains("Column added"));
        let (json, _) = render(&delete_row(
            &ctx,
            &DeleteRowArgs {
                row: 0,
                address: addr(None, Some(0)),
            },
        ));
        assert!(json.contains("Row deleted"));
        let (json, _) = render(&delete_column(
            &ctx,
            &DeleteColumnArgs {
                col: 0,
                address: addr(None, Some(0)),
            },
        ));
        assert!(json.contains("Column deleted"));
        let (json, _) = render(&get(
            &ctx,
            &GetArgs {
                address: addr(None, Some(0)),
            },
        ));
        assert!(json.contains("\"rows\": 1"));
        assert!(json.contains("\"cols\": 1"));
    }

    #[test]
    fn set_cell_out_of_range_is_not_found() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        add(
            &ctx,
            &AddArgs {
                rows: 1,
                cols: 1,
                id: None,
                style: "Table Grid".into(),
            },
        )
        .expect("table");
        let err = set_cell(
            &ctx,
            &SetCellArgs {
                row: 9,
                col: 0,
                value: "x".into(),
                address: addr(None, Some(0)),
            },
        )
        .expect_err("range");
        assert!(matches!(err, PoetError::NotFound(_)));
    }
}
