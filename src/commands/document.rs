//! Document commands: create/open/save/close/info/export — ported from
//! Words' `commands/document.py`. The app layer owns the session glue
//! around these (persist-on-new, session save/delete) exactly like the
//! Python typer wrappers. Export walkers live in `core/export.rs`.

use clap::Args;
use serde::Serialize;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `document new`.
#[derive(Debug, Args)]
pub struct NewArgs {
    /// Path for the new .docx file.
    pub path: String,
}

/// Arguments for `document open`.
#[derive(Debug, Args)]
pub struct OpenArgs {
    /// Path to the .docx file to open.
    pub path: String,
}

/// Arguments for `document save`.
#[derive(Debug, Args)]
pub struct SaveArgs {
    /// Save-as path.
    #[arg(short = 'p', long)]
    pub path: Option<String>,
    /// Output format (docx only).
    #[arg(short = 'f', long)]
    pub format: Option<String>,
}

/// Arguments for `document close`.
#[derive(Debug, Args)]
pub struct CloseArgs {}

/// Arguments for `document info`.
#[derive(Debug, Args)]
pub struct InfoArgs {}

/// Arguments for `document export`.
#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Path of the document to export.
    pub path: String,
    /// Export format: md | txt | pdf.
    pub fmt: String,
    /// Output file path.
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

/// All `document` actions.
#[derive(Debug, clap::Subcommand)]
pub enum DocumentAction {
    /// Create a new document.
    New(NewArgs),
    /// Open an existing document.
    Open(OpenArgs),
    /// Save the current document.
    Save(SaveArgs),
    /// Close the current document.
    Close(CloseArgs),
    /// Show document information.
    Info(InfoArgs),
    /// Export the document to md/txt (pdf needs an external converter).
    Export(ExportArgs),
}

/// Payload for the document commands that report a path.
#[derive(Debug, Serialize)]
pub struct PathMessage {
    /// Document path.
    pub path: String,
    /// Human-readable summary.
    pub message: String,
}

/// `document new` — create a blank document in memory.
pub fn new(ctx: &Ctx, args: &NewArgs) -> Result<Data, PoetError> {
    let mut mgr = crate::core::document::DocumentManager::new();
    mgr.create("docx")
        .map_err(|e| PoetError::Internal(format!("document create failed: {e}")))?;
    *ctx.doc.borrow_mut() = Some(mgr);
    Ok(Data::DocumentNew {
        path: args.path.clone(),
        message: "Document created".into(),
    })
}

/// `document open` — open a .docx from disk.
pub fn open(ctx: &Ctx, args: &OpenArgs) -> Result<Data, PoetError> {
    let mut mgr = crate::core::document::DocumentManager::new();
    mgr.open(&args.path)?;
    *ctx.doc.borrow_mut() = Some(mgr);
    Ok(Data::DocumentOpened {
        path: args.path.clone(),
        message: "Document opened".into(),
    })
}

/// `document save` — write the open document (default: its current path).
pub fn save(ctx: &Ctx, args: &SaveArgs) -> Result<Data, PoetError> {
    let mut doc = ctx.doc.borrow_mut();
    let target = args
        .path
        .clone()
        .or_else(|| {
            doc.as_ref()
                .and_then(|mgr| mgr.current_path())
                .map(|p| p.to_string_lossy().into_owned())
        })
        .ok_or_else(|| {
            PoetError::Validation("No path provided and document has no current path".into())
        })?;
    let format = args.format.as_deref().unwrap_or("docx");
    doc.as_mut()
        .ok_or_else(|| PoetError::Internal("document slot unavailable".into()))?
        .save(format, &target)?;
    Ok(Data::DocumentSaved {
        path: target,
        message: "Document saved".into(),
    })
}

/// `document close` — discard the open document.
pub fn close(ctx: &Ctx, _args: &CloseArgs) -> Result<Data, PoetError> {
    *ctx.doc.borrow_mut() = None;
    Ok(Data::DocumentClosed {
        message: "Document closed".into(),
    })
}

/// `document info` — structure summary of the open document.
pub fn info(ctx: &Ctx, _args: &InfoArgs) -> Result<Data, PoetError> {
    let info = ctx
        .doc
        .borrow()
        .as_ref()
        .ok_or_else(|| PoetError::DocumentState("No document is open".into()))?
        .info()?;
    Ok(Data::DocumentInfo(info))
}

/// `document export` — write the open document (or `path`, opened lazily)
/// to md/txt; pdf stays unsupported (Words parity).
pub fn export(ctx: &Ctx, args: &ExportArgs) -> Result<Data, PoetError> {
    // Words' typer wrapper computes the default out path from the raw fmt,
    // with the pdf quirk of an empty extension.
    let out = args.out.clone().unwrap_or_else(|| {
        let stem = match args.path.rfind('.') {
            Some(i) => args.path[..i].to_string(),
            None => args.path.clone(),
        };
        let ext = if args.fmt == "pdf" { "" } else { &args.fmt };
        format!("{stem}.{ext}")
    });
    let fmt = args.fmt.to_lowercase();
    if fmt != "txt" && fmt != "md" && fmt != "pdf" {
        return Err(PoetError::Validation(format!(
            "Unsupported export format: {fmt}"
        )));
    }
    if fmt == "pdf" {
        return Err(PoetError::Unsupported(
            "PDF export requires docx2pdf or LibreOffice; not configured".into(),
        ));
    }
    // Open the document only when none is open (Words' lazy open); the
    // session is not touched.
    if ctx.doc.borrow().is_none() {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.open(&args.path)?;
        *ctx.doc.borrow_mut() = Some(mgr);
    }
    let text = crate::commands::with_doc(ctx, |mgr| {
        let docx = mgr.docx()?;
        match fmt.as_str() {
            "txt" => Ok(crate::core::export::export_text(docx)),
            _ => Ok(crate::core::export::export_markdown(docx)),
        }
    })?;
    std::fs::write(&out, text).map_err(|e| PoetError::File(format!("cannot write {out}: {e}")))?;
    Ok(Data::DocumentExported {
        path: out,
        format: fmt,
        message: "Exported".into(),
    })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::output::render;

    use super::{CloseArgs, InfoArgs, NewArgs, OpenArgs, SaveArgs, close, info, new, open, save};

    #[test]
    fn new_open_save_close_info_round_trip() {
        let (ctx, dir) = setup();
        let path = dir.join("a.docx");
        let path_str = path.to_string_lossy().into_owned();

        let (json, err) = render(&new(
            &ctx,
            &NewArgs {
                path: path_str.clone(),
            },
        ));
        assert!(!err);
        assert!(json.contains("Document created"));

        // Save via explicit path (app glue normally does this after `new`).
        let (json, err) = render(&save(
            &ctx,
            &SaveArgs {
                path: Some(path_str.clone()),
                format: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("Document saved"));

        // New context (fresh process) — reopen from disk like `&&` chaining.
        let ctx2 = crate::core::Ctx::at_dir(&dir);
        let (_, err) = render(&open(
            &ctx2,
            &OpenArgs {
                path: path_str.clone(),
            },
        ));
        assert!(!err);
        let (json, err) = render(&info(&ctx2, &InfoArgs {}));
        assert!(!err);
        assert!(json.contains("paragraph_count"));
        let (_, err) = render(&close(&ctx2, &CloseArgs {}));
        assert!(!err);
        assert!(ctx2.doc.borrow().is_none());
    }

    #[test]
    fn save_without_path_or_current_path_fails() {
        let (ctx, _dir) = setup();
        let err = save(
            &ctx,
            &SaveArgs {
                path: None,
                format: None,
            },
        )
        .expect_err("no target");
        assert!(matches!(err, crate::core::error::PoetError::Validation(_)));
    }
}

#[cfg(test)]
mod export_tests {
    use crate::commands::testutil::setup;
    use crate::core::output::render;

    use super::*;

    fn seed_doc(ctx: &Ctx) {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.add_heading("Report", 1, None).expect("heading");
        mgr.add_paragraph("Intro text", None, None, false)
            .expect("para");
        let id = mgr.add_table(1, 2, None, "Table Grid").expect("table");
        mgr.set_cell(0, 0, "a", Some(&id), None).expect("cell");
        mgr.set_cell(0, 1, "b", Some(&id), None).expect("cell");
        *ctx.doc.borrow_mut() = Some(mgr);
    }

    #[test]
    fn export_txt_and_md_write_files() {
        let (ctx, dir) = setup();
        seed_doc(&ctx);
        let out = dir.join("out.txt");
        let (json, err) = render(&export(
            &ctx,
            &ExportArgs {
                path: dir.join("ignored.docx").to_string_lossy().into_owned(),
                fmt: "txt".into(),
                out: Some(out.to_string_lossy().into_owned()),
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"Exported\""));
        assert!(json.contains("\"format\": \"txt\""));
        let text = std::fs::read_to_string(&out).expect("txt");
        assert_eq!(text, "Report\nIntro text\na\tb");

        let out_md = dir.join("out.md");
        let (_, err) = render(&export(
            &ctx,
            &ExportArgs {
                path: "ignored.docx".into(),
                fmt: "md".into(),
                out: Some(out_md.to_string_lossy().into_owned()),
            },
        ));
        assert!(!err);
        let md = std::fs::read_to_string(&out_md).expect("md");
        assert!(md.contains("# Report"));
        assert!(md.contains("| a | b |"));
    }

    #[test]
    fn export_default_out_path_stems_from_doc_path() {
        let (ctx, dir) = setup();
        seed_doc(&ctx);
        // The doc path is ignored when a document is open; the default out
        // path derives from the `path` argument (Words).
        let doc_path = dir.join("sub.docx");
        let (json, err) = render(&export(
            &ctx,
            &ExportArgs {
                path: doc_path.to_string_lossy().into_owned(),
                fmt: "md".into(),
                out: None,
            },
        ));
        assert!(!err);
        assert!(json.contains(&format!(
            "\"path\": \"{}\"",
            dir.join("sub.md").to_string_lossy()
        )));
        assert!(dir.join("sub.md").exists());
    }

    #[test]
    fn export_pdf_is_unsupported_and_unknown_fmt_is_validation() {
        let (ctx, _dir) = setup();
        seed_doc(&ctx);
        let err = export(
            &ctx,
            &ExportArgs {
                path: "x.docx".into(),
                fmt: "pdf".into(),
                out: None,
            },
        )
        .expect_err("pdf");
        assert!(matches!(err, PoetError::Unsupported(ref m) if m.contains("docx2pdf")));
        let err = export(
            &ctx,
            &ExportArgs {
                path: "x.docx".into(),
                fmt: "html".into(),
                out: None,
            },
        )
        .expect_err("html");
        assert!(
            matches!(err, PoetError::Validation(ref m) if m.contains("Unsupported export format"))
        );
    }

    #[test]
    fn export_opens_path_lazily_when_nothing_is_open() {
        let (ctx, dir) = setup();
        // Build and save a document first.
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        mgr.add_paragraph("lazy text", None, None, false)
            .expect("para");
        let doc_path = dir.join("lazy.docx");
        mgr.save("docx", &doc_path).expect("save");
        // Nothing is open in this context — export must open from disk.
        let out = dir.join("lazy.txt");
        let (_, err) = render(&export(
            &ctx,
            &ExportArgs {
                path: doc_path.to_string_lossy().into_owned(),
                fmt: "txt".into(),
                out: Some(out.to_string_lossy().into_owned()),
            },
        ));
        assert!(!err);
        assert!(
            ctx.doc.borrow().is_some(),
            "export leaves the doc open (Words)"
        );
        assert!(
            ctx.session.get().is_none(),
            "export does not touch the session"
        );
        let text = std::fs::read_to_string(&out).expect("txt");
        assert!(text.contains("lazy text"));
    }
}
