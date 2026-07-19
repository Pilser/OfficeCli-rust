use clap::Args;
use handler_common::{HandlerError, OutputFormat, ViewOptions};

/// Display document content in various modes (text, outline, annotated, html, svg, pdf, forms)
#[derive(Args)]
pub struct ViewCommand {
    /// Document file path
    pub file: String,

    /// View mode: text, annotated, outline, stats, issues, html, svg, screenshot, pdf, forms
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

    /// Column filter, comma-separated (Excel only, e.g. A,B,C)
    #[arg(long)]
    pub cols: Option<String>,

    /// Page filter (e.g. 1, 2-5, 1,3,5). html mode: default=all. screenshot mode: default=1
    #[arg(long)]
    pub page: Option<String>,

    /// Open output in browser (html / svg modes)
    #[arg(long)]
    pub browser: bool,

    /// Output file path (screenshot mode; defaults to a temp file)
    #[arg(long, short)]
    pub out: Option<String>,

    /// Screenshot viewport width in pixels (default 1600)
    #[arg(long, default_value_t = 1600)]
    pub screenshot_width: u32,

    /// Screenshot viewport height in pixels (default 1200)
    #[arg(long, default_value_t = 1200)]
    pub screenshot_height: u32,

    /// Tile slides into an N-column thumbnail grid (screenshot mode, pptx only; 0 = off)
    #[arg(long, default_value_t = 0)]
    pub grid: u32,

    /// Screenshot rendering path (docx only): auto, native, html
    #[arg(long, default_value = "auto")]
    pub render: String,

    /// stats mode (docx only): also report total page count via Word repagination
    #[arg(long)]
    pub page_count: bool,

    /// Issue type filter (for issues mode)
    #[arg(long)]
    pub r#type: Option<String>,

    /// Limit number of results (for issues mode)
    #[arg(long)]
    pub limit: Option<usize>,

    /// Restrict output to a region (element path like "/slide[1]/shape[@id=2]" or xlsx cell range "Sheet1!A1:C3")
    #[arg(long)]
    pub range: Option<String>,

    /// Zoom factor for --range screenshots (e.g. "2x")
    #[arg(long)]
    pub zoom: Option<String>,

    /// Padding in pixels around cropped element
    #[arg(long, default_value_t = 0)]
    pub padding: u32,
}

pub fn handle_view(cmd: ViewCommand, format: OutputFormat) -> Result<String, HandlerError> {
    let handler = crate::open_handler(&cmd.file, false)?;

    // pdf mode: export to PDF via exporter plugin
    if cmd.mode.eq_ignore_ascii_case("pdf") {
        return handle_view_pdf(&cmd, format);
    }

    // If --range is specified and mode is screenshot, crop to element bbox
    if let Some(ref range) = cmd.range {
        if matches!(cmd.mode.as_str(), "screenshot" | "p") {
            return handle_screenshot_with_range(handler.as_ref(), &cmd, range);
        }
    }

    let opts = ViewOptions {
        start_line: cmd.start_line,
        end_line: cmd.end_line,
        max_lines: cmd.max_lines,
        cols: cmd
            .cols
            .as_ref()
            .map(|c| c.split(',').map(|s| s.to_string()).collect()),
        page: cmd.page.clone(),
        range: cmd.range.clone(),
        grid: cmd.grid,
        render: cmd.render.clone(),
    };

    let browser = cmd.browser;
    let mode = cmd.mode.as_str();
    let issue_type = cmd.r#type.as_deref();
    let issue_limit = cmd.limit;

    match format {
        OutputFormat::Text => match mode {
            "text" | "t" => handler.view_as_text(opts),
            "annotated" | "a" => handler.view_as_annotated(opts),
            "outline" | "o" => handler.view_as_outline(),
            "stats" | "s" => handler.view_as_stats(),
            "issues" | "i" => {
                let issues = handler.view_as_issues(issue_type, issue_limit)?;
                let lines: Vec<String> = issues
                    .iter()
                    .map(|i| format!("[{:?}] {}: {}", i.severity, i.issue_type, i.description))
                    .collect();
                Ok(lines.join("\n"))
            }
            "html" | "h" => {
                let html = handler.view_as_html(opts)?;
                if browser {
                    open_html_in_browser(&html, &cmd.file)?;
                }
                Ok(html)
            }
            "svg" | "g" => {
                let svg = handler.view_as_svg()?;
                if browser {
                    open_svg_in_browser(&svg, &cmd.file)?;
                }
                Ok(svg)
            }
            "screenshot" | "p" => handle_screenshot(handler.as_ref(), &cmd),
            "forms" | "f" => handler.view_as_forms(),
            other => Err(HandlerError::UnsupportedMode(format!(
                "view mode '{}' not supported. Available: text, annotated, outline, stats, issues, html, svg, screenshot, pdf, forms",
                other
            ))),
        },
        OutputFormat::Json => {
            let json_val = match mode {
                "text" | "t" => handler.view_as_text_json(opts)?,
                "outline" | "o" => handler.view_as_outline_json()?,
                "stats" | "s" => handler.view_as_stats_json()?,
                "issues" | "i" => {
                    let issues = handler.view_as_issues(issue_type, issue_limit)?;
                    serde_json::to_value(&issues)
                        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?
                }
                "annotated" | "a" => {
                    serde_json::json!({ "annotated": handler.view_as_annotated(opts)? })
                }
                "html" | "h" => {
                    serde_json::json!({ "html": handler.view_as_html(opts)? })
                }
                "svg" | "g" => {
                    serde_json::json!({ "svg": handler.view_as_svg()? })
                }
                "screenshot" | "p" => {
                    let result = handle_screenshot(handler.as_ref(), &cmd)?;
                    serde_json::json!({ "screenshot": result })
                }
                "forms" | "f" => {
                    let forms = handler.view_as_forms()?;
                    serde_json::json!({ "forms": forms })
                }
                other => {
                    return Err(HandlerError::UnsupportedMode(format!(
                        "view mode '{}' not supported. Available: text, annotated, outline, stats, issues, html, svg, screenshot, pdf, forms",
                        other
                    )))
                }
            };
            Ok(json_val.to_string())
        }
    }
}

/// Handle `view -m pdf` — export document to PDF.
fn handle_view_pdf(cmd: &ViewCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    let pdf_path = cmd.out.clone().unwrap_or_else(|| {
        std::path::Path::new(&cmd.file)
            .with_extension("pdf")
            .to_string_lossy()
            .to_string()
    });

    // For docx/pptx/xlsx: use the handler's HTML preview and a headless browser
    // to render to PDF. This mirrors the C# exporter plugin approach.
    let handler = crate::open_handler(&cmd.file, false)?;
    let opts = ViewOptions {
        page: cmd.page.clone(),
        ..Default::default()
    };
    let html = handler.view_as_html(opts)?;

    // Write HTML to temp file
    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join(format!(
        "officecli_pdf_{}.html",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    std::fs::write(&html_path, &html)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to write temp HTML: {}", e)))?;

    // Use headless browser to print to PDF
    let result = crate::screenshot::capture_pdf(
        &html_path.to_string_lossy(),
        &pdf_path,
        cmd.screenshot_width,
        cmd.screenshot_height,
    )
    .map_err(|e| HandlerError::OperationFailed(e))?;

    let _ = std::fs::remove_file(&html_path);

    Ok(format!("PDF exported: {}", result))
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
        page: cmd.page.clone(),
        range: cmd.range.clone(),
        grid: cmd.grid,
        render: cmd.render.clone(),
    };

    // Step 1: Render HTML (with optional grid tiling)
    let html = if cmd.grid > 0 {
        let stats = handler.view_as_stats_json()?;
        let page_count = stats
            .get("pages")
            .or(stats.get("slides"))
            .or(stats.get("sheets"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        wrap_in_grid(handler, page_count, cmd.grid)?
    } else {
        handler.view_as_html(opts)?
    };

    // Step 2: Write to temp file
    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join(format!(
        "officecli_screenshot_{}.html",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    std::fs::write(&html_path, &html)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to write temp HTML: {}", e)))?;

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

    Ok(format!(
        "Screenshot saved: {} (backend: {})",
        result.output_path, result.backend
    ))
}

/// Write HTML to a temp file and open in the default browser.
fn open_html_in_browser(html: &str, source_file: &str) -> Result<(), HandlerError> {
    let stem = std::path::Path::new(source_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("preview");
    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join(format!(
        "officecli_preview_{}_{}.html",
        stem,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    std::fs::write(&html_path, html)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to write temp HTML: {}", e)))?;
    println!("{}", html_path.display());
    open_path_in_browser(&html_path);
    Ok(())
}

/// Write SVG to a temp file and open in the default browser.
fn open_svg_in_browser(svg: &str, source_file: &str) -> Result<(), HandlerError> {
    let stem = std::path::Path::new(source_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("preview");
    let temp_dir = std::env::temp_dir();
    let svg_path = temp_dir.join(format!(
        "officecli_slide_{}_{}.svg",
        stem,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    std::fs::write(&svg_path, svg)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to write temp SVG: {}", e)))?;
    println!("{}", svg_path.display());
    open_path_in_browser(&svg_path);
    Ok(())
}

/// Open a file path in the system's default browser/viewer.
fn open_path_in_browser(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", &path.to_string_lossy()])
            .spawn();
    }
}

/// Handle `view -m screenshot --range <path>` — crop screenshot to a single element's bbox.
fn handle_screenshot_with_range(
    handler: &dyn handler_common::DocumentHandler,
    cmd: &ViewCommand,
    range: &str,
) -> Result<String, HandlerError> {
    // 1. Verify element exists
    let _node = handler.get(range, 0)?;

    // 2. Get text offset map for bbox
    let text_map = handler.extract_text_with_offsets()?;
    let spans = text_map.spans_for_path(range);

    // 3. Find first span with bbox
    let bbox = spans
        .iter()
        .find_map(|s| s.bbox.as_ref())
        .ok_or_else(|| HandlerError::OperationFailed("no bbox data for range".into()))?;

    // 4. Calculate zoom
    let zoom = parse_zoom(&cmd.zoom).unwrap_or(1.0);

    // 5. Generate HTML for just this element
    let html = handler.view_as_html(ViewOptions::default())?;

    let crop_w = (bbox.width * zoom + cmd.padding as f32 * 2.0) as u32;
    let crop_h = (bbox.height * zoom + cmd.padding as f32 * 2.0) as u32;

    let cropped_html = format!(
        r#"<div style="width:{}px;height:{}px;overflow:hidden;position:relative;">
            <div style="position:absolute;left:-{}px;top:-{}px;transform:scale({});transform-origin:top left;">
            {}
            </div>
        </div>"#,
        crop_w, crop_h, bbox.x, bbox.y, zoom, html
    );

    // 6. Write to temp and capture screenshot
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join(format!("officecli_range_{}.html", timestamp));
    std::fs::write(&html_path, &cropped_html)?;

    let out_path = cmd.out.clone().unwrap_or_else(|| {
        let stem = std::path::Path::new(&cmd.file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("range");
        temp_dir
            .join(format!("officecli_range_{}.png", stem))
            .to_string_lossy()
            .to_string()
    });

    let result = crate::screenshot::capture(&html_path.to_string_lossy(), &out_path, crop_w, crop_h)
        .map_err(|e| HandlerError::OperationFailed(e))?;

    let _ = std::fs::remove_file(&html_path);

    Ok(format!("Range screenshot saved: {}", result.output_path))
}

/// Wrap multiple pages/slides into a grid layout for --grid screenshots.
fn wrap_in_grid(
    handler: &dyn handler_common::DocumentHandler,
    page_count: u32,
    grid_cols: u32,
) -> Result<String, HandlerError> {
    let cols = grid_cols.max(1);
    let mut table = String::from("<table style=\"border-collapse:collapse;width:100%;\">");
    for i in 0..page_count {
        if i % cols == 0 {
            table.push_str("<tr>");
        }
        let page_opts = ViewOptions {
            page: Some((i + 1).to_string()),
            ..Default::default()
        };
        let page_html = handler.view_as_html(page_opts)?;
        table.push_str(&format!(
            "<td style=\"border:1px solid #ccc;vertical-align:top;padding:4px;\"><div class=\"tile\">{}</div></td>",
            page_html
        ));
        if i % cols == cols - 1 || i == page_count - 1 {
            table.push_str("</tr>");
        }
    }
    table.push_str("</table>");
    Ok(table)
}

/// Parse a zoom string like "2x" or "3.0" into a float multiplier.
fn parse_zoom(zoom: &Option<String>) -> Option<f32> {
    zoom.as_ref().and_then(|z| {
        if let Some(v) = z.strip_suffix('x') {
            v.parse::<f32>().ok()
        } else {
            z.parse::<f32>().ok()
        }
    })
}
