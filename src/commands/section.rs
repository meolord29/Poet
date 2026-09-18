//! Section commands — ported from Words' `commands/section.py`; section
//! breaks are paragraph-embedded `sectPr` (adr/0008).

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `section list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `section info`.
#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub index: usize,
}

/// Arguments for `section add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// new_page|new_column|even_page|odd_page|continuous.
    #[arg(long, default_value = "new_page")]
    pub start_type: String,
}

/// Arguments for `section page-break`.
#[derive(Debug, Args)]
pub struct PageBreakArgs {}

/// All `section` actions.
#[derive(Debug, clap::Subcommand)]
pub enum SectionAction {
    /// List sections.
    List(ListArgs),
    /// Show section details.
    Info(InfoArgs),
    /// Add a section break.
    Add(AddArgs),
    /// Insert an explicit page break.
    PageBreak(PageBreakArgs),
}

/// `section list` — all sections, body-final one last.
pub fn list(ctx: &Ctx, _args: &ListArgs) -> Result<Data, PoetError> {
    let sections = crate::commands::with_doc(ctx, |mgr| mgr.list_sections())?;
    let count = sections.len();
    Ok(Data::SectionList { sections, count })
}

/// `section info` — page geometry of one section (inches).
pub fn info(ctx: &Ctx, args: &InfoArgs) -> Result<Data, PoetError> {
    let detail = crate::commands::with_doc(ctx, |mgr| mgr.section_detail(args.index))?;
    Ok(Data::SectionDetail(detail))
}

/// `section add` — insert a section break with the given start type.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| mgr.add_section(&args.start_type))?;
    Ok(Data::SectionAdded {
        start_type: args.start_type.clone(),
        message: "Section added".into(),
    })
}

/// `section page-break` — a bookmarked paragraph holding a page break.
pub fn page_break(ctx: &Ctx, _args: &PageBreakArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| mgr.add_page_break(None))?;
    Ok(Data::PageBreakInserted {
        id,
        message: "Page break inserted".into(),
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
    fn section_list_info_add_page_break_flow() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"count\": 1"));
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                start_type: "continuous".into(),
            },
        ));
        assert!(!err);
        assert!(json.contains("Section added"));
        assert!(json.contains("\"start_type\": \"continuous\""));
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"count\": 2"));
        assert!(json.contains("\"new_page\"") || json.contains("\"continuous\""));
        let (json, _) = render(&info(&ctx, &InfoArgs { index: 0 }));
        assert!(json.contains("\"page_width\""));
        assert!(json.contains("\"orientation\": \"portrait\""));
        let (_, err) = render(&info(&ctx, &InfoArgs { index: 9 }));
        assert!(err, "out-of-range section is an error");
        let (json, err) = render(&page_break(&ctx, &PageBreakArgs {}));
        assert!(!err);
        assert!(json.contains("Page break inserted"));
        assert!(json.contains("\"id\": \"p1\""));
    }

    #[test]
    fn section_add_unknown_start_type_defaults_to_new_page() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        add(
            &ctx,
            &AddArgs {
                start_type: "bogus".into(),
            },
        )
        .expect("default");
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"start_type\": \"new_page\""));
    }
}
