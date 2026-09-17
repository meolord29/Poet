//! Auto-annotation: column-type inference and the operation hooks that
//! append history entries and inferred table schemas (Words'
//! `meta/type_inference.py` + `meta/annotator.py`, adr/0012).
//!
//! Inference precedence (Words' `infer_column_type`): empty → `string`; all
//! JSON booleans → `boolean`; all numbers → `number`; all-currency →
//! `currency`; all-percentage → `percentage`; all-date (start-anchored
//! patterns, no calendar validation) → `date`; all-numeric-string (commas
//! stripped) → `number`; else `string`. Mixed columns never agree, so they
//! collapse to `string` — there is no majority vote and no sampling cap.
//! One Rust-side drift: Python's `float()` accepts `"1_000"`, Rust's parser
//! does not, so that spelling infers as `string` here.

use serde_json::Value;

use crate::core::document::DocumentManager;
use crate::core::error::PoetError;
use crate::core::meta::{ParagraphAnnotation, TableColumn, TableSchema, engine};

const CURRENCY_SYMBOLS: &[(char, &str)] = &[
    ('$', "USD"),
    ('€', "EUR"),
    ('£', "GBP"),
    ('¥', "JPY"),
    ('₹', "INR"),
];

/// Infer a column type from raw cell values (Words' `infer_column_type`).
pub fn infer_column_type(values: &[Value]) -> String {
    let non_empty: Vec<&Value> = values
        .iter()
        .filter(|v| !v.is_null() && v.as_str() != Some(""))
        .collect();
    if non_empty.is_empty() {
        return "string".to_string();
    }
    if non_empty.iter().all(|v| v.is_boolean()) {
        return "boolean".to_string();
    }
    // Python quirk: `bool` is an `int` subclass, so a mixed bool/number
    // column passes Words' all-numeric check (after the all-bool check).
    if non_empty.iter().all(|v| v.is_number() || v.is_boolean()) {
        return "number".to_string();
    }
    let strings: Vec<String> = non_empty.iter().map(|v| display_string(v)).collect();
    if strings.iter().all(|s| is_currency(s)) {
        return "currency".to_string();
    }
    if strings.iter().all(|s| is_percentage(s)) {
        return "percentage".to_string();
    }
    if strings.iter().all(|s| is_date(s)) {
        return "date".to_string();
    }
    if strings.iter().all(|s| is_numeric_string(s)) {
        return "number".to_string();
    }
    "string".to_string()
}

/// Currency unit of the first value carrying a symbol (Words'
/// `extract_currency_unit`), `""` when none.
pub fn extract_currency_unit(values: &[Value]) -> String {
    for value in values {
        let Some(text) = value.as_str() else {
            continue;
        };
        for (symbol, code) in CURRENCY_SYMBOLS {
            if text.contains(*symbol) {
                return (*code).to_string();
            }
        }
    }
    String::new()
}

/// Python `str(v)` rendering used by the string predicates.
fn display_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        other => other.to_string(),
    }
}

fn is_currency(value: &str) -> bool {
    CURRENCY_SYMBOLS
        .iter()
        .any(|(symbol, _)| value.contains(*symbol))
}

fn is_percentage(value: &str) -> bool {
    value.trim().ends_with('%')
}

fn is_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() < 10 {
        return false;
    }
    let digits = |range: &[u8]| range.iter().all(u8::is_ascii_digit);
    // \d{4}-\d{2}-\d{2}
    (digits(&b[0..4]) && b[4] == b'-' && digits(&b[5..7]) && b[7] == b'-' && digits(&b[8..10]))
        // \d{2}/\d{2}/\d{4}
        || (digits(&b[0..2]) && b[2] == b'/' && digits(&b[3..5]) && b[5] == b'/' && digits(&b[6..10]))
        // \d{2}-\d{2}-\d{4}
        || (digits(&b[0..2]) && b[2] == b'-' && digits(&b[3..5]) && b[5] == b'-' && digits(&b[6..10]))
}

fn is_numeric_string(value: &str) -> bool {
    value.replace(',', "").parse::<f64>().is_ok()
}

/// First `n` characters (Words slices Python strings; characters, not bytes).
fn truncate(text: &str, n: usize) -> String {
    text.chars().take(n).collect()
}

/// Python `repr()` of a string for the annotation descriptions (single-quote
/// style with the common escapes; control characters are kept verbatim).
fn py_repr(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('\'');
    out
}

fn history(mgr: &mut DocumentManager, command: &str, target: &str, summary: &str) {
    let _ = engine::add_history_entry(mgr, command, target, summary);
}

/// Words' history-target rendering: the id when given, else
/// `index:N` — with `None` rendering literally as `index:None`
/// (the Python f-string artifact Words records).
pub fn target_or_index(id: Option<&str>, index: Option<usize>) -> String {
    match id {
        Some(id) => id.to_string(),
        None => match index {
            Some(i) => format!("index:{i}"),
            None => "index:None".to_string(),
        },
    }
}

/// `paragraph add` / `insert` hook: upsert the annotation and append history.
pub fn on_paragraph_add(
    mgr: &mut DocumentManager,
    para_id: &str,
    text: &str,
    style: Option<&str>,
) -> Result<(), PoetError> {
    engine::set_paragraph_annotation(
        mgr,
        ParagraphAnnotation {
            id: para_id.to_string(),
            kind: "paragraph".to_string(),
            description: format!(
                "Paragraph created with text: {}",
                py_repr(&truncate(text, 80))
            ),
            notes: style.unwrap_or_default().to_string(),
        },
    )?;
    history(
        mgr,
        "paragraph add",
        para_id,
        &format!("Added paragraph '{}'", truncate(text, 40)),
    );
    Ok(())
}

/// `paragraph update` hook.
pub fn on_paragraph_set(
    mgr: &mut DocumentManager,
    para_id: &str,
    text: &str,
) -> Result<(), PoetError> {
    engine::set_paragraph_annotation(
        mgr,
        ParagraphAnnotation {
            id: para_id.to_string(),
            kind: "paragraph".to_string(),
            description: format!("Text updated to: {}", py_repr(&truncate(text, 80))),
            notes: String::new(),
        },
    )?;
    history(mgr, "paragraph set", para_id, "Updated paragraph text");
    Ok(())
}

/// `heading add` hook.
pub fn on_heading_add(
    mgr: &mut DocumentManager,
    para_id: &str,
    text: &str,
    level: u8,
) -> Result<(), PoetError> {
    engine::set_paragraph_annotation(
        mgr,
        ParagraphAnnotation {
            id: para_id.to_string(),
            kind: "heading".to_string(),
            description: format!("Heading level {level}: {}", py_repr(text)),
            notes: String::new(),
        },
    )?;
    history(
        mgr,
        "heading add",
        para_id,
        &format!("Added H{level} heading '{}'", truncate(text, 40)),
    );
    Ok(())
}

/// `list add` / `add-item` hook.
pub fn on_list_add(
    mgr: &mut DocumentManager,
    para_id: &str,
    text: &str,
    ordered: bool,
    level: u8,
) -> Result<(), PoetError> {
    let kind = if ordered {
        "ordered list"
    } else {
        "bullet list"
    };
    engine::set_paragraph_annotation(
        mgr,
        ParagraphAnnotation {
            id: para_id.to_string(),
            kind: kind.to_string(),
            description: format!(
                "{kind} (level {level}) item: {}",
                py_repr(&truncate(text, 80))
            ),
            notes: String::new(),
        },
    )?;
    history(mgr, "list add", para_id, &format!("Added {kind} item"));
    Ok(())
}

/// `run add` / `run emphasize` hook: history only, no annotation.
pub fn on_run_add(mgr: &mut DocumentManager, target: &str, text: &str) -> Result<(), PoetError> {
    history(
        mgr,
        "run add",
        target,
        &format!("Added run '{}'", truncate(text, 40)),
    );
    Ok(())
}

/// `table add` hook: seed an empty schema and append history.
pub fn on_table_add(
    mgr: &mut DocumentManager,
    table_id: &str,
    rows: usize,
    cols: usize,
) -> Result<(), PoetError> {
    engine::set_table_schema(
        mgr,
        TableSchema {
            id: table_id.to_string(),
            name: String::new(),
            description: format!("{rows}x{cols} table"),
            columns: Vec::new(),
            header_row: true,
        },
    )?;
    history(
        mgr,
        "table add",
        table_id,
        &format!("Added {rows}x{cols} table"),
    );
    Ok(())
}

/// `table set-range --header` hook: rebuild the schema's columns from the
/// written values, merging onto an existing schema (its `name`/`description`
/// survive, like Words' `on_table_data`).
pub fn on_table_data(
    mgr: &mut DocumentManager,
    table_id: &str,
    data: &[Vec<Value>],
    header_row: bool,
) -> Result<(), PoetError> {
    if data.is_empty() {
        return Ok(());
    }
    let mut schema = engine::get_table_schema(mgr, table_id)?.unwrap_or(TableSchema {
        id: table_id.to_string(),
        name: String::new(),
        description: String::new(),
        columns: Vec::new(),
        header_row,
    });
    let num_cols = data[0].len();
    let body = if header_row { &data[1..] } else { data };
    let mut columns = Vec::with_capacity(num_cols);
    for ci in 0..num_cols {
        let header = if header_row {
            data[0]
                .get(ci)
                .map(display_string)
                .unwrap_or_else(|| format!("col{ci}"))
        } else {
            format!("col{ci}")
        };
        let col_values: Vec<Value> = body.iter().filter_map(|row| row.get(ci).cloned()).collect();
        columns.push(TableColumn {
            name: header,
            data_type: infer_column_type(&col_values),
            unit: String::new(),
            description: String::new(),
        });
    }
    schema.columns = columns;
    schema.header_row = header_row;
    engine::set_table_schema(mgr, schema)?;
    history(
        mgr,
        "table data",
        table_id,
        &format!("Wrote {} data rows", body.len()),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infer_column_type_matches_words_precedence() {
        let cases: Vec<(Vec<Value>, &str)> = vec![
            (vec![json!(1), json!(2), json!(3)], "number"),
            (vec![json!("a"), json!("b")], "string"),
            (vec![json!(true), json!(false)], "boolean"),
            (vec![json!("$5"), json!("$10")], "currency"),
            (vec![json!("5%"), json!("10%")], "percentage"),
            (vec![], "string"),
            (vec![json!("1"), json!("2")], "number"),
            (vec![json!("2024-01-15"), json!("2024-02-20")], "date"),
            (vec![json!("15/01/2024"), json!("20/02/2024")], "date"),
            (vec![json!("15-01-2024"), json!("20-02-2024")], "date"),
            (vec![json!("1,000"), json!("2,500")], "number"),
            (vec![json!(1), json!("6")], "number"),
            (vec![json!(1), json!("a")], "string"),
            (vec![json!(null), json!(null)], "string"),
            (vec![json!(""), json!("")], "string"),
            (vec![json!(" "), json!("2")], "string"),
            (vec![json!("9999-99-99")], "date"),
            (vec![json!("1/2/2024")], "string"),
            (vec![json!("2024-01-15T10:00")], "date"),
            (vec![json!(true), json!(1)], "number"),
            (vec![json!("$5"), json!("10%")], "string"),
        ];
        for (values, expected) in cases {
            assert_eq!(infer_column_type(&values), expected, "values: {values:?}");
        }
    }

    #[test]
    fn currency_units_extract_in_symbol_order() {
        assert_eq!(extract_currency_unit(&[json!("$5")]), "USD");
        assert_eq!(extract_currency_unit(&[json!("€5")]), "EUR");
        assert_eq!(extract_currency_unit(&[json!("£5")]), "GBP");
        assert_eq!(extract_currency_unit(&[json!("¥5")]), "JPY");
        assert_eq!(extract_currency_unit(&[json!("₹5")]), "INR");
        assert_eq!(extract_currency_unit(&[json!("x"), json!("$5")]), "USD");
        assert_eq!(extract_currency_unit(&[json!("x")]), "");
        assert_eq!(extract_currency_unit(&[]), "");
    }

    #[test]
    fn py_repr_uses_python_single_quote_style() {
        assert_eq!(py_repr("hello"), "'hello'");
        assert_eq!(py_repr("it's"), "'it\\'s'");
        assert_eq!(py_repr("a\nb"), "'a\\nb'");
    }

    #[test]
    fn truncate_counts_characters_not_bytes() {
        assert_eq!(truncate("héllo", 3), "hél");
        assert_eq!(truncate("short", 40), "short");
    }
}
