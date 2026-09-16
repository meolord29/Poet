//! The success-payload enum: one variant per command's `data` shape.
//!
//! Serialized inside the `ok` envelope (`{"status":"ok","message":...,
//! "data":<variant>}`). Shapes ported from the Words commands; field order
//! matches Words' dict insertion order where observable.

use serde::Serialize;

/// One success payload per command.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Data {
    /// `document new` — the created document's path.
    DocumentNew {
        /// Path the document will be saved to.
        path: String,
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document open` — the opened document's path.
    DocumentOpened {
        /// Path the document was opened from.
        path: String,
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document save` — the saved document's path.
    DocumentSaved {
        /// Path the document was written to.
        path: String,
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document close`.
    DocumentClosed {
        /// Human-readable summary (also the envelope message).
        message: String,
    },
    /// `document info` — structure summary; envelope message is empty.
    DocumentInfo(DocumentInfo),
}

impl Data {
    /// The envelope-level message for this payload (Words passes the same
    /// string in both places; `info` has no message).
    pub fn message(&self) -> &str {
        match self {
            Data::DocumentNew { message, .. }
            | Data::DocumentOpened { message, .. }
            | Data::DocumentSaved { message, .. }
            | Data::DocumentClosed { message } => message,
            Data::DocumentInfo(_) => "",
        }
    }
}

/// `document info` payload — ported from Words' `get_info`.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentInfo {
    /// Body-level paragraph count.
    pub paragraph_count: usize,
    /// Body-level table count.
    pub table_count: usize,
    /// Section count (body-final section included).
    pub section_count: usize,
    /// Inline drawing count.
    pub image_count: usize,
    /// Body-level bookmark count.
    pub bookmark_count: usize,
    /// Document core properties.
    pub core_properties: CoreProperties,
}

/// Core properties (Words' `get_core_properties` key set).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CoreProperties {
    /// Title.
    pub title: String,
    /// Author (engine `creator`).
    pub author: String,
    /// Subject.
    pub subject: String,
    /// Keywords (engine has no equivalent; always empty — adr/0001).
    pub keywords: String,
    /// Category (engine has no equivalent; always empty — adr/0001).
    pub category: String,
    /// Comments (engine `description`).
    pub comments: String,
    /// Creation timestamp (RFC 3339) or empty.
    pub created: String,
    /// Last-modified timestamp (RFC 3339) or empty.
    pub modified: String,
}
