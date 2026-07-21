use handler_common::*;
use lo_core::draw::{DrawElement, Drawing};
use lo_core::impress::ShapeKind;

fn apply_line_options(text: &str, opts: &ViewOptions) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let start = opts.start_line.unwrap_or(0);
    let end = opts.end_line.unwrap_or(total).min(total);
    let end = match opts.max_lines {
        Some(max) => (start + max).min(end),
        None => end,
    };
    if start >= end {
        return String::new();
    }
    lines[start..end].join("\n")
}

fn element_type_str(element: &DrawElement) -> &str {
    match element {
        DrawElement::TextBox(_) => "text-box",
        DrawElement::Shape(s) => match s.kind {
            ShapeKind::Rectangle => "rect",
            ShapeKind::Ellipse => "ellipse",
            ShapeKind::Line => "line",
        },
        DrawElement::Image(_) => "image",
    }
}

fn extract_text_from_element(element: &DrawElement) -> Option<String> {
    match element {
        DrawElement::TextBox(tb) => {
            let trimmed = tb.text.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        }
        _ => None,
    }
}

fn element_frame_info(element: &DrawElement) -> String {
    match element {
        DrawElement::TextBox(tb) => {
            let x = tb.frame.origin.x.as_mm();
            let y = tb.frame.origin.y.as_mm();
            let w = tb.frame.size.width.as_mm();
            let h = tb.frame.size.height.as_mm();
            format!("({:.1},{:.1}) {:.1}x{:.1}mm", x, y, w, h)
        }
        DrawElement::Shape(s) => {
            let x = s.frame.origin.x.as_mm();
            let y = s.frame.origin.y.as_mm();
            let w = s.frame.size.width.as_mm();
            let h = s.frame.size.height.as_mm();
            format!("({:.1},{:.1}) {:.1}x{:.1}mm", x, y, w, h)
        }
        DrawElement::Image(img) => {
            let x = img.frame.origin.x.as_mm();
            let y = img.frame.origin.y.as_mm();
            let w = img.frame.size.width.as_mm();
            let h = img.frame.size.height.as_mm();
            format!("({:.1},{:.1}) {:.1}x{:.1}mm", x, y, w, h)
        }
    }
}

pub fn view_as_text(drawing: &Drawing, opts: &ViewOptions) -> Result<String, HandlerError> {
    let mut result = String::new();
    for (i, page) in drawing.pages.iter().enumerate() {
        if i > 0 {
            result.push_str(&format!("\n--- Page {} ---\n", i + 1));
        }
        for element in &page.elements {
            if let Some(text) = extract_text_from_element(element) {
                result.push_str(&text);
                result.push('\n');
            }
        }
    }
    Ok(apply_line_options(&result, opts))
}

pub fn view_as_annotated(drawing: &Drawing, opts: &ViewOptions) -> Result<String, HandlerError> {
    let mut lines = Vec::new();
    for (i, page) in drawing.pages.iter().enumerate() {
        lines.push(format!("=== Page {} ===", i + 1));
        let mut el_idx = 0usize;
        for element in &page.elements {
            el_idx += 1;
            let etype = element_type_str(element);
            let info = element_frame_info(element);
            let text = extract_text_from_element(element)
                .map(|t| format!(" \"{}\"", t))
                .unwrap_or_default();
            lines.push(format!("  [{}] {} {}{}", el_idx, etype, info, text));
        }
    }
    let result = lines.join("\n");
    Ok(apply_line_options(&result, opts))
}

pub fn view_as_outline(drawing: &Drawing) -> Result<String, HandlerError> {
    let mut result = String::new();
    let w = drawing.page_size.width.as_mm();
    let h = drawing.page_size.height.as_mm();
    result.push_str(&format!(
        "ODG Document  {}x{:.0}mm  {} page(s)\n",
        w,
        h,
        drawing.pages.len()
    ));

    for (i, page) in drawing.pages.iter().enumerate() {
        let page_name = if page.name.is_empty() {
            format!("page[{}]", i + 1)
        } else {
            page.name.clone()
        };
        result.push_str(&format!("  /{} [page] {}x{:.0}mm\n", page_name, w, h));

        let mut el_idx = 0usize;
        for element in &page.elements {
            el_idx += 1;
            let etype = element_type_str(element);
            let info = element_frame_info(element);
            let text = extract_text_from_element(element)
                .map(|t| {
                    let preview = if t.chars().count() > 50 {
                        format!("{}...", t.chars().take(50).collect::<String>())
                    } else {
                        t
                    };
                    format!(" \"{}\"", preview)
                })
                .unwrap_or_default();
            result.push_str(&format!(
                "    /{}/element[{}] [{}] {}{}\n",
                page_name, el_idx, etype, info, text
            ));
        }
    }
    Ok(result)
}

pub fn view_as_stats(drawing: &Drawing) -> Result<String, HandlerError> {
    let stats = compute_stats(drawing);
    Ok(format!(
        "ODG Statistics:\n  Pages:           {}\n  Elements:        {}\n  Width:           {:.0}mm\n  Height:          {:.0}mm\n  Rects:           {}\n  Circles:         {}\n  Paths:           {}\n  Text elements:   {}\n  Images:          {}\n  Groups:          {}",
        stats.total_pages,
        stats.total_elements,
        stats.width,
        stats.height,
        stats.rects,
        stats.circles,
        stats.paths,
        stats.texts,
        stats.images,
        stats.groups,
    ))
}

fn compute_stats(drawing: &Drawing) -> crate::dom_types::OdgStats {
    let mut stats = crate::dom_types::OdgStats {
        total_pages: drawing.pages.len(),
        total_elements: 0,
        rects: 0,
        circles: 0,
        paths: 0,
        texts: 0,
        images: 0,
        groups: 0,
        width: drawing.page_size.width.as_mm() as f64,
        height: drawing.page_size.height.as_mm() as f64,
    };

    for page in &drawing.pages {
        for element in &page.elements {
            stats.total_elements += 1;
            match element {
                DrawElement::TextBox(_) => stats.texts += 1,
                DrawElement::Shape(s) => match s.kind {
                    ShapeKind::Rectangle => stats.rects += 1,
                    ShapeKind::Ellipse => stats.circles += 1,
                    ShapeKind::Line => stats.paths += 1,
                },
                DrawElement::Image(_) => stats.images += 1,
            }
        }
    }
    stats
}

pub fn view_as_issues(
    drawing: &Drawing,
    issue_type: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<DocumentIssue>, HandlerError> {
    let mut issues = Vec::new();
    let limit = limit.unwrap_or(50);

    for (i, page) in drawing.pages.iter().enumerate() {
        if page.elements.is_empty() && issues.len() < limit {
            let page_name = if page.name.is_empty() {
                format!("page[{}]", i + 1)
            } else {
                page.name.clone()
            };
            issues.push(DocumentIssue {
                severity: IssueSeverity::Info,
                issue_type: "EmptyPage".to_string(),
                description: format!("page '{}' contains no elements", page_name),
                path: Some(format!("/{}", page_name)),
            });
        }

        for (j, element) in page.elements.iter().enumerate() {
            if issues.len() >= limit {
                break;
            }
            match element {
                DrawElement::Shape(s) => {
                    let w = s.frame.size.width.as_mm();
                    let h = s.frame.size.height.as_mm();
                    if w < 0.01 || h < 0.01 {
                        issues.push(DocumentIssue {
                            severity: IssueSeverity::Warning,
                            issue_type: "ZeroDimension".to_string(),
                            description: format!(
                                "element {} on page {} has zero dimensions ({:.2}x{:.2}mm)",
                                j + 1,
                                i + 1,
                                w,
                                h
                            ),
                            path: Some(format!("/page[{}]/element[{}]", i + 1, j + 1)),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(filter) = issue_type {
        if !filter.is_empty() {
            issues.retain(|i| i.issue_type == filter);
        }
    }

    Ok(issues)
}

pub fn view_as_svg(drawing: &Drawing) -> Result<String, HandlerError> {
    Ok(lo_draw::svg::render_svg(drawing))
}

pub fn view_as_html(drawing: &Drawing, _opts: &ViewOptions) -> Result<String, HandlerError> {
    let svg = lo_draw::svg::render_svg(drawing);
    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>ODG Preview</title>
<style>
  body {{ margin: 0; padding: 20px; background: #f5f5f5; display: flex; justify-content: center; }}
  svg {{ max-width: 100%; height: auto; background: white; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
</style>
</head>
<body>
{}
</body>
</html>"#,
        svg
    ))
}
