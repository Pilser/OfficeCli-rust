use handler_common::{HandlerError, ViewOptions};

pub fn view_as_html(svg: &str, opts: &ViewOptions) -> Result<String, HandlerError> {
    let title = "SVG Preview";

    let highlight_css = if let Some(range) = &opts.range {
        format!(
            "  [data-path=\"{}\"] {{ outline: 3px solid #ff6b6b; outline-offset: 2px; }}\n",
            html_escape(range)
        )
    } else {
        String::new()
    };

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
  body {{ margin: 0; padding: 20px; background: #f5f5f5; display: flex; flex-direction: column; align-items: center; }}
  .svg-container {{ max-width: 100%; overflow-x: auto; background: white; box-shadow: 0 2px 8px rgba(0,0,0,0.1); border-radius: 4px; padding: 10px; }}
  .svg-container svg {{ max-width: 100%; height: auto; display: block; }}
{highlight_css}</style>
</head>
<body>
<div class="svg-container">
{svg}
</div>
</body>
</html>"#,
        title = title,
        highlight_css = highlight_css,
        svg = svg,
    ))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_as_html_basic() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect x="10" y="10" width="50" height="50"/></svg>"#;
        let result = view_as_html(svg, &ViewOptions::default()).unwrap();
        assert!(result.contains("<!DOCTYPE html>"));
        assert!(result.contains("<svg"));
        assert!(result.contains("max-width: 100%"));
    }

    #[test]
    fn test_view_as_html_with_range_highlight() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect x="10" y="10" width="50" height="50"/></svg>"#;
        let opts = ViewOptions {
            range: Some("/svg/rect[1]".to_string()),
            ..ViewOptions::default()
        };
        let result = view_as_html(svg, &opts).unwrap();
        assert!(result.contains("data-path=\"/svg/rect[1]\""));
    }
}
