//! Shared command-layer plumbing: category classification, the shared
//! document borrow, and the test fixture.

pub mod batch;
pub mod calc;
#[cfg(feature = "dev")]
pub mod dev;
pub mod document;
pub mod heading;
pub mod image;
pub mod list;
pub mod meta;
pub mod page;
pub mod paragraph;
pub mod run;
pub mod section;
pub mod style;
pub mod table;
pub mod toc;

/// Categories whose mutating commands auto-save the open document after
/// success (Words' `_AUTOSAVE`). `document` manages its own saves; `batch`,
/// `calc`, and `dev` never autosave.
pub const AUTOSAVE_CATEGORIES: &[Category] = &[
    Category::Section,
    Category::Paragraph,
    Category::Run,
    Category::Style,
    Category::Heading,
    Category::List,
    Category::Table,
    Category::Image,
    Category::Toc,
    Category::Page,
    Category::Meta,
];

/// Command categories (sub-apps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Document lifecycle.
    Document,
    /// Sections.
    Section,
    /// Paragraphs.
    Paragraph,
    /// Runs (text spans).
    Run,
    /// Styles.
    Style,
    /// Headings.
    Heading,
    /// Lists.
    List,
    /// Tables.
    Table,
    /// Images.
    Image,
    /// Table of contents.
    Toc,
    /// Page layout.
    Page,
    /// Metadata.
    Meta,
    /// Batch scripts.
    Batch,
    /// Calculation/analysis.
    Calc,
    /// Sandbox lifecycle (dev builds only; never autosaves — adr/0015).
    Dev,
}

impl Category {
    /// Whether a successful command in this category auto-saves the open
    /// document to its current path (best-effort, like Words).
    pub fn autosaves(self) -> bool {
        AUTOSAVE_CATEGORIES.contains(&self)
    }
}

/// Test fixture module (see the module doc for why it is always compiled).
pub mod testutil;

/// Run `f` with the open document manager, mapping a closed document to the
/// Words state error ("No document is open").
pub(crate) fn with_doc<T>(
    ctx: &crate::core::Ctx,
    f: impl FnOnce(
        &mut crate::core::document::DocumentManager,
    ) -> Result<T, crate::core::error::PoetError>,
) -> Result<T, crate::core::error::PoetError> {
    let mut doc = ctx.doc.borrow_mut();
    let mgr = doc.as_mut().ok_or_else(|| {
        crate::core::error::PoetError::DocumentState("No document is open".into())
    })?;
    f(mgr)
}

#[cfg(test)]
mod tests {
    use super::{AUTOSAVE_CATEGORIES, Category};

    /// The autosave decision is pinned per category with an exhaustive match:
    /// a new `Category` variant fails to compile here until this test (and
    /// `AUTOSAVE_CATEGORIES`) state its autosave behavior.
    #[test]
    fn autosave_pinned_for_every_category() {
        let all = [
            Category::Document,
            Category::Section,
            Category::Paragraph,
            Category::Run,
            Category::Style,
            Category::Heading,
            Category::List,
            Category::Table,
            Category::Image,
            Category::Toc,
            Category::Page,
            Category::Meta,
            Category::Batch,
            Category::Calc,
            Category::Dev,
        ];
        for category in all {
            let expected = match category {
                Category::Document | Category::Batch | Category::Calc | Category::Dev => false,
                Category::Section
                | Category::Paragraph
                | Category::Run
                | Category::Style
                | Category::Heading
                | Category::List
                | Category::Table
                | Category::Image
                | Category::Toc
                | Category::Page
                | Category::Meta => true,
            };
            assert_eq!(
                AUTOSAVE_CATEGORIES.contains(&category),
                expected,
                "{category:?} diverges from the autosave pin"
            );
            assert_eq!(category.autosaves(), expected, "{category:?}");
        }
    }
}
