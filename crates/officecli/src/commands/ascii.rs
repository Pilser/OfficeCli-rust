use clap::Args;
use handler_common::{HandlerError, OutputFormat};

/// Render document layout as ASCII art in the terminal
#[derive(Args)]
pub struct AsciiCommand {
    /// Document file path
    pub file: String,

    /// Annotation mode: none, bbox, text
    #[arg(long, default_value = "none")]
    pub annotate: String,

    /// Terminal width in columns (default 80)
    #[arg(long, default_value_t = 80)]
    pub width: u32,

    /// Terminal height in rows (default 40)
    #[arg(long, default_value_t = 40)]
    pub height: u32,

    /// Compact mode (trim empty rows)
    #[arg(long)]
    pub compact: bool,
}

pub fn handle_ascii(cmd: AsciiCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    let handler = crate::open_handler(&cmd.file, false)?;

    let mode = match cmd.annotate.to_lowercase().as_str() {
        "bbox" | "b" => ascii_render::AnnotateMode::BBox,
        "text" | "t" => ascii_render::AnnotateMode::Text,
        _ => ascii_render::AnnotateMode::None,
    };

    let mut renderer = ascii_render::AsciiRenderer::new(cmd.width, cmd.height);
    renderer.set_compact(cmd.compact);
    renderer.render(handler.as_ref(), mode)
}
