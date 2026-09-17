//! Page layout commands — ported from Words' `commands/page.py` +
//! `document_manager.py` section methods (adr/0011). Sections are
//! addressed by index over embedded sectPr paragraphs + the body-final
//! property; bad input values are rejected (adr/0011 policy).

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `page margins`.
#[derive(Debug, Args)]
pub struct MarginsArgs {
    /// Top margin.
    #[arg(long)]
    pub top: Option<f64>,
    /// Bottom margin.
    #[arg(long)]
    pub bottom: Option<f64>,
    /// Left margin.
    #[arg(long)]
    pub left: Option<f64>,
    /// Right margin.
    #[arg(long)]
    pub right: Option<f64>,
    /// inches|cm|points.
    #[arg(long, default_value = "inches")]
    pub unit: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page orientation`.
#[derive(Debug, Args)]
pub struct OrientationArgs {
    /// portrait | landscape.
    pub orientation: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page size`.
#[derive(Debug, Args)]
pub struct SizeArgs {
    /// Page width.
    #[arg(long)]
    pub width: Option<f64>,
    /// Page height.
    #[arg(long)]
    pub height: Option<f64>,
    /// inches|cm|points.
    #[arg(long, default_value = "inches")]
    pub unit: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page header`.
#[derive(Debug, Args)]
pub struct HeaderArgs {
    /// Header text.
    pub text: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page footer`.
#[derive(Debug, Args)]
pub struct FooterArgs {
    /// Footer text.
    pub text: String,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// Arguments for `page page-numbers`.
#[derive(Debug, Args)]
pub struct PageNumbersArgs {
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
    /// left|center|right.
    #[arg(long, default_value = "center")]
    pub align: String,
}

/// Arguments for `page columns`.
#[derive(Debug, Args)]
pub struct ColumnsArgs {
    /// Column count.
    pub count: usize,
    /// Section index.
    #[arg(long, default_value_t = 0)]
    pub section: usize,
}

/// All `page` actions.
#[derive(Debug, clap::Subcommand)]
pub enum PageAction {
    /// Set page margins.
    Margins(MarginsArgs),
    /// Set page orientation.
    Orientation(OrientationArgs),
    /// Set page size.
    Size(SizeArgs),
    /// Set the header text.
    Header(HeaderArgs),
    /// Set the footer text.
    Footer(FooterArgs),
    /// Insert page-number field.
    PageNumbers(PageNumbersArgs),
    /// Set column count.
    Columns(ColumnsArgs),
}

/// `page margins` — set only the provided sides (Words dm.py:691-701);
/// section resolution precedes unit validation, like Words.
pub fn margins(ctx: &Ctx, args: &MarginsArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.set_margins(
            args.top,
            args.bottom,
            args.left,
            args.right,
            &args.unit,
            args.section,
        )
    })?;
    Ok(Data::MarginsSet {
        section: args.section,
        top: args.top,
        bottom: args.bottom,
        left: args.left,
        right: args.right,
        unit: args.unit.clone(),
        message: "Margins set".into(),
    })
}

/// `page orientation` — portrait/landscape with width/height swap.
pub fn orientation(ctx: &Ctx, args: &OrientationArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.set_orientation(&args.orientation, args.section)
    })?;
    Ok(Data::OrientationSet {
        section: args.section,
        orientation: args.orientation.clone(),
        message: "Orientation set".into(),
    })
}

/// `page size` — set only the provided dimensions.
pub fn size(ctx: &Ctx, args: &SizeArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.set_page_size(args.width, args.height, &args.unit, args.section)
    })?;
    Ok(Data::PageSizeSet {
        section: args.section,
        width: args.width,
        height: args.height,
        unit: args.unit.clone(),
        message: "Page size set".into(),
    })
}

/// `page header` — create/unlink the section header and set its text.
pub fn header(ctx: &Ctx, args: &HeaderArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| mgr.set_header(&args.text, args.section))?;
    Ok(Data::HeaderSet {
        section: args.section,
        header: args.text.clone(),
        message: "Header set".into(),
    })
}

/// `page footer` — create/unlink the section footer and set its text.
pub fn footer(ctx: &Ctx, args: &FooterArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| mgr.set_footer(&args.text, args.section))?;
    Ok(Data::FooterSet {
        section: args.section,
        footer: args.text.clone(),
        message: "Footer set".into(),
    })
}

/// `page page-numbers` — append a PAGE field run to the section footer.
pub fn page_numbers(ctx: &Ctx, args: &PageNumbersArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| mgr.add_page_numbers(&args.align, args.section))?;
    Ok(Data::PageNumbersAdded {
        section: args.section,
        align: args.align.clone(),
        message: "Page numbers added".into(),
    })
}

/// `page columns` — set `w:cols/@w:num` verbatim.
pub fn columns(ctx: &Ctx, args: &ColumnsArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| mgr.set_columns(args.count, args.section))?;
    Ok(Data::ColumnsSet {
        section: args.section,
        columns: args.count,
        message: "Columns set".into(),
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
    fn page_commands_emit_words_payloads() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);

        let (json, err) = render(&margins(
            &ctx,
            &MarginsArgs {
                top: Some(1.0),
                bottom: None,
                left: Some(0.75),
                right: None,
                unit: "inches".into(),
                section: 0,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"top\": 1.0"));
        assert!(json.contains("\"bottom\": null"));
        assert!(json.contains("\"left\": 0.75"));
        assert!(json.contains("\"unit\": \"inches\""));
        assert!(json.contains("\"message\": \"Margins set\""));

        let (json, err) = render(&orientation(
            &ctx,
            &OrientationArgs {
                orientation: "landscape".into(),
                section: 0,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"orientation\": \"landscape\""));
        assert!(json.contains("\"message\": \"Orientation set\""));

        let (json, err) = render(&size(
            &ctx,
            &SizeArgs {
                width: Some(8.5),
                height: Some(11.0),
                unit: "inches".into(),
                section: 0,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"width\": 8.5"));
        assert!(json.contains("\"height\": 11.0"));
        assert!(json.contains("\"message\": \"Page size set\""));

        let (json, err) = render(&header(
            &ctx,
            &HeaderArgs {
                text: "Top secret".into(),
                section: 0,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"header\": \"Top secret\""));
        assert!(json.contains("\"message\": \"Header set\""));

        let (json, err) = render(&footer(
            &ctx,
            &FooterArgs {
                text: "the end".into(),
                section: 0,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"footer\": \"the end\""));
        assert!(json.contains("\"message\": \"Footer set\""));

        let (json, err) = render(&page_numbers(
            &ctx,
            &PageNumbersArgs {
                section: 0,
                align: "right".into(),
            },
        ));
        assert!(!err);
        assert!(json.contains("\"align\": \"right\""));
        assert!(json.contains("\"message\": \"Page numbers added\""));

        let (json, err) = render(&columns(
            &ctx,
            &ColumnsArgs {
                count: 2,
                section: 0,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"columns\": 2"));
        assert!(json.contains("\"message\": \"Columns set\""));
    }

    #[test]
    fn page_commands_validate_tokens_and_section_range() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);

        let err = margins(
            &ctx,
            &MarginsArgs {
                top: Some(1.0),
                bottom: None,
                left: None,
                right: None,
                unit: "mm".into(),
                section: 0,
            },
        )
        .expect_err("bad unit");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("mm")));

        let err = orientation(
            &ctx,
            &OrientationArgs {
                orientation: "sideways".into(),
                section: 0,
            },
        )
        .expect_err("bad orientation");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("sideways")));

        let err = page_numbers(
            &ctx,
            &PageNumbersArgs {
                section: 0,
                align: "middle".into(),
            },
        )
        .expect_err("bad align");
        assert!(matches!(err, PoetError::Validation(ref m) if m.contains("middle")));

        let err = columns(
            &ctx,
            &ColumnsArgs {
                count: 1,
                section: 4,
            },
        )
        .expect_err("bad section");
        assert!(
            matches!(err, PoetError::NotFound(ref m) if m == "Section index 4 out of range (0..0)")
        );
    }
}
