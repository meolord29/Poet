//! Markdown / plain-text export walkers — ports of Words'
//! `DocumentCommand._export_text` and `_export_markdown`.
//!
//! Body paragraphs render in order (headings via resolved style names,
//! adr/0008); tables follow as pipe tables (md) or tab-joined rows (txt),
//! exactly Words' walker shape.

use docx_rs::{DocumentChild, TableChild, TableRowChild};

use crate::core::content;

/// Words' `_export_text`: paragraph lines, then table rows as TSV.
pub fn export_text(docx: &docx_rs::Docx) -> String {
    let mut lines = Vec::new();
    for child in &docx.document.children {
        if let DocumentChild::Paragraph(p) = child {
            lines.push(content::paragraph_text(p));
        }
    }
    for child in &docx.document.children {
        if let DocumentChild::Table(table) = child {
            for row in &table.rows {
                let TableChild::TableRow(row) = row;
                let cells: Vec<String> = row
                    .cells
                    .iter()
                    .map(|cell| {
                        let TableRowChild::TableCell(cell) = cell;
                        content::cell_text(cell)
                    })
                    .collect();
                lines.push(cells.join("\t"));
            }
        }
    }
    lines.join("\n")
}

/// Words' `_export_markdown`: `#`-prefixed headings (via style names),
/// paragraphs, pipe tables with a `---` separator row.
pub fn export_markdown(docx: &docx_rs::Docx) -> String {
    let mut lines = Vec::new();
    for child in &docx.document.children {
        let DocumentChild::Paragraph(p) = child else {
            continue;
        };
        let style = p
            .property
            .style
            .as_ref()
            .map(|style| style.val.clone())
            .unwrap_or_else(|| "Normal".into());
        let style = content::style_display_name(docx, Some(&style)).unwrap_or(style);
        let text = content::paragraph_text(p);
        if let Some(level) = style
            .strip_prefix("Heading ")
            .and_then(|l| l.parse::<usize>().ok())
        {
            lines.push(format!("{} {text}", "#".repeat(level)));
            continue;
        }
        lines.push(text.clone());
        if text.is_empty() {
            lines.push(String::new());
        }
    }
    for child in &docx.document.children {
        let DocumentChild::Table(table) = child else {
            continue;
        };
        let rows: Vec<Vec<String>> = table
            .rows
            .iter()
            .map(|row| {
                let TableChild::TableRow(row) = row;
                row.cells
                    .iter()
                    .map(|cell| {
                        let TableRowChild::TableCell(cell) = cell;
                        content::cell_text(cell)
                    })
                    .collect()
            })
            .collect();
        let Some(first) = rows.first() else {
            continue;
        };
        lines.push(format!("| {} |", first.join(" | ")));
        lines.push(format!("| {} |", vec!["---"; first.len()].join(" | ")));
        for row in rows.iter().skip(1) {
            lines.push(format!("| {} |", row.join(" | ")));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{export_markdown, export_text};
    use crate::core::content;

    fn docx_with_paragraphs() -> docx_rs::Docx {
        let mut docx = docx_rs::Docx::new();
        content::add_heading(&mut docx, "Title", 1, None).expect("heading");
        content::add_paragraph(&mut docx, "Body text", None, None, false).expect("para");
        content::add_paragraph(&mut docx, "", None, None, false).expect("empty para");
        docx
    }

    #[test]
    fn export_text_joins_paragraphs_and_table_rows() {
        let mut docx = docx_with_paragraphs();
        let id = content::add_table(&mut docx, 2, 2, None, "Table Grid").expect("table");
        content::set_cell(&mut docx, 0, 0, "a", Some(&id), None).expect("cell");
        content::set_cell(&mut docx, 0, 1, "b", Some(&id), None).expect("cell");
        content::set_cell(&mut docx, 1, 0, "c", Some(&id), None).expect("cell");
        content::set_cell(&mut docx, 1, 1, "d", Some(&id), None).expect("cell");
        let text = export_text(&docx);
        assert_eq!(
            text, "Title\nBody text\n\na\tb\nc\td",
            "headings and paragraphs render raw; tables as TSV"
        );
    }

    #[test]
    fn export_markdown_maps_headings_and_pipe_tables() {
        let mut docx = docx_with_paragraphs();
        let id = content::add_table(&mut docx, 1, 2, None, "Table Grid").expect("table");
        content::set_cell(&mut docx, 0, 0, "h1", Some(&id), None).expect("cell");
        content::set_cell(&mut docx, 0, 1, "h2", Some(&id), None).expect("cell");
        let md = export_markdown(&docx);
        assert!(md.contains("# Title"), "heading 1 → `#`");
        assert!(md.contains("Body text"));
        // Empty paragraph adds the extra blank line, then the pipe table.
        assert!(md.contains("| h1 | h2 |"));
        assert!(md.contains("| --- | --- |"));
    }
}
