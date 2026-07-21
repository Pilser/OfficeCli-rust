use crate::reader::PdfReader;
use handler_common::HandlerError;

/// PDF rendering — converts page text content to SVG for basic preview.
/// Full rasterization (PNG) via resvg (native) or mutool (fallback).
pub struct PdfRenderer;

impl PdfRenderer {
    /// Render a PDF page to PNG bytes.
    /// Tries native rendering first (resvg-based, requires feature "png"),
    /// falls back to mutool CLI.
    pub fn render_page_to_png(
        reader: &PdfReader,
        file_path: &str,
        page_num: usize,
        scale: f32,
    ) -> Result<Vec<u8>, HandlerError> {
        #[cfg(feature = "png")]
        {
            match Self::render_page_to_png_native(reader, page_num, scale) {
                Ok(bytes) => return Ok(bytes),
                Err(_) => {}
            }
        }
        let _ = (reader, scale);
        Self::render_page_to_png_mutool(file_path, page_num)
    }

    /// Render a single page to PNG using resvg (native Rust rendering).
    /// Requires feature "png".
    #[cfg(feature = "png")]
    pub fn render_page_to_png_native(
        reader: &PdfReader,
        page_num: usize,
        scale: f32,
    ) -> Result<Vec<u8>, HandlerError> {
        let svg_content = Self::build_page_svg(reader, page_num)?;

        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_str(&svg_content, &opt)
            .map_err(|e| HandlerError::OperationFailed(format!("usvg parse: {}", e)))?;

        let size = tree.size();
        let w = (size.width() as f32 * scale) as u32;
        let h = (size.height() as f32 * scale) as u32;

        let mut pixmap = tiny_skia::Pixmap::new(w.max(1), h.max(1))
            .ok_or_else(|| HandlerError::OperationFailed("failed to create pixmap".into()))?;

        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        pixmap
            .encode_png()
            .map_err(|e| HandlerError::OperationFailed(format!("encode png: {}", e)))
    }

    /// Render a single page to PNG using mutool CLI (fallback).
    pub fn render_page_to_png_mutool(
        file_path: &str,
        page_num: usize,
    ) -> Result<Vec<u8>, HandlerError> {
        let output = std::process::Command::new("mutool")
            .args([
                "draw",
                "-F",
                "png",
                "-o",
                "-",
                "-r",
                "150",
                file_path,
                &page_num.to_string(),
            ])
            .output();

        match output {
            Ok(result) if result.status.success() => Ok(result.stdout),
            Ok(result) => Err(HandlerError::OperationFailed(format!(
                "mutool failed: {}",
                String::from_utf8_lossy(&result.stderr)
            ))),
            Err(_) => Err(HandlerError::UnsupportedMode(
                "PNG rendering requires 'mutool' (muPDF tools) — install with: brew install mupdf-tools"
                    .to_string(),
            )),
        }
    }

    /// Render a PDF page to a basic SVG preview using extracted text.
    pub fn render_page_to_svg(path: &str, page: usize) -> Result<String, HandlerError> {
        let reader = PdfReader::open(path)?;
        Self::build_page_svg(&reader, page)
    }

    /// Render all PDF pages as a single concatenated SVG document.
    pub fn render_all_pages_to_svg(
        path: &str,
        page_count: usize,
    ) -> Result<String, HandlerError> {
        let reader = PdfReader::open(path)?;
        Self::build_all_pages_svg(&reader, page_count)
    }

    /// Internal: build SVG for a single page from an already-opened reader.
    pub(crate) fn build_page_svg(
        reader: &PdfReader,
        page: usize,
    ) -> Result<String, HandlerError> {
        let page_height = get_page_height(reader, page).unwrap_or(792.0);

        let mut svg = String::new();
        svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 612 {:.0}\" width=\"612\" height=\"{:.0}\">\n",
            page_height, page_height
        ));

        svg.push_str(&format!(
            "  <rect width=\"612\" height=\"{:.0}\" fill=\"white\"/>\n",
            page_height
        ));

        if let Some(parsed) = reader.parse_page_text_blocks(page) {
            for block in &parsed.text_blocks {
                let bbox = &block.bbox;
                let svg_x = bbox.x;
                let svg_y = page_height - bbox.y;

                let escaped = block
                    .text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;");

                let font_family = block.style.font_name.as_deref().unwrap_or("Helvetica");
                let font_size = block.style.font_size.unwrap_or(12.0);

                let fill_color = block
                    .style
                    .fill_color
                    .as_ref()
                    .map(|c| match c {
                        crate::content_stream::PdfColor::Gray(g) => {
                            let v = (g * 255.0) as u8;
                            format!("rgb({},{},{})", v, v, v)
                        }
                        crate::content_stream::PdfColor::Rgb(r, g, b) => {
                            format!(
                                "rgb({},{},{})",
                                (r * 255.0) as u8,
                                (g * 255.0) as u8,
                                (b * 255.0) as u8
                            )
                        }
                        crate::content_stream::PdfColor::Cmyk(c, m, y, k) => {
                            let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
                            let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
                            let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;
                            format!("rgb({},{},{})", r, g, b)
                        }
                    })
                    .unwrap_or("black".to_string());

                svg.push_str(&format!(
                    "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{}\" font-size=\"{:.0}\" fill=\"{}\" data-path=\"/page[{}]/text[{}]\">{}</text>\n",
                    svg_x, svg_y, font_family, font_size, fill_color, page, block.index, escaped
                ));
            }

            if parsed.text_blocks.is_empty() {
                svg.push_str(&format!(
                    "  <text x=\"306\" y=\"{:.0}\" font-family=\"Helvetica\" font-size=\"14\" fill=\"#999\" text-anchor=\"middle\">(No extractable text)</text>\n",
                    page_height / 2.0
                ));
            }
        } else {
            svg.push_str(&format!(
                "  <text x=\"306\" y=\"{:.0}\" font-family=\"Helvetica\" font-size=\"14\" fill=\"#999\" text-anchor=\"middle\">(No extractable text)</text>\n",
                page_height / 2.0
            ));
        }

        svg.push_str(&format!(
            "  <text x=\"306\" y=\"{:.0}\" font-family=\"Helvetica\" font-size=\"10\" fill=\"#999\" text-anchor=\"middle\">Page {}</text>\n",
            page_height - 22.0, page
        ));

        svg.push_str("</svg>");
        Ok(svg)
    }

    /// Internal: build concatenated SVG for all pages from an already-opened reader.
    fn build_all_pages_svg(
        reader: &PdfReader,
        page_count: usize,
    ) -> Result<String, HandlerError> {
        let default_height = 792.0;

        let mut heights = Vec::with_capacity(page_count);
        let mut total_height = 0.0;
        let page_width = 612.0;
        let page_gap = 30.0;

        for p in 1..=page_count {
            let h = get_page_height(reader, p).unwrap_or(default_height);
            heights.push(h);
            total_height += h + page_gap;
        }

        let mut svg = String::new();
        svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {:.0} {:.0}\" width=\"{:.0}\" height=\"{:.0}\">\n",
            page_width, total_height, page_width, total_height
        ));

        svg.push_str(&format!(
            "  <rect width=\"{:.0}\" height=\"{:.0}\" fill=\"#f0f0f0\"/>\n",
            page_width, total_height
        ));

        let mut y_offset = 0.0;
        for p in 1..=page_count {
            let page_height = heights[p - 1];

            svg.push_str(&format!(
                "  <rect x=\"0\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\"/>\n",
                y_offset, page_width, page_height
            ));

            if let Some(parsed) = reader.parse_page_text_blocks(p) {
                for block in &parsed.text_blocks {
                    let bbox = &block.bbox;
                    let svg_x = bbox.x;
                    let svg_y = y_offset + page_height - bbox.y;

                    let escaped = block
                        .text
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                        .replace('"', "&quot;");

                    let font_family = block.style.font_name.as_deref().unwrap_or("Helvetica");
                    let font_size = block.style.font_size.unwrap_or(12.0);

                    let fill_color = block
                        .style
                        .fill_color
                        .as_ref()
                        .map(|c| match c {
                            crate::content_stream::PdfColor::Gray(g) => {
                                let v = (g * 255.0) as u8;
                                format!("rgb({},{},{})", v, v, v)
                            }
                            crate::content_stream::PdfColor::Rgb(r, g, b) => {
                                format!(
                                    "rgb({},{},{})",
                                    (r * 255.0) as u8,
                                    (g * 255.0) as u8,
                                    (b * 255.0) as u8
                                )
                            }
                            crate::content_stream::PdfColor::Cmyk(c, m, y, k) => {
                                let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
                                let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
                                let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;
                                format!("rgb({},{},{})", r, g, b)
                            }
                        })
                        .unwrap_or("black".to_string());

                    svg.push_str(&format!(
                        "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{}\" font-size=\"{:.0}\" fill=\"{}\" data-path=\"/page[{}]/text[{}]\">{}</text>\n",
                        svg_x, svg_y, font_family, font_size, fill_color, p, block.index, escaped
                    ));
                }
            }

            svg.push_str(&format!(
                "  <text x=\"306\" y=\"{:.0}\" font-family=\"Helvetica\" font-size=\"10\" fill=\"#999\" text-anchor=\"middle\">Page {}</text>\n",
                y_offset + page_height - 12.0, p
            ));

            y_offset += page_height + page_gap;
        }

        svg.push_str("</svg>");
        Ok(svg)
    }
}

/// Extract page height from /MediaBox.
fn get_page_height(reader: &PdfReader, page_num: usize) -> Option<f32> {
    let pages = reader.document().get_pages();
    let page_id = pages.get(&(page_num as u32))?;
    let page_obj = reader.document().get_object(*page_id).ok()?;
    let dict = page_obj.as_dict().ok()?;
    let media_box = dict.get(b"MediaBox").ok()?;
    if let lopdf::Object::Array(arr) = media_box {
        if arr.len() >= 4 {
            arr.get(3).and_then(|h| h.as_float().ok())
        } else {
            None
        }
    } else {
        None
    }
}
