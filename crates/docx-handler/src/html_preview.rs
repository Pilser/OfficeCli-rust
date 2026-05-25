use handler_common::HandlerError;
use crate::dom_types::{WordDom, WordElementType};

/// Render the Word document as HTML for browser preview.
pub fn view_as_html(dom: &WordDom) -> Result<String, HandlerError> {
    let body = dom.body()
        .ok_or_else(|| HandlerError::OperationFailed("body element not found".to_string()))?;

    let mut html_body = String::new();
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for child in &body.children {
        let type_name_str = child.element_type.to_path_name();
        let type_name = type_name_str.to_string();
        let idx = *type_counts.entry(type_name).or_insert(0);
        *type_counts.get_mut(child.element_type.to_path_name()).unwrap() += 1;
        render_node_html(child, &mut html_body, &format!("/body/{}[{}]", type_name_str, idx));
    }

    Ok(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
body {{ font-family: "Segoe UI", Arial, sans-serif; max-width: 800px; margin: 40px auto; padding: 0 20px; line-height: 1.6; color: #333; }}
h1 {{ color: #1a1a1a; border-bottom: 2px solid #e0e0e0; }}
h2 {{ color: #2a2a2a; }}
h3 {{ color: #3a3a3a; }}
table {{ border-collapse: collapse; margin: 1em 0; }}
td, th {{ border: 1px solid #ccc; padding: 6px 10px; }}
th {{ background: #f5f5f5; font-weight: bold; }}
.annotation {{ color: #888; font-size: 0.85em; }}
</style>
</head>
<body>
{html_body}
</body>
</html>"#))
}

fn render_node_html(node: &crate::dom_types::WordNode, output: &mut String, path: &str) {
    match node.element_type {
        WordElementType::Paragraph => {
            let text = node.paragraph_text();
            if text.is_empty() { return; }

            let heading_level = node.heading_level();
            let tag = match heading_level {
                Some(0) => "h1",
                Some(1) => "h2",
                Some(2) => "h3",
                Some(3) => "h4",
                Some(4) => "h5",
                Some(5) => "h6",
                _ => "p",
            };

            let style = build_paragraph_style(node);
            let style_attr = if style.is_empty() { String::new() } else { format!(" style=\"{}\"", style) };

            let escaped = html_escape(&text);
            output.push_str(&format!("<{tag}{style_attr} data-path=\"{path}\">{escaped}</{tag}>\n"));
        }
        WordElementType::Table => {
            render_table_html(node, output, path);
        }
        _ => {
            // Generic: just render children
            let mut child_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for child in &node.children {
                let type_name_str = child.element_type.to_path_name();
                let type_name = type_name_str.to_string();
                let idx = *child_counts.entry(type_name).or_insert(0);
                *child_counts.get_mut(type_name_str).unwrap() += 1;
                let child_path = format!("{}/{}[{}]", path, type_name_str, idx);
                render_node_html(child, output, &child_path);
            }
        }
    }
}

fn render_table_html(table: &crate::dom_types::WordNode, output: &mut String, path: &str) {
    output.push_str(&format!("<table data-path=\"{path}\">\n"));

    let mut row_idx = 0;
    for child in &table.children {
        if child.element_type == WordElementType::TableRow {
            let is_header = row_idx == 0;
            if row_idx == 0 { output.push_str("<thead>\n"); }
            if row_idx == 1 { output.push_str("<tbody>\n"); }

            output.push_str("<tr>\n");
            for cell in &child.children {
                if cell.element_type == WordElementType::TableCell {
                    let text = cell.paragraph_text();
                    let cell_tag = if is_header { "th" } else { "td" };
                    output.push_str(&format!("<{cell_tag}>{}</{cell_tag}>\n", html_escape(&text)));
                }
            }
            output.push_str("</tr>\n");
            row_idx += 1;
        }
    }

    if row_idx > 0 { output.push_str("</tbody>\n"); }
    output.push_str("</table>\n");
}

fn build_paragraph_style(node: &crate::dom_types::WordNode) -> String {
    let mut parts = Vec::new();
    let mut has_bold = false;
    let mut has_italic = false;
    let mut has_underline = false;

    for child in &node.children {
        if child.element_type == WordElementType::Run {
            for (key, value) in &child.attributes {
                match key.as_ref() {
                    "bold" => has_bold = value != "0" && value != "false",
                    "italic" => has_italic = value != "0" && value != "false",
                    "underline" => has_underline = value != "0" && value != "false" && value != "none",
                    _ => {}
                }
            }
        }
    }

    if has_bold { parts.push("font-weight: bold"); }
    if has_italic { parts.push("font-style: italic"); }
    if has_underline { parts.push("text-decoration: underline"); }

    parts.join("; ")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}