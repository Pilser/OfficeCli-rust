use handler_common::HandlerError;
use crate::reader::PdfReader;

/// PDF rendering — converts page text content to SVG for basic preview.
/// Full rasterization (PNG) requires external tools like poppler/mutool.
pub struct PdfRenderer;

impl PdfRenderer {
    /// Render a PDF page to PNG bytes.
    /// This requires an external tool (poppler/mutool) — returns an error if not available.
    pub fn render_page_to_png(path: &str, page: usize) -> Result<Vec<u8>, HandlerError> {
        // Try using mutool (muPDF command-line tool) if available
        let output = std::process::Command::new("mutool")
            .args(["draw", "-F", "png", "-o", "-", "-r", "150", path, &page.to_string()])
            .output();

        match output {
            Ok(result) if result.status.success() => Ok(result.stdout),
            Ok(result) => Err(HandlerError::OperationFailed(
                format!("mutool failed: {}", String::from_utf8_lossy(&result.stderr))
            )),
            Err(_) => Err(HandlerError::UnsupportedMode(
                "PNG rendering requires 'mutool' (muPDF tools) — install with: brew install mupdf-tools".to_string()
            )),
        }
    }

    /// Render a PDF page to a basic SVG preview using extracted text.
    /// This creates a text-based SVG that preserves text content and layout position hints.
    pub fn render_page_to_svg(path: &str, page: usize) -> Result<String, HandlerError> {
        let reader = PdfReader::open(path)?;
        let page_text = reader.extract_page_text(page).unwrap_or_default();

        let mut svg = String::new();
        svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 612 792\" width=\"612\" height=\"792\">\n");

        // Background
        svg.push_str("  <rect width=\"612\" height=\"792\" fill=\"white\"/>\n");

        // Render text lines at estimated positions
        let mut y_pos = 40.0;
        let line_height = 14.0;
        for line in page_text.lines() {
            // Escape XML special characters
            let escaped = line.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");

            svg.push_str(&format!(
                "  <text x=\"30\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"12\" fill=\"black\">{}</text>\n",
                y_pos, escaped
            ));
            y_pos += line_height;
        }

        // If no text, show placeholder
        if page_text.trim().is_empty() {
            svg.push_str("  <text x=\"306\" y=\"396\" font-family=\"Helvetica\" font-size=\"14\" fill=\"#999\" text-anchor=\"middle\">(No extractable text)</text>\n");
        }

        // Page number footer
        svg.push_str(&format!(
            "  <text x=\"306\" y=\"770\" font-family=\"Helvetica\" font-size=\"10\" fill=\"#999\" text-anchor=\"middle\">Page {}</text>\n",
            page
        ));

        svg.push_str("</svg>");
        Ok(svg)
    }
}