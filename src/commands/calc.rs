//! Calculation and analysis commands — ported from Words' `calc_cmd.py`.
//! File-path based (no session, never autosaves); the table is read fresh
//! from disk per invocation. The rhai expression grammar accepted by
//! `transform add_column` is documented on [`TransformArgs`] (adr/0013).

use std::path::Path;

use clap::Args;
use serde_json::Value;

use crate::core::error::PoetError;
use crate::core::output::Data;
use polars::prelude::DataFrame;

/// Shared calc addressing: `--id`/`--index` plus the reader's `--range`.
#[derive(Debug, Args)]
pub struct AddressArgs {
    /// Table bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Table positional index.
    #[arg(long)]
    pub index: Option<usize>,
    /// Row window `start:end`, 1-based, both ends inclusive; the header row
    /// is always kept.
    #[arg(long)]
    pub range: Option<String>,
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
///
/// `--operations` is a JSON array applied in order:
/// `{"type":"sort","by":"Col","descending":false}`,
/// `{"type":"rename","mapping":{"Old":"New"}}`,
/// `{"type":"drop","columns":["X"]}`, `{"type":"select","columns":["X"]}`,
/// `{"type":"fill_null","column":"X","value":0}` (omit `column` for the
/// whole table), and
/// `{"type":"add_column","name":"New","expression":"col(\"X\") * 2"}`.
///
/// # add_column expression grammar (rhai)
///
/// `col("Name")` reads the current row's cell (number/string/bool/null);
/// numeric and string literals, `+ - * / %`, comparisons
/// (`== != > < >= <=`), `&& || !`, and `if cond { a } else { b }` are
/// available. Examples: `col("Q1") * 2`, `col("Name") + "!"`,
/// `if col("Flag") { "yes" } else { "no" }`. A missing column, a type
/// mismatch, or a mixed number/text result is a `calculation_error`.
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

/// Read + coerce the addressed table into a frame (adr/0013).
fn read_frame(path: &str, address: &AddressArgs) -> Result<DataFrame, PoetError> {
    let table = crate::core::calc::read_table(
        Path::new(path),
        address.id.as_deref(),
        address.index,
        address.range.as_deref(),
    )?;
    crate::core::calc::build_frame(&table)
}

/// `calc read`.
pub fn read(ctx: &crate::core::Ctx, args: &ReadArgs) -> Result<Data, PoetError> {
    let _ = ctx;
    let df = read_frame(&args.path, &args.address)?;
    let columns = df
        .get_column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let data = crate::core::calc::rows_to_values(&df)?;
    Ok(Data::CalcRead {
        path: args.path.clone(),
        table_id: args.address.id.clone(),
        table_index: args.address.index,
        rows: df.height(),
        columns,
        data,
        message: "Table read successfully".into(),
    })
}

/// `calc stats`.
pub fn stats(ctx: &crate::core::Ctx, args: &StatsArgs) -> Result<Data, PoetError> {
    let _ = ctx;
    let df = read_frame(&args.path, &args.address)?;
    Ok(Data::CalcStats {
        path: args.path.clone(),
        statistics: crate::core::calc::statistics(&df, args.column.as_deref()),
        message: "Statistics calculated".into(),
    })
}

/// `calc aggregate`.
pub fn aggregate(ctx: &crate::core::Ctx, args: &AggregateArgs) -> Result<Data, PoetError> {
    let _ = ctx;
    let df = read_frame(&args.path, &args.address)?;
    let result =
        crate::core::calc::aggregate(&df, &args.group_by, &args.agg_column, &args.agg_func)?;
    let result = crate::core::calc::rows_to_values(&result)?;
    Ok(Data::CalcAggregate {
        path: args.path.clone(),
        group_by: args.group_by.clone(),
        agg_column: args.agg_column.clone(),
        agg_func: args.agg_func.clone(),
        result,
        message: "Data aggregated successfully".into(),
    })
}

/// `calc filter`.
pub fn filter(ctx: &crate::core::Ctx, args: &FilterArgs) -> Result<Data, PoetError> {
    let _ = ctx;
    let df = read_frame(&args.path, &args.address)?;
    let result = crate::core::calc::filter(&df, &args.column, &args.operator, &args.value)?;
    let result = crate::core::calc::rows_to_values(&result)?;
    Ok(Data::CalcFilter {
        path: args.path.clone(),
        column: args.column.clone(),
        operator: args.operator.clone(),
        value: args.value.clone(),
        result,
        message: "Data filtered successfully".into(),
    })
}

/// `calc transform`.
pub fn transform(ctx: &crate::core::Ctx, args: &TransformArgs) -> Result<Data, PoetError> {
    let _ = ctx;
    let operations: Value = serde_json::from_str(&args.operations)
        .map_err(|e| PoetError::Validation(format!("Invalid operations JSON: {e}")))?;
    let Value::Array(ops) = &operations else {
        return Err(PoetError::Validation(
            "operations must be a JSON array".into(),
        ));
    };
    let mut df = read_frame(&args.path, &args.address)?;
    crate::core::calc::transform(&mut df, ops)?;
    let result = crate::core::calc::rows_to_values(&df)?;
    Ok(Data::CalcTransform {
        path: args.path.clone(),
        operations,
        result,
        message: "Data transformed successfully".into(),
    })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::document::DocumentManager;
    use crate::core::error::PoetError;
    use crate::core::output::render;

    use super::*;

    fn write_doc(ctx: &crate::core::Ctx, dir: &std::path::Path) -> String {
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        let id = mgr
            .add_table(4, 3, Some("sales"), "Table Grid")
            .expect("table");
        assert_eq!(id, "sales");
        *ctx.doc.borrow_mut() = Some(mgr);
        crate::commands::table::set_range(
            ctx,
            &crate::commands::table::SetRangeArgs {
                values: r#"[
                    ["Product", "Q1", "Q2"],
                    ["Widget A", 15000, 18000],
                    ["Widget B", 12000, 14500],
                    ["Total", 27000, 32500]
                ]"#
                .into(),
                address: crate::commands::table::AddressArgs {
                    id: Some("sales".into()),
                    index: None,
                },
                header: true,
            },
        )
        .expect("fill");
        let path = dir.join("data.docx");
        crate::commands::with_doc(ctx, |mgr| mgr.save("docx", &path)).expect("save");
        path.to_string_lossy().into_owned()
    }

    fn address() -> AddressArgs {
        AddressArgs {
            id: Some("sales".into()),
            index: None,
            range: None,
        }
    }

    #[test]
    fn read_stats_aggregate_filter_transform_envelopes() {
        let (ctx, dir) = setup();
        let path = write_doc(&ctx, &dir);

        let got = read(
            &ctx,
            &ReadArgs {
                path: path.clone(),
                address: address(),
            },
        )
        .expect("read");
        let Data::CalcRead {
            rows,
            columns,
            data,
            message,
            table_id,
            ..
        } = got
        else {
            panic!("wrong variant");
        };
        assert_eq!(table_id.as_deref(), Some("sales"));
        assert_eq!(rows, 3, "header excluded");
        assert_eq!(columns, vec!["Product", "Q1", "Q2"]);
        assert_eq!(data[0]["Q1"], 15000, "int coercion");
        assert_eq!(message, "Table read successfully");

        let got = stats(
            &ctx,
            &StatsArgs {
                path: path.clone(),
                column: Some("Q1".into()),
                address: address(),
            },
        )
        .expect("stats");
        let Data::CalcStats {
            statistics,
            message,
            ..
        } = got
        else {
            panic!("wrong variant");
        };
        assert_eq!(statistics["Q1"]["count"], 3);
        assert_eq!(message, "Statistics calculated");

        let got = aggregate(
            &ctx,
            &AggregateArgs {
                path: path.clone(),
                group_by: "Product".into(),
                agg_column: "Q1".into(),
                agg_func: "sum".into(),
                address: address(),
            },
        )
        .expect("aggregate");
        let Data::CalcAggregate {
            result, message, ..
        } = got
        else {
            panic!("wrong variant");
        };
        assert_eq!(result.len(), 3);
        assert_eq!(message, "Data aggregated successfully");

        let got = filter(
            &ctx,
            &FilterArgs {
                path: path.clone(),
                column: "Q1".into(),
                operator: ">".into(),
                value: "14000".into(),
                address: address(),
            },
        )
        .expect("filter");
        let Data::CalcFilter {
            result,
            value,
            message,
            ..
        } = got
        else {
            panic!("wrong variant");
        };
        assert_eq!(value, "14000", "original string echoed");
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|r| r["Q1"] == 15000));
        assert!(result.iter().any(|r| r["Q1"] == 27000));
        assert_eq!(message, "Data filtered successfully");

        let got = transform(
            &ctx,
            &TransformArgs {
                path,
                operations: r#"[{"type":"sort","by":"Q1","descending":true}]"#.into(),
                address: address(),
            },
        )
        .expect("transform");
        let Data::CalcTransform {
            result, message, ..
        } = got
        else {
            panic!("wrong variant");
        };
        assert_eq!(result[0]["Q1"], 27000);
        assert_eq!(message, "Data transformed successfully");
    }

    #[test]
    fn range_window_exposed_on_commands() {
        let (ctx, dir) = setup();
        let path = write_doc(&ctx, &dir);
        let mut addr = address();
        addr.range = Some("2:3".into());
        let got = read(
            &ctx,
            &ReadArgs {
                path,
                address: addr,
            },
        )
        .expect("read");
        let Data::CalcRead { rows, data, .. } = got else {
            panic!("wrong variant");
        };
        assert_eq!(rows, 2, "header + data rows 2,3");
        assert_eq!(data[0]["Product"], "Widget B");
    }

    #[test]
    fn calc_errors_surface_with_codes() {
        let (ctx, dir) = setup();
        let path = write_doc(&ctx, &dir);
        let missing = read(
            &ctx,
            &ReadArgs {
                path: "/no/such/file.docx".into(),
                address: address(),
            },
        );
        assert!(matches!(missing, Err(PoetError::NotFound(_))));

        let bad_id = read(
            &ctx,
            &ReadArgs {
                path: path.clone(),
                address: AddressArgs {
                    id: Some("nope".into()),
                    index: None,
                    range: None,
                },
            },
        );
        assert_eq!(
            bad_id.unwrap_err().to_string(),
            "not found: No table with id 'nope'"
        );

        let bad_range = read(
            &ctx,
            &ReadArgs {
                path: path.clone(),
                address: AddressArgs {
                    id: None,
                    index: Some(0),
                    range: Some("abc".into()),
                },
            },
        );
        assert!(matches!(bad_range, Err(PoetError::Validation(_))));

        let bad_func = aggregate(
            &ctx,
            &AggregateArgs {
                path,
                group_by: "Product".into(),
                agg_column: "Q1".into(),
                agg_func: "nope".into(),
                address: address(),
            },
        );
        assert_eq!(
            bad_func.unwrap_err().to_string(),
            "calculation error: Unknown aggregation function: nope"
        );

        let (json, is_error) = render(&Err(PoetError::Calculation("x".into())));
        assert!(is_error);
        assert!(json.contains("\"code\": \"calculation_error\""));
    }
}
