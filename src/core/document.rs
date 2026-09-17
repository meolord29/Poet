//! `DocumentManager` — the .docx engine handle (Words' `document_manager.py`,
//! lifecycle subset for phase 1: create/open/save/close/info plus bookmark
//! access for addressing).
//!
//! One manager owns one open `docx_rs::Docx` and its current path. Saving
//! clone-packs so the in-memory document stays open after writing (python-docx
//! semantics). Core-property keys mirror Words' `get_core_properties`; note
//! the engine has no keywords/category fields (always `""`) and no comments
//! field (mapped from `description`) — recorded in adr/0001.
//!
//! The docx-rs reader does not parse `docProps/core.xml`, so core properties
//! of a document opened from disk are side-read from the package (zip level,
//! the adr/0001 escape hatch); in-memory documents use the model values.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::core::bookmark::BookmarkManager;
use crate::core::error::PoetError;
use crate::models::data::{
    CellText, CoreProperties, DocumentInfo, FindResult, ImageInfo, ParagraphInfo, RunInfo,
    SectionDetail, SectionInfo, TableInfo,
};

/// Owns the open document for one CLI invocation.
#[derive(Debug, Default, Clone)]
pub struct DocumentManager {
    docx: Option<docx_rs::Docx>,
    current_path: Option<PathBuf>,
    /// Raw `docProps/core.xml` of the file this document was opened from.
    ///
    /// The docx-rs reader does not parse core properties and its writer
    /// emits epoch placeholders for them, so the original part is carried
    /// across open→save and patched back into the archive (adr/0001).
    core_xml: Option<String>,
}

impl DocumentManager {
    /// A manager with nothing open.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a document is currently open.
    pub fn is_open(&self) -> bool {
        self.docx.is_some()
    }

    /// Path the document was opened from / last saved to.
    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    /// Immutable access to the open document.
    pub fn docx(&self) -> Result<&docx_rs::Docx, PoetError> {
        self.docx
            .as_ref()
            .ok_or_else(|| PoetError::DocumentState("No document is open".into()))
    }

    /// Mutable access to the open document.
    pub fn docx_mut(&mut self) -> Result<&mut docx_rs::Docx, PoetError> {
        self.docx
            .as_mut()
            .ok_or_else(|| PoetError::DocumentState("No document is open".into()))
    }

    /// Create a blank document; only `docx` is accepted (Words parity).
    pub fn create(&mut self, format: &str) -> Result<(), PoetError> {
        if !format.is_empty() && !format.eq_ignore_ascii_case("docx") {
            return Err(PoetError::Validation(format!(
                "Unsupported format: {format} (only 'docx' supported)"
            )));
        }
        let mut docx = docx_rs::Docx::new();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        docx.doc_props.core = std::mem::take(&mut docx.doc_props.core)
            .created_at(&now)
            .updated_at(&now);
        self.docx = Some(docx);
        self.current_path = None;
        self.core_xml = None;
        Ok(())
    }

    /// Open an existing .docx from disk.
    pub fn open(&mut self, path: impl AsRef<Path>) -> Result<(), PoetError> {
        let file_path = path.as_ref();
        if !file_path.exists() {
            return Err(PoetError::NotFound(format!(
                "File not found: {}",
                file_path.display()
            )));
        }
        let bytes = fs::read(file_path)
            .map_err(|e| PoetError::File(format!("cannot read {}: {e}", file_path.display())))?;
        let docx = docx_rs::read_docx(&bytes)
            .map_err(|e| PoetError::File(format!("cannot open {}: {e}", file_path.display())))?;
        self.docx = Some(docx);
        self.current_path = Some(file_path.to_path_buf());
        self.core_xml = read_part(file_path, "docProps/core.xml");
        Ok(())
    }

    /// Save the open document to `path` (clone-pack; the doc stays open).
    /// The `format` argument is accepted for CLI parity and ignored: the
    /// engine writes .docx only. Field-code text is normalized first
    /// (adr/0008) so documents with TOC fields survive reopen+save.
    pub fn save(&mut self, _format: &str, path: impl AsRef<Path>) -> Result<(), PoetError> {
        let docx = self.docx_mut()?;
        crate::core::content::normalize_field_texts(docx);
        let file_path = path.as_ref();
        let file = fs::File::create(file_path)
            .map_err(|e| PoetError::File(format!("cannot write {}: {e}", file_path.display())))?;
        docx.clone()
            .pack(file)
            .map_err(|e| PoetError::File(format!("cannot pack {}: {e}", file_path.display())))?;
        if let Some(xml) = &self.core_xml {
            patch_part(file_path, "docProps/core.xml", xml)?;
        }
        self.current_path = Some(file_path.to_path_buf());
        Ok(())
    }

    /// Discard the in-memory document.
    pub fn close(&mut self) {
        self.docx = None;
        self.current_path = None;
        self.core_xml = None;
    }

    /// Structure summary (Words' `get_info` shape). Sections: embedded
    /// sectPr paragraphs + the body-final one (adr/0008). Images count all
    /// inline drawings, including inside tables (python-docx
    /// `inline_shapes` walks the whole body).
    pub fn info(&self) -> Result<DocumentInfo, PoetError> {
        let docx = self.docx()?;
        let children = &docx.document.children;
        let paragraph_count = children
            .iter()
            .filter(|c| matches!(c, docx_rs::DocumentChild::Paragraph(_)))
            .count();
        let table_count = children
            .iter()
            .filter(|c| matches!(c, docx_rs::DocumentChild::Table(_)))
            .count();
        let section_count = 1 + children
            .iter()
            .filter(|c| match c {
                docx_rs::DocumentChild::Section(_) => true,
                docx_rs::DocumentChild::Paragraph(p) => p.property.section_property.is_some(),
                _ => false,
            })
            .count();
        let image_count = count_drawings(children);
        let bookmark_count = children
            .iter()
            .filter(|c| matches!(c, docx_rs::DocumentChild::BookmarkStart(_)))
            .count();
        Ok(DocumentInfo {
            paragraph_count,
            table_count,
            section_count,
            image_count,
            bookmark_count,
            core_properties: self.core_properties(),
        })
    }

    /// Core properties mapped onto Words' key set. Documents backed by a
    /// file side-read the package; in-memory documents use the model.
    fn core_properties(&self) -> CoreProperties {
        if let Some(path) = self.current_path.as_deref()
            && let Some(props) = read_core_properties(path)
        {
            return props;
        }
        let core = match self.docx.as_ref() {
            Some(d) => d.doc_props.core.clone(),
            None => docx_rs::Docx::new().doc_props.core,
        };
        let Ok(value) = serde_json::to_value(core) else {
            return CoreProperties::default();
        };
        let cfg = value.get("config").cloned().unwrap_or_default();
        let get = |key: &str| {
            cfg.get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        CoreProperties {
            title: get("title"),
            author: get("creator"),
            subject: get("subject"),
            keywords: String::new(),
            category: String::new(),
            comments: get("description"),
            created: get("created"),
            modified: get("modified"),
        }
    }

    /// Bookmark manager over the open document's body children.
    pub fn bookmarks(&mut self) -> Result<BookmarkManager<'_>, PoetError> {
        Ok(BookmarkManager::new(
            &mut self.docx_mut()?.document.children,
        ))
    }
}

/// Thin delegates: content operations over the open document. Each method
/// forwards to the same-named [`crate::core::content`] function; see there
/// (and the adr pointers) for semantics.
macro_rules! content_delegate {
    ($($(#[$meta:meta])* $name:ident (&mut self $(, $arg:ident : $ty:ty)* ) -> $ret:ty;)*) => {
        $(
            $(#[$meta])*
            #[allow(clippy::too_many_arguments)]
            pub fn $name(&mut self $(, $arg : $ty)*) -> $ret {
                crate::core::content::$name(self.docx_mut()? $(, $arg)*)
            }
        )*
    };
}

impl DocumentManager {
    content_delegate! {
        /// `paragraph add`.
        add_paragraph(&mut self, text: &str, style: Option<&str>, id: Option<&str>, page_break: bool) -> Result<String, PoetError>;
        /// `paragraph insert`.
        insert_paragraph(&mut self, index: usize, text: &str, style: Option<&str>, id: Option<&str>, page_break: bool) -> Result<String, PoetError>;
        /// `paragraph update`.
        update_paragraph(&mut self, text: &str, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `paragraph delete`.
        delete_paragraph(&mut self, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `paragraph clear`.
        clear_paragraph(&mut self, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `paragraph move`.
        move_paragraph(&mut self, direction: &str, id: Option<&str>, index: Option<usize>) -> Result<usize, PoetError>;
        /// `paragraph get` (body).
        get_paragraph_text(&mut self, id: Option<&str>, index: Option<usize>) -> Result<String, PoetError>;
        /// `paragraph get/delete` with cell addressing — cell text.
        get_cell_paragraph_text(&mut self, table: Option<usize>, row: Option<usize>, col: Option<usize>, para: Option<usize>) -> Result<CellText, PoetError>;
        /// `paragraph delete` with cell addressing.
        delete_cell_paragraph(&mut self, table: Option<usize>, row: Option<usize>, col: Option<usize>, para: Option<usize>) -> Result<(), PoetError>;
        /// `heading add`.
        add_heading(&mut self, text: &str, level: u8, id: Option<&str>) -> Result<String, PoetError>;
        /// `heading set-level`.
        set_heading_level(&mut self, level: u8, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `list add` / `add-item`.
        add_list_item(&mut self, text: &str, ordered: bool, level: u8, id: Option<&str>) -> Result<String, PoetError>;
        /// `list convert`.
        convert_to_list(&mut self, ordered: bool, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `list set-level`.
        set_list_level(&mut self, level: u8, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `run add`.
        add_run(&mut self, text: &str, id: Option<&str>, index: Option<usize>, bold: Option<bool>, italic: Option<bool>, underline: Option<bool>, font: Option<&str>, size: Option<f64>, color: Option<&str>) -> Result<(), PoetError>;
        /// `run get` (body).
        get_runs(&mut self, id: Option<&str>, index: Option<usize>) -> Result<Vec<RunInfo>, PoetError>;
        /// `run get` with cell addressing.
        get_cell_runs(&mut self, table: Option<usize>, row: Option<usize>, col: Option<usize>, para: Option<usize>) -> Result<Vec<RunInfo>, PoetError>;
        /// `run clear`.
        clear_runs(&mut self, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `table add`.
        add_table(&mut self, rows: usize, cols: usize, id: Option<&str>, style: &str) -> Result<String, PoetError>;
        /// `table set-cell`.
        set_cell(&mut self, row: usize, col: usize, value: &str, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `table add-row`.
        add_row(&mut self, id: Option<&str>, index: Option<usize>, values: Option<&[serde_json::Value]>) -> Result<(), PoetError>;
        /// `table add-column`.
        add_column(&mut self, id: Option<&str>, index: Option<usize>, values: Option<&[serde_json::Value]>) -> Result<(), PoetError>;
        /// `table delete-row`.
        delete_row(&mut self, row: usize, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `table delete-column`.
        delete_column(&mut self, col: usize, id: Option<&str>, index: Option<usize>) -> Result<(), PoetError>;
        /// `section add`.
        add_section(&mut self, start_type: &str) -> Result<(), PoetError>;
        /// `section page-break`.
        add_page_break(&mut self, id: Option<&str>) -> Result<String, PoetError>;
        /// `image add`.
        add_image(&mut self, path: &str, width: Option<f64>, height: Option<f64>, id: Option<&str>) -> Result<String, PoetError>;
        /// `image resize`.
        resize_image(&mut self, index: usize, width: Option<f64>, height: Option<f64>) -> Result<(), PoetError>;
        /// `image delete`.
        delete_image(&mut self, index: usize) -> Result<(), PoetError>;
        /// `toc add`.
        add_toc(&mut self, levels: &str, id: Option<&str>) -> Result<String, PoetError>;
    }

    /// `paragraph list`.
    pub fn list_paragraphs(&self) -> Result<Vec<ParagraphInfo>, PoetError> {
        Ok(crate::core::content::list_paragraphs(self.docx()?))
    }

    /// `paragraph count`.
    pub fn paragraph_count(&self) -> Result<usize, PoetError> {
        Ok(crate::core::content::paragraph_count(self.docx()?))
    }

    /// `paragraph find`.
    pub fn find_text(&self, text: &str) -> Result<Vec<FindResult>, PoetError> {
        Ok(crate::core::content::find_text(self.docx()?, text))
    }

    /// `paragraph replace`.
    pub fn replace_text(&mut self, find: &str, replace: &str) -> Result<usize, PoetError> {
        Ok(crate::core::content::replace_text(
            self.docx_mut()?,
            find,
            replace,
        ))
    }

    /// `table list`.
    pub fn list_tables(&self) -> Result<Vec<TableInfo>, PoetError> {
        Ok(crate::core::content::list_tables(self.docx()?))
    }

    /// `table get`.
    pub fn table_data(
        &self,
        id: Option<&str>,
        index: Option<usize>,
    ) -> Result<(usize, usize, Vec<Vec<String>>), PoetError> {
        crate::core::content::table_data(self.docx()?, id, index)
    }

    /// `section list`.
    pub fn list_sections(&self) -> Result<Vec<SectionInfo>, PoetError> {
        Ok(crate::core::content::list_sections(self.docx()?))
    }

    /// `section info`.
    pub fn section_detail(&self, index: usize) -> Result<SectionDetail, PoetError> {
        crate::core::content::section_detail(self.docx()?, index)
    }

    /// `image list` / `image get`.
    pub fn images(&self) -> Result<Vec<ImageInfo>, PoetError> {
        Ok(crate::core::content::images(self.docx()?))
    }
}

/// Extract the text of the first `<tag ...>text</tag>` occurrence.
fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = xml.find(&open)? + open.len();
    let rest = &xml[start..];
    let body_at = rest.find('>')? + 1;
    let close = format!("</{tag}>");
    let end = rest[body_at..].find(&close)?;
    Some(rest[body_at..body_at + end].to_string())
}

/// Side-read `docProps/core.xml` from a .docx package (the docx-rs reader
/// does not populate core properties — adr/0001). Returns `None` when the
/// file or part is unreadable.
fn read_core_properties(path: &Path) -> Option<CoreProperties> {
    let xml = read_part(path, "docProps/core.xml")?;
    let get = |tag: &str| extract_xml_tag(&xml, tag).unwrap_or_default();
    Some(CoreProperties {
        title: get("dc:title"),
        author: get("dc:creator"),
        subject: get("dc:subject"),
        keywords: get("cp:keywords"),
        category: get("cp:category"),
        comments: get("dc:description"),
        created: get("dcterms:created"),
        modified: get("dcterms:modified"),
    })
}

/// Read one part from a .docx package as a UTF-8 string.
fn read_part(path: &Path, name: &str) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut xml = String::new();
    archive.by_name(name).ok()?.read_to_string(&mut xml).ok()?;
    Some(xml)
}

/// Replace one part's content inside a .docx package by rewriting the
/// archive (all other entries raw-copied, preserving compression).
fn patch_part(path: &Path, name: &str, content: &str) -> Result<(), PoetError> {
    let display = path.display();
    let src = fs::File::open(path)
        .map_err(|e| PoetError::File(format!("cannot reopen {display}: {e}")))?;
    let mut archive = zip::ZipArchive::new(src)
        .map_err(|e| PoetError::File(format!("cannot reread {display}: {e}")))?;
    let tmp = path.with_extension("poet-tmp");
    let out = fs::File::create(&tmp)
        .map_err(|e| PoetError::File(format!("cannot write {}: {e}", tmp.display())))?;
    let mut writer = zip::ZipWriter::new(out);
    for index in 0..archive.len() {
        let entry_name = archive
            .by_index(index)
            .map(|f| f.name().to_string())
            .map_err(|e| PoetError::File(format!("cannot repack {display}: {e}")))?;
        if entry_name == name {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .map_err(|e| PoetError::File(format!("cannot repack {display}: {e}")))?;
            std::io::Write::write_all(&mut writer, content.as_bytes())
                .map_err(|e| PoetError::File(format!("cannot repack {display}: {e}")))?;
        } else {
            let entry = archive
                .by_index(index)
                .map_err(|e| PoetError::File(format!("cannot repack {display}: {e}")))?;
            writer
                .raw_copy_file(entry)
                .map_err(|e| PoetError::File(format!("cannot repack {display}: {e}")))?;
        }
    }
    writer
        .finish()
        .map_err(|e| PoetError::File(format!("cannot repack {display}: {e}")))?;
    fs::rename(&tmp, path).map_err(|e| PoetError::File(format!("cannot finalize {display}: {e}")))
}

fn count_drawings(children: &[docx_rs::DocumentChild]) -> usize {
    let mut count = 0;
    for child in children {
        if let docx_rs::DocumentChild::Paragraph(p) = child {
            for pc in &p.children {
                if let docx_rs::ParagraphChild::Run(r) = pc {
                    for rc in &r.children {
                        if matches!(rc, docx_rs::RunChild::Drawing(_)) {
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::DocumentManager;

    #[test]
    fn create_rejects_non_docx_format() {
        let mut mgr = DocumentManager::new();
        let err = mgr.create("pdf").expect_err("reject");
        assert!(matches!(err, crate::core::error::PoetError::Validation(_)));
        assert!(!mgr.is_open());
    }

    #[test]
    fn operations_without_open_document_are_state_errors() {
        let mut mgr = DocumentManager::new();
        assert!(mgr.docx().is_err());
        assert!(mgr.save("docx", "/tmp/x.docx").is_err());
        assert!(mgr.info().is_err());
    }

    #[test]
    fn info_on_fresh_document_has_zero_counts_and_one_section() {
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        let info = mgr.info().expect("info");
        assert_eq!(info.paragraph_count, 0);
        assert_eq!(info.table_count, 0);
        assert_eq!(info.section_count, 1);
        assert_eq!(info.image_count, 0);
        assert_eq!(info.bookmark_count, 0);
    }

    #[test]
    fn save_open_round_trip_preserves_counts_and_stays_open() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("a.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.save("docx", &path).expect("save");
        assert!(path.exists());
        assert!(mgr.is_open());
        mgr.close();
        mgr.open(&path).expect("reopen");
        assert_eq!(mgr.info().expect("info").paragraph_count, 0);
        assert_eq!(
            mgr.current_path().expect("path"),
            path.as_path(),
            "save/open records current_path"
        );
    }

    #[test]
    fn open_missing_file_is_not_found() {
        let mut mgr = DocumentManager::new();
        let err = mgr.open("/nonexistent/a.docx").expect_err("missing");
        assert!(matches!(err, crate::core::error::PoetError::NotFound(_)));
    }

    /// Fidelity probe (adr/0001): a body-level bookmark must survive
    /// save→reopen so `--id` addressing works across processes.
    #[test]
    fn bookmark_survives_save_reopen_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("bm.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.docx_mut().expect("open doc").document.children.push(
            docx_rs::DocumentChild::Paragraph(Box::new(docx_rs::Paragraph::new())),
        );
        let name = mgr
            .bookmarks()
            .expect("bookmarks")
            .ensure(0, Some("intro"), "p")
            .expect("wrap");
        assert_eq!(name, "intro");
        mgr.save("docx", &path).expect("save");
        mgr.close();

        mgr.open(&path).expect("reopen");
        let bm = mgr.bookmarks().expect("bookmarks");
        let list = bm.list();
        assert_eq!(list.len(), 1, "bookmark must survive the round trip");
        assert_eq!(list[0].name, "intro");
        assert_eq!(list[0].kind, "paragraph");
        assert_eq!(bm.find("intro"), Some(1));
    }

    /// Fidelity probe: the core properties stamped at creation survive a
    /// save→reopen round trip.
    #[test]
    fn core_properties_survive_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("cp.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.save("docx", &path).expect("save");
        mgr.close();
        mgr.open(&path).expect("reopen");
        let info = mgr.info().expect("info");
        assert!(!info.core_properties.created.is_empty());
        assert_eq!(
            info.core_properties.created, info.core_properties.modified,
            "freshly created doc stamps both timestamps identically"
        );
    }

    #[test]
    fn extract_xml_tag_handles_attributes_and_missing_tags() {
        let xml = r#"<cp:coreProperties><dcterms:created xsi:type="dcterms:W3CDTF">2026-09-16T08:02:34Z</dcterms:created><dc:creator>unknown</dc:creator></cp:coreProperties>"#;
        assert_eq!(
            super::extract_xml_tag(xml, "dcterms:created").as_deref(),
            Some("2026-09-16T08:02:34Z")
        );
        assert_eq!(
            super::extract_xml_tag(xml, "dc:creator").as_deref(),
            Some("unknown")
        );
        assert_eq!(super::extract_xml_tag(xml, "dc:title"), None);
    }

    #[test]
    fn read_core_properties_maps_dublin_core_keys() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("cp.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.save("docx", &path).expect("save");
        let props = super::read_core_properties(&path).expect("side-read");
        assert!(!props.created.is_empty());
        assert!(!props.author.is_empty());
    }

    /// Regression: saving a document that was opened from disk must not
    /// clobber its core properties with the engine's epoch placeholders.
    #[test]
    fn save_after_open_preserves_core_properties() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("preserve.docx");
        let mut mgr = DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.save("docx", &path).expect("initial save");
        let created = super::read_core_properties(&path)
            .expect("side-read")
            .created;
        assert!(!created.is_empty());
        assert_ne!(created, "1970-01-01T00:00:00Z");

        mgr.close();
        mgr.open(&path).expect("reopen");
        mgr.save("docx", &path).expect("resave");
        let after = super::read_core_properties(&path).expect("side-read");
        assert_eq!(after.created, created, "created must survive resave");
    }
}
