//! Phase-3 integration: style an existing phase-2-style report (bold
//! emphasis on a phrase, paragraph border, footer, page numbers, landscape
//! section, style application), save, reopen from disk and verify the
//! formatting/layout survived the round trip
//! (`docs/plans/phase-3-design.md` §3-4).

use std::path::Path;
use std::process::ExitCode;

use poet::commands::testutil::setup;
use poet::core::Ctx;

/// One CLI invocation against a fresh context rooted at `dir` — the same
/// shape as a shell `&&` chain, where every process auto-opens the session
/// document from disk.
fn run(dir: &Path, args: &[&str]) -> (String, ExitCode) {
    let ctx = Ctx::at_dir(dir);
    let argv: Vec<String> = std::iter::once("poet".to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    let (json, code) = poet::app::run_with(ctx, argv);
    (
        json.unwrap_or_else(|| panic!("clap rejected {args:?}: no document emitted")),
        code,
    )
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let (json, code) = run(dir, args);
    assert_eq!(code, ExitCode::SUCCESS, "command failed: {args:?} → {json}");
    json
}

/// Package-level probe: a part's XML (the docx-rs reader cannot see
/// `w:cols` — adr/0011).
fn part_xml(path: &Path, name: &str) -> String {
    let file = std::fs::File::open(path).expect("open package");
    let mut archive = zip::ZipArchive::new(file).expect("zip");
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut archive.by_name(name).expect("part"), &mut xml)
        .expect("utf8");
    xml
}

#[test]
fn styled_report_survives_save_reopen_round_trip() {
    let (_ctx, dir) = setup();
    let path = dir.join("report.docx");
    let path_str = path.to_string_lossy().into_owned();

    // Build the base report through chained commands.
    ok(&dir, &["document", "new", &path_str]);
    ok(&dir, &["heading", "add", "Annual Review", "--level", "1"]);
    ok(
        &dir,
        &["paragraph", "add", "Revenue grew by 22% year over year."],
    );

    // Design pass: emphasis, border, style, page layout.
    ok(
        &dir,
        &[
            "run",
            "emphasize",
            "22%",
            "--index",
            "1",
            "--bold",
            "--color",
            "CC0000",
        ],
    );
    ok(
        &dir,
        &[
            "run", "format", "--index", "1", "--font", "Georgia", "--size", "11",
        ],
    );
    ok(
        &dir,
        &[
            "paragraph",
            "border",
            "--index",
            "1",
            "--position",
            "bottom",
            "--color",
            "3366CC",
        ],
    );
    ok(&dir, &["style", "apply", "Quote", "--index", "1"]);
    ok(&dir, &["style", "list"]);
    ok(&dir, &["style", "list", "--type", "table"]);
    ok(
        &dir,
        &["page", "margins", "--top", "0.75", "--left", "0.75"],
    );
    ok(&dir, &["page", "orientation", "landscape"]);
    ok(&dir, &["page", "header", "Internal Use Only"]);
    ok(&dir, &["page", "footer", "Annual Review 2026"]);
    ok(&dir, &["page", "page-numbers", "--align", "right"]);
    ok(&dir, &["page", "columns", "1"]);

    // Reopen from disk (fresh process semantics) and verify.
    let reopen = Ctx::at_dir(&dir);
    let doc = reopen.doc.borrow().clone().expect("auto-opened document");
    let docx = doc.docx().expect("docx");

    // Emphasis: three runs with the matched one bold + colored.
    let runs = poet::core::content::get_runs(docx, None, Some(1)).expect("runs");
    assert_eq!(runs.len(), 3, "emphasize split the paragraph");
    assert_eq!(runs[1].text, "22%");
    assert_eq!(runs[1].bold, Some(true));
    assert_eq!(runs[1].color.as_deref(), Some("CC0000"));
    assert_eq!(runs[0].bold, None);
    assert_eq!(
        runs[0].font.as_deref(),
        Some("Georgia"),
        "run format applied"
    );
    assert_eq!(runs[0].size, Some(11.0));

    // Border + style on the report paragraph (bookmarks interleave children,
    // so locate the second paragraph child).
    let paragraph = docx
        .document
        .children
        .iter()
        .filter_map(|child| match child {
            docx_rs::DocumentChild::Paragraph(p) => Some(p),
            _ => None,
        })
        .nth(1)
        .expect("report paragraph");
    let borders = paragraph
        .property
        .borders
        .as_ref()
        .expect("borders survive");
    let value = serde_json::to_value(borders).expect("serde");
    assert_eq!(value["bottom"]["color"], "3366CC");
    let style = paragraph.property.style.as_ref().expect("style survives");
    assert_eq!(style.val, "Quote");

    // Page layout on the body-final section.
    let property = &docx.document.section_property;
    let margin = serde_json::to_value(&property.page_margin).expect("serde");
    assert_eq!(margin["top"], 1080, "0.75 inch survives");
    let size = serde_json::to_value(&property.page_size).expect("serde");
    let (w, h) = (
        size["w"].as_f64().unwrap_or(0.0),
        size["h"].as_f64().unwrap_or(0.0),
    );
    assert!(w > h, "landscape swap survives");
    assert!(
        property.header.is_some() && property.footer.is_some(),
        "header/footer parts survive on the body-final section"
    );
    let document_xml = part_xml(&path, "word/document.xml");
    assert!(document_xml.contains("w:cols"), "columns written");
    let footer_xml = part_xml(&path, "word/footer1.xml");
    assert!(footer_xml.contains("PAGE"), "page-number field survives");
    assert!(
        part_xml(&path, "word/header1.xml").contains("Internal Use Only"),
        "header text survives"
    );

    drop(doc);

    // Read-side commands report the layout too.
    let json = ok(&dir, &["section", "info", "--index", "0"]);
    assert!(json.contains("\"orientation\": \"landscape\""));
    let json = ok(&dir, &["run", "get", "--index", "1"]);
    assert!(json.contains("\"bold\": true"));

    // Error contract spot checks (validated-inputs policy, adr/0011).
    let (json, code) = run(&dir, &["page", "margins", "--unit", "parsec"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert!(json.contains("\"code\": \"validation_error\""));
    let (json, code) = run(&dir, &["style", "apply", "Bogus", "--index", "1"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert!(json.contains("\"code\": \"not_found\""));
    assert!(json.contains("no style with name 'Bogus'"));
    let (json, code) = run(&dir, &["run", "emphasize", "x", "--index", "1"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert!(json.contains("\"code\": \"validation_error\""));
}
