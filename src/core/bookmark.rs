//! Bookmark-based addressing for document elements.
//!
//! Every paragraph/table/heading/list item/image/TOC Poet creates is wrapped
//! in an OOXML bookmark (`<w:bookmarkStart>` ... `<w:bookmarkEnd>`) as
//! body-level siblings, giving it a stable, human-readable id. Commands
//! locate elements by id (`--id`); documents without bookmarks fall back to
//! positional `--index`. This is a direct port of Words' `bookmark.py`;
//! docx-rs models both bookmark markers as first-class document children, so
//! the sibling-wrapping mechanism is identical (adr/0004).

use std::fmt::Write as _;

use docx_rs::DocumentChild;

use crate::core::error::PoetError;

/// Summary of a body-level bookmark (the `list_bookmarks` payload shape).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BookmarkInfo {
    /// Numeric bookmark id (`w:id`).
    pub id: usize,
    /// Stable name used for `--id` addressing.
    pub name: String,
    /// What the bookmark wraps: `paragraph`, `table`, or `unknown`.
    pub kind: String,
}

/// Operates on the body children of one open document.
pub struct BookmarkManager<'a> {
    children: &'a mut Vec<DocumentChild>,
}

fn is_block(child: &DocumentChild) -> bool {
    matches!(child, DocumentChild::Paragraph(_) | DocumentChild::Table(_))
}

fn start_name(child: &DocumentChild) -> Option<&str> {
    match child {
        DocumentChild::BookmarkStart(b) => Some(&b.name),
        _ => None,
    }
}

impl<'a> BookmarkManager<'a> {
    /// Manager over the given body children.
    pub fn new(children: &'a mut Vec<DocumentChild>) -> Self {
        BookmarkManager { children }
    }

    /// Return the bookmark name for the element at `index`, creating one if
    /// missing (Words' `ensure_bookmark`). An explicit `name` must be unique.
    ///
    /// Index contract: `index` addresses the element's position *before*
    /// wrapping; wrapping inserts a start marker at `index` and shifts the
    /// element to `index + 1`. Re-calling with the original index (or the
    /// shifted one) is idempotent and returns the existing name.
    pub fn ensure(
        &mut self,
        index: usize,
        name: Option<&str>,
        prefix: &str,
    ) -> Result<String, PoetError> {
        if let Some(existing) = self.name_around(index) {
            return Ok(existing);
        }
        // Idempotency for the original (pre-wrap) index: a start marker
        // already sits at `index` wrapping the element that follows it.
        if let Some(DocumentChild::BookmarkStart(b)) = self.children.get(index)
            && self.children.get(index + 1).is_some_and(is_block)
        {
            return Ok(b.name.clone());
        }
        let name = match name {
            Some(n) => n.to_string(),
            None => self.next_default_name(prefix),
        };
        self.assert_unique(&name, None)?;
        self.add(index, &name)?;
        Ok(name)
    }

    /// Wrap the element at `index` with a start/end pair; returns the id.
    ///
    /// # Panics-free contract
    /// `index` must address a block element; anything else is a
    /// [`PoetError::Internal`], never a panic.
    pub fn add(&mut self, index: usize, name: &str) -> Result<usize, PoetError> {
        if !self.children.get(index).is_some_and(is_block) {
            return Err(PoetError::Internal(format!(
                "bookmark target at index {index} is not a block element"
            )));
        }
        let id = self.next_id();
        self.children.insert(
            index,
            DocumentChild::BookmarkStart(docx_rs::BookmarkStart::new(id, name)),
        );
        self.children.insert(
            index + 2,
            DocumentChild::BookmarkEnd(docx_rs::BookmarkEnd::new(id)),
        );
        Ok(id)
    }

    /// Index of the block element wrapped by `name`, if present.
    pub fn find(&self, name: &str) -> Option<usize> {
        let start = self
            .children
            .iter()
            .position(|c| start_name(c) == Some(name))?;
        self.block_after(start)
    }

    /// Bookmark name immediately preceding the element at `index`, if any.
    pub fn name_around(&self, index: usize) -> Option<String> {
        if index == 0 {
            return None;
        }
        self.children
            .get(index - 1)
            .and_then(start_name)
            .map(str::to_string)
    }

    /// Remove the bookmark pair wrapping the element at `index` (the element
    /// itself is kept). Indices shift after removal.
    pub fn remove_around(&mut self, index: usize) {
        let Some(name) = self.name_around(index) else {
            return;
        };
        let id = self.children.get(index - 1).and_then(|c| match c {
            DocumentChild::BookmarkStart(b) => Some(b.id),
            _ => None,
        });
        self.children.remove(index - 1);
        // The element moved back by one; its end marker now sits somewhere
        // after `index - 1`.
        let end_at = id.and_then(|id| {
            self.children[index - 1..]
                .iter()
                .position(move |c| matches!(c, DocumentChild::BookmarkEnd(b) if b.id == id))
                .map(|offset| index - 1 + offset)
        });
        if let Some(at) = end_at {
            self.children.remove(at);
        }
        let _ = name;
    }

    /// Rename a bookmark; the new name must not collide.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), PoetError> {
        let at = self
            .children
            .iter()
            .position(|c| start_name(c) == Some(old));
        let Some(at) = at else {
            return Err(PoetError::NotFound(format!("No bookmark named '{old}'")));
        };
        self.assert_unique(new, Some(old))?;
        if let DocumentChild::BookmarkStart(b) = &mut self.children[at] {
            b.name = new.to_string();
        }
        Ok(())
    }

    /// All body-level bookmarks with the kind of element they wrap.
    pub fn list(&self) -> Vec<BookmarkInfo> {
        let mut out = Vec::new();
        for (i, child) in self.children.iter().enumerate() {
            if let DocumentChild::BookmarkStart(b) = child {
                let kind = self
                    .block_after(i)
                    .and_then(|at| self.children.get(at))
                    .map(|c| {
                        if matches!(c, DocumentChild::Table(_)) {
                            "table"
                        } else {
                            "paragraph"
                        }
                    })
                    .unwrap_or("unknown");
                out.push(BookmarkInfo {
                    id: b.id,
                    name: b.name.clone(),
                    kind: kind.to_string(),
                });
            }
        }
        out
    }

    /// Next free bookmark id: max id anywhere in the document (including
    /// inline/cell bookmarks) + 1 — Words' `next_id` semantics (first id 0).
    pub fn next_id(&self) -> usize {
        let mut max: Option<usize> = None;
        scan_children(self.children, &mut max);
        max.map_or(0, |m| m + 1)
    }

    /// Whether a bookmark with this name exists.
    pub fn has(&self, name: &str) -> bool {
        self.children.iter().any(|c| start_name(c) == Some(name))
    }

    /// `{prefix}{max+1}` over body-level bookmark names, like Words.
    fn next_default_name(&self, prefix: &str) -> String {
        let mut max: usize = 0;
        for child in self.children.iter() {
            if let Some(name) = start_name(child)
                && let Some(rest) = name.strip_prefix(prefix)
                && !rest.is_empty()
                && let Ok(n) = rest.parse::<usize>()
            {
                max = max.max(n);
            }
        }
        let mut name = String::new();
        let _ = write!(name, "{prefix}{}", max + 1);
        name
    }

    fn assert_unique(&self, name: &str, exclude: Option<&str>) -> Result<(), PoetError> {
        if self.has(name) && Some(name) != exclude {
            return Err(PoetError::Conflict(format!(
                "A bookmark named '{name}' already exists"
            )));
        }
        Ok(())
    }

    fn block_after(&self, start: usize) -> Option<usize> {
        self.children[start + 1..]
            .iter()
            .position(is_block)
            .map(|offset| start + 1 + offset)
    }
}

fn scan_children(children: &[DocumentChild], max: &mut Option<usize>) {
    for child in children {
        match child {
            DocumentChild::BookmarkStart(b) => *max = Some((*max).unwrap_or(0).max(b.id)),
            DocumentChild::BookmarkEnd(b) => *max = Some((*max).unwrap_or(0).max(b.id)),
            DocumentChild::Paragraph(p) => {
                for pc in &p.children {
                    if let docx_rs::ParagraphChild::BookmarkStart(b) = pc {
                        *max = Some((*max).unwrap_or(0).max(b.id));
                    }
                    if let docx_rs::ParagraphChild::BookmarkEnd(b) = pc {
                        *max = Some((*max).unwrap_or(0).max(b.id));
                    }
                }
            }
            DocumentChild::Table(t) => scan_table(t, max),
            _ => {}
        }
    }
}

fn scan_table(table: &docx_rs::Table, max: &mut Option<usize>) {
    for row in &table.rows {
        let docx_rs::TableChild::TableRow(row) = row;
        for cell in &row.cells {
            let docx_rs::TableRowChild::TableCell(cell) = cell;
            for content in &cell.children {
                match content {
                    docx_rs::TableCellContent::Paragraph(p) => {
                        for pc in &p.children {
                            if let docx_rs::ParagraphChild::BookmarkStart(b) = pc {
                                *max = Some((*max).unwrap_or(0).max(b.id));
                            }
                            if let docx_rs::ParagraphChild::BookmarkEnd(b) = pc {
                                *max = Some((*max).unwrap_or(0).max(b.id));
                            }
                        }
                    }
                    docx_rs::TableCellContent::Table(nested) => scan_table(nested, max),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BookmarkManager;
    use crate::core::error::PoetError;
    use docx_rs::{DocumentChild, Paragraph};

    fn body() -> Vec<DocumentChild> {
        vec![
            DocumentChild::Paragraph(Box::new(Paragraph::new())),
            DocumentChild::Paragraph(Box::new(Paragraph::new())),
            DocumentChild::Paragraph(Box::new(Paragraph::new())),
        ]
    }

    #[test]
    fn ensure_allocates_prefixed_names_in_order() {
        let mut children = body();
        let mut bm = BookmarkManager::new(&mut children);
        assert_eq!(bm.ensure(0, None, "p").expect("ensure"), "p1");
        // The second paragraph shifted to index 3 after the first wrap.
        assert_eq!(bm.ensure(3, None, "p").expect("ensure"), "p2");
    }

    #[test]
    fn ensure_is_idempotent_and_finds_elements() {
        let mut children = body();
        let mut bm = BookmarkManager::new(&mut children);
        let name = bm.ensure(0, None, "p").expect("ensure");
        // Original index (pre-wrap) still resolves to the same bookmark.
        assert_eq!(bm.ensure(0, None, "p").expect("re-ensure"), name);
        // Shifted index (post-wrap) as well.
        assert_eq!(bm.ensure(1, None, "p").expect("re-ensure shifted"), name);
        assert_eq!(bm.find(&name), Some(1));
        assert_eq!(bm.name_around(1).as_deref(), Some(name.as_str()));
    }

    #[test]
    fn custom_duplicate_name_conflicts() {
        let mut children = body();
        let mut bm = BookmarkManager::new(&mut children);
        bm.ensure(0, Some("sales"), "p").expect("first");
        let err = bm.ensure(2, Some("sales"), "p").expect_err("dup");
        assert!(matches!(err, PoetError::Conflict(_)));
    }

    #[test]
    fn next_id_is_max_plus_one_across_namespaces() {
        let mut children = body();
        let mut bm = BookmarkManager::new(&mut children);
        bm.ensure(0, Some("t7"), "t").expect("seed");
        assert_eq!(bm.next_id(), 1);
        let name = bm.ensure(3, None, "p").expect("alloc");
        assert_eq!(name, "p1");
        assert_eq!(bm.next_id(), 2);
    }

    #[test]
    fn remove_around_drops_pair_and_keeps_element() {
        let mut children = body();
        {
            let mut bm = BookmarkManager::new(&mut children);
            bm.ensure(1, Some("mid"), "p").expect("wrap");
        }
        assert_eq!(children.len(), 5);
        let removed;
        {
            let mut bm = BookmarkManager::new(&mut children);
            // The wrapped element now sits at index 2.
            bm.remove_around(2);
            removed = bm.has("mid");
        }
        assert_eq!(children.len(), 3);
        assert!(!removed);
    }

    #[test]
    fn rename_updates_and_validates() {
        let mut children = body();
        let mut bm = BookmarkManager::new(&mut children);
        bm.ensure(0, Some("old"), "p").expect("wrap");
        bm.rename("old", "new").expect("rename");
        assert!(bm.has("new"));
        assert!(!bm.has("old"));
        assert!(matches!(
            bm.rename("ghost", "x"),
            Err(PoetError::NotFound(_))
        ));
    }

    #[test]
    fn list_reports_kind() {
        let mut children = body();
        let mut bm = BookmarkManager::new(&mut children);
        bm.ensure(0, Some("p1"), "p").expect("wrap p");
        let list = bm.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, "paragraph");
        assert_eq!(list[0].name, "p1");
    }
}
