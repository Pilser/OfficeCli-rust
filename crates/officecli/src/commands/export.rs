use clap::Args;
use handler_common::{HandlerError, OutputFormat};

/// Export document to PDF (or other formats via rendering pipeline)
#[derive(Args)]
pub struct ExportCommand {
    /// Document file path
    pub file: String,

    /// Output format (default: pdf)
    #[arg(long, default_value = "pdf")]
    pub format: String,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Paper size: A4, Letter, Legal
    #[arg(long)]
    pub paper: Option<String>,

    /// Margin in points (default: 72)
    #[arg(long)]
    pub margin: Option<String>,
}

pub fn handle_export(cmd: ExportCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    let out_path = cmd.output.unwrap_or_else(|| {
        let p = std::path::Path::new(&cmd.file);
        p.with_extension("pdf")
            .to_string_lossy()
            .to_string()
    });

    let handler = crate::open_handler(&cmd.file, false)?;

    use layout_engine::LayoutEngine;

    let (page_w, page_h) = match cmd.paper.as_deref() {
        Some("letter") => (612.0, 792.0),
        Some("legal") => (612.0, 1008.0),
        _ => (595.0, 842.0),
    };
    let margin: f32 = cmd
        .margin
        .as_deref()
        .and_then(|m| m.parse().ok())
        .unwrap_or(72.0);

    let engine = LayoutEngine::new(page_w, page_h, margin);
    let pages = engine
        .layout(handler.as_ref())
        .map_err(|e| HandlerError::OperationFailed(e))?;

    let opts = render_pdf::PdfOptions {
        paper_size: cmd.paper.unwrap_or_else(|| "A4".to_string()),
        margin: margin.to_string(),
        embed_fonts: false,
    };

    let pdf_bytes = render_pdf::PdfRenderer
        .render_document(&pages, &opts)
        .map_err(|e| HandlerError::OperationFailed(e))?;

    std::fs::write(&out_path, &pdf_bytes)
        .map_err(|e| HandlerError::OperationFailed(format!("write: {}", e)))?;

    Ok(format!("Exported PDF: {}", out_path))
}
