//! Metadata engine: the payload models, the custom-XML part format, and the
//! mutation operations (Words' `meta/models.py` + `meta/engine.py`).
//!
//! Storage (adr/0012): the payload is compact JSON wrapped in a `<meta>`
//! element and stored as the package part `customXml/words_meta.xml` — the
//! same part name, namespace, version string, and wrapper bytes as Words, so
//! files written by either tool carry metadata the other can read. The
//! docx-rs reader drops custom-XML parts, so the payload lives on
//! [`crate::core::document::DocumentManager`] and is side-read from / written
//! into the zip at open/save (the adr/0001 escape hatch).
//!
//! Parity notes: missing or corrupt payloads silently fall back to defaults
//! (Words' `_load`); `describe` renames `paragraphs` to `paragraph_annotations`
//! and exposes `history_count` but not `history`/`version`; the history cap
//! keeps the newest [`MAX_HISTORY`] entries; `get_history(0)` returns
//! everything (mirroring the Python `-0` slice artifact).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::error::PoetError;

/// XML namespace of the `<meta>` wrapper (byte-level format parity with
/// Words; deliberately not rebranded).
pub const META_NS: &str = "https://words.local/meta";

/// Zip path of the metadata part (Words' OPC partname
/// `/customXml/words_meta.xml` without the leading slash).
pub const META_PARTNAME: &str = "customXml/words_meta.xml";

/// Payload schema version (Words' constant).
pub const META_VERSION: &str = "words_meta_v1";

/// Maximum number of retained history entries (oldest dropped first).
pub const MAX_HISTORY: usize = 1000;

/// Document-level metadata (Words' `DocumentMeta`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DocumentMeta {
    /// Document title.
    #[serde(default)]
    pub title: String,
    /// Free-form description.
    #[serde(default)]
    pub description: String,
    /// Author name.
    #[serde(default)]
    pub author: String,
    /// Topic tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Names of data sources backing the document.
    #[serde(default)]
    pub data_sources: Vec<String>,
}

/// Named-section metadata (Words' `SectionMeta`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SectionMeta {
    /// Section name (upsert key as given via the CLI).
    pub name: String,
    /// What the section is for.
    #[serde(default)]
    pub purpose: String,
    /// Longer description.
    #[serde(default)]
    pub description: String,
    /// Declared orientation (informational only).
    #[serde(default)]
    pub orientation: String,
    /// Declared page size (informational only).
    #[serde(default)]
    pub page_size: String,
}

/// One column of a table schema (Words' `TableColumn`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableColumn {
    /// Column name (header text).
    pub name: String,
    /// Inferred or declared type name (Words' inference vocabulary).
    #[serde(default = "default_data_type")]
    pub data_type: String,
    /// Unit annotation (e.g. currency code).
    #[serde(default)]
    pub unit: String,
    /// Column description.
    #[serde(default)]
    pub description: String,
}

fn default_data_type() -> String {
    "string".to_string()
}

/// A table's schema, keyed by the table's bookmark id (Words' `TableSchema`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableSchema {
    /// Bookmark id of the table.
    pub id: String,
    /// Human-readable name.
    #[serde(default)]
    pub name: String,
    /// Longer description.
    #[serde(default)]
    pub description: String,
    /// Column schemas in order.
    #[serde(default)]
    pub columns: Vec<TableColumn>,
    /// Whether the first row is a header.
    #[serde(default = "default_true")]
    pub header_row: bool,
}

fn default_true() -> bool {
    true
}

/// Annotation for one paragraph (Words' `ParagraphAnnotation`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParagraphAnnotation {
    /// Bookmark id of the paragraph.
    pub id: String,
    /// Element kind (`paragraph`, `heading`, `ordered list`, ...).
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Short description.
    #[serde(default)]
    pub description: String,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
}

fn default_kind() -> String {
    "paragraph".to_string()
}

/// One timestamped operation record (Words' `HistoryEntry`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Local timestamp, Words' `datetime.isoformat()` shape (no offset).
    pub timestamp: String,
    /// Operation name (`paragraph add`, ...).
    pub command: String,
    /// Target bookmark id or index rendering.
    pub target: String,
    /// Human-readable summary.
    pub summary: String,
    /// Additional structured detail (always `{}` today, like Words).
    #[serde(default)]
    pub details: Value,
}

/// The full metadata payload stored inside the `<meta>` wrapper (Words'
/// `_default()` shape and key order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaPayload {
    /// Schema version (forced to [`META_VERSION`] on every write).
    pub version: String,
    /// Document-level metadata; `None` serializes as `{}` like Words.
    #[serde(
        serialize_with = "serialize_document",
        deserialize_with = "deserialize_document",
        default
    )]
    pub document: Option<DocumentMeta>,
    /// Named-section metadata in insertion order.
    #[serde(default)]
    pub sections: Vec<SectionMeta>,
    /// Table schemas in insertion order (keyed by bookmark id).
    #[serde(default)]
    pub tables: Vec<TableSchema>,
    /// Paragraph annotations in insertion order.
    #[serde(default)]
    pub paragraphs: Vec<ParagraphAnnotation>,
    /// Operation history, oldest first.
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

fn serialize_document<S: serde::Serializer>(
    doc: &Option<DocumentMeta>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match doc {
        Some(doc) => doc.serialize(serializer),
        None => Value::Object(serde_json::Map::new()).serialize(serializer),
    }
}

fn deserialize_document<'de, D>(deserializer: D) -> Result<Option<DocumentMeta>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(None),
        Value::Object(ref map) if map.is_empty() => Ok(None),
        v => DocumentMeta::deserialize(v)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

impl Default for MetaPayload {
    fn default() -> Self {
        MetaPayload {
            version: META_VERSION.to_string(),
            document: None,
            sections: Vec::new(),
            tables: Vec::new(),
            paragraphs: Vec::new(),
            history: Vec::new(),
        }
    }
}

/// Wrap a JSON payload into the `<meta>` XML element (Words' `_wrap` bytes:
/// declaration, newline, namespaced open tag, compact JSON, close tag, no
/// trailing newline).
pub fn wrap(json_str: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<meta xmlns=\"{META_NS}\">{json_str}</meta>"
    )
}

/// Extract the raw JSON between the `<meta ...>` open tag and the last
/// `</meta>` close tag (Words' `_unwrap` string surgery). Malformed wrappers
/// degrade to `"{}"`.
pub fn unwrap_meta(blob: &str) -> &str {
    let Some(start) = blob.find("<meta") else {
        return "{}";
    };
    let Some(gt) = blob[start..].find('>').map(|at| start + at) else {
        return "{}";
    };
    let Some(lt) = blob.rfind("</meta>") else {
        return "{}";
    };
    if gt >= lt {
        return "{}";
    }
    &blob[gt + 1..lt]
}

/// Parse a raw part blob into a payload. Missing or corrupt content yields
/// the default payload (Words' `_load` fallback); unknown object keys are
/// ignored, missing keys take defaults.
pub fn parse_payload(blob: &str) -> MetaPayload {
    serde_json::from_str(unwrap_meta(blob)).unwrap_or_default()
}

/// Serialize a payload to the wrapped part bytes.
pub fn dump_payload(payload: &MetaPayload) -> String {
    let mut forced = payload.clone();
    forced.version = META_VERSION.to_string();
    let json = serde_json::to_string(&forced).unwrap_or_else(|_| "{{}}".to_string());
    wrap(&json)
}

impl MetaPayload {
    /// Replace the document metadata (Words' replace semantics).
    pub fn set_document(&mut self, meta: DocumentMeta) {
        self.document = Some(meta);
    }

    /// Document metadata, `None` when unset.
    pub fn get_document(&self) -> Option<&DocumentMeta> {
        self.document.as_ref()
    }

    /// Upsert section metadata: the first entry named `name` is replaced in
    /// place, otherwise appended (Words' `set_section_meta`). The stored
    /// entry keeps its own `name` (possibly from user JSON) while lookups
    /// use the CLI `name` — mirroring Words exactly.
    pub fn set_section(&mut self, name: &str, meta: SectionMeta) {
        match self.sections.iter_mut().find(|s| s.name == name) {
            Some(slot) => *slot = meta,
            None => self.sections.push(meta),
        }
    }

    /// First section whose stored name matches.
    pub fn get_section(&self, name: &str) -> Option<&SectionMeta> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// Upsert a table schema by its bookmark id (Words' `set_table_schema`).
    pub fn set_table(&mut self, schema: TableSchema) {
        match self.tables.iter_mut().find(|t| t.id == schema.id) {
            Some(slot) => *slot = schema,
            None => self.tables.push(schema),
        }
    }

    /// First table schema whose id matches.
    pub fn get_table(&self, table_id: &str) -> Option<&TableSchema> {
        self.tables.iter().find(|t| t.id == table_id)
    }

    /// Upsert a paragraph annotation by its bookmark id.
    pub fn set_paragraph(&mut self, annotation: ParagraphAnnotation) {
        match self.paragraphs.iter_mut().find(|p| p.id == annotation.id) {
            Some(slot) => *slot = annotation,
            None => self.paragraphs.push(annotation),
        }
    }

    /// First paragraph annotation whose id matches.
    pub fn get_paragraph(&self, para_id: &str) -> Option<&ParagraphAnnotation> {
        self.paragraphs.iter().find(|p| p.id == para_id)
    }

    /// Append a history entry, keeping at most the newest [`MAX_HISTORY`]
    /// entries.
    pub fn add_history_entry(&mut self, entry: HistoryEntry) {
        self.history.push(entry);
        if self.history.len() > MAX_HISTORY {
            let excess = self.history.len() - MAX_HISTORY;
            self.history.drain(0..excess);
        }
    }

    /// The newest `limit` entries in chronological order; `limit == 0`
    /// returns everything (Words' `-0` slice artifact).
    pub fn get_history(&self, limit: usize) -> &[HistoryEntry] {
        if limit == 0 || limit >= self.history.len() {
            &self.history[..]
        } else {
            &self.history[self.history.len() - limit..]
        }
    }

    /// The `describe` map: raw content plus `history_count`, with the
    /// `paragraphs` list renamed to `paragraph_annotations` (Words' shape;
    /// `history` and `version` are not exposed).
    pub fn describe(&self) -> Value {
        serde_json::json!({
            "document": self.document.clone().map(|d| serde_json::to_value(d).unwrap_or_default()).unwrap_or_else(|| Value::Object(serde_json::Map::new())),
            "sections": serde_json::to_value(&self.sections).unwrap_or_default(),
            "tables": serde_json::to_value(&self.tables).unwrap_or_default(),
            "paragraph_annotations": serde_json::to_value(&self.paragraphs).unwrap_or_default(),
            "history_count": self.history.len(),
        })
    }
}

/// Words' `MetaEngine` operation surface, routed through the open
/// [`DocumentManager`](crate::core::document::DocumentManager). Storage lives
/// on the manager (adr/0012), so these are free functions over it.
pub mod engine {
    use super::*;

    /// Access (initializing) the payload of the open document.
    pub fn payload(
        mgr: &mut crate::core::document::DocumentManager,
    ) -> Result<&mut MetaPayload, PoetError> {
        mgr.ensure_meta()
    }

    /// Replace the open document's document metadata.
    pub fn set_document_meta(
        mgr: &mut crate::core::document::DocumentManager,
        meta: DocumentMeta,
    ) -> Result<(), PoetError> {
        payload(mgr)?.set_document(meta);
        Ok(())
    }

    /// The open document's document metadata, if set.
    pub fn get_document_meta(
        mgr: &mut crate::core::document::DocumentManager,
    ) -> Result<Option<DocumentMeta>, PoetError> {
        Ok(payload(mgr)?.get_document().cloned())
    }

    /// Upsert named-section metadata.
    pub fn set_section_meta(
        mgr: &mut crate::core::document::DocumentManager,
        name: &str,
        meta: SectionMeta,
    ) -> Result<(), PoetError> {
        payload(mgr)?.set_section(name, meta);
        Ok(())
    }

    /// Named-section metadata, if set.
    pub fn get_section_meta(
        mgr: &mut crate::core::document::DocumentManager,
        name: &str,
    ) -> Result<Option<SectionMeta>, PoetError> {
        Ok(payload(mgr)?.get_section(name).cloned())
    }

    /// Upsert a table schema by id.
    pub fn set_table_schema(
        mgr: &mut crate::core::document::DocumentManager,
        schema: TableSchema,
    ) -> Result<(), PoetError> {
        payload(mgr)?.set_table(schema);
        Ok(())
    }

    /// A table schema by id, if set.
    pub fn get_table_schema(
        mgr: &mut crate::core::document::DocumentManager,
        table_id: &str,
    ) -> Result<Option<TableSchema>, PoetError> {
        Ok(payload(mgr)?.get_table(table_id).cloned())
    }

    /// Upsert a paragraph annotation by id.
    pub fn set_paragraph_annotation(
        mgr: &mut crate::core::document::DocumentManager,
        annotation: ParagraphAnnotation,
    ) -> Result<(), PoetError> {
        payload(mgr)?.set_paragraph(annotation);
        Ok(())
    }

    /// A paragraph annotation by id, if set.
    pub fn get_paragraph_annotation(
        mgr: &mut crate::core::document::DocumentManager,
        para_id: &str,
    ) -> Result<Option<ParagraphAnnotation>, PoetError> {
        Ok(payload(mgr)?.get_paragraph(para_id).cloned())
    }

    /// Append a history entry (timestamped now, Words' `_now`).
    pub fn add_history_entry(
        mgr: &mut crate::core::document::DocumentManager,
        command: &str,
        target: &str,
        summary: &str,
    ) -> Result<(), PoetError> {
        let entry = HistoryEntry {
            timestamp: now_iso(),
            command: command.to_string(),
            target: target.to_string(),
            summary: summary.to_string(),
            details: Value::Object(serde_json::Map::new()),
        };
        payload(mgr)?.add_history_entry(entry);
        Ok(())
    }

    /// The newest `limit` history entries (chronological).
    pub fn get_history(
        mgr: &mut crate::core::document::DocumentManager,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, PoetError> {
        Ok(payload(mgr)?.get_history(limit).to_vec())
    }

    /// The `describe` map of the open document.
    pub fn describe(mgr: &mut crate::core::document::DocumentManager) -> Result<Value, PoetError> {
        Ok(payload(mgr)?.describe())
    }

    /// Local timestamp matching Words' `datetime.now().isoformat()` (micro
    /// precision, no offset suffix).
    pub fn now_iso() -> String {
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.6f")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_default_matches_words_default_shape() {
        let json = serde_json::to_string(&MetaPayload::default()).expect("serialize");
        assert_eq!(
            json,
            "{\"version\":\"words_meta_v1\",\"document\":{},\"sections\":[],\"tables\":[],\"paragraphs\":[],\"history\":[]}"
        );
    }

    #[test]
    fn wrap_unwrap_round_trip_uses_words_bytes() {
        let wrapped = wrap("{\"a\":1}");
        assert_eq!(
            wrapped,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<meta xmlns=\"https://words.local/meta\">{\"a\":1}</meta>"
        );
        assert_eq!(unwrap_meta(&wrapped), "{\"a\":1}");
    }

    #[test]
    fn unwrap_degrades_malformed_wrappers_to_empty_object() {
        for blob in ["", "<other>x</other>", "<meta no-close", "</meta>early"] {
            assert_eq!(unwrap_meta(blob), "{}", "blob: {blob}");
        }
    }

    #[test]
    fn parse_payload_recovers_from_corrupt_content() {
        assert_eq!(parse_payload("not json"), MetaPayload::default());
        assert_eq!(parse_payload(&wrap("[1,2]")), MetaPayload::default());
        assert_eq!(parse_payload(&wrap("{}")), MetaPayload::default());
    }

    #[test]
    fn parse_payload_fills_missing_keys_and_ignores_unknown() {
        let payload = parse_payload(&wrap(
            r#"{"version":"x","document":{"title":"T"},"extra":1}"#,
        ));
        assert_eq!(
            payload.version, "x",
            "a stored version is kept (Words' setdefault only fills missing)"
        );
        assert_eq!(
            payload.document,
            Some(DocumentMeta {
                title: "T".into(),
                ..Default::default()
            })
        );
        assert!(payload.sections.is_empty());
    }

    #[test]
    fn set_get_document_round_trip() {
        let mut payload = MetaPayload::default();
        assert!(payload.get_document().is_none());
        payload.set_document(DocumentMeta {
            title: "Q4".into(),
            tags: vec!["q4".into(), "2024".into()],
            ..Default::default()
        });
        let doc = payload.get_document().expect("set");
        assert_eq!(doc.title, "Q4");
        assert_eq!(doc.tags, vec!["q4", "2024"]);
        let json = serde_json::to_string(&payload).expect("serialize");
        assert!(json.contains("\"document\":{\"title\":\"Q4\""));
    }

    #[test]
    fn section_upsert_replaces_in_place() {
        let mut payload = MetaPayload::default();
        payload.set_section(
            "Body",
            SectionMeta {
                name: "Body".into(),
                purpose: "first".into(),
                ..Default::default()
            },
        );
        payload.set_section(
            "Body",
            SectionMeta {
                name: "Body".into(),
                purpose: "second".into(),
                ..Default::default()
            },
        );
        assert_eq!(payload.sections.len(), 1);
        assert_eq!(
            payload.get_section("Body").expect("found").purpose,
            "second"
        );
        assert!(payload.get_section("Nope").is_none());
    }

    #[test]
    fn table_upsert_keys_on_id() {
        let mut payload = MetaPayload::default();
        payload.set_table(TableSchema {
            id: "t1".into(),
            name: "one".into(),
            ..Default::default()
        });
        payload.set_table(TableSchema {
            id: "t1".into(),
            name: "two".into(),
            ..Default::default()
        });
        assert_eq!(payload.tables.len(), 1);
        assert_eq!(payload.get_table("t1").expect("found").name, "two");
    }

    #[test]
    fn history_cap_keeps_newest_thousand() {
        let mut payload = MetaPayload::default();
        for i in 0..(MAX_HISTORY + 25) {
            payload.add_history_entry(HistoryEntry {
                timestamp: format!("t{i}"),
                command: "c".into(),
                target: "x".into(),
                summary: "s".into(),
                details: Value::Object(serde_json::Map::new()),
            });
        }
        assert_eq!(payload.history.len(), MAX_HISTORY);
        assert_eq!(payload.history[0].timestamp, "t25");
        assert_eq!(
            payload.history[MAX_HISTORY - 1].timestamp,
            format!("t{}", MAX_HISTORY + 24)
        );
    }

    #[test]
    fn get_history_returns_newest_limit_chronological() {
        let mut payload = MetaPayload::default();
        for i in 0..10 {
            payload.add_history_entry(HistoryEntry {
                timestamp: format!("t{i}"),
                command: "c".into(),
                target: "x".into(),
                summary: "s".into(),
                details: Value::Object(serde_json::Map::new()),
            });
        }
        let five = payload.get_history(5);
        assert_eq!(five.len(), 5);
        assert_eq!(five[0].timestamp, "t5");
        assert_eq!(five[4].timestamp, "t9");
        assert_eq!(payload.get_history(50).len(), 10);
        assert_eq!(payload.get_history(0).len(), 10, "limit 0 means all");
    }

    #[test]
    fn describe_renames_paragraphs_and_counts_history() {
        let mut payload = MetaPayload::default();
        payload.set_document(DocumentMeta {
            title: "X".into(),
            ..Default::default()
        });
        payload.set_paragraph(ParagraphAnnotation {
            id: "p1".into(),
            ..Default::default()
        });
        payload.add_history_entry(HistoryEntry {
            timestamp: "t".into(),
            command: "c".into(),
            target: "p1".into(),
            summary: "s".into(),
            details: Value::Object(serde_json::Map::new()),
        });
        let described = payload.describe();
        let obj = described.as_object().expect("object");
        assert_eq!(
            obj.keys().collect::<Vec<_>>(),
            [
                "document",
                "sections",
                "tables",
                "paragraph_annotations",
                "history_count"
            ]
        );
        assert_eq!(obj["document"]["title"], "X");
        assert_eq!(obj["history_count"], 1);
        assert!(obj.get("history").is_none());
        assert!(obj.get("version").is_none());
    }

    #[test]
    fn dump_payload_forces_version_and_wraps() {
        let payload = MetaPayload {
            version: "bogus".into(),
            ..Default::default()
        };
        let dumped = dump_payload(&payload);
        assert!(dumped.contains("\"version\":\"words_meta_v1\""));
        assert_eq!(parse_payload(&dumped), {
            let mut p = payload;
            p.version = META_VERSION.into();
            p
        });
    }
}
