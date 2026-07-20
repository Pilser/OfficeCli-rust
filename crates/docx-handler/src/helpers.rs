use crate::dom_types::{WordElementType, WordNode};
use handler_common::HandlerError;

/// Split a run at a text offset.
/// Given a run node and a byte offset within its text content, split it into two runs:
/// - Left run: text from 0..offset (preserving original rPr)
/// - Right run: text from offset..end (preserving original rPr)
///
/// Returns (left_run, right_run). If offset is 0, left is None.
/// If offset equals text length, right is None.
pub fn split_run_at_offset(run: &WordNode, offset: usize) -> (Option<WordNode>, Option<WordNode>) {
    // Find the w:t element and its text
    let text_node = run
        .children
        .iter()
        .find(|c| c.element_type == WordElementType::Text);
    if text_node.is_none() {
        // No text in this run; can't split
        return (Some(run.clone()), None);
    }
    let text_node = text_node.unwrap();
    let text = text_node.text_content.as_deref().unwrap_or("");

    if offset == 0 {
        return (None, Some(run.clone()));
    }
    if offset >= text.len() {
        return (Some(run.clone()), None);
    }

    // Get run properties to clone into both halves
    let rpr = run.run_properties().cloned();

    // Left run: text[0..offset]
    let left_text = text[..offset].to_string();
    let left_preserve =
        text_node.has_preserve_space() || left_text.ends_with(' ') || left_text.starts_with(' ');
    let left_t = WordNode::new(WordElementType::Text).with_text(&left_text);
    let mut left_t = left_t;
    if left_preserve {
        left_t
            .attributes
            .insert("xml:space".to_string(), "preserve".to_string());
        left_t.preserve_space = true;
    }

    let mut left_children = Vec::new();
    if let Some(rpr) = &rpr {
        left_children.push(rpr.clone());
    }
    left_children.push(left_t);

    let left_run = WordNode::new(WordElementType::Run).with_children(left_children);

    // Right run: text[offset..end]
    let right_text = text[offset..].to_string();
    let _right_preserve = true; // Always preserve space on the right part after split
    let mut right_t = WordNode::new(WordElementType::Text).with_text(&right_text);
    right_t
        .attributes
        .insert("xml:space".to_string(), "preserve".to_string());
    right_t.preserve_space = true;

    let mut right_children = Vec::new();
    if let Some(rpr) = &rpr {
        right_children.push(rpr.clone());
    }
    right_children.push(right_t);

    let right_run = WordNode::new(WordElementType::Run).with_children(right_children);

    (Some(left_run), Some(right_run))
}

/// Build a new run carrying `text`, inheriting the source run's rPr (so the
/// replacement keeps the original formatting unless overridden later).
pub fn build_run_with_text(source_run: &WordNode, text: &str) -> WordNode {
    let mut children = Vec::new();
    if let Some(rpr) = source_run.run_properties() {
        children.push(rpr.clone());
    }
    let mut t = WordNode::new(WordElementType::Text).with_text(text);
    if text.starts_with(' ') || text.ends_with(' ') {
        t.attributes
            .insert("xml:space".to_string(), "preserve".to_string());
        t.preserve_space = true;
    }
    children.push(t);
    WordNode::new(WordElementType::Run).with_children(children)
}

/// Find the run and offset within a paragraph that corresponds to a global text offset.
/// Returns (run_index_0based, offset_within_run_text).
pub fn find_run_at_offset(
    para: &WordNode,
    para_text_offset: usize,
) -> Result<(usize, usize), HandlerError> {
    let mut current_offset = 0;
    let runs = para.runs();

    for (i, run) in runs.iter().enumerate() {
        let run_text = run.paragraph_text();
        let run_len = run_text.len();

        if current_offset + run_len > para_text_offset {
            // The target offset falls within this run
            let offset_in_run = para_text_offset - current_offset;
            return Ok((i, offset_in_run));
        }
        current_offset += run_len;
    }

    Err(HandlerError::PathNotFound(format!(
        "offset {} beyond paragraph text length {}",
        para_text_offset, current_offset
    )))
}

/// Count body-level content elements (paragraphs + tables) for path indexing.
pub fn count_body_content_elements(body: &WordNode) -> usize {
    body.children
        .iter()
        .filter(|c| c.element_type.is_body_child())
        .count()
}

/// Find the 1-based index of a body child element among its siblings of the same type.
pub fn find_sibling_index(parent: &WordNode, target: &WordNode) -> usize {
    let mut idx = 0;
    for child in &parent.children {
        if child.element_type == target.element_type {
            idx += 1;
            if std::ptr::eq(child, target) {
                return idx;
            }
        }
    }
    idx
}

/// Find a paragraph by its paraId attribute.
pub fn find_paragraph_by_para_id(dom: &crate::dom_types::WordDom, para_id: &str) -> Option<usize> {
    let body = dom.body()?;
    let mut para_idx = 0;
    for child in &body.children {
        if child.element_type == WordElementType::Paragraph {
            para_idx += 1;
            if child.attributes.get("paraId").map(|s| s.as_str()) == Some(para_id) {
                return Some(para_idx);
            }
        }
    }
    None
}

/// Generate a bookmark ID by finding the max existing w:id across all BookmarkStart
/// nodes in the document body and incrementing by 1. Returns "1" if no bookmarks exist.
pub fn generate_bookmark_id(dom: &crate::dom_types::WordDom) -> String {
    (max_bookmark_id(dom) + 1).to_string()
}

pub fn max_bookmark_id(dom: &crate::dom_types::WordDom) -> i32 {
    let body = dom
        .root
        .children
        .iter()
        .find(|c| c.element_type == crate::dom_types::WordElementType::Body);
    body.map(max_bookmark_id_in_node).unwrap_or(0)
}

fn max_bookmark_id_in_node(node: &crate::dom_types::WordNode) -> i32 {
    let self_id = if node.element_type == crate::dom_types::WordElementType::BookmarkStart {
        node.attributes
            .get("id")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
    } else {
        0
    };
    let child_max = node
        .children
        .iter()
        .map(max_bookmark_id_in_node)
        .max()
        .unwrap_or(0);
    self_id.max(child_max)
}

/// Validate a bookmark name. Rejects characters that break path selectors or
/// bare attribute selectors. Allowed: letters, digits, '.', '_', '-'.
pub fn validate_bookmark_name(name: &str) -> Result<(), HandlerError> {
    if name.is_empty() {
        return Err(HandlerError::InvalidArgument(
            "'name' property is required for bookmark".to_string(),
        ));
    }
    // Path-special characters break selectors
    if name.contains('/') || name.contains('[') || name.contains(']') {
        return Err(HandlerError::InvalidArgument(format!(
            "Bookmark name '{}' contains path-special characters ('/', '[', ']'). \
             Use only letters, digits, '.', '_', '-' in bookmark names.",
            name
        )));
    }
    // Whitespace, leading @, quotes break bare attribute selectors
    if name.chars().any(char::is_whitespace)
        || name.starts_with('@')
        || name.contains('\'')
        || name.contains('"')
    {
        return Err(HandlerError::InvalidArgument(format!(
            "Bookmark name '{}' contains whitespace or quote/@ chars that prevent \
             addressing via bare attribute selectors. Use only letters, digits, '.', '_', '-'.",
            name
        )));
    }
    Ok(())
}

/// Quote an attribute predicate value if the bare form would be rejected by
/// ValidateAndNormalizePredicate. Bare values must have no whitespace, no
/// leading '@' or quote chars. Embedded double quotes are rejected outright.
pub fn quote_attr_value_if_needed(value: &str) -> Result<String, HandlerError> {
    if value.contains('"') {
        return Err(HandlerError::InvalidArgument(format!(
            "Name '{}' contains embedded double-quote, which cannot be represented \
             in an attribute selector.",
            value
        )));
    }
    let needs_quote = value.is_empty()
        || value.starts_with('@')
        || value.starts_with('\'')
        || value.chars().any(char::is_whitespace);
    if needs_quote {
        Ok(format!("\"{}\"", value))
    } else {
        Ok(value.to_string())
    }
}

/// Generate a unique paragraph ID (hex string, 8 chars).
pub fn generate_para_id() -> String {
    use uuid::Uuid;
    let uuid = Uuid::new_v4();
    // Take first 8 hex chars for a short paraId
    uuid.to_string().replace('-', "")[..8].to_string()
}

/// Build a w:rPr (run properties) XML element from format properties.
/// Full vocabulary: bold/b, italic/i, underline/u, strike/strikeout,
/// font/fontFamily, size/fontSize, color/fontColor, bgColor/highlight/bg,
/// shading/shd, caps, smallCaps, vanish/hidden, kern, characterSpacing,
/// border, emphasisMark, lang, rightToLeft
pub fn build_run_properties(props: &std::collections::HashMap<String, String>) -> Option<WordNode> {
    if props.is_empty() {
        return None;
    }

    let mut rpr = WordNode::new(WordElementType::RunProperties);
    let mut children = Vec::new();

    for (key, value) in props {
        match key.as_str() {
            "bold" | "b" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown("b".to_string())));
                } else if value == "false" || value == "0" {
                    children.push(
                        WordNode::new(WordElementType::Unknown("b".to_string()))
                            .with_attribute("val", "0"),
                    );
                }
            }
            "italic" | "i" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown("i".to_string())));
                } else if value == "false" || value == "0" {
                    children.push(
                        WordNode::new(WordElementType::Unknown("i".to_string()))
                            .with_attribute("val", "0"),
                    );
                }
            }
            "underline" | "u" => {
                let val = if value.is_empty() {
                    "single"
                } else {
                    value.as_str()
                };
                children.push(
                    WordNode::new(WordElementType::Unknown("u".to_string()))
                        .with_attribute("val", val),
                );
            }
            "strike" | "strikeout" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "strike".to_string(),
                    )));
                }
            }
            "font" | "fontFamily" | "font.name" => {
                children.push(
                    WordNode::new(WordElementType::Unknown("rFonts".to_string()))
                        .with_attribute("ascii", value.as_str())
                        .with_attribute("hAnsi", value.as_str())
                        .with_attribute("cs", value.as_str()),
                );
            }
            "size" | "fontSize" | "font.size" => {
                let half_points = if let Ok(pt) = value.parse::<f32>() {
                    (pt * 2.0) as usize
                } else {
                    24
                };
                children.push(
                    WordNode::new(WordElementType::Unknown("sz".to_string()))
                        .with_attribute("val", half_points.to_string().as_str()),
                );
                children.push(
                    WordNode::new(WordElementType::Unknown("szCs".to_string()))
                        .with_attribute("val", half_points.to_string().as_str()),
                );
            }
            "color" | "fontColor" | "font.color" => {
                let color_val = value.strip_prefix('#').unwrap_or(value);
                children.push(
                    WordNode::new(WordElementType::Unknown("color".to_string()))
                        .with_attribute("val", color_val),
                );
            }
            "bgColor" | "highlight" | "bg" => {
                let color_val = value.strip_prefix('#').unwrap_or(value);
                if matches!(
                    color_val.to_lowercase().as_str(),
                    "yellow"
                        | "green"
                        | "cyan"
                        | "magenta"
                        | "blue"
                        | "red"
                        | "darkblue"
                        | "darkcyan"
                        | "darkgreen"
                        | "darkmagenta"
                        | "darkred"
                        | "darkyellow"
                        | "white"
                        | "lightgray"
                        | "darkgray"
                        | "black"
                        | "none"
                ) {
                    children.push(
                        WordNode::new(WordElementType::Unknown("highlight".to_string()))
                            .with_attribute("val", color_val.to_lowercase()),
                    );
                } else {
                    children.push(
                        WordNode::new(WordElementType::Unknown("shd".to_string()))
                            .with_attribute("val", "clear")
                            .with_attribute("color", "auto")
                            .with_attribute("fill", color_val),
                    );
                }
            }
            "shading" | "shd" => {
                let value = value.strip_prefix('#').unwrap_or(value);
                let parts: Vec<&str> = value.split(';').collect();
                let (pat, fill, clr) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => ("clear", parts[0], parts[1]),
                    _ => ("clear", value, "auto"),
                };
                children.push(
                    WordNode::new(WordElementType::Unknown("shd".to_string()))
                        .with_attribute("val", pat)
                        .with_attribute("color", clr)
                        .with_attribute("fill", fill),
                );
            }
            "caps" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown("caps".to_string())));
                }
            }
            "smallCaps" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "smallCaps".to_string(),
                    )));
                }
            }
            "vanish" | "hidden" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "vanish".to_string(),
                    )));
                }
            }
            "kern" => {
                let kern_val = value.parse::<usize>().unwrap_or(0);
                children.push(
                    WordNode::new(WordElementType::Unknown("kern".to_string()))
                        .with_attribute("val", kern_val.to_string().as_str()),
                );
            }
            "characterSpacing" | "spacing" => {
                let spacing_val = value.parse::<i32>().unwrap_or(0);
                children.push(
                    WordNode::new(WordElementType::Unknown("spacing".to_string()))
                        .with_attribute("val", spacing_val.to_string().as_str()),
                );
            }
            "emphasisMark" => {
                children.push(
                    WordNode::new(WordElementType::Unknown("em".to_string()))
                        .with_attribute("val", value.as_str()),
                );
            }
            "lang" => {
                children.push(
                    WordNode::new(WordElementType::Unknown("lang".to_string()))
                        .with_attribute("val", value.as_str()),
                );
            }
            "rightToLeft" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown("rtl".to_string())));
                }
            }
            "font.bold" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown("b".to_string())));
                }
            }
            "font.italic" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown("i".to_string())));
                }
            }
            "font.underline" => {
                children.push(
                    WordNode::new(WordElementType::Unknown("u".to_string()))
                        .with_attribute("val", value.as_str()),
                );
            }
            "font.strike" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "strike".to_string(),
                    )));
                }
            }
            "vertAlign" | "verticalAlign" => {
                if value == "superscript" || value == "subscript" {
                    children.push(
                        WordNode::new(WordElementType::Unknown("vertAlign".to_string()))
                            .with_attribute("val", value.as_str()),
                    );
                }
            }
            "dstrike" | "doubleStrike" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "dstrike".to_string(),
                    )));
                }
            }
            "position" => {
                if let Ok(pos) = value.parse::<i32>() {
                    children.push(
                        WordNode::new(WordElementType::Unknown("position".to_string()))
                            .with_attribute("val", pos.to_string().as_str()),
                    );
                }
            }
            "noProof" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "noProof".to_string(),
                    )));
                }
            }
            _ => {}
        }
    }

    if children.is_empty() {
        return None;
    }

    rpr.children = children;
    Some(rpr)
}

/// Build paragraph properties from format properties.
/// Full vocabulary: style/pStyle, alignment/jc, indentLeft, indentRight,
/// indent, firstLine, hanging, spacingBefore, spacingAfter, lineSpacing,
/// keepLines, keepNext, outlineLevel, numId, numLevel, pageBreakBefore,
/// widowControl, border, shading/shd
pub fn build_paragraph_properties(
    props: &std::collections::HashMap<String, String>,
) -> Option<WordNode> {
    if props.is_empty() {
        return None;
    }

    let mut ppr = WordNode::new(WordElementType::ParagraphProperties);
    let mut children = Vec::new();

    // Collect indent attributes into a single w:ind node
    let mut ind_attrs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // Collect spacing attributes into a single w:spacing node
    let mut spacing_attrs: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // Collect numbering properties
    let mut num_id: Option<String> = None;
    let mut num_level: Option<String> = None;
    // Collect border entries
    let mut border_entries: Vec<WordNode> = Vec::new();
    let mut has_shading = false;
    let mut shading_val = "";

    for (key, value) in props {
        match key.as_str() {
            "style" | "pStyle" => {
                children.push(
                    WordNode::new(WordElementType::Unknown("pStyle".to_string()))
                        .with_attribute("val", value.as_str()),
                );
            }
            "alignment" | "jc" => {
                children.push(
                    WordNode::new(WordElementType::Unknown("jc".to_string()))
                        .with_attribute("val", value.as_str()),
                );
            }
            "indentLeft" => {
                ind_attrs.insert("left".to_string(), value.clone());
            }
            "indentRight" => {
                ind_attrs.insert("right".to_string(), value.clone());
            }
            "indent" => {
                ind_attrs.insert("left".to_string(), value.clone());
            }
            "firstLine" => {
                ind_attrs.insert("firstLine".to_string(), value.clone());
            }
            "hanging" => {
                ind_attrs.insert("hanging".to_string(), value.clone());
            }
            "spacingBefore" => {
                spacing_attrs.insert("before".to_string(), value.clone());
            }
            "spacingAfter" => {
                spacing_attrs.insert("after".to_string(), value.clone());
            }
            "lineSpacing" | "line" => {
                spacing_attrs.insert("line".to_string(), value.clone());
            }
            "spacing" => {
                // Generic spacing: try to parse as "before=X;after=Y;line=Z"
                if value.contains(';') {
                    for pair in value.split(';') {
                        if let Some(eq) = pair.find('=') {
                            let k = &pair[..eq];
                            let v = &pair[eq + 1..];
                            spacing_attrs.insert(k.to_string(), v.to_string());
                        }
                    }
                } else {
                    spacing_attrs.insert("after".to_string(), value.clone());
                }
            }
            "keepLines" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "keepLines".to_string(),
                    )));
                }
            }
            "keepNext" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "keepNext".to_string(),
                    )));
                }
            }
            "outlineLevel" => {
                children.push(
                    WordNode::new(WordElementType::Unknown("outlineLvl".to_string()))
                        .with_attribute("val", value.as_str()),
                );
            }
            "numId" => {
                num_id = Some(value.clone());
            }
            "numLevel" => {
                num_level = Some(value.clone());
            }
            "pageBreakBefore" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "pageBreakBefore".to_string(),
                    )));
                }
            }
            "widowControl" => {
                if value == "false" || value == "0" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "widowControl".to_string(),
                    )));
                }
            }
            "border" => {
                // Parse border format: "top=single;bottom=single;..." or "all=single" or "none"
                border_entries = build_pborder_children(value);
            }
            "borderBottom" => {
                let mut node = WordNode::new(WordElementType::Unknown("bottom".to_string()));
                node.attributes = parse_border_side_value(value);
                border_entries.push(node);
            }
            "borderTop" => {
                let mut node = WordNode::new(WordElementType::Unknown("top".to_string()));
                node.attributes = parse_border_side_value(value);
                border_entries.push(node);
            }
            "borderLeft" => {
                let mut node = WordNode::new(WordElementType::Unknown("left".to_string()));
                node.attributes = parse_border_side_value(value);
                border_entries.push(node);
            }
            "borderRight" => {
                let mut node = WordNode::new(WordElementType::Unknown("right".to_string()));
                node.attributes = parse_border_side_value(value);
                border_entries.push(node);
            }
            "borderAround" => {
                let attrs = parse_border_side_value(value);
                for side in ["top", "bottom", "left", "right"] {
                    let mut node = WordNode::new(WordElementType::Unknown(side.to_string()));
                    node.attributes = attrs.clone();
                    border_entries.push(node);
                }
            }
            "shading" | "shd" => {
                has_shading = true;
                shading_val = value.as_str();
            }
            "tabs" => {
                let mut tabs_node = WordNode::new(WordElementType::Unknown("tabs".to_string()));
                for tab_spec in value.split('|') {
                    let tab_spec = tab_spec.trim();
                    if tab_spec.is_empty() {
                        continue;
                    }
                    let mut pos_val = "1440";
                    let mut align_val = "left";
                    let mut leader_val = "none";
                    for pair in tab_spec.split(';') {
                        if let Some(eq) = pair.find('=') {
                            let k = pair[..eq].trim();
                            let v = pair[eq + 1..].trim();
                            match k {
                                "pos" | "position" => pos_val = v,
                                "align" | "alignment" => align_val = v,
                                "leader" => leader_val = v,
                                _ => {}
                            }
                        }
                    }
                    tabs_node.children.push(
                        WordNode::new(WordElementType::Unknown("tab".to_string()))
                            .with_attribute("val", align_val)
                            .with_attribute("pos", pos_val)
                            .with_attribute("leader", leader_val),
                    );
                }
                if !tabs_node.children.is_empty() {
                    children.push(tabs_node);
                }
            }
            "textAlignment" | "vertAlign" => {
                if matches!(value.as_str(), "auto" | "top" | "center" | "baseline" | "bottom") {
                    children.push(
                        WordNode::new(WordElementType::Unknown("textAlignment".to_string()))
                            .with_attribute("val", value.as_str()),
                    );
                }
            }
            "contextualSpacing" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "contextualSpacing".to_string(),
                    )));
                }
            }
            "suppressLineNumbers" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "suppressLineNumbers".to_string(),
                    )));
                }
            }
            "suppressAutoHyphens" => {
                if value == "true" || value == "1" {
                    children.push(WordNode::new(WordElementType::Unknown(
                        "suppressAutoHyphens".to_string(),
                    )));
                }
            }
            _ => {}
        }
    }

    // Build single w:ind node from collected attributes
    if !ind_attrs.is_empty() {
        let mut ind = WordNode::new(WordElementType::Unknown("ind".to_string()));
        ind.attributes = ind_attrs;
        children.push(ind);
    }

    // Build single w:spacing node from collected attributes
    if !spacing_attrs.is_empty() {
        let mut spacing = WordNode::new(WordElementType::Unknown("spacing".to_string()));
        spacing.attributes = spacing_attrs;
        children.push(spacing);
    }

    // Build numbering properties
    if num_id.is_some() || num_level.is_some() {
        let mut num_pr = WordNode::new(WordElementType::Unknown("numPr".to_string()));
        if let Some(id) = num_id {
            num_pr.children.push(
                WordNode::new(WordElementType::Unknown("numId".to_string()))
                    .with_attribute("val", id.as_str()),
            );
        } else {
            num_pr.children.push(
                WordNode::new(WordElementType::Unknown("numId".to_string()))
                    .with_attribute("val", "0"),
            );
        }
        if let Some(level) = num_level {
            num_pr.children.push(
                WordNode::new(WordElementType::Unknown("ilvl".to_string()))
                    .with_attribute("val", level.as_str()),
            );
        } else {
            num_pr.children.push(
                WordNode::new(WordElementType::Unknown("ilvl".to_string()))
                    .with_attribute("val", "0"),
            );
        }
        children.push(num_pr);
    }

    // Build border node
    if !border_entries.is_empty() {
        let mut p_bdr = WordNode::new(WordElementType::Unknown("pBdr".to_string()));
        p_bdr.children = border_entries;
        children.push(p_bdr);
    }

    // Build shading node
    if has_shading {
        let shd = crate::mutations::build_shd_node(shading_val);
        children.push(shd);
    }

    if children.is_empty() {
        return None;
    }

    ppr.children = children;
    Some(ppr)
}

/// Parse a border side value string into OOXML attribute map.
/// Format: "color=1F4E79;size=8;space=1;val=single"
/// All keys are optional; defaults are applied for missing fields.
fn parse_border_side_value(value: &str) -> std::collections::HashMap<String, String> {
    let mut attrs = std::collections::HashMap::new();
    attrs.insert("val".to_string(), "single".to_string());
    attrs.insert("sz".to_string(), "4".to_string());
    attrs.insert("space".to_string(), "1".to_string());
    attrs.insert("color".to_string(), "000000".to_string());

    for pair in value.split(';') {
        if let Some(eq) = pair.find('=') {
            let k = pair[..eq].trim();
            let v = pair[eq + 1..].trim();
            match k {
                "val" => {
                    attrs.insert("val".to_string(), v.to_string());
                }
                "size" => {
                    attrs.insert("sz".to_string(), v.to_string());
                }
                "space" => {
                    attrs.insert("space".to_string(), v.to_string());
                }
                "color" => {
                    attrs.insert("color".to_string(), v.to_string());
                }
                _ => {}
            }
        }
    }
    attrs
}

fn build_pborder_children(value: &str) -> Vec<WordNode> {
    let mut entries = Vec::new();
    if value == "none" || value == "0" {
        for border_name in ["top", "bottom", "left", "right"] {
            entries.push(
                WordNode::new(WordElementType::Unknown(border_name.to_string()))
                    .with_attribute("val", "none")
                    .with_attribute("sz", "0")
                    .with_attribute("space", "0")
                    .with_attribute("color", "auto"),
            );
        }
    } else if value.starts_with("all=") || value == "single" || value == "thin" {
        let style = value.strip_prefix("all=").unwrap_or("single");
        for border_name in ["top", "bottom", "left", "right"] {
            entries.push(
                WordNode::new(WordElementType::Unknown(border_name.to_string()))
                    .with_attribute("val", style)
                    .with_attribute("sz", "4")
                    .with_attribute("space", "1")
                    .with_attribute("color", "auto"),
            );
        }
    } else {
        for pair in value.split(';') {
            if let Some(eq) = pair.find('=') {
                let name = &pair[..eq];
                let style = &pair[eq + 1..];
                entries.push(
                    WordNode::new(WordElementType::Unknown(name.to_string()))
                        .with_attribute("val", style)
                        .with_attribute("sz", "4")
                        .with_attribute("space", "1")
                        .with_attribute("color", "auto"),
                );
            }
        }
    }
    entries
}
