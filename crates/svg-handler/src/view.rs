use crate::dom_types::{SvgNodeType, SvgStats};
use handler_common::*;
use std::collections::HashMap;

pub fn view_as_text(xml: &str, opts: &ViewOptions) -> Result<String, HandlerError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

    let mut lines = Vec::new();
    let root = doc.root_element();
    collect_text_xml(&root, &mut lines);

    let result = lines.join("\n");
    Ok(apply_line_options(&result, opts))
}

pub fn view_as_annotated(xml: &str, opts: &ViewOptions) -> Result<String, HandlerError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

    let mut lines = Vec::new();
    let root = doc.root_element();
    collect_annotated(&root, "/svg", &mut lines);

    let result = lines.join("\n");
    Ok(apply_line_options(&result, opts))
}

pub fn view_as_outline(xml: &str) -> Result<String, HandlerError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

    let result = build_outline(&doc.root_element(), 0, "");
    Ok(result)
}

pub fn view_as_stats(xml: &str) -> Result<String, HandlerError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

    let stats = compute_stats(&doc);
    Ok(format!(
        "SVG Stats:\n  Elements:       {}\n  Width:          {}\n  Height:         {}\n  Groups:         {}\n  Rects:          {}\n  Circles:        {}\n  Ellipses:       {}\n  Paths:          {}\n  Text elements:  {}\n  Images:         {}",
        stats.total_elements,
        stats.width,
        stats.height,
        stats.groups,
        stats.rects,
        stats.circles,
        stats.ellipses,
        stats.paths,
        stats.texts,
        stats.images,
    ))
}

pub fn view_as_issues(
    xml: &str,
    issue_type: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<DocumentIssue>, HandlerError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

    let root = doc.root_element();
    let mut issues = Vec::new();

    if issue_type.map_or(true, |t| t == "namespace" || t.is_empty()) {
        let default_ns = root.namespaces().iter().find(|ns| ns.name().is_none());
        match default_ns {
            Some(ns) if ns.uri() == "http://www.w3.org/2000/svg" => {}
            Some(ns) => issues.push(DocumentIssue {
                severity: IssueSeverity::Warning,
                issue_type: "namespace".to_string(),
                description: format!("SVG uses non-standard default namespace: {}", ns.uri()),
                path: Some("/svg".to_string()),
            }),
            None => issues.push(DocumentIssue {
                severity: IssueSeverity::Error,
                issue_type: "namespace".to_string(),
                description: "SVG missing required xmlns attribute".to_string(),
                path: Some("/svg".to_string()),
            }),
        }
    }

    if issue_type.map_or(true, |t| t == "viewbox" || t.is_empty()) {
        if root.attribute("viewBox").is_none() {
            issues.push(DocumentIssue {
                severity: IssueSeverity::Warning,
                issue_type: "viewbox".to_string(),
                description: "SVG missing viewBox attribute (may not scale responsively)".to_string(),
                path: Some("/svg".to_string()),
            });
        }
    }

    if issue_type.map_or(true, |t| t == "empty-group" || t.is_empty()) {
        find_empty_groups(&root, "/svg", &mut issues);
    }

    if issue_type.map_or(true, |t| t == "empty-text" || t.is_empty()) {
        find_empty_text(&root, "/svg", &mut issues);
    }

    if let Some(limit) = limit {
        issues.truncate(limit);
    }

    if let Some(issue_type) = issue_type {
        if !issue_type.is_empty() {
            issues.retain(|i| i.issue_type == issue_type);
        }
    }

    Ok(issues)
}

pub fn view_as_html(xml: &str, _opts: &ViewOptions) -> Result<String, HandlerError> {
    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>SVG Preview</title>
<style>
  body {{ margin: 0; padding: 20px; background: #f5f5f5; display: flex; justify-content: center; }}
  svg {{ max-width: 100%; height: auto; background: white; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
</style>
</head>
<body>
{}
</body>
</html>"#,
        xml
    ))
}

fn collect_text_xml(node: &roxmltree::Node, lines: &mut Vec<String>) {
    if !node.is_element() {
        return;
    }
    let tag = node.tag_name().name();
    if tag == "text" || tag == "tspan" {
        if let Some(text) = node.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }
    }
    for child in node.children() {
        collect_text_xml(&child, lines);
    }
}

fn collect_annotated(node: &roxmltree::Node, current_path: &str, lines: &mut Vec<String>) {
    if !node.is_element() {
        return;
    }

    let tag = node.tag_name().name();
    let node_type = SvgNodeType::from_tag(tag);

    if tag == "text" || tag == "tspan" {
        if let Some(text) = node.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                lines.push(format!("{} [{}] {}", current_path, node_type.as_str(), trimmed));
            }
        }
    }

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            let count = tag_counts.entry(child_tag.to_string()).or_insert(0);
            *count += 1;
            let child_path = format!("{}/{}[{}]", current_path, child_tag, count);
            collect_annotated(&child, &child_path, lines);
        }
    }
}

fn build_outline(node: &roxmltree::Node, depth: usize, parent_path: &str) -> String {
    if !node.is_element() {
        return String::new();
    }

    let tag = node.tag_name().name();
    let node_type = SvgNodeType::from_tag(tag);
    let indent = "  ".repeat(depth);

    let current_path = if depth == 0 {
        format!("/{}", tag)
    } else {
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        let mut my_index = 1usize;
        if let Some(parent) = node.parent() {
            for child in parent.children() {
                if child.is_element() && child.tag_name().name() == tag {
                    let count = tag_counts.entry(tag.to_string()).or_insert(0);
                    *count += 1;
                    if child == *node {
                        my_index = *count;
                    }
                }
            }
        }
        format!("{}/{}[{}]", parent_path, tag, my_index)
    };

    let info = element_info(node);
    let mut result = format!("{}{} [{}] {}\n", indent, current_path, node_type.as_str(), info);

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            let count = tag_counts.entry(child_tag.to_string()).or_insert(0);
            *count += 1;
            let _child_path = format!("{}/{}[{}]", current_path, child_tag, count);
            result.push_str(&build_outline_at(&child, depth + 1, &current_path, tag_counts.clone()));
            tag_counts = HashMap::new();
        }
    }

    result
}

fn build_outline_at(node: &roxmltree::Node, depth: usize, parent_path: &str, _sibling_counts: HashMap<String, usize>) -> String {
    if !node.is_element() {
        return String::new();
    }

    let tag = node.tag_name().name();
    let node_type = SvgNodeType::from_tag(tag);
    let indent = "  ".repeat(depth);

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut my_index = 1usize;
    if let Some(parent) = node.parent() {
        for child in parent.children() {
            if child.is_element() && child.tag_name().name() == tag {
                let count = tag_counts.entry(tag.to_string()).or_insert(0);
                *count += 1;
                if child == *node {
                    my_index = *count;
                }
            }
        }
    }
    let current_path = format!("{}/{}[{}]", parent_path, tag, my_index);

    let info = element_info(node);
    let mut result = format!("{}{} [{}] {}\n", indent, current_path, node_type.as_str(), info);

    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            let mut child_counts = HashMap::new();
            let mut child_index = 1usize;
            for c in node.children() {
                if c.is_element() && c.tag_name().name() == child_tag {
                    let cnt = child_counts.entry(child_tag.to_string()).or_insert(0);
                    *cnt += 1;
                    if c == child {
                        child_index = *cnt;
                    }
                }
            }
            let _child_path = format!("{}/{}[{}]", current_path, child_tag, child_index);
            result.push_str(&build_outline_at(&child, depth + 1, &current_path, child_counts));
        }
    }

    result
}

fn element_info(node: &roxmltree::Node) -> String {
    let tag = node.tag_name().name();
    let mut parts = Vec::new();

    match tag {
        "svg" => {
            if let Some(vb) = node.attribute("viewBox") {
                parts.push(vb.to_string());
            } else {
                if let Some(w) = node.attribute("width") {
                    parts.push(format!("width={}", w));
                }
                if let Some(h) = node.attribute("height") {
                    parts.push(format!("height={}", h));
                }
            }
        }
        "rect" => {
            for attr in &["x", "y", "width", "height", "rx", "ry"] {
                if let Some(v) = node.attribute(*attr) {
                    parts.push(v.to_string());
                }
            }
        }
        "circle" => {
            for attr in &["cx", "cy", "r"] {
                if let Some(v) = node.attribute(*attr) {
                    parts.push(v.to_string());
                }
            }
        }
        "ellipse" => {
            for attr in &["cx", "cy", "rx", "ry"] {
                if let Some(v) = node.attribute(*attr) {
                    parts.push(v.to_string());
                }
            }
        }
        "line" => {
            for attr in &["x1", "y1", "x2", "y2"] {
                if let Some(v) = node.attribute(*attr) {
                    parts.push(v.to_string());
                }
            }
        }
        "text" | "tspan" => {
            if let Some(text) = node.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }
        "path" => {
            if let Some(d) = node.attribute("d") {
                let preview = if d.len() > 40 {
                    format!("{}...", &d[..40])
                } else {
                    d.to_string()
                };
                parts.push(preview);
            }
        }
        "image" => {
            if let Some(href) = node.attribute("href").or_else(|| node.attribute("xlink:href")) {
                let preview = if href.len() > 40 {
                    format!("{}...", &href[..40])
                } else {
                    href.to_string()
                };
                parts.push(preview);
            }
        }
        "use" => {
            if let Some(href) = node.attribute("href").or_else(|| node.attribute("xlink:href")) {
                parts.push(href.to_string());
            }
        }
        _ => {}
    }

    parts.join(" ")
}

fn compute_stats(doc: &roxmltree::Document) -> SvgStats {
    let root = doc.root_element();

    let width = root
        .attribute("width")
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            root.attribute("viewBox")
                .and_then(|vb| vb.split_whitespace().nth(2))
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    let height = root
        .attribute("height")
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            root.attribute("viewBox")
                .and_then(|vb| vb.split_whitespace().nth(3))
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    let mut rects = 0usize;
    let mut circles = 0usize;
    let mut ellipses = 0usize;
    let mut paths = 0usize;
    let mut texts = 0usize;
    let mut images = 0usize;
    let mut groups = 0usize;
    let mut total = 0usize;

    for node in doc.descendants() {
        if !node.is_element() {
            continue;
        }
        total += 1;
        match node.tag_name().name() {
            "rect" => rects += 1,
            "circle" => circles += 1,
            "ellipse" => ellipses += 1,
            "path" => paths += 1,
            "text" => texts += 1,
            "image" => images += 1,
            "g" => groups += 1,
            _ => {}
        }
    }

    SvgStats {
        total_elements: total,
        rects,
        circles,
        ellipses,
        paths,
        texts,
        images,
        groups,
        width,
        height,
    }
}

fn find_empty_groups(node: &roxmltree::Node, path: &str, issues: &mut Vec<DocumentIssue>) {
    if !node.is_element() {
        return;
    }

    let tag = node.tag_name().name();
    if tag == "g" {
        let has_element_child = node.children().any(|c| c.is_element());
        if !has_element_child {
            issues.push(DocumentIssue {
                severity: IssueSeverity::Warning,
                issue_type: "empty-group".to_string(),
                description: "Empty <g> element serves no purpose".to_string(),
                path: Some(path.to_string()),
            });
        }
    }

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            let count = tag_counts.entry(child_tag.to_string()).or_insert(0);
            *count += 1;
            let child_path = format!("{}/{}[{}]", path, child_tag, count);
            find_empty_groups(&child, &child_path, issues);
        }
    }
}

fn find_empty_text(node: &roxmltree::Node, path: &str, issues: &mut Vec<DocumentIssue>) {
    if !node.is_element() {
        return;
    }

    let tag = node.tag_name().name();
    if tag == "text" {
        let has_text = node.text().map(|t| !t.trim().is_empty()).unwrap_or(false);
        let has_tspan = node.children().any(|c| c.is_element() && c.has_tag_name("tspan"));
        if !has_text && !has_tspan {
            issues.push(DocumentIssue {
                severity: IssueSeverity::Info,
                issue_type: "empty-text".to_string(),
                description: "<text> element has no content".to_string(),
                path: Some(path.to_string()),
            });
        }
    }

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            let count = tag_counts.entry(child_tag.to_string()).or_insert(0);
            *count += 1;
            let child_path = format!("{}/{}[{}]", path, child_tag, count);
            find_empty_text(&child, &child_path, issues);
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_svg() -> &'static str {
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">
  <rect x="10" y="10" width="80" height="80" fill="red"/>
  <g>
    <circle cx="50" cy="50" r="30" fill="blue"/>
  </g>
  <text x="10" y="50" font-size="16">Hello World</text>
</svg>"#
    }

    #[test]
    fn test_view_as_text() {
        let result = view_as_text(sample_svg(), &ViewOptions::default()).unwrap();
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_view_as_annotated() {
        let result = view_as_annotated(sample_svg(), &ViewOptions::default()).unwrap();
        assert!(result.contains("Hello World"));
        assert!(result.contains("[text]"));
    }

    #[test]
    fn test_view_as_outline() {
        let result = view_as_outline(sample_svg()).unwrap();
        assert!(result.contains("/svg"));
        assert!(result.contains("rect"));
        assert!(result.contains("circle"));
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_view_as_stats() {
        let result = view_as_stats(sample_svg()).unwrap();
        assert!(result.contains("Elements:"));
        assert!(result.contains("Width:"));
    }

    #[test]
    fn test_view_as_svg() {
        let dom = view_as_text(sample_svg(), &ViewOptions::default()).unwrap();
        // just check text view works (svg view is direct in handler)
    }

    #[test]
    fn test_view_as_html() {
        let result = view_as_html(sample_svg(), &ViewOptions::default()).unwrap();
        assert!(result.contains("<!DOCTYPE html>"));
        assert!(result.contains("<svg"));
    }

    #[test]
    fn test_view_as_issues() {
        let issues = view_as_issues(sample_svg(), None, None).unwrap();
        let issue_types: Vec<&str> = issues.iter().map(|i| i.issue_type.as_str()).collect();
        // Should have no errors for valid SVG
        assert!(issues.is_empty() || issues.iter().all(|i| i.severity != IssueSeverity::Error));
    }

    #[test]
    fn test_view_as_issues_missing_viewbox() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"/> "#;
        let issues = view_as_issues(svg, None, None).unwrap();
        assert!(issues.iter().any(|i| i.issue_type == "viewbox"));
    }

    #[test]
    fn test_view_as_text_empty_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"/> "#;
        let result = view_as_text(svg, &ViewOptions::default()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_line_options() {
        let text = "a\nb\nc\nd\ne";
        let opts = ViewOptions {
            start_line: Some(1),
            end_line: Some(4),
            max_lines: None,
            ..ViewOptions::default()
        };
        assert_eq!(apply_line_options(text, &opts), "b\nc\nd");
    }
}
