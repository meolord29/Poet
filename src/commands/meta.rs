//! Metadata commands — ported from Words' `commands/meta_cmd.py`. JSON
//! parsing/validation happens in this layer (like Words' typer wrapper);
//! storage goes through [`crate::core::meta::engine`] (adr/0012).

use clap::Args;
use serde_json::Value;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::meta::{DocumentMeta, SectionMeta, TableSchema, engine};
use crate::core::output::Data;

/// Arguments for `meta describe`.
#[derive(Debug, Args)]
pub struct DescribeArgs {}

/// Arguments for `meta get-document`.
#[derive(Debug, Args)]
pub struct GetDocumentArgs {}

/// Arguments for `meta set-document`.
#[derive(Debug, Args)]
pub struct SetDocumentArgs {
    /// JSON document metadata.
    pub metadata: String,
}

/// Arguments for `meta get-section`.
#[derive(Debug, Args)]
pub struct GetSectionArgs {
    /// Section name.
    pub name: String,
}

/// Arguments for `meta set-section`.
#[derive(Debug, Args)]
pub struct SetSectionArgs {
    /// Section name.
    pub name: String,
    /// JSON section metadata.
    pub metadata: String,
}

/// Arguments for `meta get-table`.
#[derive(Debug, Args)]
pub struct GetTableArgs {
    /// Table bookmark id.
    pub id: String,
}

/// Arguments for `meta set-table`.
#[derive(Debug, Args)]
pub struct SetTableArgs {
    /// Table bookmark id.
    pub id: String,
    /// JSON table schema.
    pub schema: String,
}

/// Arguments for `meta history`.
#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Maximum entries to return.
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}

/// All `meta` actions.
#[derive(Debug, clap::Subcommand)]
pub enum MetaAction {
    /// Summarize stored metadata.
    Describe(DescribeArgs),
    /// Get document metadata.
    GetDocument(GetDocumentArgs),
    /// Set document metadata.
    SetDocument(SetDocumentArgs),
    /// Get section metadata.
    GetSection(GetSectionArgs),
    /// Set section metadata.
    SetSection(SetSectionArgs),
    /// Get a table schema.
    GetTable(GetTableArgs),
    /// Set a table schema.
    SetTable(SetTableArgs),
    /// Show the operation history.
    History(HistoryArgs),
}

fn parse_json(text: &str) -> Result<Value, PoetError> {
    serde_json::from_str(text).map_err(|e| PoetError::Validation(format!("Invalid JSON: {e}")))
}

fn dump<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
}

fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

/// `meta describe`.
pub fn describe(ctx: &Ctx, _args: &DescribeArgs) -> Result<Data, PoetError> {
    let described = crate::commands::with_doc(ctx, engine::describe)?;
    Ok(Data::MetaDescribe(described))
}

/// `meta get-document`.
pub fn get_document(ctx: &Ctx, _args: &GetDocumentArgs) -> Result<Data, PoetError> {
    let meta = crate::commands::with_doc(ctx, engine::get_document_meta)?;
    Ok(Data::MetaDocumentGot {
        metadata: meta.map(|m| dump(&m)).unwrap_or_else(empty_object),
        message: "Document metadata retrieved".into(),
    })
}

/// `meta set-document` — replace semantics, JSON validated here.
pub fn set_document(ctx: &Ctx, args: &SetDocumentArgs) -> Result<Data, PoetError> {
    let value = parse_json(&args.metadata)?;
    let meta: DocumentMeta = serde_json::from_value(value)
        .map_err(|e| PoetError::Validation(format!("Invalid document metadata: {e}")))?;
    crate::commands::with_doc(ctx, |mgr| engine::set_document_meta(mgr, meta.clone()))?;
    Ok(Data::MetaDocumentSet {
        metadata: dump(&meta),
        message: "Document metadata set".into(),
    })
}

/// `meta get-section`.
pub fn get_section(ctx: &Ctx, args: &GetSectionArgs) -> Result<Data, PoetError> {
    let meta = crate::commands::with_doc(ctx, |mgr| engine::get_section_meta(mgr, &args.name))?;
    Ok(Data::MetaSectionGot {
        name: args.name.clone(),
        metadata: meta.map(|m| dump(&m)).unwrap_or_else(empty_object),
    })
}

/// `meta set-section` — upsert by name; the JSON `name` wins when present
/// (Words' `setdefault("name", ...)`).
pub fn set_section(ctx: &Ctx, args: &SetSectionArgs) -> Result<Data, PoetError> {
    let mut value = parse_json(&args.metadata)?;
    if let Some(object) = value.as_object_mut()
        && !object.contains_key("name")
    {
        object.insert("name".into(), Value::String(args.name.clone()));
    }
    let meta: SectionMeta = serde_json::from_value(value)
        .map_err(|e| PoetError::Validation(format!("Invalid section metadata: {e}")))?;
    crate::commands::with_doc(ctx, |mgr| {
        engine::set_section_meta(mgr, &args.name, meta.clone())
    })?;
    Ok(Data::MetaSectionSet {
        name: args.name.clone(),
        metadata: dump(&meta),
        message: "Section metadata set".into(),
    })
}

/// `meta get-table`.
pub fn get_table(ctx: &Ctx, args: &GetTableArgs) -> Result<Data, PoetError> {
    let schema = crate::commands::with_doc(ctx, |mgr| engine::get_table_schema(mgr, &args.id))?;
    Ok(Data::MetaTableGot {
        id: args.id.clone(),
        schema: schema.map(|s| dump(&s)).unwrap_or_else(empty_object),
    })
}

/// `meta set-table` — upsert by id; the CLI id overrides any JSON id
/// (Words' forced `data["id"] = table_id`).
pub fn set_table(ctx: &Ctx, args: &SetTableArgs) -> Result<Data, PoetError> {
    let mut value = parse_json(&args.schema)?;
    if let Some(object) = value.as_object_mut() {
        object.insert("id".into(), Value::String(args.id.clone()));
    }
    let schema: TableSchema = serde_json::from_value(value)
        .map_err(|e| PoetError::Validation(format!("Invalid table schema: {e}")))?;
    crate::commands::with_doc(ctx, |mgr| engine::set_table_schema(mgr, schema.clone()))?;
    Ok(Data::MetaTableSet {
        id: args.id.clone(),
        schema: dump(&schema),
        message: "Table schema set".into(),
    })
}

/// `meta history` — the newest entries (limit 0 = all, like Words).
pub fn history(ctx: &Ctx, args: &HistoryArgs) -> Result<Data, PoetError> {
    let entries = crate::commands::with_doc(ctx, |mgr| engine::get_history(mgr, args.limit))?;
    Ok(Data::MetaHistory { history: entries })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::output::render;

    use super::*;

    #[test]
    fn set_and_get_document_round_trips() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let set = set_document(
            &ctx,
            &SetDocumentArgs {
                metadata: r#"{"title": "Q4", "tags": ["q4", "2024"]}"#.into(),
            },
        )
        .expect("set");
        let Data::MetaDocumentSet { metadata, .. } = set else {
            panic!("wrong variant");
        };
        assert_eq!(metadata["title"], "Q4");
        let got = get_document(&ctx, &GetDocumentArgs {}).expect("get");
        let Data::MetaDocumentGot { metadata, .. } = got else {
            panic!("wrong variant");
        };
        assert_eq!(metadata["tags"][1], "2024");
    }

    #[test]
    fn get_document_on_fresh_doc_is_empty_object() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let got = get_document(&ctx, &GetDocumentArgs {}).expect("get");
        let Data::MetaDocumentGot { metadata, .. } = got else {
            panic!("wrong variant");
        };
        assert_eq!(metadata, serde_json::json!({}));
    }

    #[test]
    fn set_and_get_section_injects_name() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        set_section(
            &ctx,
            &SetSectionArgs {
                name: "Body".into(),
                metadata: r#"{"purpose": "main"}"#.into(),
            },
        )
        .expect("set");
        let got = get_section(
            &ctx,
            &GetSectionArgs {
                name: "Body".into(),
            },
        )
        .expect("get");
        let Data::MetaSectionGot { name, metadata } = got else {
            panic!("wrong variant");
        };
        assert_eq!(name, "Body");
        assert_eq!(metadata["name"], "Body", "name injected by setdefault");
        assert_eq!(metadata["purpose"], "main");
    }

    #[test]
    fn set_table_forces_cli_id() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        set_table(
            &ctx,
            &SetTableArgs {
                id: "sales".into(),
                schema: r#"{"id": "bogus", "name": "Sales"}"#.into(),
            },
        )
        .expect("set");
        let got = get_table(&ctx, &GetTableArgs { id: "sales".into() }).expect("get");
        let Data::MetaTableGot { schema, .. } = got else {
            panic!("wrong variant");
        };
        assert_eq!(schema["id"], "sales");
        assert_eq!(schema["name"], "Sales");
    }

    #[test]
    fn describe_shape_and_history_after_paragraph_add() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        crate::commands::paragraph::add(
            &ctx,
            &crate::commands::paragraph::AddArgs {
                text: "hi".into(),
                style: None,
                id: Some("p".into()),
                page_break: false,
            },
        )
        .expect("paragraph");
        let described = describe(&ctx, &DescribeArgs {}).expect("describe");
        let Data::MetaDescribe(value) = described else {
            panic!("wrong variant");
        };
        assert_eq!(value["document"], serde_json::json!({}));
        assert_eq!(value["history_count"], 1);
        assert_eq!(value["paragraph_annotations"][0]["id"], "p");

        let history = history(&ctx, &HistoryArgs { limit: 50 }).expect("history");
        let Data::MetaHistory { history } = history else {
            panic!("wrong variant");
        };
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].command, "paragraph add");
        assert_eq!(history[0].target, "p");
        assert!(!history[0].timestamp.is_empty());
    }

    #[test]
    fn bad_json_is_a_validation_error() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let err = set_document(
            &ctx,
            &SetDocumentArgs {
                metadata: "not json".into(),
            },
        )
        .expect_err("reject");
        assert!(matches!(err, PoetError::Validation(_)));
    }

    #[test]
    fn meta_without_open_document_is_state_error() {
        let (ctx, _dir) = setup();
        let result = get_document(&ctx, &GetDocumentArgs {});
        let (json, is_error) = render(&result);
        assert!(is_error);
        assert!(json.contains("\"document_state\""));
    }

    fn open_doc(ctx: &Ctx) {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        *ctx.doc.borrow_mut() = Some(mgr);
    }
}
