//! Shared Extended Properties (`docProps/app.xml`) read/set helpers for all
//! OOXML document types. Mirrors `Core/ExtendedPropertiesHandler.cs`.
//!
//! The Rust port doesn't have an OpenXML SDK wrapper, so we model
//! `app.xml` as a free-form XML string and pull the fields we care about
//! via quick-xml traversal. That keeps the dependency footprint to zero
//! for what is a 12-field read/3-field write helper.

use crate::document_node::DocumentNode;
use std::collections::HashMap;

/// Populate the `node.Format` dictionary with the extended properties from
/// `app.xml`. Caller passes the raw XML bytes (or `None` when the part is
/// absent); the function is a no-op in that case.
///
/// Field name mapping matches the C# implementation (ExtendedPropertiesHandler
/// §2): the in-document key is `extended.<lowercase_field>`. Integer-valued
/// fields (`pages`, `words`, `characters`, `lines`, `paragraphs`, `totalTime`)
/// store the parsed integer when the source parses, otherwise the original
/// string — callers can read either kind via `serde_json::Value`.
pub fn populate_extended_properties(xml_bytes: Option<&[u8]>, node: &mut DocumentNode) {
    let Some(xml_bytes) = xml_bytes else {
        return;
    };
    let Ok(xml) = std::str::from_utf8(xml_bytes) else {
        return;
    };

    // Single-pass scan: every <Foo>text</Foo> under <Properties> gets cached
    // in a HashMap by local name → trimmed text.
    let fields = collect_properties(xml);
    if fields.is_empty() {
        return;
    }

    store_string(node, &fields, "Template", "extended.template");
    store_string(node, &fields, "Manager", "extended.manager");
    store_string(node, &fields, "Company", "extended.company");
    store_string(node, &fields, "Application", "extended.application");
    store_string(node, &fields, "AppVersion", "extended.applicationVersion");
    store_int_or_string(node, &fields, "Pages", "extended.pages");
    store_int_or_string(node, &fields, "Words", "extended.words");
    store_int_or_string(node, &fields, "Characters", "extended.characters");
    store_int_or_string(node, &fields, "Lines", "extended.lines");
    store_int_or_string(node, &fields, "Paragraphs", "extended.paragraphs");
    store_int_or_string(node, &fields, "TotalTime", "extended.totalTime");
}

/// Try to set an `extended.*` property in the supplied `app.xml` bytes.
/// Returns `Ok(true)` when handled, `Ok(false)` when `key` is not a known
/// extended property (caller should treat as unsupported).
///
/// Only `extended.template`, `extended.manager`, `extended.company` are
/// writable; the rest are computed by Office and treated as read-only.
pub fn try_set_extended_property(
    xml_bytes: &mut Vec<u8>,
    key: &str,
    value: &str,
) -> Result<bool, String> {
    let element = match key {
        "extended.template" => "Template",
        "extended.manager" => "Manager",
        "extended.company" => "Company",
        _ => return Ok(false),
    };
    let xml = std::str::from_utf8(xml_bytes).map_err(|e| format!("app.xml not UTF-8: {}", e))?;
    let updated = ensure_or_replace_child(xml, element, value);
    *xml_bytes = updated.into_bytes();
    Ok(true)
}

/// Build a minimal valid `app.xml` skeleton for documents that don't yet
/// have one (used by the create-* flows that need to round-trip extended
/// props after creation).
pub fn minimal_app_xml() -> Vec<u8> {
    let xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        "\n",
        r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">"#,
        "<Application>OfficeCLI</Application>",
        "</Properties>",
        "\n"
    );
    xml.as_bytes().to_vec()
}

// ─── helpers ───────────────────────────────────────────────────

/// Quick-and-dirty single-pass scan of `app.xml` extracting child elements
/// of `<Properties>`. We avoid pulling a full DOM because the file is
/// tiny and we already have `quick-xml` patterns elsewhere.
fn collect_properties(xml: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let bytes = xml.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        // Skip declarations, comments, closing tags.
        if starts_with(bytes, i, b"<?")
            || starts_with(bytes, i, b"<!")
            || starts_with(bytes, i, b"</")
        {
            // advance past the '>' on this tag
            i = match bytes[i..].iter().position(|&b| b == b'>') {
                Some(p) => i + p + 1,
                None => break,
            };
            continue;
        }
        // Open tag: read local name (strip any namespace prefix).
        let start = i + 1;
        let name_end = match bytes[start..].iter().position(|&b| matches!(b, b' ' | b'>' | b'/')) {
            Some(p) => start + p,
            None => break,
        };
        let raw_name = &xml[start..name_end];
        let local = raw_name.rsplit(':').next().unwrap_or(raw_name);
        // Skip the root Properties element itself.
        if local == "Properties" {
            i = match bytes[name_end..].iter().position(|&b| b == b'>') {
                Some(p) => name_end + p + 1,
                None => break,
            };
            continue;
        }
        // Find end of opening tag.
        let tag_end = match bytes[name_end..].iter().position(|&b| b == b'>') {
            Some(p) => name_end + p,
            None => break,
        };
        // Empty element <Foo/> has no text.
        if bytes[tag_end - 1] == b'/' {
            i = tag_end + 1;
            continue;
        }
        let text_start = tag_end + 1;
        // Find matching close tag </local>.
        let close_marker = format!("</{}>", raw_name);
        let close_pos = match xml[text_start..].find(&close_marker) {
            Some(p) => text_start + p,
            None => {
                i = text_start;
                continue;
            }
        };
        let text = xml[text_start..close_pos].trim();
        if !text.is_empty() {
            out.insert(local.to_string(), text.to_string());
        }
        i = close_pos + close_marker.len();
    }
    out
}

fn starts_with(haystack: &[u8], pos: usize, needle: &[u8]) -> bool {
    haystack.get(pos..pos + needle.len()) == Some(needle)
}

fn store_string(
    node: &mut DocumentNode,
    fields: &HashMap<String, String>,
    element: &str,
    key: &str,
) {
    if let Some(value) = fields.get(element) {
        node.format
            .insert(key.to_string(), Some(serde_json::Value::String(value.clone())));
    }
}

fn store_int_or_string(
    node: &mut DocumentNode,
    fields: &HashMap<String, String>,
    element: &str,
    key: &str,
) {
    let Some(text) = fields.get(element) else {
        return;
    };
    let value = match text.parse::<i64>() {
        Ok(n) => serde_json::Value::Number(n.into()),
        Err(_) => serde_json::Value::String(text.clone()),
    };
    node.format.insert(key.to_string(), Some(value));
}

fn ensure_or_replace_child(xml: &str, element: &str, value: &str) -> String {
    let open = format!("<{}>", element);
    let close = format!("</{}>", element);
    if let Some(start) = xml.find(&open) {
        let text_start = start + open.len();
        let end = match xml[text_start..].find(&close) {
            Some(p) => text_start + p,
            None => return xml.to_string(),
        };
        let mut out = String::with_capacity(xml.len() + value.len());
        out.push_str(&xml[..text_start]);
        out.push_str(value);
        out.push_str(&xml[end..]);
        return out;
    }
    // Not present: insert before </Properties>.
    let insertion = format!("<{}>{}</{}>", element, value, element);
    if let Some(end_idx) = xml.rfind("</Properties>") {
        let mut out = String::with_capacity(xml.len() + insertion.len());
        out.push_str(&xml[..end_idx]);
        out.push_str(&insertion);
        out.push_str(&xml[end_idx..]);
        return out;
    }
    // No closing Properties tag at all — synthesise one.
    format!(
        "<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\">{}</Properties>",
        insertion
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> DocumentNode {
        DocumentNode::new("/", "root")
    }

    #[test]
    fn empty_input_is_noop() {
        let mut n = node();
        populate_extended_properties(None, &mut n);
        assert!(n.format.is_empty());
        let mut n2 = node();
        populate_extended_properties(Some(b""), &mut n2);
        assert!(n2.format.is_empty());
    }

    #[test]
    fn reads_all_fields_with_int_coercion() {
        let xml = br#"<?xml version="1.0"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
  <Template>Normal.dotm</Template>
  <Manager>Alice</Manager>
  <Company>ACME</Company>
  <Application>Microsoft Office Word</Application>
  <AppVersion>16.0000</AppVersion>
  <Pages>3</Pages>
  <Words>412</Words>
  <Characters>2381</Characters>
  <Lines>32</Lines>
  <Paragraphs>14</Paragraphs>
  <TotalTime>15</TotalTime>
  <NonExistentField>ignored</NonExistentField>
</Properties>"#;
        let mut n = node();
        populate_extended_properties(Some(xml), &mut n);
        assert_eq!(n.format["extended.template"].clone().unwrap(), "Normal.dotm");
        assert_eq!(n.format["extended.company"].clone().unwrap(), "ACME");
        assert_eq!(
            n.format["extended.application"].clone().unwrap(),
            "Microsoft Office Word"
        );
        assert_eq!(n.format["extended.pages"].clone().unwrap(), 3);
        assert_eq!(n.format["extended.words"].clone().unwrap(), 412);
        assert_eq!(n.format["extended.totalTime"].clone().unwrap(), 15);
        assert!(!n.format.contains_key("extended.nonexistentfield"));
    }

    #[test]
    fn non_numeric_int_field_falls_back_to_string() {
        let xml = br#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Pages>not-a-number</Pages></Properties>"#;
        let mut n = node();
        populate_extended_properties(Some(xml), &mut n);
        assert_eq!(n.format["extended.pages"].clone().unwrap(), "not-a-number");
    }

    #[test]
    fn set_writes_known_property() {
        let mut bytes = br#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Template>Old</Template></Properties>"#.to_vec();
        let handled = try_set_extended_property(&mut bytes, "extended.template", "New").unwrap();
        assert!(handled);
        let updated = std::str::from_utf8(&bytes).unwrap();
        assert!(updated.contains("<Template>New</Template>"));
        assert!(!updated.contains("<Template>Old</Template>"));
    }

    #[test]
    fn set_inserts_missing_property() {
        let mut bytes = br#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Template>Old</Template></Properties>"#
            .to_vec();
        let handled = try_set_extended_property(&mut bytes, "extended.company", "ACME").unwrap();
        assert!(handled);
        let updated = std::str::from_utf8(&bytes).unwrap();
        assert!(updated.contains("<Company>ACME</Company>"));
    }

    #[test]
    fn set_rejects_unknown_property() {
        let mut bytes = br#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"/>"#
            .to_vec();
        let handled = try_set_extended_property(&mut bytes, "extended.nope", "x").unwrap();
        assert!(!handled);
    }

    #[test]
    fn minimal_app_xml_is_well_formed() {
        let bytes = minimal_app_xml();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("extended-properties"));
        assert!(xml.contains("<Application>OfficeCLI</Application>"));
    }
}
