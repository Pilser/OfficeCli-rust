//! Template merge for OOXML documents.
//! Replaces {{key}} placeholders in text nodes with values from a HashMap.

use handler_common::{DocumentHandler, HandlerError, MergeResult};
use oxml::OxmlPackage;
use std::collections::{HashMap, HashSet};

/// Regex pattern matching {{key}} placeholders.
const PLACEHOLDER_PATTERN: &str = r"\{\{\s*(\w[\w.\-\[\] ]*?)\s*\}\}";

/// Generic merge over OOXML parts: replace {{key}} in specified text element tags.
pub fn merge_ooxml_parts(
    package: &mut OxmlPackage,
    part_paths: &[String],
    text_tag: &str,
    data: &HashMap<String, String>,
) -> Result<MergeResult, HandlerError> {
    let mut total_replaced = 0;
    let mut all_unresolved = HashSet::new();

    let re = regex::Regex::new(PLACEHOLDER_PATTERN)
        .map_err(|e| HandlerError::OperationFailed(format!("regex error: {}", e)))?;

    for part_path in part_paths {
        let xml = match package.read_part_xml(part_path) {
            Ok(x) => x,
            Err(_) => continue,
        };

        let (modified_xml, replaced, unresolved) =
            replace_placeholders_in_xml(&xml, text_tag, &re, data);

        if replaced > 0 {
            package
                .write_part_xml(part_path, &modified_xml)
                .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
        }

        total_replaced += replaced;
        all_unresolved.extend(unresolved);
    }

    Ok(MergeResult {
        replaced_count: total_replaced,
        unresolved_count: all_unresolved.len(),
    })
}

/// Collect standard part paths for a DOCX merge.
pub fn docx_merge_parts(package: &OxmlPackage) -> Vec<String> {
    let all_parts = package.list_parts();
    let mut target_parts = vec!["word/document.xml".to_string()];

    for p in all_parts {
        if p.starts_with("word/header") || p.starts_with("word/footer") {
            target_parts.push(p.clone());
        }
    }
    target_parts
}

/// Collect standard part paths for an XLSX merge.
pub fn xlsx_merge_parts(package: &OxmlPackage) -> Vec<String> {
    let all_parts = package.list_parts();
    let mut target_parts = Vec::new();

    for p in all_parts {
        if p.starts_with("xl/worksheets/") || *p == "xl/sharedStrings.xml" {
            target_parts.push(p.clone());
        }
    }
    target_parts
}

/// Collect standard part paths for a PPTX merge.
pub fn pptx_merge_parts(package: &OxmlPackage) -> Vec<String> {
    let all_parts = package.list_parts();
    let mut target_parts = Vec::new();

    for p in all_parts {
        if p.starts_with("ppt/slides/") {
            target_parts.push(p.clone());
        }
    }
    target_parts
}

/// Replace {{key}} placeholders inside specific text element tags.
fn replace_placeholders_in_xml(
    xml: &str,
    text_tag: &str,
    re: &regex::Regex,
    data: &HashMap<String, String>,
) -> (String, usize, HashSet<String>) {
    let open_tag = format!("<{}>", text_tag);
    let open_tag_with_attrs = format!("<{} ", text_tag);
    let close_tag = format!("</{}>", text_tag);

    let mut result = String::with_capacity(xml.len());
    let mut replaced_count = 0;
    let mut unresolved_keys = HashSet::new();

    let mut pos = 0;
    while pos < xml.len() {
        let plain_start_tag = xml[pos..].find(&open_tag);
        let attr_start_tag = xml[pos..].find(&open_tag_with_attrs);

        let tag_offset = match (plain_start_tag, attr_start_tag) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let tag_offset = match tag_offset {
            Some(o) => o,
            None => {
                result.push_str(&xml[pos..]);
                break;
            }
        };

        let abs_tag_start = pos + tag_offset;
        result.push_str(&xml[pos..abs_tag_start]);

        let tag_content_start = xml[abs_tag_start..]
            .find('>')
            .map(|o| abs_tag_start + o + 1)
            .unwrap_or(xml.len());

        result.push_str(&xml[abs_tag_start..tag_content_start]);

        let close_offset = xml[tag_content_start..]
            .find(&close_tag)
            .map(|o| tag_content_start + o)
            .unwrap_or(xml.len());

        let text_content = &xml[tag_content_start..close_offset];
        let (new_text, rep, unres) = replace_in_text(text_content, re, data);
        replaced_count += rep;
        unresolved_keys.extend(unres);

        result.push_str(&new_text);
        result.push_str(&close_tag);
        pos = close_offset + close_tag.len();
    }

    (result, replaced_count, unresolved_keys)
}

fn replace_in_text(
    text: &str,
    re: &regex::Regex,
    data: &HashMap<String, String>,
) -> (String, usize, HashSet<String>) {
    let mut replaced = 0;
    let mut unresolved = HashSet::new();

    let result = re.replace_all(text, |caps: &regex::Captures| {
        let key = caps[1].trim();
        if let Some(value) = data.get(key) {
            replaced += 1;
            value.clone()
        } else {
            unresolved.insert(key.to_string());
            caps[0].to_string()
        }
    });

    (result.to_string(), replaced, unresolved)
}

// ─── Template Engine v2 ───────────────────────────────────────────

use std::collections::HashMap as HashMap_;

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateNode {
    Text(String),
    Placeholder(String, Option<HashMap_<String, String>>),
    Loop { var: String, body: Vec<TemplateNode> },
    Conditional { expr: String, then: Vec<TemplateNode>, else_: Vec<TemplateNode>, unless: bool },
    Image { src: String, width: u32, height: u32 },
}

/// Parse a document's text content to find template syntax.
/// Supports:
/// - `{{key}}` — simple placeholder
/// - `{{#each items}}...{{/each}}` — loops
/// - `{{#if condition}}...{{else}}...{{/if}}` — conditionals
/// - `{{#unless condition}}...{{/unless}}` — inverted conditionals
pub fn parse_template(document_text: &str) -> Result<Vec<TemplateNode>, String> {
    let mut nodes = Vec::new();
    let mut remaining = document_text;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("{{") {
            if start > 0 {
                nodes.push(TemplateNode::Text(remaining[..start].to_string()));
            }

            let after_open = &remaining[start + 2..];

            if after_open.starts_with("#each ") || after_open.starts_with("#if ") || after_open.starts_with("#unless ") {
                let is_each = after_open.starts_with("#each ");
                let is_if = after_open.starts_with("#if ");
                let is_unless = after_open.starts_with("#unless ");

                let tag_end = after_open.find("}}").ok_or("Unclosed block tag")?;
                let keyword = if is_each { "#each" } else if is_if { "#if" } else { "#unless" };
                let expr = after_open[keyword.len() + 1..tag_end].trim().to_string();

                let close_tag = if is_each { "{{/each}}" } else if is_if { "{{/if}}" } else { "{{/unless}}" };
                let body_start = start + 2 + tag_end + 2;

                if let Some(body_end) = remaining[body_start..].find(close_tag) {
                    let body_text = &remaining[body_start..body_start + body_end];

                    if is_each {
                        nodes.push(TemplateNode::Loop {
                            var: expr,
                            body: parse_template(body_text)?,
                        });
                    } else {
                        let (then_text, else_text) = split_else_body(body_text);
                        let then_body = parse_template(&then_text)?;
                        let else_body = if let Some(et) = else_text {
                            parse_template(&et)?
                        } else {
                            Vec::new()
                        };

                        nodes.push(TemplateNode::Conditional {
                            expr,
                            then: then_body,
                            else_: else_body,
                            unless: is_unless,
                        });
                    }

                    remaining = &remaining[body_start + body_end + close_tag.len()..];
                    continue;
                } else {
                    return Err(format!("Unclosed block tag: missing {}", close_tag));
                }
            }

            if let Some(end) = after_open.find("}}") {
                let content = after_open[..end].trim();
                let parts: Vec<&str> = content.split('|').collect();
                let name = parts[0].trim().to_string();
                let filters = if parts.len() > 1 {
                    let mut map = HashMap_::new();
                    for f in parts[1..].iter() {
                        let f = f.trim();
                        if let Some(eq) = f.find('=') {
                            map.insert(f[..eq].trim().to_string(), f[eq + 1..].trim().to_string());
                        } else {
                            map.insert(f.to_string(), "true".to_string());
                        }
                    }
                    Some(map)
                } else {
                    None
                };

                nodes.push(TemplateNode::Placeholder(name, filters));
                remaining = &remaining[start + 2 + end + 2..];
                continue;
            }
        }

        nodes.push(TemplateNode::Text(remaining.to_string()));
        break;
    }

    Ok(nodes)
}

/// Split body text on `{{else}}` marker. Returns (before, after_else).
fn split_else_body(body: &str) -> (String, Option<String>) {
    if let Some(pos) = body.find("{{else}}") {
        (body[..pos].to_string(), Some(body[pos + 8..].to_string()))
    } else {
        (body.to_string(), None)
    }
}

/// Evaluate a template against JSON data, returning resolved nodes.
pub fn evaluate_template(
    template: &[TemplateNode],
    data: &serde_json::Value,
) -> Result<Vec<TemplateNode>, String> {
    let mut result = Vec::new();

    for node in template {
        match node {
            TemplateNode::Text(t) => result.push(TemplateNode::Text(t.clone())),
            TemplateNode::Placeholder(name, filters) => {
                let resolved = resolve_value(name, data).unwrap_or(&serde_json::Value::Null);
                let formatted = apply_filters(json_value_to_string(resolved), filters);
                result.push(TemplateNode::Text(formatted));
            }
            TemplateNode::Loop { var, body } => {
                let arr = resolve_value(var, data);
                if let Some(serde_json::Value::Array(items)) = arr {
                    for item in items {
                        let evaluated = evaluate_template(body, item)?;
                        result.extend(evaluated);
                    }
                }
            }
            TemplateNode::Conditional { expr, then, else_, unless } => {
                let val = resolve_value(expr, data);
                let truthy = matches!(val, Some(v) if !v.is_null() && v.as_bool() != Some(false));
                let show_then = if *unless { !truthy } else { truthy };
                let branch = if show_then { then } else { else_ };
                let evaluated = evaluate_template(branch, data)?;
                result.extend(evaluated);
            }
            TemplateNode::Image { .. } => {
                result.push(node.clone());
            }
        }
    }

    Ok(result)
}

/// New merge function that handles loops, conditionals, etc.
pub fn merge_v2(
    doc_handler: &dyn DocumentHandler,
    data: &serde_json::Value,
) -> Result<String, HandlerError> {
    let text_map = doc_handler
        .extract_text_with_offsets()
        .map_err(|e| HandlerError::OperationFailed(format!("extract text: {}", e)))?;

    let template = parse_template(&text_map.full_text)
        .map_err(|e| HandlerError::OperationFailed(e))?;

    let resolved = evaluate_template(&template, data)
        .map_err(|e| HandlerError::OperationFailed(e))?;

    let mut out = String::new();
    for node in &resolved {
        if let TemplateNode::Text(t) = node {
            out.push_str(t);
        }
    }

    Ok(format!("Template v2 merge complete. Output length: {}", out.len()))
}

fn resolve_value<'a>(path: &str, data: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = data;
    for part in parts {
        if let Some(bracket) = part.find('[') {
            let name = &part[..bracket];
            let idx_str = &part[bracket + 1..part.len() - 1];
            let idx: usize = idx_str.parse().ok()?;
            current = current.get(name)?.get(idx)?;
        } else {
            current = current.get(part)?;
        }
    }
    Some(current)
}

fn json_value_to_string(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn apply_filters(value: String, filters: &Option<HashMap_<String, String>>) -> String {
    match filters {
        None => value,
        Some(filters) => {
            let mut result = value;
            for (key, val) in filters {
                match key.as_str() {
                    "uppercase" => result = result.to_uppercase(),
                    "lowercase" => result = result.to_lowercase(),
                    "capitalize" => {
                        let mut c = result.chars();
                        result = c.next().map(|f| f.to_uppercase().to_string() + c.as_str()).unwrap_or_default();
                    }
                    "trim" => result = result.trim().to_string(),
                    "default" => {
                        if result.is_empty() {
                            result = val.clone();
                        }
                    }
                    _ => {}
                }
            }
            result
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_simple_placeholder() {
        let nodes = parse_template("Hello {{name}}!").unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0], TemplateNode::Text("Hello ".to_string()));
        assert_eq!(nodes[1], TemplateNode::Placeholder("name".to_string(), None));
        assert_eq!(nodes[2], TemplateNode::Text("!".to_string()));
    }

    #[test]
    fn test_parse_loop_tag() {
        let nodes = parse_template("{{#each items}} {{name}} {{/each}}").unwrap();
        assert_eq!(nodes.len(), 1);
        if let TemplateNode::Loop { var, body } = &nodes[0] {
            assert_eq!(var, "items");
            assert_eq!(body.len(), 3);
            assert_eq!(body[0], TemplateNode::Text(" ".to_string()));
            assert_eq!(body[1], TemplateNode::Placeholder("name".to_string(), None));
            assert_eq!(body[2], TemplateNode::Text(" ".to_string()));
        } else {
            panic!("expected Loop node");
        }
    }

    #[test]
    fn test_parse_if_else() {
        let nodes = parse_template("{{#if show}}yes{{else}}no{{/if}}").unwrap();
        assert_eq!(nodes.len(), 1);
        if let TemplateNode::Conditional { expr, then, else_, unless } = &nodes[0] {
            assert_eq!(expr, "show");
            assert_eq!(then.len(), 1);
            assert_eq!(then[0], TemplateNode::Text("yes".to_string()));
            assert_eq!(else_.len(), 1);
            assert_eq!(else_[0], TemplateNode::Text("no".to_string()));
            assert!(!unless);
        } else {
            panic!("expected Conditional node");
        }
    }

    #[test]
    fn test_parse_unless() {
        let nodes = parse_template("{{#unless hidden}}visible{{/unless}}").unwrap();
        assert_eq!(nodes.len(), 1);
        if let TemplateNode::Conditional { expr, then, else_, unless } = &nodes[0] {
            assert_eq!(expr, "hidden");
            assert_eq!(then.len(), 1);
            assert_eq!(then[0], TemplateNode::Text("visible".to_string()));
            assert!(else_.is_empty());
            assert!(unless);
        } else {
            panic!("expected Conditional node");
        }
    }

    #[test]
    fn test_evaluate_simple() {
        let template = parse_template("Hello {{name}}!").unwrap();
        let data = json!({"name": "World"});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "Hello World!");
    }

    #[test]
    fn test_evaluate_loop() {
        let template = parse_template("{{#each items}}- {{name}}\n{{/each}}").unwrap();
        let data = json!({"items": [{"name": "A"}, {"name": "B"}, {"name": "C"}]});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "- A\n- B\n- C\n");
    }

    #[test]
    fn test_evaluate_if_true() {
        let template = parse_template("{{#if show}}yes{{else}}no{{/if}}").unwrap();
        let data = json!({"show": true});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "yes");
    }

    #[test]
    fn test_evaluate_if_false() {
        let template = parse_template("{{#if show}}yes{{else}}no{{/if}}").unwrap();
        let data = json!({"show": false});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "no");
    }

    #[test]
    fn test_evaluate_unless() {
        let template = parse_template("{{#unless hidden}}visible{{/unless}}").unwrap();
        let data = json!({"hidden": true});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "");
    }

    #[test]
    fn test_filters() {
        let template = parse_template("{{name|uppercase}}").unwrap();
        let data = json!({"name": "hello"});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "HELLO");
    }

    #[test]
    fn test_nested_path() {
        let template = parse_template("{{user.name}}").unwrap();
        let data = json!({"user": {"name": "Alice"}});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "Alice");
    }

    #[test]
    fn test_parse_with_filters() {
        let nodes = parse_template("{{name|uppercase|trim}}").unwrap();
        assert_eq!(nodes.len(), 1);
        if let TemplateNode::Placeholder(name, filters) = &nodes[0] {
            assert_eq!(name, "name");
            let filters = filters.as_ref().unwrap();
            assert_eq!(filters.get("uppercase").unwrap(), "true");
            assert_eq!(filters.get("trim").unwrap(), "true");
        } else {
            panic!("expected Placeholder");
        }
    }

    #[test]
    fn test_default_filter() {
        let template = parse_template("{{missing|default=fallback}}").unwrap();
        let data = json!({});
        let result = evaluate_template(&template, &data).unwrap();
        let mut out = String::new();
        for node in &result {
            if let TemplateNode::Text(t) = node {
                out.push_str(t);
            }
        }
        assert_eq!(out, "fallback");
    }
}
