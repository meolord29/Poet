//! Style commands — ported from Words' `commands/style.py` +
//! `document_manager.py` (`list_styles`, `apply_style`). The registry is
//! the document's styles part plus Poet's builtin catalog (adr/0010).

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `style list`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// paragraph|character|table|list.
    #[arg(long)]
    pub r#type: Option<String>,
}

/// Arguments for `style apply`.
#[derive(Debug, Args)]
pub struct ApplyArgs {
    /// Style name.
    pub style: String,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// All `style` actions.
#[derive(Debug, clap::Subcommand)]
pub enum StyleAction {
    /// List styles.
    List(ListArgs),
    /// Apply a style to a paragraph.
    Apply(ApplyArgs),
}

/// `style list` — registry styles, optionally filtered by type. Unknown
/// `--type` tokens are rejected (adr/0011 policy; Words silently returned
/// everything).
pub fn list(ctx: &Ctx, args: &ListArgs) -> Result<Data, PoetError> {
    let styles = crate::commands::with_doc(ctx, |mgr| mgr.list_styles(args.r#type.as_deref()))?;
    let count = styles.len();
    Ok(Data::StylesListed { styles, count })
}

/// `style apply` — set `w:pStyle` on the target paragraph. Paragraph
/// resolution happens first (Words dm.py:837 order), then the registry
/// lookup rejects unknown names with Words' KeyError wording.
pub fn apply(ctx: &Ctx, args: &ApplyArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.apply_style(&args.style, args.id.as_deref(), args.index)
    })?;
    Ok(Data::StyleApplied {
        id: args.id.clone(),
        index: args.index,
        style: args.style.clone(),
        message: format!("Style '{}' applied", args.style),
    })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::Ctx;
    use crate::core::error::PoetError;
    use crate::core::output::render;

    use super::*;

    fn open_doc(ctx: &Ctx) {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        *ctx.doc.borrow_mut() = Some(mgr);
    }

    #[test]
    fn style_list_filters_and_apply_reports_words_message() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let (json, err) = render(&list(&ctx, &ListArgs { r#type: None }));
        assert!(!err);
        assert!(json.contains("\"styles\": ["));
        assert!(json.contains("\"name\": \"Table Grid\""));
        assert!(json.contains("\"type\": \"TABLE (3)\""));
        assert!(json.contains("\"builtin\": true"));
        // Words' `style list` payload carries no message key at all.
        let data = render(&list(&ctx, &ListArgs { r#type: None })).0;
        let data_only = data
            .split("\"data\":")
            .nth(1)
            .unwrap_or_default()
            .to_string();
        assert!(!data_only.contains("\"message\""));

        let (json, err) = render(&list(
            &ctx,
            &ListArgs {
                r#type: Some("character".into()),
            },
        ));
        assert!(!err);
        assert!(json.contains("\"type\": \"CHARACTER (2)\""));
        assert!(
            !json.contains("PARAGRAPH (1)"),
            "filter excludes paragraphs"
        );

        crate::commands::paragraph::add(
            &ctx,
            &crate::commands::paragraph::AddArgs {
                text: "styled".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("para");
        let (json, err) = render(&apply(
            &ctx,
            &ApplyArgs {
                style: "Intense Quote".into(),
                id: None,
                index: Some(0),
            },
        ));
        assert!(!err);
        assert!(json.contains("\"style\": \"Intense Quote\""));
        assert!(json.contains("\"message\": \"Style 'Intense Quote' applied\""));

        let err = apply(
            &ctx,
            &ApplyArgs {
                style: "Bogus".into(),
                id: None,
                index: Some(0),
            },
        )
        .expect_err("unknown style");
        assert!(matches!(err, PoetError::NotFound(ref m) if m == "no style with name 'Bogus'"));

        let err = list(
            &ctx,
            &ListArgs {
                r#type: Some("fonts".into()),
            },
        )
        .expect_err("bad type");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("fonts")));
    }
}

#[cfg(test)]
mod per_command_tests {
    use crate::commands::testutil::setup;
    use crate::core::error::PoetError;
    use crate::models::data::Data;

    use super::*;

    fn open_doc(ctx: &crate::core::Ctx) {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        *ctx.doc.borrow_mut() = Some(mgr);
    }

    #[test]
    fn list_returns_registry_styles_with_type_rendering() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let data = list(&ctx, &ListArgs { r#type: None }).expect("style list");
        let Data::StylesListed { styles, count } = data else {
            panic!("expected StylesListed");
        };
        assert_eq!(count, styles.len());
        assert!(styles.iter().any(|s| s.name == "Table Grid"));
        assert!(styles.iter().all(|s| s.r#type.contains('(')));
    }

    #[test]
    fn apply_reports_the_style_and_rejects_unknown_names() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        crate::commands::paragraph::add(
            &ctx,
            &crate::commands::paragraph::AddArgs {
                text: "Styled.".into(),
                style: None,
                id: Some("p1".into()),
                page_break: false,
            },
        )
        .expect("paragraph add");
        let data = apply(
            &ctx,
            &ApplyArgs {
                style: "Intense Quote".into(),
                id: Some("p1".into()),
                index: None,
            },
        )
        .expect("style apply");
        let Data::StyleApplied { style, .. } = data else {
            panic!("expected StyleApplied");
        };
        assert_eq!(style, "Intense Quote");
        let err = apply(
            &ctx,
            &ApplyArgs {
                style: "Bogus".into(),
                id: Some("p1".into()),
                index: None,
            },
        )
        .expect_err("unknown style");
        assert!(matches!(err, PoetError::NotFound(_)));
    }
}
