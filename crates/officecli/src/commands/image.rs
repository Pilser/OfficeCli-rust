use clap::{Args, Subcommand};
use handler_common::{HandlerError, OutputFormat};

#[derive(Args)]
pub struct ImageCommand {
    #[command(subcommand)]
    pub action: ImageAction,
}

#[derive(Subcommand)]
pub enum ImageAction {
    /// Extract all images from a document
    Extract {
        /// Document file path
        file: String,
        /// Output directory for extracted images
        output: String,
    },
    /// Compress images in a document
    Compress {
        /// Document file path
        file: String,
        /// JPEG quality (1-100, default 85)
        quality: Option<u8>,
    },
    /// Resize an image file
    Resize {
        /// Input image file path
        input: String,
        /// Output image file path
        output: String,
        /// Target width in pixels
        width: u32,
        /// Target height in pixels
        height: u32,
    },
    /// Convert image to another format
    Convert {
        /// Input image file path
        input: String,
        /// Output image file path
        output: String,
        /// Target format (png, jpeg, gif, webp, bmp)
        format: String,
    },
    /// Get image info (dimensions, format)
    Info {
        /// Image file path
        path: String,
    },
}

pub fn handle_image(cmd: ImageCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    match cmd.action {
        ImageAction::Extract { file, output } => {
            std::fs::create_dir_all(&output)
                .map_err(|e| HandlerError::OperationFailed(format!("create dir: {}", e)))?;
            let handler = crate::open_handler(&file, false)?;
            let result = image_toolkit::extract_images(handler.as_ref(), &output)
                .map_err(|e| HandlerError::OperationFailed(e))?;
            Ok(format!("Extracted {} image(s) to {}", result.len(), output))
        }
        ImageAction::Compress { file, quality } => {
            let handler = crate::open_handler(&file, true)?;
            image_toolkit::compress_images(handler.as_ref(), quality.unwrap_or(85))
                .map_err(|e| HandlerError::OperationFailed(e))?;
            handler.save()?;
            Ok(format!("Compressed images in {}", file))
        }
        ImageAction::Resize {
            input,
            output,
            width,
            height,
        } => {
            let img = image::open(&input)
                .map_err(|e| HandlerError::OperationFailed(format!("open: {}", e)))?;
            let resized =
                img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
            resized
                .save(&output)
                .map_err(|e| HandlerError::OperationFailed(format!("save: {}", e)))?;
            Ok(format!(
                "Resized {} → {} ({}x{})",
                input, output, width, height
            ))
        }
        ImageAction::Convert {
            input,
            output,
            format,
        } => {
            let img = image::open(&input)
                .map_err(|e| HandlerError::OperationFailed(format!("open: {}", e)))?;
            match format.to_lowercase().as_str() {
                "png" => img.save(&output).map_err(|e| HandlerError::OperationFailed(format!("save: {}", e))),
                "jpeg" | "jpg" => {
                    let mut buf = std::io::Cursor::new(Vec::new());
                    let encoder =
                        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
                    img.write_with_encoder(encoder)
                        .map_err(|e| HandlerError::OperationFailed(format!("encode: {}", e)))?;
                    std::fs::write(&output, buf.into_inner())
                        .map_err(|e| HandlerError::OperationFailed(format!("write: {}", e)))
                }
                "gif" => img.save(&output).map_err(|e| HandlerError::OperationFailed(format!("save: {}", e))),
                "webp" => img.save(&output).map_err(|e| HandlerError::OperationFailed(format!("save: {}", e))),
                "bmp" => img.save(&output).map_err(|e| HandlerError::OperationFailed(format!("save: {}", e))),
                _ => {
                    return Err(HandlerError::InvalidArgument(format!(
                        "unsupported format: {}",
                        format
                    )))
                }
            }
            .map_err(|e| HandlerError::OperationFailed(format!("save: {}", e)))?;
            Ok(format!("Converted {} → {} as {}", input, output, format))
        }
        ImageAction::Info { path } => {
            let img = image::open(&path)
                .map_err(|e| HandlerError::OperationFailed(format!("open: {}", e)))?;
            Ok(format!(
                "{}: {}x{} {:?}",
                path,
                img.width(),
                img.height(),
                img.color()
            ))
        }
    }
}
