//! List commands — ported from Words' `commands/list.py`; numbering via
//! real definitions (adr/0005).

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `list add` / `list add-item`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// List item text.
    pub text: String,
    /// Numbered list (default: bullet).
    #[arg(long)]
    pub ordered: bool,
    /// Nesting level.
    #[arg(long, default_value_t = 1)]
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `list convert`.
#[derive(Debug, Args)]
pub struct ConvertArgs {
    /// Convert to numbered (default: bullet).
    #[arg(long)]
    pub ordered: bool,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// Arguments for `list set-level`.
#[derive(Debug, Args)]
pub struct SetLevelArgs {
    /// New nesting level.
    pub level: u8,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
    /// Positional index.
    #[arg(long)]
    pub index: Option<usize>,
}

/// All `list` actions.
#[derive(Debug, clap::Subcommand)]
pub enum ListAction {
    /// Add a list item.
    Add(AddArgs),
    /// Add an item to an existing list.
    AddItem(AddArgs),
    /// Convert a paragraph to a list item.
    Convert(ConvertArgs),
    /// Change a list item's level.
    SetLevel(SetLevelArgs),
}

/// `list add` / `list add-item` — append a numbered/bulleted item.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| {
        let id = mgr.add_list_item(&args.text, args.ordered, args.level, args.id.as_deref())?;
        crate::core::annotate::on_list_add(mgr, &id, &args.text, args.ordered, args.level)?;
        Ok(id)
    })?;
    Ok(Data::ListItemAdded {
        id,
        text: args.text.clone(),
        list_type: if args.ordered { "ordered" } else { "bullet" }.into(),
        level: args.level,
        message: "List item added".into(),
    })
}

/// `list add-item` — alias of `add` (Words parity).
pub fn add_item(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    add(ctx, args)
}

/// `list convert` — restyle a paragraph as a list item.
pub fn convert(ctx: &Ctx, args: &ConvertArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.convert_to_list(args.ordered, args.id.as_deref(), args.index)
    })?;
    Ok(Data::ListConverted {
        id: args.id.clone(),
        index: args.index,
        list_type: if args.ordered { "ordered" } else { "bullet" }.into(),
        message: "Paragraph converted to list".into(),
    })
}

/// `list set-level` — change the nesting level, keep the flavor.
pub fn set_level(ctx: &Ctx, args: &SetLevelArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.set_list_level(args.level, args.id.as_deref(), args.index)
    })?;
    Ok(Data::ListLevelSet {
        id: args.id.clone(),
        index: args.index,
        level: args.level,
        message: format!("List level set to {}", args.level),
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

    #[test]
    fn list_add_payload_and_convert_flow() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                text: "item".into(),
                ordered: true,
                level: 2,
                id: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"List item added\""));
        assert!(json.contains("\"list_type\": \"ordered\""));
        assert!(json.contains("\"id\": \"l1\""));
        crate::commands::paragraph::add(
            &ctx,
            &crate::commands::paragraph::AddArgs {
                text: "to convert".into(),
                style: None,
                id: None,
                page_break: false,
            },
        )
        .expect("para");
        let (json, err) = render(&convert(
            &ctx,
            &ConvertArgs {
                ordered: false,
                id: Some("p1".into()),
                index: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("Paragraph converted to list"));
        assert!(json.contains("\"list_type\": \"bullet\""));
        let (json, err) = render(&set_level(
            &ctx,
            &SetLevelArgs {
                level: 3,
                id: Some("p1".into()),
                index: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("List level set to 3"));
        let (json, _) = render(&crate::commands::paragraph::list(
            &ctx,
            &crate::commands::paragraph::ListArgs {},
        ));
        assert!(
            json.contains("\"List Number 2\""),
            "item keeps ordered flavor: {json}"
        );
        assert!(json.contains("\"List Bullet 3\""));
    }

    #[test]
    fn list_level_outside_1_9_is_validation_error() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let err = add(
            &ctx,
            &AddArgs {
                text: "x".into(),
                ordered: false,
                level: 42,
                id: None,
            },
        )
        .expect_err("level");
        assert!(matches!(err, PoetError::Validation(_)));
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
    fn add_creates_a_numbered_item_when_ordered() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let data = add(
            &ctx,
            &AddArgs {
                text: "First item".into(),
                ordered: true,
                level: 1,
                id: Some("l1".into()),
            },
        )
        .expect("list add");
        let Data::ListItemAdded { id, list_type, .. } = data else {
            panic!("expected ListItemAdded");
        };
        assert_eq!(id, "l1");
        assert_eq!(list_type, "ordered");
        let err = add(
            &ctx,
            &AddArgs {
                text: "Bad".into(),
                ordered: false,
                level: 0,
                id: None,
            },
        )
        .expect_err("level out of range");
        assert!(matches!(err, PoetError::Validation(_)));
    }

    #[test]
    fn add_item_continues_the_previous_list() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        add(
            &ctx,
            &AddArgs {
                text: "First item".into(),
                ordered: false,
                level: 1,
                id: None,
            },
        )
        .expect("list add");
        let data = add_item(
            &ctx,
            &AddArgs {
                text: "Second item".into(),
                ordered: false,
                level: 1,
                id: None,
            },
        )
        .expect("list add-item");
        let Data::ListItemAdded { text, .. } = data else {
            panic!("expected ListItemAdded");
        };
        assert_eq!(text, "Second item");
    }

    #[test]
    fn convert_turns_a_paragraph_into_a_list_item() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        crate::commands::paragraph::add(
            &ctx,
            &crate::commands::paragraph::AddArgs {
                text: "Plain text.".into(),
                style: None,
                id: Some("p1".into()),
                page_break: false,
            },
        )
        .expect("paragraph add");
        let data = convert(
            &ctx,
            &ConvertArgs {
                ordered: true,
                id: Some("p1".into()),
                index: None,
            },
        )
        .expect("convert");
        let Data::ListConverted { list_type, .. } = data else {
            panic!("expected ListConverted");
        };
        assert_eq!(list_type, "ordered");
    }

    #[test]
    fn set_level_re_nests_the_item() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        add(
            &ctx,
            &AddArgs {
                text: "First item".into(),
                ordered: false,
                level: 1,
                id: Some("l1".into()),
            },
        )
        .expect("list add");
        let data = set_level(
            &ctx,
            &SetLevelArgs {
                level: 2,
                id: Some("l1".into()),
                index: None,
            },
        )
        .expect("set level");
        let Data::ListLevelSet { level, .. } = data else {
            panic!("expected ListLevelSet");
        };
        assert_eq!(level, 2);
    }
}
