//! Image commands — ported from Words' `commands/image.py`; panic-free
//! decoding per adr/0008.

use clap::Args;

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::Data;

/// Arguments for `image add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Path to the image file.
    pub path: String,
    /// Width in inches.
    #[arg(long)]
    pub width: Option<f64>,
    /// Height in inches.
    #[arg(long)]
    pub height: Option<f64>,
    /// Bookmark id.
    #[arg(long)]
    pub id: Option<String>,
}

/// Arguments for `image list`.
#[derive(Debug, Args)]
pub struct ListArgs {}

/// Arguments for `image get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Image index.
    #[arg(default_value_t = 0)]
    pub index: usize,
}

/// Arguments for `image resize`.
#[derive(Debug, Args)]
pub struct ResizeArgs {
    /// Image index.
    pub index: usize,
    /// Width in inches.
    #[arg(long)]
    pub width: Option<f64>,
    /// Height in inches.
    #[arg(long)]
    pub height: Option<f64>,
}

/// Arguments for `image delete`.
#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Image index.
    pub index: usize,
}

/// All `image` actions.
#[derive(Debug, clap::Subcommand)]
pub enum ImageAction {
    /// Add an image.
    Add(AddArgs),
    /// List images.
    List(ListArgs),
    /// Get image details.
    Get(GetArgs),
    /// Resize an image.
    Resize(ResizeArgs),
    /// Delete an image.
    Delete(DeleteArgs),
}

/// `image add` — host the picture in a new bookmarked paragraph.
pub fn add(ctx: &Ctx, args: &AddArgs) -> Result<Data, PoetError> {
    let id = crate::commands::with_doc(ctx, |mgr| {
        mgr.add_image(&args.path, args.width, args.height, args.id.as_deref())
    })?;
    Ok(Data::ImageAdded {
        id,
        path: args.path.clone(),
        width: args.width,
        height: args.height,
        message: "Image added".into(),
    })
}

/// `image list`.
pub fn list(ctx: &Ctx, _args: &ListArgs) -> Result<Data, PoetError> {
    let images = crate::commands::with_doc(ctx, |mgr| mgr.images())?;
    let count = images.len();
    Ok(Data::ImageList { images, count })
}

/// `image get` — one image's geometry (index defaults to 0).
pub fn get(ctx: &Ctx, args: &GetArgs) -> Result<Data, PoetError> {
    let images = crate::commands::with_doc(ctx, |mgr| mgr.images())?;
    let image = images
        .into_iter()
        .nth(args.index)
        .ok_or_else(|| PoetError::NotFound(format!("Image index {} out of range", args.index)))?;
    Ok(Data::ImageGot(image))
}

/// `image resize` — set width/height (inches).
pub fn resize(ctx: &Ctx, args: &ResizeArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| {
        mgr.resize_image(args.index, args.width, args.height)
    })?;
    Ok(Data::ImageResized {
        index: args.index,
        width: args.width,
        height: args.height,
        message: "Image resized".into(),
    })
}

/// `image delete` — remove the hosting paragraph.
pub fn delete(ctx: &Ctx, args: &DeleteArgs) -> Result<Data, PoetError> {
    crate::commands::with_doc(ctx, |mgr| mgr.delete_image(args.index))?;
    Ok(Data::ImageDeleted {
        index: args.index,
        message: "Image deleted".into(),
    })
}

#[cfg(test)]
mod tests {
    use crate::commands::testutil::setup;
    use crate::core::output::render;

    use super::*;

    fn open_doc(ctx: &Ctx) {
        let mut mgr = crate::core::document::DocumentManager::new();
        mgr.create("docx").expect("create");
        *ctx.doc.borrow_mut() = Some(mgr);
    }

    fn png(dir: &std::path::Path) -> String {
        let path = dir.join("img.png");
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(4, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .expect("png");
        std::fs::write(&path, png.into_inner()).expect("write");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn image_add_list_get_resize_delete_flow() {
        let (ctx, dir) = setup();
        open_doc(&ctx);
        let path = png(&dir);
        let (json, err) = render(&add(
            &ctx,
            &AddArgs {
                path: path.clone(),
                width: Some(2.0),
                height: None,
                id: None,
            },
        ));
        assert!(!err);
        assert!(json.contains("\"message\": \"Image added\""));
        assert!(json.contains("\"id\": \"img1\""));
        assert!(json.contains("\"width\": 2.0"));
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"count\": 1"));
        assert!(json.contains("\"width_inches\""));
        let (json, _) = render(&get(&ctx, &GetArgs { index: 0 }));
        assert!(json.contains("\"index\": 0"));
        let (json, _) = render(&resize(
            &ctx,
            &ResizeArgs {
                index: 0,
                width: Some(1.0),
                height: Some(0.5),
            },
        ));
        assert!(json.contains("Image resized"));
        let (json, err) = render(&get(&ctx, &GetArgs { index: 3 }));
        assert!(err, "out-of-range get");
        assert!(json.contains("out of range"));
        let (json, err) = render(&delete(&ctx, &DeleteArgs { index: 0 }));
        assert!(!err);
        assert!(json.contains("Image deleted"));
        let (json, _) = render(&list(&ctx, &ListArgs {}));
        assert!(json.contains("\"count\": 0"));
    }

    #[test]
    fn image_add_missing_file_is_not_found() {
        let (ctx, _dir) = setup();
        open_doc(&ctx);
        let err = add(
            &ctx,
            &AddArgs {
                path: "/nonexistent/x.png".into(),
                width: None,
                height: None,
                id: None,
            },
        )
        .expect_err("missing");
        assert!(matches!(err, PoetError::NotFound(_)));
    }
}
