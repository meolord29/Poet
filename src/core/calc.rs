//! Calc engine: table reader, analysis strategies, and the transform chain
//! (Words' `calc/reader.py` + `calc/strategies.py` + `calc/analyzer.py`,
//! adr/0013).
//!
//! Every command reads the .docx fresh from disk (no session) and resolves
//! the table by `--id` or `--index` (default first table). The reader keeps
//! Words' coercion quirks (case-insensitive true/false, strict int/float
//! shapes, comma-carrying strings stay strings) and its duplicate-header
//! rule (first occurrence wins, `base_N` for repeats, generated names not
//! re-checked so a later column can replace an earlier one).
//!
//! Deviations from Words are recorded in adr/0013; the notable ones: mixed
//! number/text columns produce a clean `calculation_error` instead of a raw
//! polars TypeError, int+float columns upcast to Float64, group-by order is
//! stable (first appearance), `contains` is a literal substring match, and
//! the `--range` flag is user-facing (malformed ranges are validation
//! errors, not silently ignored).

use std::collections::HashMap;
use std::path::Path;

use polars::prelude::*;
use rhai::{Dynamic, Engine};
use serde_json::Value;

use crate::core::error::PoetError;
use crate::core::meta;

/// One coerced cell value (Words' `_coerce_value` output).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Blank/whitespace-only cell.
    Null,
    /// `true`/`false` (any casing).
    Bool(bool),
    /// Strict integer text (`-?\d+`).
    Int(i64),
    /// Strict decimal text (`-?\d+\.\d+`).
    Float(f64),
    /// Anything else, stripped.
    Str(String),
}

/// A read table: header names (deduplicated) and coerced body cells.
#[derive(Debug, Clone)]
pub struct Table {
    /// Column names after Words' dedup rule.
    pub columns: Vec<String>,
    /// Body rows (header excluded), padded to `columns.len()` with nulls.
    pub rows: Vec<Vec<Cell>>,
}

/// Read + coerce a table from a .docx on disk (Words' `TableReader.read`).
pub fn read_table(
    path: &Path,
    id: Option<&str>,
    index: Option<usize>,
    range: Option<&str>,
) -> Result<Table, PoetError> {
    if !path.exists() {
        return Err(PoetError::NotFound(format!(
            "File not found: {}",
            path.display()
        )));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| PoetError::File(format!("cannot read {}: {e}", path.display())))?;
    let docx = docx_rs::read_docx(&bytes)
        .map_err(|e| PoetError::File(format!("cannot open {}: {e}", path.display())))?;
    // Words defaults to the first table, with its own "no tables" error.
    let index = match (id, index) {
        (Some(_), _) => index,
        (None, index) => {
            let has_tables = crate::core::content::list_tables(&docx)
                .iter()
                .any(|_| true);
            if !has_tables {
                return Err(PoetError::NotFound("Document has no tables".into()));
            }
            Some(index.unwrap_or(0))
        }
    };
    let (rows, _, data) = crate::core::content::table_data(&docx, id, index)?;
    let data = apply_range(data, range)?;
    split_table(rows, data)
}

/// Words' `_apply_range`: `start:end`, 1-based, both ends inclusive, header
/// always kept, indices counted over the full row list (header = row 0),
/// out-of-range clamped like Python slices. Unlike the Words API (where a
/// malformed range was silently ignored), the user-facing flag validates.
fn apply_range(
    mut data: Vec<Vec<String>>,
    range: Option<&str>,
) -> Result<Vec<Vec<String>>, PoetError> {
    let Some(spec) = range else {
        return Ok(data);
    };
    let spec = spec.trim();
    let invalid = || {
        PoetError::Validation(format!(
            "Invalid range: '{spec}' (expected 'start:end', 1-based inclusive)"
        ))
    };
    let Some((start_s, end_s)) = spec.split_once(':') else {
        return Err(invalid());
    };
    let numeric = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if (!start_s.is_empty() && !numeric(start_s)) || (!end_s.is_empty() && !numeric(end_s)) {
        return Err(invalid());
    }
    let start: usize = start_s.parse().unwrap_or(0);
    let end: usize = if end_s.is_empty() {
        data.len()
    } else {
        (end_s.parse::<usize>().unwrap_or(0) + 1).min(data.len())
    };
    if start == 0 {
        data.truncate(end);
        return Ok(data);
    }
    let header = data.first().cloned().unwrap_or_default();
    let begin = start.min(data.len());
    let mut out = vec![header];
    if end > begin {
        out.extend(data.drain(begin..end));
    }
    Ok(out)
}

/// Split header/body and coerce body cells (Words' reader pipeline).
fn split_table(row_count: usize, data: Vec<Vec<String>>) -> Result<Table, PoetError> {
    if row_count == 0 || data.is_empty() {
        return Ok(Table {
            columns: Vec::new(),
            rows: Vec::new(),
        });
    }
    let headers: Vec<String> = data[0].clone();
    let columns = dedup_headers(&headers);
    let width = columns.len();
    let rows = data[1..]
        .iter()
        .map(|row| {
            (0..width)
                .map(|ci| {
                    row.get(ci)
                        .map(|text| coerce_cell(text))
                        .unwrap_or(Cell::Null)
                })
                .collect()
        })
        .collect();
    Ok(Table { columns, rows })
}

/// Words' header dedup: `""` falls back to `col`; the k-th repeat of a base
/// gets `base_k`; generated names are not re-checked, so a literal `name_1`
/// can collide with a generated one (the later column then wins).
fn dedup_headers(headers: &[String]) -> Vec<String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    headers
        .iter()
        .map(|name| {
            let base = if name.is_empty() {
                "col"
            } else {
                name.as_str()
            };
            let count = counts.entry(base).or_insert(0);
            let final_name = if *count == 0 {
                base.to_string()
            } else {
                format!("{base}_{count}")
            };
            *count += 1;
            final_name
        })
        .collect()
}

/// Words' `_coerce_value`: strip; empty → null; case-insensitive true/false;
/// strict `-?\d+` int; strict `-?\d+\.\d+` float; else the stripped text.
pub fn coerce_cell(raw: &str) -> Cell {
    let text = raw.trim();
    if text.is_empty() {
        return Cell::Null;
    }
    if text.eq_ignore_ascii_case("true") {
        return Cell::Bool(true);
    }
    if text.eq_ignore_ascii_case("false") {
        return Cell::Bool(false);
    }
    if strict_int(text) {
        return text.parse::<i64>().map(Cell::Int).unwrap_or(Cell::Null);
    }
    if strict_float(text) {
        return text.parse::<f64>().map(Cell::Float).unwrap_or(Cell::Null);
    }
    Cell::Str(text.to_string())
}

fn strict_int(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit())
}

fn strict_float(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    match body.split_once('.') {
        Some((int_part, frac_part)) => {
            !int_part.is_empty()
                && !frac_part.is_empty()
                && strict_int(int_part)
                && frac_part.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

/// Build a polars `DataFrame` from a coerced table, resolving one dtype per
/// column (adr/0013): all-int → Int64, any float → Float64 (Words crashed on
/// int+float), bool/int mixes → Int64 (Python `bool`-is-`int` semantics),
/// bool → Boolean, text → String; a column mixing numbers/bools with text is
/// a `calculation_error` (Words leaked a raw TypeError here).
pub fn build_frame(table: &Table) -> Result<DataFrame, PoetError> {
    let width = table.columns.len();
    let mut named_columns: Vec<(String, Vec<Cell>)> = Vec::with_capacity(width);
    for (ci, name) in table.columns.iter().enumerate() {
        let values: Vec<Cell> = table
            .rows
            .iter()
            .map(|row| row.get(ci).cloned().unwrap_or(Cell::Null))
            .collect();
        match named_columns
            .iter()
            .position(|(existing, _)| existing == name)
        {
            Some(at) => named_columns[at].1 = values,
            None => named_columns.push((name.clone(), values)),
        }
    }
    let columns: Vec<Column> = named_columns
        .into_iter()
        .map(|(name, values)| series_of(&name, &values).map(Series::into_column))
        .collect::<Result<_, _>>()?;
    DataFrame::new(columns).map_err(polars_error)
}

fn polars_error(err: PolarsError) -> PoetError {
    PoetError::Calculation(err.to_string())
}

/// The resolved storage dtype of one column's cells.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ColType {
    Int,
    Float,
    Bool,
    Str,
}

fn resolve_type(values: &[Cell]) -> Result<ColType, PoetError> {
    let mut saw_float = false;
    let mut saw_int = false;
    let mut saw_bool = false;
    let mut saw_str = false;
    for cell in values {
        match cell {
            Cell::Null => {}
            Cell::Bool(_) => saw_bool = true,
            Cell::Int(_) => saw_int = true,
            Cell::Float(_) => saw_float = true,
            Cell::Str(_) => saw_str = true,
        }
    }
    let name_free = values.iter().all(|c| !matches!(c, Cell::Str(_)));
    if saw_str {
        if saw_int || saw_float || saw_bool {
            return Err(PoetError::Calculation(
                "column has mixed value types (numbers/booleans and text)".into(),
            ));
        }
        return Ok(ColType::Str);
    }
    let _ = name_free;
    if saw_float {
        return Ok(ColType::Float);
    }
    if saw_int {
        return Ok(ColType::Int);
    }
    if saw_bool {
        return Ok(ColType::Bool);
    }
    // All-null columns are inert for stats/aggregation, like Words' Null
    // dtype; store as strings so numerics stay unambiguous.
    Ok(ColType::Str)
}

fn series_of(name: &str, values: &[Cell]) -> Result<Series, PoetError> {
    match resolve_type(values)? {
        ColType::Int => Ok(Series::new(
            PlSmallStr::from(name),
            values
                .iter()
                .map(|c| match c {
                    Cell::Int(i) => Some(*i),
                    Cell::Bool(b) => Some(i64::from(*b)),
                    _ => None,
                })
                .collect::<Vec<Option<i64>>>(),
        )),
        ColType::Float => Ok(Series::new(
            PlSmallStr::from(name),
            values
                .iter()
                .map(|c| match c {
                    Cell::Float(f) => Some(*f),
                    Cell::Int(i) => Some(*i as f64),
                    Cell::Bool(b) => Some(f64::from(u8::from(*b))),
                    _ => None,
                })
                .collect::<Vec<Option<f64>>>(),
        )),
        ColType::Bool => Ok(Series::new(
            PlSmallStr::from(name),
            values
                .iter()
                .map(|c| match c {
                    Cell::Bool(b) => Some(*b),
                    _ => None,
                })
                .collect::<Vec<Option<bool>>>(),
        )),
        ColType::Str => Ok(Series::new(
            PlSmallStr::from(name),
            values
                .iter()
                .map(|c| match c {
                    Cell::Str(s) => Some(s.clone()),
                    Cell::Int(i) => Some(i.to_string()),
                    Cell::Float(f) => Some(f.to_string()),
                    Cell::Bool(b) => Some(if *b { "True".into() } else { "False".into() }),
                    Cell::Null => None,
                })
                .collect::<Vec<Option<String>>>(),
        )),
    }
}

/// One numeric column's statistics (Words' `StatsStrategy` key order and
/// float wrapping: everything but `count` is an f64, `std` null below two
/// samples, `mean/min/max/median` null with no data).
fn column_stats(series: &Series) -> Option<Value> {
    let cast = series.cast(&DataType::Float64).ok()?;
    let floats = cast.f64().ok()?;
    let count = series.len() - series.null_count();
    let mean = floats.mean();
    let min = floats.min();
    let max = floats.max();
    let std = if count > 1 { floats.std(1) } else { None };
    let median = floats.median();
    let opt_json = |v: Option<f64>| v.map(Value::from).unwrap_or(Value::Null);
    Some(serde_json::json!({
        "count": count,
        "mean": opt_json(mean),
        "min": opt_json(min),
        "max": opt_json(max),
        "std": opt_json(std),
        "median": opt_json(median),
    }))
}

fn is_numeric(series: &Series) -> bool {
    matches!(series.dtype(), DataType::Int64 | DataType::Float64)
}

/// `calc stats` statistics map (Words' `StatsStrategy`): numeric columns
/// only; with `column`, exactly that key (`{}` when not numeric/missing).
pub fn statistics(df: &DataFrame, column: Option<&str>) -> Value {
    let mut out = serde_json::Map::new();
    for name in df.get_column_names() {
        let name = name.to_string();
        if let Some(wanted) = column
            && wanted != name
        {
            continue;
        }
        let Ok(col) = df.column(&name) else {
            continue;
        };
        let series = col.as_materialized_series();
        // Full stats omit non-numeric columns; an explicit `--column` still
        // yields its key with `{}` (Words' `stats.get(column, {})`).
        if !is_numeric(series) && column.is_none() {
            continue;
        }
        let entry = if is_numeric(series) {
            column_stats(series).unwrap_or_else(|| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        out.insert(name, entry);
    }
    if let Some(wanted) = column
        && !out.contains_key(wanted)
    {
        out.insert(wanted.to_string(), serde_json::json!({}));
    }
    Value::Object(out)
}

/// `calc aggregate` — group-wise aggregation (Words' `AggregateStrategy`).
/// Group order is stable (first appearance); the result column keeps the
/// aggregation column's name.
pub fn aggregate(
    df: &DataFrame,
    group_by: &str,
    agg_column: &str,
    agg_func: &str,
) -> Result<DataFrame, PoetError> {
    let expr = match agg_func {
        "sum" => col(agg_column).sum(),
        "mean" => col(agg_column).mean(),
        "min" => col(agg_column).min(),
        "max" => col(agg_column).max(),
        "count" => col(agg_column).count(),
        "median" => col(agg_column).median(),
        other => {
            return Err(PoetError::Calculation(format!(
                "Unknown aggregation function: {other}"
            )));
        }
    }
    .alias(agg_column);
    df.clone()
        .lazy()
        .group_by_stable([col(group_by)])
        .agg([expr])
        .collect()
        .map_err(polars_error)
}

/// The comparison/`--value` coercion of Words' `calc_cmd.filter`: numeric
/// columns coerce the raw text (float when it contains `.`, else int), with
/// a string fallback that surfaces as a polars comparison error — matching
/// Words' behavior envelope-for-envelope.
fn filter_predicate(
    df: &DataFrame,
    column: &str,
    operator: &str,
    raw: &str,
) -> Result<Expr, PoetError> {
    let unknown = || PoetError::Calculation(format!("Unknown operator: {operator}"));
    let dtype = df.column(column).map_err(polars_error)?.dtype().clone();
    let coerced = match dtype {
        DataType::Int64 => raw.parse::<i64>().map(lit).unwrap_or_else(|_| lit(raw)),
        DataType::Float64 => {
            if raw.contains('.') {
                raw.parse::<f64>().map(lit).unwrap_or_else(|_| lit(raw))
            } else {
                raw.parse::<i64>().map(lit).unwrap_or_else(|_| lit(raw))
            }
        }
        _ => lit(raw),
    };
    match operator {
        "==" => Ok(col(column).eq(coerced)),
        "!=" => Ok(col(column).neq(coerced)),
        ">" => Ok(col(column).gt(coerced)),
        "<" => Ok(col(column).lt(coerced)),
        ">=" => Ok(col(column).gt_eq(coerced)),
        "<=" => Ok(col(column).lt_eq(coerced)),
        "contains" => Ok(col(column).str().contains_literal(coerced)),
        "startswith" => Ok(col(column).str().starts_with(coerced)),
        "endswith" => Ok(col(column).str().ends_with(coerced)),
        _ => Err(unknown()),
    }
}

/// `calc filter` — full rows matching the predicate (Words' `FilterStrategy`
/// output shape via `to_dicts`).
pub fn filter(
    df: &DataFrame,
    column: &str,
    operator: &str,
    raw_value: &str,
) -> Result<DataFrame, PoetError> {
    let predicate = filter_predicate(df, column, operator, raw_value)?;
    df.clone()
        .lazy()
        .filter(predicate)
        .collect()
        .map_err(polars_error)
}

/// The transform chain (Words' `DataAnalyzer.transform`): ops applied
/// sequentially over the frame.
pub fn transform(df: &mut DataFrame, operations: &[Value]) -> Result<(), PoetError> {
    for op in operations {
        let Value::Object(map) = op else {
            return Err(PoetError::Calculation(format!(
                "Unknown operation type: {op}"
            )));
        };
        let op_type = map.get("type").and_then(Value::as_str).unwrap_or_default();
        match op_type {
            "sort" => {
                let by = map.get("by").and_then(Value::as_str).unwrap_or_default();
                let descending = map
                    .get("descending")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                *df = df
                    .sort(
                        [by],
                        SortMultipleOptions::default()
                            .with_order_descending(descending)
                            .with_maintain_order(true),
                    )
                    .map_err(polars_error)?;
            }
            "rename" => {
                let Some(mapping) = map.get("mapping").and_then(Value::as_object) else {
                    continue;
                };
                for (from, to) in mapping {
                    let Some(to) = to.as_str() else {
                        continue;
                    };
                    df.rename(from.as_str(), PlSmallStr::from(to))
                        .map_err(polars_error)?;
                }
            }
            "drop" => {
                let names: Vec<String> = strings_of(map.get("columns"));
                for name in &names {
                    *df = df.drop(name.as_str()).map_err(polars_error)?;
                }
            }
            "select" => {
                let names: Vec<String> = strings_of(map.get("columns"));
                *df = df
                    .select(names.iter().map(String::as_str))
                    .map_err(polars_error)?;
            }
            "fill_null" => {
                let column = map.get("column").and_then(Value::as_str);
                let value = map.get("value").cloned().unwrap_or(Value::Null);
                fill_null(df, column, &value)?;
            }
            "add_column" => {
                let name = map.get("name").and_then(Value::as_str).unwrap_or_default();
                let expression = map
                    .get("expression")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                add_column(df, name, expression)?;
            }
            other => {
                return Err(PoetError::Calculation(format!(
                    "Unknown operation type: {other}"
                )));
            }
        }
    }
    Ok(())
}

fn strings_of(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// `fill_null` (Words: whole-frame when `column` is absent). Compatible
/// value/dtype pairs fill; mismatches are `calculation_error`s for a
/// targeted column and silently skipped for the whole frame — Words'
/// silent dtype cast is validated away (adr/0013 policy).
fn fill_null(df: &mut DataFrame, column: Option<&str>, value: &Value) -> Result<(), PoetError> {
    match column {
        Some(column) => {
            let series = df
                .column(column)
                .map_err(polars_error)?
                .as_materialized_series()
                .clone();
            let filled = fill_series(&series, value).ok_or_else(|| {
                PoetError::Calculation(format!(
                    "cannot fill column '{column}' ({}) with a value of that type",
                    series.dtype()
                ))
            })?;
            df.with_column(filled.into_column()).map_err(polars_error)?;
        }
        None => {
            let names: Vec<String> = df
                .get_column_names()
                .iter()
                .map(|name| name.to_string())
                .collect();
            for name in names {
                let Ok(series) = df.column(name.as_str()) else {
                    continue;
                };
                let series = series.as_materialized_series().clone();
                if let Some(filled) = fill_series(&series, value) {
                    df.with_column(filled.into_column()).map_err(polars_error)?;
                }
            }
        }
    }
    Ok(())
}

fn fill_series(series: &Series, value: &Value) -> Option<Series> {
    match (series.dtype(), value) {
        (DataType::Int64, Value::Number(n)) => {
            let v = n.as_i64()?;
            Some(
                series
                    .i64()
                    .ok()?
                    .fill_null_with_values(v)
                    .ok()?
                    .into_series(),
            )
        }
        (DataType::Int64, Value::Null) => Some(series.clone()),
        (DataType::Float64, Value::Number(n)) => {
            let v = n.as_f64()?;
            Some(
                series
                    .f64()
                    .ok()?
                    .fill_null_with_values(v)
                    .ok()?
                    .into_series(),
            )
        }
        (DataType::Boolean, Value::Bool(b)) => Some(
            series
                .bool()
                .ok()?
                .fill_null_with_values(*b)
                .ok()?
                .into_series(),
        ),
        (DataType::String, Value::String(s)) => {
            let strings: Vec<Option<String>> = series
                .str()
                .ok()?
                .into_iter()
                .map(|v| match v {
                    Some(existing) => Some(existing.to_string()),
                    None => Some(s.clone()),
                })
                .collect();
            Some(Series::new(series.name().clone(), strings))
        }
        _ => None,
    }
}

/// `add_column` — evaluate the rhai expression per row (adr/0013) and append
/// (or replace) the named column.
fn add_column(df: &mut DataFrame, name: &str, expression: &str) -> Result<(), PoetError> {
    if name.is_empty() {
        return Err(PoetError::Calculation(
            "add_column requires a 'name'".to_string(),
        ));
    }
    let names = df.get_column_names().to_vec();
    let mut rows = Vec::with_capacity(df.height());
    for ri in 0..df.height() {
        let mut row = rhai::Map::new();
        for column in &names {
            let series = df.column(column.as_str()).map_err(polars_error)?;
            let dynamic = any_to_dynamic(&series.get(ri).map_err(polars_error)?);
            row.insert(column.as_str().into(), dynamic);
        }
        rows.push(row);
    }
    let mut engine = Engine::new();
    let row_cell: std::rc::Rc<std::cell::RefCell<rhai::Map>> =
        std::rc::Rc::new(std::cell::RefCell::new(rhai::Map::new()));
    let lookup_cell = std::rc::Rc::clone(&row_cell);
    engine.register_fn(
        "col",
        move |name: &str| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let map = lookup_cell.borrow();
            match map.get(name) {
                Some(value) => Ok(value.clone()),
                None => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("column '{name}' not found").into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    );
    let mut cells = Vec::with_capacity(rows.len());
    for row in rows {
        row_cell.borrow_mut().clear();
        *row_cell.borrow_mut() = row;
        cells.push(eval_expression(&engine, expression)?);
    }
    let series = series_of(name, &cells)?;
    df.with_column(series.into_column()).map_err(polars_error)?;
    Ok(())
}

fn any_to_dynamic(value: &AnyValue) -> Dynamic {
    match value {
        AnyValue::Null => Dynamic::UNIT,
        AnyValue::Boolean(b) => Dynamic::from(*b),
        AnyValue::Int64(i) => Dynamic::from(*i),
        AnyValue::UInt32(u) => Dynamic::from(*u),
        AnyValue::Float64(f) => Dynamic::from(*f),
        AnyValue::String(s) => Dynamic::from(s.to_string()),
        other => Dynamic::from(other.to_string()),
    }
}

/// One rhai evaluation: `col("Name")` resolves against the registered
/// current-row map; a missing column surfaces as a `calculation_error`, as
/// do type mismatches (e.g. `"a" * 2`).
fn eval_expression(engine: &Engine, expression: &str) -> Result<Cell, PoetError> {
    let result: Dynamic = engine.eval(expression).map_err(expression_error)?;
    if result.is_unit() {
        return Ok(Cell::Null);
    }
    if result.is_bool() {
        return Ok(Cell::Bool(result.as_bool().unwrap_or(false)));
    }
    if result.is_int() {
        return Ok(Cell::Int(result.as_int().unwrap_or(0)));
    }
    if result.is_float() {
        return Ok(Cell::Float(result.as_float().unwrap_or(0.0)));
    }
    if let Ok(text) = result.into_string() {
        return Ok(Cell::Str(text));
    }
    Err(PoetError::Calculation(
        "expression result type is not supported (use number/string/bool)".into(),
    ))
}

fn expression_error(err: Box<rhai::EvalAltResult>) -> PoetError {
    PoetError::Calculation(format!("expression error: {err}"))
}

/// Materialize rows as JSON objects in column order (Words' `to_dicts`).
pub fn rows_to_values(df: &DataFrame) -> Result<Vec<Value>, PoetError> {
    let names: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let mut out = Vec::with_capacity(df.height());
    for ri in 0..df.height() {
        let mut row = serde_json::Map::new();
        for name in &names {
            let series = df.column(name.as_str()).map_err(polars_error)?;
            let any = series.get(ri).map_err(polars_error)?;
            row.insert(name.to_string(), any_to_json(&any));
        }
        out.push(Value::Object(row));
    }
    Ok(out)
}

fn any_to_json(value: &AnyValue) -> Value {
    match value {
        AnyValue::Null => Value::Null,
        AnyValue::Boolean(b) => Value::from(*b),
        AnyValue::Int64(i) => Value::from(*i),
        AnyValue::UInt32(u) => Value::from(*u),
        AnyValue::UInt64(u) => Value::from(*u),
        AnyValue::Float64(f) => Value::from(*f),
        AnyValue::String(s) => Value::from(*s),
        other => Value::from(other.to_string()),
    }
}

/// Timestamp helper reuse is deliberate: calc shares the meta module's
/// version constant vocabulary; this alias documents the linkage point.
#[allow(dead_code)]
fn meta_version_for_docs() -> &'static str {
    meta::META_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn table(columns: &[&str], rows: &[&[Cell]]) -> Table {
        Table {
            columns: columns.iter().map(|s| s.to_string()).collect(),
            rows: rows.iter().map(|r| r.to_vec()).collect(),
        }
    }

    fn cell(v: serde_json::Value) -> Cell {
        match v {
            serde_json::Value::Bool(b) => Cell::Bool(b),
            serde_json::Value::Number(n) if n.is_i64() => Cell::Int(n.as_i64().unwrap()),
            serde_json::Value::Number(n) => Cell::Float(n.as_f64().unwrap()),
            serde_json::Value::String(s) => coerce_cell(&s),
            serde_json::Value::Null => Cell::Null,
            other => Cell::Str(other.to_string()),
        }
    }

    #[test]
    fn coerce_cell_matches_words_table() {
        let cases = [
            ("15000", Cell::Int(15000)),
            (" 42 ", Cell::Int(42)),
            ("2.75", Cell::Float(2.75)),
            ("-7", Cell::Int(-7)),
            ("TRUE", Cell::Bool(true)),
            ("false", Cell::Bool(false)),
            ("yes", Cell::Str("yes".into())),
            ("3,000", Cell::Str("3,000".into())),
            ("0", Cell::Int(0)),
            ("", Cell::Null),
            ("   ", Cell::Null),
            (".5", Cell::Str(".5".into())),
            ("5.", Cell::Str("5.".into())),
            ("1e5", Cell::Str("1e5".into())),
            ("+5", Cell::Str("+5".into())),
            ("007", Cell::Int(7)),
        ];
        for (raw, expected) in cases {
            assert_eq!(coerce_cell(raw), expected, "raw: {raw}");
        }
    }

    #[test]
    fn dedup_headers_follows_words_quirks() {
        let dedup = |names: &[&str]| {
            dedup_headers(&names.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };
        assert_eq!(
            dedup(&["name", "name", "name"]),
            ["name", "name_1", "name_2"]
        );
        assert_eq!(dedup(&["", "x", ""]), ["col", "x", "col_1"]);
        assert_eq!(
            dedup(&["name", "name_1", "name"]),
            ["name", "name_1", "name_1"]
        );
        assert_eq!(dedup(&["A", "A", "A_1", "A"]), ["A", "A_1", "A_1", "A_2"]);
    }

    #[test]
    fn range_is_one_based_inclusive_and_keeps_header() {
        let data: Vec<Vec<String>> = (0..5)
            .map(|i| vec![format!("r{i}"), format!("v{i}")])
            .collect();
        let apply = |spec: &str| apply_range(data.clone(), Some(spec)).unwrap();
        assert_eq!(apply("1:2").len(), 3, "header + data rows 1,2");
        assert_eq!(apply("2:3")[1], vec!["r2".to_string(), "v2".to_string()]);
        assert_eq!(apply("0:1").len(), 2);
        assert_eq!(apply(":2").len(), 3);
        assert_eq!(apply("1:").len(), 5);
        assert_eq!(apply(":").len(), 5);
        assert_eq!(apply("3:100").len(), 3, "clamped: header + rows 3,4");
        let header_only = apply("4:3");
        assert_eq!(header_only.len(), 1, "start > end keeps only header");
        assert!(apply_range(data.clone(), Some("abc")).is_err());
        assert!(apply_range(data.clone(), Some("1-2")).is_err());
    }

    #[test]
    fn build_frame_resolves_dtypes_and_rejects_mixed() {
        let numeric = build_frame(&table(
            &["A", "B"],
            &[
                &[cell(json!(1)), cell(json!("x"))],
                &[cell(json!(2)), cell(json!("y"))],
            ],
        ))
        .unwrap();
        assert_eq!(numeric.column("A").unwrap().dtype(), &DataType::Int64);

        let upcast =
            build_frame(&table(&["A"], &[&[cell(json!(1))], &[cell(json!(2.5))]])).unwrap();
        assert_eq!(upcast.column("A").unwrap().dtype(), &DataType::Float64);

        let bools = build_frame(&table(
            &["A"],
            &[&[cell(json!(true))], &[cell(json!(false))]],
        ))
        .unwrap();
        assert_eq!(bools.column("A").unwrap().dtype(), &DataType::Boolean);

        let mixed = build_frame(&table(&["A"], &[&[cell(json!(1))], &[cell(json!("abc"))]]));
        assert!(matches!(mixed, Err(PoetError::Calculation(_))));

        let int_bool =
            build_frame(&table(&["A"], &[&[cell(json!(true))], &[cell(json!(1))]])).unwrap();
        assert_eq!(int_bool.column("A").unwrap().dtype(), &DataType::Int64);

        let all_null = build_frame(&table(&["A"], &[&[Cell::Null]])).unwrap();
        assert_eq!(all_null.column("A").unwrap().dtype(), &DataType::String);
    }

    #[test]
    fn duplicate_header_names_collapse_last_wins() {
        let t = Table {
            columns: vec!["A".into(), "A_1".into(), "A_1".into()],
            rows: vec![vec![Cell::Int(1), Cell::Int(2), Cell::Int(3)]],
        };
        let df = build_frame(&t).unwrap();
        assert_eq!(df.width(), 2, "collapsed like the Words dict");
        let a1 = df.column("A_1").unwrap().as_materialized_series();
        assert_eq!(a1.get(0).unwrap(), AnyValue::Int64(3), "later column wins");
    }

    #[test]
    fn statistics_match_words_shapes_and_floats() {
        let df = build_frame(&table(
            &["Q1", "Name"],
            &[
                &[cell(json!(15000)), cell(json!("a"))],
                &[cell(json!(12000)), cell(json!("b"))],
                &[cell(json!(27000)), cell(json!("c"))],
            ],
        ))
        .unwrap();
        let stats = statistics(&df, None);
        let q1 = &stats["Q1"];
        assert_eq!(q1["count"], 3);
        assert_eq!(q1["mean"], serde_json::json!(18000.0_f64));
        assert_eq!(q1["min"], serde_json::json!(12000.0_f64), "float-wrapped");
        assert_eq!(q1["max"], serde_json::json!(27000.0_f64));
        assert_eq!(q1["median"], serde_json::json!(15000.0_f64));
        assert_eq!(q1["std"], serde_json::json!(7937.253933193771_f64));
        assert!(stats.get("Name").is_none(), "non-numeric omitted");

        assert_eq!(statistics(&df, Some("Q1"))["Q1"]["count"], 3);
        assert_eq!(
            statistics(&df, Some("Nope")),
            serde_json::json!({"Nope": {}})
        );
    }

    #[test]
    fn aggregate_sums_and_reports_unknown_functions() {
        let df = build_frame(&table(
            &["Product", "Q1"],
            &[
                &[cell(json!("A")), cell(json!(10))],
                &[cell(json!("B")), cell(json!(20))],
                &[cell(json!("A")), cell(json!(40))],
            ],
        ))
        .unwrap();
        let out = aggregate(&df, "Product", "Q1", "sum").unwrap();
        let rows = rows_to_values(&out).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["Product"], "A");
        assert_eq!(rows[0]["Q1"], 50, "int sum stays int, stable group order");
        assert_eq!(rows[1]["Q1"], 20);

        let mean = aggregate(&df, "Product", "Q1", "mean").unwrap();
        let rows = rows_to_values(&mean).unwrap();
        assert_eq!(rows[0]["Q1"], serde_json::json!(25.0_f64));

        let err = aggregate(&df, "Product", "Q1", "nope").unwrap_err();
        assert!(
            err.to_string()
                .contains("Unknown aggregation function: nope")
        );
    }

    #[test]
    fn filter_operators_match_words_semantics() {
        let df = build_frame(&table(
            &["Product", "Q1"],
            &[
                &[cell(json!("Widget A")), cell(json!(10))],
                &[cell(json!("widget b")), cell(json!(20))],
                &[cell(json!("Widget A2")), cell(json!(30))],
            ],
        ))
        .unwrap();
        let run = |column: &str, op: &str, value: &str| {
            let out = filter(&df, column, op, value).unwrap();
            rows_to_values(&out).unwrap().len()
        };
        assert_eq!(run("Q1", ">", "15"), 2);
        assert_eq!(run("Q1", ">=", "20"), 2);
        assert_eq!(run("Q1", "<", "20"), 1);
        assert_eq!(run("Q1", "<=", "20"), 2);
        assert_eq!(run("Q1", "==", "20"), 1);
        assert_eq!(run("Q1", "!=", "20"), 2);
        assert_eq!(run("Product", "contains", "Widget"), 2, "case-sensitive");
        assert_eq!(run("Product", "startswith", "Widget"), 2);
        assert_eq!(run("Product", "endswith", "b"), 1);
        assert_eq!(
            rows_to_values(&filter(&df, "Q1", ">", "999").unwrap())
                .unwrap()
                .len(),
            0,
            "empty result is valid"
        );
        let err = filter(&df, "Product", "~", "x").unwrap_err();
        assert!(err.to_string().contains("Unknown operator: ~"));
    }

    #[test]
    fn transform_ops_match_words_and_unknown_rejected() {
        let mut df = build_frame(&table(
            &["A", "B"],
            &[
                &[cell(json!(3)), cell(json!("x"))],
                &[cell(json!(1)), cell(json!("y"))],
                &[cell(json!(2)), cell(json!("z"))],
            ],
        ))
        .unwrap();
        transform(
            &mut df,
            &[json!({"type": "sort", "by": "A", "descending": false})],
        )
        .unwrap();
        assert_eq!(df.column("A").unwrap().get(0).unwrap(), AnyValue::Int64(1));

        transform(
            &mut df,
            &[json!({"type": "sort", "by": "A", "descending": true})],
        )
        .unwrap();
        assert_eq!(df.column("A").unwrap().get(0).unwrap(), AnyValue::Int64(3));

        transform(
            &mut df,
            &[json!({"type": "rename", "mapping": {"A": "Alpha"}})],
        )
        .unwrap();
        assert!(df.column("Alpha").is_ok());

        transform(&mut df, &[json!({"type": "select", "columns": ["Alpha"]})]).unwrap();
        assert_eq!(df.width(), 1);

        transform(&mut df, &[json!({"type": "drop", "columns": ["Alpha"]})]).unwrap();
        assert_eq!(df.width(), 0);

        let err = transform(&mut df, &[json!({"type": "nope"})]);
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("Unknown operation type: nope")
        );
    }

    #[test]
    fn fill_null_fills_compatible_and_skips_elsewhere() {
        let mut df = build_frame(&table(
            &["A", "B"],
            &[
                &[cell(json!(1)), cell(json!("x"))],
                &[Cell::Null, Cell::Null],
                &[cell(json!(3)), cell(json!("z"))],
            ],
        ))
        .unwrap();
        transform(&mut df, &[json!({"type": "fill_null", "value": 0})]).unwrap();
        let rows = rows_to_values(&df).unwrap();
        assert_eq!(rows[1]["A"], 0, "numeric null filled");
        assert_eq!(rows[1]["B"], Value::Null, "string null left alone");

        let mut df2 = df.clone();
        transform(
            &mut df2,
            &[json!({"type": "fill_null", "column": "B", "value": "filler"})],
        )
        .unwrap();
        let rows = rows_to_values(&df2).unwrap();
        assert_eq!(rows[1]["B"], "filler");

        let mut df3 = df.clone();
        let err = transform(
            &mut df3,
            &[json!({"type": "fill_null", "column": "A", "value": "x"})],
        );
        assert!(err.is_err(), "string into int column is an error");
    }

    #[test]
    fn add_column_evaluates_rhai_expressions() {
        let mut df = build_frame(&table(
            &["Q1", "Name", "Flag"],
            &[
                &[cell(json!(10)), cell(json!("a")), cell(json!(true))],
                &[cell(json!(20)), cell(json!("b")), cell(json!(false))],
                &[cell(json!(30)), cell(json!("c")), cell(json!(true))],
            ],
        ))
        .unwrap();

        transform(
            &mut df,
            &[json!({"type": "add_column", "name": "D", "expression": "col(\"Q1\") * 2"})],
        )
        .unwrap();
        let d = df.column("D").unwrap().as_materialized_series();
        assert_eq!(d.dtype(), &DataType::Int64, "integral results stay ints");
        assert_eq!(d.get(2).unwrap(), AnyValue::Int64(60));

        transform(
            &mut df,
            &[json!({"type": "add_column", "name": "S", "expression": "col(\"Name\") + \"!\""})],
        )
        .unwrap();
        let s = df.column("S").unwrap().as_materialized_series();
        assert_eq!(s.get(0).unwrap(), AnyValue::String("a!"));

        transform(
            &mut df,
            &[json!({"type": "add_column", "name": "L", "expression": "if col(\"Flag\") { \"on\" } else { \"off\" }"})],
        )
        .unwrap();
        let l = df.column("L").unwrap().as_materialized_series();
        assert_eq!(l.get(1).unwrap(), AnyValue::String("off"));

        let mut df2 = df.clone();
        let err = transform(
            &mut df2,
            &[json!({"type": "add_column", "name": "X", "expression": "col(\"Nope\")"})],
        );
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("column 'Nope' not found")
        );

        let mut df3 = df.clone();
        let err = transform(
            &mut df3,
            &[json!({"type": "add_column", "name": "X", "expression": "\"a\" * 2"})],
        );
        assert!(err.is_err(), "type mismatch is a calculation error");
    }
}
