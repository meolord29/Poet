//! Shared command-layer plumbing: category classification, the dispatch
//! macros for stubbed categories, and the test fixture.

pub mod batch;
pub mod calc;
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
/// success (Words' `_AUTOSAVE`). `document` manages its own saves; `batch`
/// and `calc` never autosave.
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
}

impl Category {
    /// Whether a successful command in this category auto-saves the open
    /// document to its current path (best-effort, like Words).
    pub fn autosaves(self) -> bool {
        AUTOSAVE_CATEGORIES.contains(&self)
    }
}

/// Declare a stubbed command module: a `Subcommand` enum already defined in
/// the module plus one `pub fn` per action returning `NotImplemented`.
///
/// The stub keeps the CLI surface complete while phases 2–4 port the bodies.
macro_rules! stub_actions {
    ($($fn_name:ident => $variant:ident),* $(,)?) => {
        $(
            #[doc = concat!("Stub until a later phase ports this action's body.")]
            pub fn $fn_name(
                _ctx: &crate::core::Ctx,
                _args: &$variant,
            ) -> Result<crate::core::output::Data, crate::core::error::PoetError> {
                Err(crate::core::error::PoetError::NotImplemented(concat!(
                    stringify!($fn_name),
                    " (later phase)"
                ).to_string()))
            }
        )*
    };
}

pub(crate) use stub_actions;

/// Test fixture module (see the module doc for why it is always compiled).
pub mod testutil;
