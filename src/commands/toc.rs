//! Table-of-contents commands — ported from Words' `commands/toc.py`
//! (field-code construction; the update command is a no-op hint, like
//! Words, and works without an open document).

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `toc add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Heading levels to include, e.g. "1-3".
    #[arg(long, default_value = "1-3")]
    pub levels: String,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `toc update`.
#[derive(Debug, Args)]
pub struct UpdateArgs {}

/// All `toc` actions.
#[derive(Debug, clap::Subcommand)]
pub enum TocAction {
    /// Insert a TOC field.
    Add(AddArgs),
    /// Refresh hint (Word updates the field on open).
    Update(UpdateArgs),
}

/// `toc add` — insert the TOC field paragraph.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| mgr.add_toc(&args.levels, args.id.as_deref()))?;
    Ok(Data::TocAdded {
        id,
        levels: args.levels.clone(),
        message: "Table of contents inserted (update field in Word to populate)".into(),
    })
}

/// `toc update` — no-op hint; Word refreshes fields on open. Requires no
/// open document (Words parity).
pub fn update(ctx: &Ctx, _args: &UpdateArgs) -> Result<Data, PoetError> {
    let _ = ctx;
    Ok(Data::TocUpdateHint {
        message: "TOC fields refresh automatically when the document is opened in Word.".into(),
    })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::output::render;

    use super::*;

    #[test]
    fn toc_add_payload_and_update_hint_without_document() {
        let (ctx, _dir) = setup();
        // `toc update` is a pure hint and needs no open document (Words).
        let (json, err) = render(&update(&ctx, &UpdateArgs {}));
        assert!(!err);
        assert!(json.contains("TOC fields refresh automatically"));

        let err = add(
            &ctx,
            &AddArgs {
                levels: "1-3".into(),
                id: None,
            },
        )
        .expect_err("no open document");
        assert!(matches!(err, PoetError::DocumentState(_)));
    }

    #[test]
    fn toc_add_payload_with_document() {
        let (ctx, _dir) = setup();
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        *ctx.doc.borrow_mut() = Some(mgr);
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                levels: "1-2".into(),
                id: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("Table of contents inserted"));
        assert!(json.contains("\"id\": \"toc1\""));
        assert!(json.contains("\"levels\": \"1-2\""));
    }
}
