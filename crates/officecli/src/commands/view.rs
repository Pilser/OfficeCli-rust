use clap::Args;
use handler_common::{HandlerError, OutputFormat, ViewOptions};

/// Display document content in various modes (text, outline, annotated, html, svg)
#[derive(Args)]
pub struct ViewCommand {
    /// Document file path
    pub file: String,

    /// View mode: text, annotated, outline, stats, issues, html, svg, screenshot, pdf
    #[arg(short, long, default_value = "text")]
    pub mode: String,

    /// Start line number
    #[arg(long)]
    pub start_line: Option<usize>,

    /// End line number
    #[arg(long)]
    pub end_line: Option<usize>,

    /// Max lines to display
    #[arg(long)]
    pub max_lines: Option<usize>,

    /// Column filter (for Excel)
    #[arg(long)]
    pub cols: Option<String>,

    /// Page number (for PDF / slide number for PowerPoint)
    #[arg(long)]
    pub page: Option<usize>,

    /// Output file path (screenshot mode; defaults to a temp file)
    #[arg(long, short)]
    pub out: Option<String>,

    /// Screenshot viewport width in pixels (default 1600)
    #[arg(long, default_value_t = 1600)]
    pub screenshot_width: u32,

    /// Screenshot viewport height in pixels (default 1200)
    #[arg(long, default_value_t = 1200)]
    pub screenshot_height: u32,
}

pub fn handle_view(cmd: ViewCommand, format: OutputFormat) -> Result<String, HandlerError> {
    let handler = crate::open_handler(&cmd.file, false)?;
    let opts = ViewOptions {
        start_line: cmd.start_line,
        end_line: cmd.end_line,
        max_lines: cmd.max_lines,
        cols: cmd
            .cols
            .as_ref()
            .map(|c| c.split(',').map(|s| s.to_string()).collect()),
        page: cmd.page,
    };

    match format {
        OutputFormat::Text => match cmd.mode.as_str() {
            "text" => handler.view_as_text(opts),
            "annotated" => handler.view_as_annotated(opts),
            "outline" => handler.view_as_outline(),
            "stats" => handler.view_as_stats(),
            "issues" => {
                let issues = handler.view_as_issues(None, None)?;
                let lines: Vec<String> = issues
                    .iter()
                    .map(|i| format!("[{:?}] {}: {}", i.severity, i.issue_type, i.description))
                    .collect();
                Ok(lines.join("\n"))
            }
            "html" => handler.view_as_html(opts),
            "svg" => handler.view_as_svg(),
            "screenshot" => handle_screenshot(handler.as_ref(), &cmd),
            other => Err(HandlerError::UnsupportedMode(format!(
                "view mode '{}' not supported by this format",
                other
            ))),
        },
        OutputFormat::Json => {
            let json_val = match cmd.mode.as_str() {
                "text" => handler.view_as_text_json(opts)?,
                "outline" => handler.view_as_outline_json()?,
                "stats" => handler.view_as_stats_json()?,
                "issues" => {
                    let issues = handler.view_as_issues(None, None)?;
                    serde_json::to_value(&issues)
                        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?
                }
                "annotated" => {
                    serde_json::json!({ "annotated": handler.view_as_annotated(opts)? })
                }
                "html" => {
                    serde_json::json!({ "html": handler.view_as_html(opts)? })
                }
                "svg" => {
                    serde_json::json!({ "svg": handler.view_as_svg()? })
                }
                "screenshot" => {
                    let result = handle_screenshot(handler.as_ref(), &cmd)?;
                    serde_json::json!({ "screenshot": result })
                }
                other => {
                    return Err(HandlerError::UnsupportedMode(format!(
                        "view mode '{}' not supported by this format",
                        other
                    )))
                }
            };
            Ok(json_val.to_string())
        }
    }
}

/// Handle `view -m screenshot` — render HTML, then capture to PNG via headless browser.
fn handle_screenshot(
    handler: &dyn handler_common::DocumentHandler,
    cmd: &ViewCommand,
) -> Result<String, HandlerError> {
    let opts = ViewOptions {
        start_line: cmd.start_line,
        end_line: cmd.end_line,
        max_lines: cmd.max_lines,
        cols: cmd
            .cols
            .as_ref()
            .map(|c| c.split(',').map(|s| s.to_string()).collect()),
        page: cmd.page,
    };

    // Step 1: Render HTML
    let html = handler.view_as_html(opts)?;

    // Step 2: Write to temp file
    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join(format!(
        "officecli_screenshot_{}.html",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    std::fs::write(&html_path, &html).map_err(|e| {
        HandlerError::OperationFailed(format!("failed to write temp HTML: {}", e))
    })?;

    // Step 3: Determine output path
    let out_path = cmd.out.clone().unwrap_or_else(|| {
        let stem = std::path::Path::new(&cmd.file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("screenshot");
        temp_dir
            .join(format!("officecli_screenshot_{}.png", stem))
            .to_string_lossy()
            .to_string()
    });

    // Step 4: Capture screenshot
    let result = crate::screenshot::capture(
        &html_path.to_string_lossy(),
        &out_path,
        cmd.screenshot_width,
        cmd.screenshot_height,
    )
    .map_err(|e| HandlerError::OperationFailed(e))?;

    // Clean up temp HTML
    let _ = std::fs::remove_file(&html_path);

    Ok(format!("Screenshot saved: {} (backend: {})", result.output_path, result.backend))
}
