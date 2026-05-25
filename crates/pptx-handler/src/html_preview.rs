use handler_common::HandlerError;
use oxml::OxmlPackage;

/// Render the PowerPoint presentation as HTML for browser preview.
pub fn view_as_html(package: &OxmlPackage) -> Result<String, HandlerError> {
    let presentation = crate::navigation::build_presentation(package)?;

    let mut slides_html = String::new();

    for (i, slide) in presentation.slides.iter().enumerate() {
        let slide_num = i + 1;
        slides_html.push_str(&format!(
            "<div class=\"slide\" data-path=\"/slide[{}]\">\n<div class=\"slide-number\">Slide {}</div>\n",
            slide_num, slide_num
        ));

        for (j, shape) in slide.shapes.iter().enumerate() {
            let shape_id = j + 1;
            let shape_path = format!("/slide[{}]/shape[{}]", slide_num, shape_id);

            if !shape.text.is_empty() {
                let text = &shape.text;
                let escaped = html_escape(text);
                slides_html.push_str(&format!(
                    "<div class=\"shape\" data-path=\"{}\">{}</div>\n",
                    shape_path, escaped
                ));
            }
        }

        slides_html.push_str("</div>\n");
    }

    Ok(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
body {{ font-family: "Segoe UI", Arial, sans-serif; margin: 20px; background: #f5f5f5; }}
.slide {{ background: white; border: 1px solid #ddd; margin: 20px auto; max-width: 800px; padding: 40px; min-height: 200px; }}
.slide-number {{ color: #888; font-size: 0.8em; margin-bottom: 10px; }}
.shape {{ margin: 10px 0; padding: 8px; }}
h1 {{ text-align: center; }}
</style>
</head>
<body>
<h1>PowerPoint Preview</h1>
{}
</body>
</html>"#, slides_html))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}