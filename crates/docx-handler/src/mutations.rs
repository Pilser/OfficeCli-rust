use crate::dom_types::{WordDom, WordElementType, WordNode};
use crate::helpers::{build_paragraph_properties, build_run_properties};
use crate::navigation::{navigate_to_element, navigate_to_element_mut, parse_path};
use handler_common::{HandlerError, InsertPosition};
use std::collections::HashMap;

/// Set properties on an element at a given path.
/// Supported property changes:
/// - On paragraphs: text, style, alignment, indent, spacing
/// - On runs: text, bold, italic, underline, font, size, color, bgColor
/// - On text (w:t): text content
/// - On tables: style, width
/// - On rows: height
/// - On cells: text, width
/// - On bookmarkStart: name, text, id
/// - On bookmarkEnd: id
///
/// Returns list of unrecognized property keys (empty = all applied).
pub fn set_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let segments = parse_path(path)?;
    if segments.is_empty() {
        return Err(HandlerError::InvalidPath("empty path".to_string()));
    }

    // Determine what type of element we're modifying
    let last_seg = &segments[segments.len() - 1];
    let target_type = last_seg.name.as_str();

    match target_type {
        "p" => set_paragraph_properties(dom, path, properties),
        "r" => set_run_properties(dom, path, properties),
        "t" => set_text_content(dom, path, properties),
        "tbl" => set_table_properties(dom, path, properties),
        "tr" => set_row_properties(dom, path, properties),
        "tc" => set_cell_properties(dom, path, properties),
        "bookmarkStart" => set_bookmark_properties(dom, path, properties),
        "bookmarkEnd" => set_bookmark_end_properties(dom, path, properties),
        other => Err(HandlerError::UnsupportedProperty(format!(
            "cannot set properties on element type: {}",
            other
        ))),
    }
}

/// Set paragraph properties (style, alignment, indent, spacing).
fn set_paragraph_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let para = navigate_to_element_mut(dom, path)?;

    // Check if text property is set — this changes paragraph text
    if let Some(new_text) = properties.get("text") {
        set_paragraph_text(para, new_text);
    }

    // Build or replace paragraph properties
    let ppr_props: HashMap<String, String> = properties
        .iter()
        .filter(|(k, _)| {
            k.as_str() != "text" && k.starts_with("style")
                || k.starts_with("alignment")
                || k.starts_with("indent")
                || k.starts_with("spacing")
                || k.as_str() == "pStyle"
                || k.as_str() == "jc"
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !ppr_props.is_empty() {
        // Remove existing pPr if present
        para.children
            .retain(|c| c.element_type != WordElementType::ParagraphProperties);

        // Add new pPr
        if let Some(new_ppr) = build_paragraph_properties(&ppr_props) {
            // Insert pPr at the beginning of children (convention in OOXML)
            para.children.insert(0, new_ppr);
        }
    }

    // Collect unrecognized property keys
    let recognized = ["text", "style", "pStyle", "alignment", "jc",
        "indentLeft", "indentRight", "spacingBefore", "spacingAfter"];
    let unsupported: Vec<String> = properties.keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Set paragraph text by replacing all runs with a single run containing the new text.
fn set_paragraph_text(para: &mut WordNode, new_text: &str) {
    // Remove all existing runs (and hyperlinks that contain runs)
    para.children.retain(|c| {
        c.element_type != WordElementType::Run
            && c.element_type != WordElementType::Hyperlink
            && c.element_type != WordElementType::BookmarkStart
            && c.element_type != WordElementType::BookmarkEnd
    });

    // Add a new run with the text
    let text_node = if new_text.starts_with(' ') || new_text.ends_with(' ') {
        let mut tn = WordNode::new(WordElementType::Text).with_text(new_text);
        tn.attributes
            .insert("xml:space".to_string(), "preserve".to_string());
        tn.preserve_space = true;
        tn
    } else {
        WordNode::new(WordElementType::Text).with_text(new_text)
    };

    let run = WordNode::new(WordElementType::Run).with_children(vec![text_node]);

    para.children.push(run);
}

/// Set run properties (bold, italic, font, size, color).
fn set_run_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let run = navigate_to_element_mut(dom, path)?;

    // Check if text property is set
    if let Some(new_text) = properties.get("text") {
        // Replace the w:t content
        let text_children: Vec<usize> = run
            .children
            .iter()
            .enumerate()
            .filter(|(_, c)| c.element_type == WordElementType::Text)
            .map(|(i, _)| i)
            .collect();

        if text_children.is_empty() {
            // Add a new w:t element
            let text_node = if new_text.starts_with(' ') || new_text.ends_with(' ') {
                let mut tn = WordNode::new(WordElementType::Text).with_text(new_text);
                tn.attributes
                    .insert("xml:space".to_string(), "preserve".to_string());
                tn.preserve_space = true;
                tn
            } else {
                WordNode::new(WordElementType::Text).with_text(new_text)
            };
            run.children.push(text_node);
        } else {
            // Replace existing w:t text content
            for idx in text_children {
                run.children[idx].text_content = Some(new_text.to_string());
                if new_text.starts_with(' ') || new_text.ends_with(' ') {
                    run.children[idx]
                        .attributes
                        .insert("xml:space".to_string(), "preserve".to_string());
                    run.children[idx].preserve_space = true;
                }
            }
        }
    }

    // Build or replace run properties
    let run_props: HashMap<String, String> = properties
        .iter()
        .filter(|(k, _)| k.as_str() != "text")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !run_props.is_empty() {
        // Remove existing rPr
        run.children
            .retain(|c| c.element_type != WordElementType::RunProperties);

        // Add new rPr
        if let Some(new_rpr) = build_run_properties(&run_props) {
            // Insert rPr at the beginning of run children
            run.children.insert(0, new_rpr);
        }
    }

    let recognized = ["text", "bold", "b", "italic", "i", "underline", "u",
        "strike", "strikeout", "font", "fontFamily", "size", "fontSize",
        "color", "fontColor", "bgColor", "highlight", "bg"];
    let unsupported: Vec<String> = properties.keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Set text content directly on a w:t element.
fn set_text_content(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    if let Some(new_text) = properties.get("text") {
        let text_node = navigate_to_element_mut(dom, path)?;
        text_node.text_content = Some(new_text.to_string());
        if new_text.starts_with(' ') || new_text.ends_with(' ') {
            text_node
                .attributes
                .insert("xml:space".to_string(), "preserve".to_string());
            text_node.preserve_space = true;
        }
        let unsupported: Vec<String> = properties.keys()
            .filter(|k| k.as_str() != "text")
            .cloned()
            .collect();
        Ok(unsupported)
    } else {
        Err(HandlerError::UnsupportedProperty(
            "text node only supports 'text' property".to_string(),
        ))
    }
}

/// Set table properties.
fn set_table_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let table = navigate_to_element_mut(dom, path)?;
    // Remove existing tblPr
    table
        .children
        .retain(|c| c.element_type != WordElementType::TableProperties);

    let mut tbl_pr = WordNode::new(WordElementType::TableProperties);
    let mut children = Vec::new();

    for (key, value) in properties {
        match key.as_str() {
            "style" | "tblStyle" => {
                let tbl_style = WordNode::new(WordElementType::Unknown("tblStyle".to_string()))
                    .with_attribute("val", value.as_str());
                children.push(tbl_style);
            }
            "width" => {
                let tbl_w = WordNode::new(WordElementType::Unknown("tblW".to_string()))
                    .with_attribute("w", value.as_str())
                    .with_attribute("type", "dxa");
                children.push(tbl_w);
            }
            _ => {}
        }
    }

    if !children.is_empty() {
        tbl_pr.children = children;
        table.children.insert(0, tbl_pr);
    }

    let recognized = ["style", "tblStyle", "width"];
    let unsupported: Vec<String> = properties.keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Set row properties (minimal).
fn set_row_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // Minimal: mostly height
    let row = navigate_to_element_mut(dom, path)?;
    row.children
        .retain(|c| c.element_type != WordElementType::TableRowProperties);

    if let Some(height) = properties.get("height") {
        let tr_pr =
            WordNode::new(WordElementType::TableRowProperties).with_children(vec![WordNode::new(
                WordElementType::Unknown("trHeight".to_string()),
            )
            .with_attribute("val", height.as_str())
            .with_attribute("hRule", "atLeast")]);
        row.children.insert(0, tr_pr);
    }

    let recognized = ["height"];
    let unsupported: Vec<String> = properties.keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Set cell properties.
fn set_cell_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let cell = navigate_to_element_mut(dom, path)?;

    // If "text" property is set, replace paragraph text in the cell
    if let Some(new_text) = properties.get("text") {
        // Find the first paragraph and set its text
        for child in &mut cell.children {
            if child.element_type == WordElementType::Paragraph {
                set_paragraph_text(child, new_text);
                break;
            }
        }
    }

    // Cell width
    let cell_props: HashMap<String, String> = properties
        .iter()
        .filter(|(k, _)| k.as_str() != "text")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !cell_props.is_empty() {
        cell.children
            .retain(|c| c.element_type != WordElementType::TableCellProperties);

        let mut tc_pr = WordNode::new(WordElementType::TableCellProperties);
        let mut children = Vec::new();

        if let Some(width) = cell_props.get("width") {
            let tc_w = WordNode::new(WordElementType::Unknown("tcW".to_string()))
                .with_attribute("w", width.as_str())
                .with_attribute("type", "dxa");
            children.push(tc_w);
        }

        if !children.is_empty() {
            tc_pr.children = children;
            cell.children.insert(0, tc_pr);
        }
    }

    let recognized = ["text", "width"];
    let unsupported: Vec<String> = properties.keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Remove an element at the given path.
/// Returns the path of the removed element.
pub fn remove_element(dom: &mut WordDom, path: &str) -> Result<Option<String>, HandlerError> {
    let segments = parse_path(path)?;
    if segments.len() < 2 {
        return Err(HandlerError::InvalidPath(format!(
            "cannot remove root element: {}",
            path
        )));
    }

    // Navigate to parent
    let parent_segments = &segments[..segments.len() - 1];
    let parent_path_str = format_path_segments(parent_segments);

    let parent = navigate_to_element_mut(dom, &parent_path_str)?;

    let last_seg = &segments[segments.len() - 1];
    let target_type = resolve_element_type_from_name(&last_seg.name);

    // Find matching children and their indices
    let matching_indices: Vec<usize> = parent
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.element_type == target_type
                || matches!(&c.element_type, WordElementType::Unknown(ref n) if n == &last_seg.name)
        })
        .map(|(i, _)| i)
        .collect();

    if matching_indices.is_empty() {
        return Err(HandlerError::PathNotFound(format!(
            "no {} children at {}",
            last_seg.name, parent_path_str
        )));
    }

    let idx = last_seg.index.unwrap_or(1);
    if idx == 0 || idx > matching_indices.len() {
        return Err(HandlerError::PathNotFound(format!(
            "index {} out of range at {}",
            idx, path
        )));
    }

    let child_idx = matching_indices[idx - 1];
    parent.children.remove(child_idx);

    Ok(Some(path.to_string()))
}

fn resolve_element_type_from_name(name: &str) -> WordElementType {
    crate::navigation::resolve_element_type_from_name(name)
}

fn format_path_segments(segments: &[handler_common::PathSegment]) -> String {
    let mut result = String::new();
    for seg in segments {
        result.push('/');
        result.push_str(&seg.to_path_fragment());
    }
    result
}

/// Move an element from source to target parent.
pub fn move_element(
    dom: &mut WordDom,
    source: &str,
    target_parent: Option<&str>,
    position: InsertPosition,
) -> Result<String, HandlerError> {
    // Clone the source element first
    let source_node = navigate_to_element(dom, source)?.clone();

    // Remove from source
    remove_element(dom, source)?;

    // Add to target
    let target = target_parent.unwrap_or("/body");
    let elem_type = source_node.element_type.to_path_name();

    let new_path = crate::add::add_element(
        dom,
        target,
        elem_type,
        position,
        &std::collections::HashMap::new(),
        None,
    )?;

    // Now replace the added empty element with the cloned source content
    let target_node = navigate_to_element_mut(dom, &new_path)?;
    *target_node = source_node;

    Ok(new_path)
}

// ─── Bookmark Set Properties ──────────────────────────────────

/// Set properties on a BookmarkStart element.
/// Supported properties:
/// - name: rename the bookmark (rejects duplicates)
/// - text: replace content between BookmarkStart and BookmarkEnd
/// - id: update the bookmark ID (updates both start and paired end)
fn set_bookmark_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // First pass: read-only to gather info
    let node = navigate_to_element(dom, path)?;
    if node.element_type != WordElementType::BookmarkStart {
        return Err(HandlerError::InvalidArgument(format!(
            "path does not point to a bookmarkStart: {:?}",
            node.element_type
        )));
    }
    let current_id = node.attributes.get("id").cloned().unwrap_or_default();

    // Validate name if present
    if let Some(new_name) = properties.get("name") {
        crate::helpers::validate_bookmark_name(new_name)?;
        let body = dom.root.children.iter().find(|c| c.element_type == WordElementType::Body);
        if let Some(body) = body {
            if find_other_bookmark_by_name(body, new_name) {
                return Err(HandlerError::InvalidArgument(format!(
                    "bookmark name '{}' already exists; pick a unique name.", new_name
                )));
            }
        }
    }

    // Validate id if present
    if let Some(new_id) = properties.get("id") {
        let id_val: i32 = new_id.parse().map_err(|_| HandlerError::InvalidArgument(format!(
            "bookmark id must be a non-negative integer, got: {}", new_id
        )))?;
        if id_val < 0 {
            return Err(HandlerError::InvalidArgument("bookmark id must be non-negative".to_string()));
        }
    }

    // Second pass: mutations
    // Handle 'name' property
    if let Some(new_name) = properties.get("name") {
        let node = navigate_to_element_mut(dom, path)?;
        node.attributes.insert("name".to_string(), new_name.clone());
    }

    // Handle 'text' property: replace content between BookmarkStart and BookmarkEnd
    if let Some(new_text) = properties.get("text") {
        let parent_path = crate::navigation::parent_path(path)
            .ok_or_else(|| HandlerError::InvalidPath("bookmark has no parent".to_string()))?;
        let parent = navigate_to_element_mut(dom, &parent_path)?;

        let start_idx = parent.children.iter().position(|c| c.element_type == WordElementType::BookmarkStart
            && c.attributes.get("id").map(|s| s.as_str()) == Some(&current_id))
            .ok_or_else(|| HandlerError::PathNotFound("bookmarkStart not found in parent".to_string()))?;

        let end_idx = parent.children.iter().position(|c| c.element_type == WordElementType::BookmarkEnd
            && c.attributes.get("id").map(|s| s.as_str()) == Some(&current_id))
            .ok_or_else(|| HandlerError::PathNotFound("bookmarkEnd not found in parent".to_string()))?;

        // Collect indices of content to remove (between start and end)
        let remove_indices: Vec<usize> = (start_idx + 1..end_idx)
            .filter(|i| {
                let child = &parent.children[*i];
                matches!(child.element_type, WordElementType::Run | WordElementType::Text | WordElementType::Hyperlink)
            })
            .collect();

        // Remove in reverse to keep indices stable
        for idx in remove_indices.iter().rev() {
            parent.children.remove(*idx);
        }

        // Insert new run after BookmarkStart
        let run = crate::add::make_run_with_text(new_text, &HashMap::new());
        let new_start_idx = parent.children.iter().position(|c| c.element_type == WordElementType::BookmarkStart
            && c.attributes.get("id").map(|s| s.as_str()) == Some(&current_id))
            .unwrap_or(start_idx);
        parent.children.insert(new_start_idx + 1, run);
    }

    // Handle 'id' property: update both BookmarkStart and paired BookmarkEnd
    if let Some(new_id) = properties.get("id") {
        let node = navigate_to_element_mut(dom, path)?;
        node.attributes.insert("id".to_string(), new_id.clone());
        update_paired_bookmark_end(dom, &current_id, new_id)?;
    }

    let recognized = ["name", "text", "id"];
    let unsupported: Vec<String> = properties.keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Set properties on a BookmarkEnd element (minimal: only id update).
fn set_bookmark_end_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let node = navigate_to_element_mut(dom, path)?;

    if node.element_type != WordElementType::BookmarkEnd {
        return Err(HandlerError::InvalidArgument(format!(
            "path does not point to a bookmarkEnd: {:?}",
            node.element_type
        )));
    }

    // Handle 'id' property
    if let Some(new_id) = properties.get("id") {
        node.attributes.insert("id".to_string(), new_id.clone());
    }

    let recognized = ["id"];
    let unsupported: Vec<String> = properties.keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Check if any BookmarkStart in the document has the given name.
fn find_other_bookmark_by_name(node: &WordNode, name: &str) -> bool {
    if node.element_type == WordElementType::BookmarkStart
        && node.attributes.get("name").map(|s| s.as_str()) == Some(name) {
            return true;
        }
    node.children.iter().any(|c| find_other_bookmark_by_name(c, name))
}

/// Update all BookmarkEnd nodes matching the old ID to the new ID.
fn update_paired_bookmark_end(
    dom: &mut WordDom,
    old_id: &str,
    new_id: &str,
) -> Result<(), HandlerError> {
    let body_idx = dom
        .root
        .children
        .iter()
        .position(|c| c.element_type == WordElementType::Body)
        .ok_or_else(|| HandlerError::OperationFailed("body element not found".to_string()))?;

    update_bookmark_end_in_node(&mut dom.root.children[body_idx], old_id, new_id);
    Ok(())
}

fn update_bookmark_end_in_node(node: &mut WordNode, old_id: &str, new_id: &str) {
    if node.element_type == WordElementType::BookmarkEnd
        && node.attributes.get("id").map(|s| s.as_str()) == Some(old_id) {
            node.attributes.insert("id".to_string(), new_id.to_string());
        }
    for child in &mut node.children {
        update_bookmark_end_in_node(child, old_id, new_id);
    }
}
