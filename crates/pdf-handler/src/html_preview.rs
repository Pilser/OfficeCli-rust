use handler_common::HandlerError;
use crate::reader::PdfReader;

/// Render the PDF document as HTML for browser preview.
pub fn view_as_html(reader: &PdfReader) -> Result<String, HandlerError> {
    let mut pages_html = String::new();

    for i in 1..=reader.page_count() {
        let page_text = reader.extract_page_text(i).unwrap_or_default();
        let escaped = html_escape(&page_text);

        pages_html.push_str(&format!(
            "<div class=\"page\" data-path=\"/page[{}]\">\n<div class=\"page-number\">Page {}</div>\n<pre>{}</pre>\n</div>\n",
            i, i, escaped
        ));
    }

    Ok(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
body {{ font-family: "Segoe UI", Arial, sans-serif; margin: 20px; background: #f5f5f5; }}
.page {{ background: white; border: 1px solid #ddd; margin: 20px auto; max-width: 800px; padding: 40px; }}
.page-number {{ color: #888; font-size: 0.8em; margin-bottom: 10px; }}
pre {{ white-space: pre-wrap; line-height: 1.5; }}
h1 {{ text-align: center; }}
</style>
</head>
<body>
<h1>PDF Preview</h1>
{}
</body>
</html>"#, pages_html))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}