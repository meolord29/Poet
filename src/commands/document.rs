//! Document commands: create/open/save/close/info (export lands in phase 2).
//!
//! Ported from Words' `commands/document.py`. The app layer owns the session
//! glue around these (persist-on-new, session save/delete) exactly like the
//! Python typer wrappers.

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

/// Arguments for `document export` (body lands in phase 2).
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

/// `document export` — implemented in phase 2 (`docs/plans/phase-2-content.md`).
pub fn export(_ctx: &Ctx, args: &ExportArgs) -> Result<Data, PoetError> {
    Err(PoetError::NotImplemented(format!(
        "document export {} (phase 2)",
        args.fmt
    )))
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
