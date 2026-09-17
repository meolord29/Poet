//! Heading commands — ported from Words' `commands/heading.py`.

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `heading add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Heading text.
    pub text: String,
    /// Heading level 1-9.
    #[arg(short = 'l', long, default_value_t = 1)]
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `heading set-level`.
#[derive(Debug, Args)]
pub struct SetLevelArgs {
    /// New heading level.
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Arguments for `heading list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// All `heading` actions.
#[derive(Debug, clap::Subcommand)]
pub enum HeadingAction {
    /// Add a heading.
    Add(AddArgs),
    /// Change a heading's level.
    SetLevel(SetLevelArgs),
    /// List headings.
    List(ListArgs),
}

/// `heading add` — a Heading{level}-styled paragraph, bookmarked.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| {
        mgr.add_heading(&args.text, args.level, args.id.as_deref())
    })?;
    Ok(Data::HeadingAdded {
        id,
        text: args.text.clone(),
        level: args.level,
        message: format!("Heading {} added", args.level),
    })
}

/// `heading set-level` — restyle an existing paragraph.
pub fn set_level(ctx: &Ctx, args: &SetLevelArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.set_heading_level(args.level, args.id.as_deref(), args.index)
    })?;
    Ok(Data::HeadingLevelSet {
        id: args.id.clone(),
        index: args.index,
        level: args.level,
        message: format!("Heading level set to {}", args.level),
    })
}

/// `heading list` — paragraphs whose style is a heading style.
pub fn list(ctx: &Ctx, _args: &ListArgs) -> Result<Data, PoetError> {
    let headings = crate::commands::with_doc(ctx, |mgr| mgr.list_paragraphs())?
        .into_iter()
        .filter(|p| {
            p.style
                .as_deref()
                .is_some_and(|style| style.starts_with("Heading"))
        })
        .collect();
    Ok(Data::HeadingList { headings })
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

    #[test]
    fn heading_add_list_set_level_flow() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                text: "Intro".into(),
                level: 2,
                id: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"Heading 2 added\""));
        assert!(json.contains("\"id\": \"h1\""));
        // A plain paragraph must not appear in `heading list`.
        crate::commands::paragraph::add(
            &ctx,
            &crate::commands::paragraph::AddArgs {
                text: "Body".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("para");
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"headings\""));
        assert!(json.contains("\"Heading 2\""));
        assert!(
            !json.contains("\"text\": \"Body\""),
            "non-headings filtered out"
        );
        let (json, err) = render(&set_level(
            &ctx,
            &SetLevelArgs {
                level: 1,
                id: Some("h1".into()),
                index: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("Heading level set to 1"));
    }

    #[test]
    fn heading_level_outside_1_9_is_validation_error() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let err = add(
            &ctx,
            &AddArgs {
                text: "x".into(),
                level: 10,
                id: None,
            },
        )
        .expect_err("level");
        assert!(matches!(err, PoetError::Validation(_)));
    }
}
