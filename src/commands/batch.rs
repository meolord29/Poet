//! Batch processing — ported from Words' `commands/batch.py` + the
//! `docs/reference/batch-scripts.md` format: a JSON **array of flat command
//! objects** `{"cmd", "action", ...}` (kebab-case actions). Dispatch calls
//! the same in-process command functions on the shared `Ctx` — the analog of
//! Words' handler map over command-class instances sharing one manager — so
//! `document new/open` set the path later `save` entries reuse, and stop on
//! first error. Batch never autosaves and never touches the session file;
//! saves happen only through explicit `document save` entries.
//!
//! Error shape parity: command-body failures surface as plain error
//! envelopes (no prefix, like Words' returned `format_error` results);
//! batch-side problems — unknown category/action, missing required key —
//! are prefixed `Command {i} ({cmd} {action}): ...` (Words' raised
//! exceptions). `commands_executed` counts attempts including the failure;
//! the outer envelope stays `ok` unless the script itself is missing or
//! invalid JSON.

use std::path::Path;

use clap::Args;
use serde_json::Value;

type Obj = serde_json::Map<String, Value>;

use crate::core::error::PoetError;
use crate::core::output::{self, Data};

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

/// The three built-in templates, byte-for-byte the JSON payloads of Words'
/// `TEMPLATES` (compact source form; `template` writes them with the
/// `json.dump(indent=2)`-equivalent pretty serializer, no trailing newline).
const TEMPLATES: &[(&str, &str)] = &[
    (
        "basic",
        r#"[
  {"cmd": "document", "action": "new", "path": "output.docx"},
  {"cmd": "heading", "action": "add", "text": "My Document", "level": 1},
  {"cmd": "paragraph", "action": "add", "text": "Hello World."},
  {"cmd": "document", "action": "save", "path": "output.docx"}
]"#,
    ),
    (
        "report",
        r#"[
  {"cmd": "document", "action": "new", "path": "report.docx"},
  {"cmd": "heading", "action": "add", "text": "Quarterly Report", "level": 1},
  {"cmd": "heading", "action": "add", "text": "Summary", "level": 2},
  {"cmd": "paragraph", "action": "add", "text": "This report summarizes Q4 performance."},
  {"cmd": "paragraph", "action": "add", "text": "Revenue grew 15% year over year."},
  {"cmd": "document", "action": "save", "path": "report.docx"}
]"#,
    ),
    (
        "data_table",
        r#"[
  {"cmd": "document", "action": "new", "path": "data.docx"},
  {"cmd": "heading", "action": "add", "text": "Sales Data", "level": 1},
  {"cmd": "table", "action": "add", "rows": 4, "cols": 3, "id": "sales"},
  {"cmd": "table", "action": "set-range", "id": "sales", "header": true, "values": [
    ["Product", "Q1", "Q2"],
    ["Widget A", 15000, 18000],
    ["Widget B", 12000, 14500],
    ["Total", 27000, 32500]
  ]},
  {"cmd": "document", "action": "save", "path": "data.docx"}
]"#,
    ),
];

/// Words' working-path tracking across `document new/open/save` entries.
#[derive(Debug, Default)]
struct DocumentPath(Option<String>);

impl DocumentPath {
    fn set(&mut self, path: impl Into<String>) {
        self.0 = Some(path.into());
    }

    /// Words' save fallback: `cmd.path || remembered || "output.docx"`.
    fn or_default(&self, path: Option<String>) -> String {
        path.or_else(|| self.0.clone())
            .unwrap_or_else(|| "output.docx".into())
    }
}

/// `batch run`.
pub fn run(ctx: &crate::core::Ctx, args: &RunArgs) -> Result<Data, PoetError> {
    let path = Path::new(&args.script);
    if !path.exists() {
        return Err(PoetError::NotFound(format!(
            "Script not found: {}",
            args.script
        )));
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|e| PoetError::File(format!("cannot read {}: {e}", args.script)))?;
    let script: Value = serde_json::from_str(&raw)
        .map_err(|e| PoetError::Validation(format!("Invalid batch script JSON: {e}")))?;
    let Value::Array(commands) = script else {
        return Err(PoetError::Validation(
            "Batch script must be a JSON array of command objects".into(),
        ));
    };
    let mut tracker = DocumentPath::default();
    let mut results: Vec<Value> = Vec::with_capacity(commands.len());
    for (index, cmd) in commands.iter().enumerate() {
        match execute(ctx, &mut tracker, index, cmd) {
            Ok(data) => results.push(envelope_value(&output::ok(&data))),
            Err(err) => {
                results.push(envelope_value(&output::error(&err)));
                break;
            }
        }
    }
    Ok(Data::BatchRun {
        script: args.script.clone(),
        commands_executed: results.len(),
        results,
        message: "Batch script executed".into(),
    })
}

/// `batch template`.
pub fn template(_ctx: &crate::core::Ctx, args: &TemplateArgs) -> Result<Data, PoetError> {
    let Some((_, body)) = TEMPLATES.iter().find(|(name, _)| *name == args.name) else {
        return Err(PoetError::Validation(format!(
            "Template '{}' not found. Available: basic, report, data_table",
            args.name
        )));
    };
    let script: Value = serde_json::from_str(body)
        .map_err(|e| PoetError::Internal(format!("built-in template is invalid JSON: {e}")))?;
    let count = script.as_array().map(Vec::len).unwrap_or(0);
    let rendered = serde_json::to_string_pretty(&script)
        .map_err(|e| PoetError::Internal(format!("cannot serialize template: {e}")))?;
    std::fs::write(&args.output, rendered)
        .map_err(|e| PoetError::Validation(format!("Failed to write template: {e}")))?;
    Ok(Data::BatchTemplate {
        template: args.name.clone(),
        output: args.output.clone(),
        commands: count,
        message: "Template generated".into(),
    })
}

fn envelope_value(envelope: &output::Envelope) -> Value {
    serde_json::to_value(envelope).unwrap_or(Value::Null)
}

fn raised(index: usize, cmd: &str, action: &str, message: String) -> PoetError {
    PoetError::Validation(format!("Command {index} ({cmd} {action}): {message}"))
}

// Flat-key accessors with Words' `.get()` defaults; a present-but-wrong-typed
// key is a raised error (Words failed inside the command instead).

fn key_str(cmd: &Obj, key: &str) -> Option<String> {
    cmd.get(key).and_then(Value::as_str).map(str::to_string)
}

fn key_bool(cmd: &Obj, key: &str, default: bool) -> bool {
    cmd.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn key_usize(cmd: &Obj, key: &str) -> Result<Option<usize>, PoetError> {
    match cmd.get(key) {
        None => Ok(None),
        Some(Value::Number(n)) => n.as_u64().map(|v| Some(v as usize)).ok_or_else(|| {
            PoetError::Validation(format!("'{key}' must be a non-negative integer"))
        }),
        Some(_) => Err(PoetError::Validation(format!(
            "'{key}' must be a non-negative integer"
        ))),
    }
}

fn key_f64(cmd: &Obj, key: &str) -> Result<Option<f64>, PoetError> {
    match cmd.get(key) {
        None => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_f64()),
        Some(_) => Err(PoetError::Validation(format!("'{key}' must be a number"))),
    }
}

fn required(cmd: &Obj, key: &str) -> Result<String, String> {
    key_str(cmd, key).ok_or_else(|| format!("'{key}'"))
}

fn table_address(cmd: &Obj) -> crate::commands::table::AddressArgs {
    crate::commands::table::AddressArgs {
        id: key_str(cmd, "id"),
        index: key_usize(cmd, "index").ok().flatten(),
    }
}

fn para_address(cmd: &Obj) -> crate::commands::paragraph::AddressArgs {
    crate::commands::paragraph::AddressArgs {
        id: key_str(cmd, "id"),
        index: key_usize(cmd, "index").ok().flatten(),
    }
}

fn run_address(cmd: &Obj) -> crate::commands::run::AddressArgs {
    crate::commands::run::AddressArgs {
        id: key_str(cmd, "id"),
        index: key_usize(cmd, "index").ok().flatten(),
    }
}

fn format_flags(cmd: &Obj) -> Result<crate::commands::run::FormatFlags, String> {
    Ok(crate::commands::run::FormatFlags {
        bold: key_bool(cmd, "bold", false),
        no_bold: false,
        italic: key_bool(cmd, "italic", false),
        no_italic: false,
        underline: key_bool(cmd, "underline", false),
        no_underline: false,
        font: key_str(cmd, "font"),
        size: key_f64(cmd, "size").map_err(|e| e.to_string())?,
        color: key_str(cmd, "color"),
    })
}

fn json_string(cmd: &Obj, key: &str, default: &str) -> Result<String, String> {
    match cmd.get(key) {
        None => Ok(default.to_string()),
        Some(value) => {
            serde_json::to_string(value).map_err(|_| format!("'{key}' must be a JSON value"))
        }
    }
}

fn execute(
    ctx: &crate::core::Ctx,
    tracker: &mut DocumentPath,
    index: usize,
    cmd: &Value,
) -> Result<Data, PoetError> {
    let Value::Object(map) = cmd else {
        return Err(raised(
            index,
            "?",
            "?",
            "batch entries must be JSON objects".into(),
        ));
    };
    let cmd_name = key_str(map, "cmd").unwrap_or_else(|| "?".into());
    let action = key_str(map, "action").unwrap_or_else(|| "?".into());
    let unknown = || {
        raised(
            index,
            &cmd_name,
            &action,
            format!("Unknown command type: {cmd_name}"),
        )
    };
    match cmd_name.as_str() {
        "document" => exec_document(ctx, tracker, index, map, &action),
        "section" => exec_section(ctx, index, map, &action),
        "paragraph" => exec_paragraph(ctx, index, map, &action),
        "run" => exec_run(ctx, index, map, &action),
        "style" => exec_style(ctx, index, map, &action),
        "heading" => exec_heading(ctx, index, map, &action),
        "list" => exec_list(ctx, index, map, &action),
        "table" => exec_table(ctx, index, map, &action),
        "image" => exec_image(ctx, index, map, &action),
        "toc" => exec_toc(ctx, index, map, &action),
        "page" => exec_page(ctx, index, map, &action),
        "meta" => exec_meta(ctx, index, map, &action),
        _ => Err(unknown()),
    }
}

fn exec_document(
    ctx: &crate::core::Ctx,
    tracker: &mut DocumentPath,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::document as doc;
    match action {
        "new" => {
            let path = key_str(cmd, "path").unwrap_or_else(|| "output.docx".into());
            tracker.set(path.clone());
            doc::new(ctx, &doc::NewArgs { path })
        }
        "open" => {
            let path = required(cmd, "path")
                .map_err(|message| raised(index, "document", action, message))?;
            tracker.set(path.clone());
            doc::open(ctx, &doc::OpenArgs { path })
        }
        "save" | "save-as" => {
            let path = tracker.or_default(key_str(cmd, "path"));
            tracker.set(path.clone());
            doc::save(
                ctx,
                &doc::SaveArgs {
                    path: Some(path),
                    format: None,
                },
            )
        }
        "close" => doc::close(ctx, &doc::CloseArgs {}),
        "info" => doc::info(ctx, &doc::InfoArgs {}),
        other => Err(raised(
            index,
            "document",
            action,
            format!("Unknown document action: {other}"),
        )),
    }
}

fn exec_section(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::section as sec;
    match action {
        "add" => sec::add(
            ctx,
            &sec::AddArgs {
                start_type: key_str(cmd, "start_type").unwrap_or_else(|| "new_page".into()),
            },
        ),
        "list" => sec::list(ctx, &sec::ListArgs {}),
        "info" => sec::info(
            ctx,
            &sec::InfoArgs {
                index: key_usize(cmd, "index")?.unwrap_or(0),
            },
        ),
        "page-break" => sec::page_break(ctx, &sec::PageBreakArgs {}),
        other => Err(raised(
            index,
            "section",
            action,
            format!("Unknown section action: {other}"),
        )),
    }
}

fn exec_paragraph(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::paragraph as para;
    match action {
        "add" => para::add(
            ctx,
            &para::AddArgs {
                text: key_str(cmd, "text").unwrap_or_default(),
                style: key_str(cmd, "style"),
                id: key_str(cmd, "id"),
                page_break: key_bool(cmd, "page_break", false),
            },
        ),
        "insert" => {
            let index_value = key_usize(cmd, "index")?
                .ok_or_else(|| "'index'".to_string())
                .map_err(|message| raised(index, "paragraph", action, message))?;
            para::insert(
                ctx,
                &para::InsertArgs {
                    index: index_value,
                    text: key_str(cmd, "text").unwrap_or_default(),
                    style: key_str(cmd, "style"),
                    id: key_str(cmd, "id"),
                    page_break: key_bool(cmd, "page_break", false),
                },
            )
        }
        "get" => para::get(
            ctx,
            &para::GetArgs {
                address: para_address(cmd),
                cell: crate::commands::paragraph::CellArgs {
                    table: None,
                    row: None,
                    col: None,
                    para: None,
                },
            },
        ),
        "update" => para::update(
            ctx,
            &para::UpdateArgs {
                text: key_str(cmd, "text").unwrap_or_default(),
                address: para_address(cmd),
            },
        ),
        "delete" => para::delete(
            ctx,
            &para::DeleteArgs {
                address: para_address(cmd),
                cell: crate::commands::paragraph::CellArgs {
                    table: None,
                    row: None,
                    col: None,
                    para: None,
                },
            },
        ),
        "list" => para::list(ctx, &para::ListArgs {}),
        "move" => para::r#move(
            ctx,
            &para::MoveArgs {
                direction: key_str(cmd, "direction").unwrap_or_else(|| "up".into()),
                address: para_address(cmd),
            },
        ),
        "clear" => para::clear(
            ctx,
            &para::ClearArgs {
                address: para_address(cmd),
            },
        ),
        "find" => para::find(
            ctx,
            &para::FindArgs {
                text: key_str(cmd, "text").unwrap_or_default(),
            },
        ),
        "replace" => para::replace(
            ctx,
            &para::ReplaceArgs {
                find: key_str(cmd, "find").unwrap_or_default(),
                replace: key_str(cmd, "replace").unwrap_or_default(),
            },
        ),
        "count" => para::count(ctx, &para::CountArgs {}),
        other => Err(raised(
            index,
            "paragraph",
            action,
            format!("Unknown paragraph action: {other}"),
        )),
    }
}

fn exec_run(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::run as run_cmd;
    match action {
        "add" => {
            let format =
                format_flags(cmd).map_err(|message| raised(index, "run", action, message))?;
            run_cmd::add(
                ctx,
                &run_cmd::AddArgs {
                    text: key_str(cmd, "text").unwrap_or_default(),
                    id: key_str(cmd, "id"),
                    index: key_usize(cmd, "index")?,
                    format,
                },
            )
        }
        "get" => run_cmd::get(
            ctx,
            &run_cmd::GetArgs {
                address: run_address(cmd),
                cell: crate::commands::run::CellArgs {
                    table: None,
                    row: None,
                    col: None,
                    para: None,
                },
            },
        ),
        "clear" => run_cmd::clear(
            ctx,
            &run_cmd::ClearArgs {
                address: run_address(cmd),
            },
        ),
        "format" => {
            let format =
                format_flags(cmd).map_err(|message| raised(index, "run", action, message))?;
            run_cmd::format(
                ctx,
                &run_cmd::FormatArgs {
                    address: run_address(cmd),
                    format,
                    run_index: key_usize(cmd, "run_index")?,
                },
            )
        }
        other => Err(raised(
            index,
            "run",
            action,
            format!("Unknown run action: {other}"),
        )),
    }
}

fn exec_style(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::style as style_cmd;
    match action {
        "list" => style_cmd::list(
            ctx,
            &style_cmd::ListArgs {
                r#type: key_str(cmd, "type"),
            },
        ),
        "apply" => {
            let style = required(cmd, "style")
                .map_err(|message| raised(index, "style", action, message))?;
            style_cmd::apply(
                ctx,
                &style_cmd::ApplyArgs {
                    style,
                    id: key_str(cmd, "id"),
                    index: key_usize(cmd, "index")?,
                },
            )
        }
        other => Err(raised(
            index,
            "style",
            action,
            format!("Unknown style action: {other}"),
        )),
    }
}

fn exec_heading(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::heading as heading_cmd;
    match action {
        "add" => heading_cmd::add(
            ctx,
            &heading_cmd::AddArgs {
                text: key_str(cmd, "text").unwrap_or_default(),
                level: key_usize(cmd, "level")?.unwrap_or(1) as u8,
                id: key_str(cmd, "id"),
            },
        ),
        "set-level" => {
            let level = key_usize(cmd, "level")?
                .ok_or_else(|| "'level'".to_string())
                .map_err(|message| raised(index, "heading", action, message))?;
            heading_cmd::set_level(
                ctx,
                &heading_cmd::SetLevelArgs {
                    level: level as u8,
                    id: key_str(cmd, "id"),
                    index: key_usize(cmd, "index")?,
                },
            )
        }
        "list" => heading_cmd::list(ctx, &heading_cmd::ListArgs {}),
        other => Err(raised(
            index,
            "heading",
            action,
            format!("Unknown heading action: {other}"),
        )),
    }
}

fn exec_list(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::list as list_cmd;
    match action {
        "add" | "add-item" => list_cmd::add(
            ctx,
            &list_cmd::AddArgs {
                text: key_str(cmd, "text").unwrap_or_default(),
                ordered: key_bool(cmd, "ordered", false),
                level: key_usize(cmd, "level")?.unwrap_or(1) as u8,
                id: key_str(cmd, "id"),
            },
        ),
        "convert" => list_cmd::convert(
            ctx,
            &list_cmd::ConvertArgs {
                ordered: key_bool(cmd, "ordered", false),
                id: key_str(cmd, "id"),
                index: key_usize(cmd, "index")?,
            },
        ),
        "set-level" => {
            let level = key_usize(cmd, "level")?
                .ok_or_else(|| "'level'".to_string())
                .map_err(|message| raised(index, "list", action, message))?;
            list_cmd::set_level(
                ctx,
                &list_cmd::SetLevelArgs {
                    level: level as u8,
                    id: key_str(cmd, "id"),
                    index: key_usize(cmd, "index")?,
                },
            )
        }
        other => Err(raised(
            index,
            "list",
            action,
            format!("Unknown list action: {other}"),
        )),
    }
}

fn exec_table(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::table as table_cmd;
    match action {
        "add" => table_cmd::add(
            ctx,
            &table_cmd::AddArgs {
                rows: key_usize(cmd, "rows")?.unwrap_or(1),
                cols: key_usize(cmd, "cols")?.unwrap_or(1),
                id: key_str(cmd, "id"),
                style: key_str(cmd, "style").unwrap_or_else(|| "Table Grid".into()),
            },
        ),
        "list" => table_cmd::list(ctx, &table_cmd::ListArgs {}),
        "get" => table_cmd::get(
            ctx,
            &table_cmd::GetArgs {
                address: table_address(cmd),
            },
        ),
        "set-cell" => {
            let row = key_usize(cmd, "row")?
                .ok_or_else(|| "'row'".to_string())
                .map_err(|message| raised(index, "table", action, message))?;
            let col = key_usize(cmd, "col")?
                .ok_or_else(|| "'col'".to_string())
                .map_err(|message| raised(index, "table", action, message))?;
            table_cmd::set_cell(
                ctx,
                &table_cmd::SetCellArgs {
                    row,
                    col,
                    value: key_str(cmd, "value").unwrap_or_default(),
                    address: table_address(cmd),
                },
            )
        }
        "set-range" => table_cmd::set_range(
            ctx,
            &table_cmd::SetRangeArgs {
                values: json_string(cmd, "values", "[]")
                    .map_err(|message| raised(index, "table", action, message))?,
                address: table_address(cmd),
                header: key_bool(cmd, "header", false),
            },
        ),
        "add-row" | "add-column" => {
            let values =
                match cmd.get("values") {
                    None => None,
                    Some(value) => Some(serde_json::to_string(value).map_err(|_| {
                        raised(index, "table", action, "'values' must be JSON".into())
                    })?),
                };
            table_cmd::add_row(
                ctx,
                &table_cmd::AppendArgs {
                    address: table_address(cmd),
                    values,
                },
            )
        }
        "delete-row" => {
            let row = key_usize(cmd, "row")?
                .ok_or_else(|| "'row'".to_string())
                .map_err(|message| raised(index, "table", action, message))?;
            table_cmd::delete_row(
                ctx,
                &table_cmd::DeleteRowArgs {
                    row,
                    address: table_address(cmd),
                },
            )
        }
        "delete-column" => {
            let col = key_usize(cmd, "col")?
                .ok_or_else(|| "'col'".to_string())
                .map_err(|message| raised(index, "table", action, message))?;
            table_cmd::delete_column(
                ctx,
                &table_cmd::DeleteColumnArgs {
                    col,
                    address: table_address(cmd),
                },
            )
        }
        other => Err(raised(
            index,
            "table",
            action,
            format!("Unknown table action: {other}"),
        )),
    }
}

fn exec_image(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::image as image_cmd;
    match action {
        "add" => {
            let path =
                required(cmd, "path").map_err(|message| raised(index, "image", action, message))?;
            image_cmd::add(
                ctx,
                &image_cmd::AddArgs {
                    path,
                    width: key_f64(cmd, "width")?,
                    height: key_f64(cmd, "height")?,
                    id: key_str(cmd, "id"),
                },
            )
        }
        "list" => image_cmd::list(ctx, &image_cmd::ListArgs {}),
        "get" => image_cmd::get(
            ctx,
            &image_cmd::GetArgs {
                index: key_usize(cmd, "index")?.unwrap_or(0),
            },
        ),
        "resize" => {
            let index_value = key_usize(cmd, "index")?
                .ok_or_else(|| "'index'".to_string())
                .map_err(|message| raised(index, "image", action, message))?;
            image_cmd::resize(
                ctx,
                &image_cmd::ResizeArgs {
                    index: index_value,
                    width: key_f64(cmd, "width")?,
                    height: key_f64(cmd, "height")?,
                },
            )
        }
        "delete" => {
            let index_value = key_usize(cmd, "index")?
                .ok_or_else(|| "'index'".to_string())
                .map_err(|message| raised(index, "image", action, message))?;
            image_cmd::delete(ctx, &image_cmd::DeleteArgs { index: index_value })
        }
        other => Err(raised(
            index,
            "image",
            action,
            format!("Unknown image action: {other}"),
        )),
    }
}

fn exec_toc(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::toc as toc_cmd;
    match action {
        "add" => toc_cmd::add(
            ctx,
            &toc_cmd::AddArgs {
                levels: key_str(cmd, "levels").unwrap_or_else(|| "1-3".into()),
                id: key_str(cmd, "id"),
            },
        ),
        "update" => toc_cmd::update(ctx, &toc_cmd::UpdateArgs {}),
        other => Err(raised(
            index,
            "toc",
            action,
            format!("Unknown toc action: {other}"),
        )),
    }
}

fn exec_page(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::page as page_cmd;
    let section = key_usize(cmd, "section")?.unwrap_or(0);
    match action {
        "margins" => page_cmd::margins(
            ctx,
            &page_cmd::MarginsArgs {
                top: key_f64(cmd, "top")?,
                bottom: key_f64(cmd, "bottom")?,
                left: key_f64(cmd, "left")?,
                right: key_f64(cmd, "right")?,
                unit: key_str(cmd, "unit").unwrap_or_else(|| "inches".into()),
                section,
            },
        ),
        "orientation" => {
            let orientation = required(cmd, "orientation")
                .map_err(|message| raised(index, "page", action, message))?;
            page_cmd::orientation(
                ctx,
                &page_cmd::OrientationArgs {
                    orientation,
                    section,
                },
            )
        }
        "size" => page_cmd::size(
            ctx,
            &page_cmd::SizeArgs {
                width: key_f64(cmd, "width")?,
                height: key_f64(cmd, "height")?,
                unit: key_str(cmd, "unit").unwrap_or_else(|| "inches".into()),
                section,
            },
        ),
        "header" => page_cmd::header(
            ctx,
            &page_cmd::HeaderArgs {
                text: key_str(cmd, "text").unwrap_or_default(),
                section,
            },
        ),
        "footer" => page_cmd::footer(
            ctx,
            &page_cmd::FooterArgs {
                text: key_str(cmd, "text").unwrap_or_default(),
                section,
            },
        ),
        "page-numbers" => page_cmd::page_numbers(
            ctx,
            &page_cmd::PageNumbersArgs {
                section,
                align: key_str(cmd, "align").unwrap_or_else(|| "center".into()),
            },
        ),
        "columns" => {
            let count = key_usize(cmd, "count")?
                .ok_or_else(|| "'count'".to_string())
                .map_err(|message| raised(index, "page", action, message))?;
            page_cmd::columns(ctx, &page_cmd::ColumnsArgs { count, section })
        }
        other => Err(raised(
            index,
            "page",
            action,
            format!("Unknown page action: {other}"),
        )),
    }
}

fn exec_meta(
    ctx: &crate::core::Ctx,
    index: usize,
    cmd: &Obj,
    action: &str,
) -> Result<Data, PoetError> {
    use crate::commands::meta as meta_cmd;
    match action {
        "describe" => meta_cmd::describe(ctx, &meta_cmd::DescribeArgs {}),
        "get-document" => meta_cmd::get_document(ctx, &meta_cmd::GetDocumentArgs {}),
        "set-document" => {
            let metadata = json_string(cmd, "metadata", "{}")
                .map_err(|message| raised(index, "meta", action, message))?;
            meta_cmd::set_document(ctx, &meta_cmd::SetDocumentArgs { metadata })
        }
        "get-section" => {
            let name =
                required(cmd, "name").map_err(|message| raised(index, "meta", action, message))?;
            meta_cmd::get_section(ctx, &meta_cmd::GetSectionArgs { name })
        }
        "set-section" => {
            let name =
                required(cmd, "name").map_err(|message| raised(index, "meta", action, message))?;
            let metadata = json_string(cmd, "metadata", "{}")
                .map_err(|message| raised(index, "meta", action, message))?;
            meta_cmd::set_section(ctx, &meta_cmd::SetSectionArgs { name, metadata })
        }
        "get-table" => {
            let id =
                required(cmd, "id").map_err(|message| raised(index, "meta", action, message))?;
            meta_cmd::get_table(ctx, &meta_cmd::GetTableArgs { id })
        }
        "set-table" => {
            let id =
                required(cmd, "id").map_err(|message| raised(index, "meta", action, message))?;
            let schema = json_string(cmd, "schema", "{}")
                .map_err(|message| raised(index, "meta", action, message))?;
            meta_cmd::set_table(ctx, &meta_cmd::SetTableArgs { id, schema })
        }
        "history" => meta_cmd::history(
            ctx,
            &meta_cmd::HistoryArgs {
                limit: key_usize(cmd, "limit")?.unwrap_or(50),
            },
        ),
        other => Err(raised(
            index,
            "meta",
            action,
            format!("Unknown meta action: {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::output::Data;

    use super::*;

    fn run_script(ctx: &crate::core::Ctx, dir: &std::path::Path, script: &str) -> Data {
        let path = dir.join("batch.json");
        std::fs::write(&path, script).expect("write script");
        let args = RunArgs {
            script: path.to_string_lossy().into_owned(),
        };
        run(ctx, &args).expect("run")
    }

    fn as_batch(data: &Data) -> (&usize, &Vec<Value>) {
        let Data::BatchRun {
            commands_executed,
            results,
            ..
        } = data
        else {
            panic!("wrong variant");
        };
        (commands_executed, results)
    }

    #[test]
    fn run_full_script_and_persist() {
        let (ctx, dir) = setup();
        let out = dir.join("out.docx").to_string_lossy().into_owned();
        let data = run_script(
            &ctx,
            &dir,
            &format!(
                r#"[
                {{"cmd": "document", "action": "new", "path": "{out}"}},
                {{"cmd": "heading", "action": "add", "text": "H", "level": 1}},
                {{"cmd": "paragraph", "action": "add", "text": "Body."}},
                {{"cmd": "page", "action": "footer", "text": "Confidential"}},
                {{"cmd": "table", "action": "add", "rows": 2, "cols": 2, "id": "t"}},
                {{"cmd": "table", "action": "set-cell", "id": "t", "row": 0, "col": 0, "value": "A"}},
                {{"cmd": "document", "action": "save", "path": "{out}"}}
            ]"#
            ),
        );
        let (executed, results) = as_batch(&data);
        assert_eq!(*executed, 7);
        assert!(results.iter().all(|r| r["status"] == "ok"));
        assert!(dir.join("out.docx").exists(), "explicit save persists");
    }

    #[test]
    fn stop_on_first_error_counts_the_failure() {
        let (ctx, dir) = setup();
        let data = run_script(
            &ctx,
            &dir,
            r#"[
                {"cmd": "document", "action": "new", "path": "x.docx"},
                {"cmd": "paragraph", "action": "add", "text": "ok"},
                {"cmd": "bogus", "action": "x"},
                {"cmd": "paragraph", "action": "list"}
            ]"#,
        );
        let (executed, results) = as_batch(&data);
        assert_eq!(*executed, 3, "the failing command is counted");
        assert_eq!(results[2]["status"], "error");
        assert!(
            results[2]["message"]
                .as_str()
                .unwrap()
                .contains("Command 2 (bogus x): Unknown command type: bogus")
        );
    }

    #[test]
    fn unknown_action_in_known_category_stops() {
        let (ctx, dir) = setup();
        let data = run_script(
            &ctx,
            &dir,
            r#"[
                {"cmd": "document", "action": "new", "path": "x.docx"},
                {"cmd": "paragraph", "action": "totally-fake"},
                {"cmd": "paragraph", "action": "count"}
            ]"#,
        );
        let (executed, results) = as_batch(&data);
        assert_eq!(*executed, 2);
        assert!(
            results[1]["message"]
                .as_str()
                .unwrap()
                .contains("Unknown paragraph action: totally-fake")
        );
    }

    #[test]
    fn missing_required_key_is_prefixed_error() {
        let (ctx, dir) = setup();
        let data = run_script(
            &ctx,
            &dir,
            r#"[
                {"cmd": "document", "action": "open"},
                {"cmd": "paragraph", "action": "count"}
            ]"#,
        );
        let (executed, results) = as_batch(&data);
        assert_eq!(*executed, 1);
        assert_eq!(
            results[0]["message"],
            "validation error: Command 0 (document open): 'path'"
        );
    }

    #[test]
    fn save_falls_back_to_tracked_path() {
        let (ctx, dir) = setup();
        let tracked = dir.join("tracked.docx").to_string_lossy().into_owned();
        let data = run_script(
            &ctx,
            &dir,
            &format!(
                r#"[
                {{"cmd": "document", "action": "new", "path": "{tracked}"}},
                {{"cmd": "paragraph", "action": "add", "text": "Body."}},
                {{"cmd": "document", "action": "save"}}
            ]"#
            ),
        );
        let (executed, results) = as_batch(&data);
        assert_eq!(*executed, 3);
        assert!(results.iter().all(|r| r["status"] == "ok"));
        assert!(dir.join("tracked.docx").exists());
    }

    #[test]
    fn missing_script_is_outer_error() {
        let (ctx, _dir) = setup();
        let err = run(
            &ctx,
            &RunArgs {
                script: "nope/missing.json".into(),
            },
        )
        .expect_err("missing script");
        assert_eq!(
            err.to_string(),
            "not found: Script not found: nope/missing.json"
        );
    }

    #[test]
    fn templates_match_words_and_bad_name_rejected() {
        let (ctx, dir) = setup();
        for (name, expected_count) in [("basic", 4), ("report", 6), ("data_table", 5)] {
            let output = dir.join(format!("{name}.json"));
            let data = template(
                &ctx,
                &TemplateArgs {
                    name: name.into(),
                    output: output.to_string_lossy().into_owned(),
                },
            )
            .expect("template");
            let Data::BatchTemplate { commands, .. } = data else {
                panic!("wrong variant");
            };
            assert_eq!(commands, expected_count);
            let written = std::fs::read_to_string(&output).expect("file");
            assert!(!written.ends_with('\n'), "no trailing newline");
            assert!(written.starts_with('['));
            if name == "basic" {
                // json.dump(indent=2) expansion, byte-for-byte.
                assert_eq!(
                    written,
                    "[\n  {\n    \"cmd\": \"document\",\n    \"action\": \"new\",\n    \"path\": \"output.docx\"\n  },\n  {\n    \"cmd\": \"heading\",\n    \"action\": \"add\",\n    \"text\": \"My Document\",\n    \"level\": 1\n  },\n  {\n    \"cmd\": \"paragraph\",\n    \"action\": \"add\",\n    \"text\": \"Hello World.\"\n  },\n  {\n    \"cmd\": \"document\",\n    \"action\": \"save\",\n    \"path\": \"output.docx\"\n  }\n]"
                );
            }
        }
        let err = template(
            &ctx,
            &TemplateArgs {
                name: "nope".into(),
                output: dir.join("x.json").to_string_lossy().into_owned(),
            },
        );
        assert_eq!(
            err.unwrap_err().to_string(),
            "validation error: Template 'nope' not found. Available: basic, report, data_table"
        );
    }

    #[test]
    fn batch_meta_actions_record_history() {
        let (ctx, dir) = setup();
        let docx = dir.join("m.docx").to_string_lossy().into_owned();
        let data = run_script(
            &ctx,
            &dir,
            &format!(
                r#"[
                {{"cmd": "document", "action": "new", "path": "{docx}"}},
                {{"cmd": "paragraph", "action": "add", "text": "hi", "id": "p"}},
                {{"cmd": "meta", "action": "set-document", "metadata": {{"title": "T"}}}},
                {{"cmd": "meta", "action": "history", "limit": 10}},
                {{"cmd": "document", "action": "save", "path": "{docx}"}}
            ]"#
            ),
        );
        let (executed, results) = as_batch(&data);
        assert_eq!(*executed, 5);
        assert!(results.iter().all(|r| r["status"] == "ok"));
        let history = &results[3]["data"]["history"];
        assert!(!history.as_array().expect("array").is_empty());
        assert_eq!(results[2]["data"]["metadata"]["title"], "T");
    }
}

#[cfg(test)]
mod per_command_tests {
    use crate::commands::testutil::setup;
    use crate::core::error::PoetError;
    use crate::models::data::Data;
    use std::path::Path;

    use super::*;

    #[test]
    fn template_writes_a_script_and_reports_the_command_count() {
        let (ctx, dir) = setup();
        let output = dir.join("template.json");
        let data = template(
            &ctx,
            &TemplateArgs {
                name: "basic".into(),
                output: output.to_string_lossy().into_owned(),
            },
        )
        .expect("template");
        let Data::BatchTemplate {
            template: name_echo,
            commands,
            ..
        } = data
        else {
            panic!("expected BatchTemplate");
        };
        assert_eq!(name_echo, "basic");
        assert!(commands > 0);
        assert!(Path::new(&output).exists(), "template file must be written");
        let err = template(
            &ctx,
            &TemplateArgs {
                name: "bogus".into(),
                output: dir.join("nope.json").to_string_lossy().into_owned(),
            },
        )
        .expect_err("unknown template name");
        assert!(matches!(err, PoetError::Validation(_)));
    }
}
