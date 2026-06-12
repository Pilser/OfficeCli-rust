//! Template merge for OOXML documents.
//! Replaces {{key}} placeholders in text nodes with values from a HashMap.

use handler_common::{HandlerError, MergeResult};
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
