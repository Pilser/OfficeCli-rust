use crate::dom_types::{WordDom, WordElementType, WordNode};
use crate::helpers::{build_paragraph_properties, build_run_properties};
use crate::navigation::{navigate_to_element, navigate_to_element_mut, parse_path};
use handler_common::{
    self, extract_find_replace_props, replace_in_string, FindReplaceOptions, HandlerError,
    InsertPosition,
};
use oxml::OxmlPackage;
use std::collections::HashMap;

/// Set properties on an element at a given path.
/// Dispatches to element-type-specific handlers matching the C# WordHandler.Set.Dispatch
/// routing. Supported targets and their property vocabularies:
///
/// **Paragraph (p)**: text, style/pStyle, alignment/jc, indent, spacing, lineSpacing,
///   keepLines, keepNext, outlineLevel, numId, numLevel, border, shading, pageBreakBefore
///
/// **Run (r)**: text, bold/b, italic/i, underline/u, strike, font/fontFamily, size/fontSize,
///   color/fontColor, bgColor/highlight, shading/shd, caps, smallCaps, vanish/hidden,
///   kern, spacing, characterSpacing, border, emphasisMark, lang, rightToLeft, font.font*
///
/// **Text (t)**: text
///
/// **Table (tbl)**: style/tblStyle, width, border, shading, alignment, indent,
///   firstRow, lastRow, firstCol, lastCol, rowBandSize, colBandSize, layout
///
/// **Row (tr)**: height, cantSplit, tableHeader, hidden
///
/// **Cell (tc)**: text, width, shading, border, vAlign, vMerge, gridSpan, noWrap, textDirection
///
/// **Bookmark**: name, text, id
///
/// **SDT**: alias/name, tag, lock, text
///
/// **Section (sectPr)**: pageWidth, pageHeight, orientation, marginLeft/Right/Top/Bottom,
///   columns, headerDistance, footerDistance, gutter
///
/// **Body/document-level** (path "/"): protection, protectionEnforced, docDefaults, defaultTabStop
///
/// **Styles** (/styles/*): basedOn, next, name, qFormat, uiPriority, hidden,
///   pPr/rPr properties (routed through style element helpers)
///
/// **Comments**: text, author, initials, date
///
/// **Footnote/Endnote**: text
///
/// **Hyperlink**: url/target, tooltip
///
/// Returns list of unrecognized property keys (empty = all applied).
pub fn set_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // Find/replace short-circuit: when the property map carries `find` (+ optional
    // `replace`, `caseSensitive`, `wholeWord`, `regex`), operate on text content
    // instead of formatting props. Mirrors C# FindHelpers integration at Set.find=...
    if properties.contains_key("find") {
        return apply_find_replace(dom, path, properties);
    }

    let segments = parse_path(path)?;
    if segments.is_empty() {
        return Err(HandlerError::InvalidPath("empty path".to_string()));
    }

    // Path-based routing: some paths target special elements before
    // the last segment type is considered
    let path_str = path.to_lowercase();

    // Document-level properties (path "/" or "/body")
    if path_str == "/" || path_str == "/body" {
        return set_document_properties(dom, path, properties);
    }

    // Section properties (/sectPr or body/sectPr)
    if path_str.contains("sectpr") {
        return set_section_properties(dom, path, properties);
    }

    // Styles routing
    // Styles/comments/footnotes/endnotes are routed to part-aware setters
    // in handler.rs before parse_dom() is called, so they never reach here.

    // Hyperlink routing
    if path_str.contains("hyperlink") {
        return set_hyperlink_properties(dom, path, properties);
    }

    // SDT routing
    if path_str.contains("sdt") {
        return set_sdt_properties(dom, path, properties);
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
        "sdt" => set_sdt_properties(dom, path, properties),
        "sectPr" => set_section_properties(dom, path, properties),
        other => Err(HandlerError::UnsupportedProperty(format!(
            "cannot set properties on element type: {}",
            other
        ))),
    }
}

/// Set paragraph properties. Full vocabulary matching C# WordHandler.Set.Element:
/// text, style/pStyle, alignment/jc, indent (indentLeft, indentRight, firstLine, hanging),
/// spacing (spacingBefore, spacingAfter, lineSpacing), keepLines, keepNext, outlineLevel,
/// numId, numLevel, border, shading/shd, pageBreakBefore, widowControl
fn set_paragraph_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // Property keys that are paragraph-level (w:pPr) — everything else is
    // treated as run-level and applied to the paragraph's runs.
    const PARA_LEVEL_KEYS: &[&str] = &[
        "style",
        "pStyle",
        "alignment",
        "jc",
        "indentLeft",
        "indentRight",
        "indent",
        "firstLine",
        "hanging",
        "spacingBefore",
        "spacingAfter",
        "lineSpacing",
        "spacing",
        "keepLines",
        "keepNext",
        "outlineLevel",
        "numId",
        "numLevel",
        "listStyle",
        "border",
        "shading",
        "shd",
        "pageBreakBefore",
        "widowControl",
    ];

    // Run-level keys (forwarded to runs)
    const RUN_LEVEL_KEYS: &[&str] = &[
        "bold",
        "b",
        "italic",
        "i",
        "underline",
        "u",
        "strike",
        "strikeout",
        "font",
        "fontFamily",
        "size",
        "fontSize",
        "color",
        "fontColor",
        "bgColor",
        "highlight",
        "bg",
        "shading",
        "shd",
        "caps",
        "smallCaps",
        "vanish",
        "hidden",
        "kern",
        "characterSpacing",
        "emphasisMark",
        "lang",
        "rightToLeft",
    ];

    let para = navigate_to_element_mut(dom, path)?;

    // Check if text property is set — this changes paragraph text
    if let Some(new_text) = properties.get("text") {
        set_paragraph_text(para, new_text);
    }

    // Apply run-level properties to all runs in this paragraph
    let run_props: HashMap<String, String> = properties
        .iter()
        .filter(|(k, _)| RUN_LEVEL_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !run_props.is_empty() {
        for child in &mut para.children {
            if child.element_type == WordElementType::Run {
                apply_run_props_to_run(child, &run_props);
            } else if child.element_type == WordElementType::Hyperlink {
                // Apply to runs inside hyperlinks too
                for link_child in &mut child.children {
                    if link_child.element_type == WordElementType::Run {
                        apply_run_props_to_run(link_child, &run_props);
                    }
                }
            }
        }
    }

    // Apply paragraph-level properties to pPr
    let ppr_props: HashMap<String, String> = properties
        .iter()
        .filter(|(k, _)| PARA_LEVEL_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !ppr_props.is_empty() {
        // Merge with existing pPr instead of replacing entirely
        // Remove existing pPr if present (we'll rebuild it with merged props)
        let existing_ppr = para
            .children
            .iter()
            .find(|c| c.element_type == WordElementType::ParagraphProperties);
        let merged_props = if let Some(ppr_node) = existing_ppr {
            // Start with existing pPr children converted to props
            merge_ppr_into_props(ppr_node, &ppr_props)
        } else {
            ppr_props.clone()
        };

        para.children
            .retain(|c| c.element_type != WordElementType::ParagraphProperties);

        if let Some(new_ppr) = build_paragraph_properties(&merged_props) {
            para.children.insert(0, new_ppr);
        }
    }

    // Handle pageBreak property: insert <w:r><w:br w:type="page"/></w:r>
    // at the beginning of the paragraph content (after pPr).
    if let Some(val) = properties
        .get("pageBreak")
        .or_else(|| properties.get("page-break"))
    {
        if val == "true" || val == "1" {
            // Remove any existing page break run we may have inserted before
            para.children.retain(|c| {
                if c.element_type != WordElementType::Run {
                    return true;
                }
                let is_page_break = c.children.len() == 1
                    && c.children[0].element_type == WordElementType::Break
                    && c.children[0].attributes.get("type").map(|s| s.as_str()) == Some("page");
                !is_page_break
            });

            let br_node = WordNode::new(WordElementType::Break)
                .with_attribute("type", "page");
            let run = WordNode::new(WordElementType::Run)
                .with_children(vec![br_node]);

            // Insert after pPr if it exists, otherwise at position 0
            let insert_pos = if para
                .children
                .first()
                .map_or(false, |c| c.element_type == WordElementType::ParagraphProperties)
            {
                1
            } else {
                0
            };
            para.children.insert(insert_pos, run);
        }
    }

    // Recognized = text + pageBreak + page-break + all PARA_LEVEL_KEYS + all RUN_LEVEL_KEYS
    let recognized: Vec<&str> = {
        let mut v = vec!["text", "pageBreak", "page-break"];
        v.extend_from_slice(PARA_LEVEL_KEYS);
        v.extend_from_slice(RUN_LEVEL_KEYS);
        v
    };
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Apply run-level properties to a single w:r element.
/// Merges with existing rPr if present, otherwise creates a new rPr.
fn apply_run_props_to_run(run: &mut WordNode, props: &HashMap<String, String>) {
    // Find existing rPr
    let existing_rpr = run
        .children
        .iter()
        .find(|c| c.element_type == WordElementType::RunProperties);

    let merged_props = if let Some(rpr_node) = existing_rpr {
        merge_rpr_into_props(rpr_node, props)
    } else {
        props.clone()
    };

    run.children
        .retain(|c| c.element_type != WordElementType::RunProperties);

    if let Some(new_rpr) = build_run_properties(&merged_props) {
        run.children.insert(0, new_rpr);
    }
}

/// Merge existing pPr node children into the new properties map, preserving
/// properties that aren't being overwritten.
fn merge_ppr_into_props(
    ppr_node: &WordNode,
    new_props: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = HashMap::new();

    // Extract existing pPr properties from child elements
    for child in &ppr_node.children {
        let name = match &child.element_type {
            WordElementType::Unknown(n) => n.as_str(),
            _ => continue,
        };
        match name {
            "pStyle" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("style") && !new_props.contains_key("pStyle") {
                        merged.insert("pStyle".to_string(), val.clone());
                    }
                }
            }
            "jc" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("alignment") && !new_props.contains_key("jc") {
                        merged.insert("jc".to_string(), val.clone());
                    }
                }
            }
            "ind" => {
                for (attr, key) in [
                    ("left", "indentLeft"),
                    ("right", "indentRight"),
                    ("firstLine", "firstLine"),
                    ("hanging", "hanging"),
                ] {
                    if let Some(val) = child.attributes.get(attr) {
                        if !new_props.contains_key(key) {
                            merged.insert(key.to_string(), val.clone());
                        }
                    }
                }
            }
            "spacing" => {
                for (attr, key) in [
                    ("before", "spacingBefore"),
                    ("after", "spacingAfter"),
                    ("line", "lineSpacing"),
                ] {
                    if let Some(val) = child.attributes.get(attr) {
                        if !new_props.contains_key(key) {
                            merged.insert(key.to_string(), val.clone());
                        }
                    }
                }
            }
            "keepLines" => {
                if !new_props.contains_key("keepLines") {
                    merged.insert("keepLines".to_string(), "true".to_string());
                }
            }
            "keepNext" => {
                if !new_props.contains_key("keepNext") {
                    merged.insert("keepNext".to_string(), "true".to_string());
                }
            }
            "pageBreakBefore" => {
                if !new_props.contains_key("pageBreakBefore") {
                    merged.insert("pageBreakBefore".to_string(), "true".to_string());
                }
            }
            "widowControl" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("widowControl") {
                        merged.insert("widowControl".to_string(), val.clone());
                    }
                }
            }
            "outlineLvl" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("outlineLevel") {
                        merged.insert("outlineLevel".to_string(), val.clone());
                    }
                }
            }
            "numPr" => {
                for nc in &child.children {
                    let nc_name = match &nc.element_type {
                        WordElementType::Unknown(n) => n.as_str(),
                        _ => continue,
                    };
                    if nc_name == "numId" {
                        if let Some(val) = nc.attributes.get("val") {
                            if !new_props.contains_key("numId") {
                                merged.insert("numId".to_string(), val.clone());
                            }
                        }
                    }
                    if nc_name == "ilvl" {
                        if let Some(val) = nc.attributes.get("val") {
                            if !new_props.contains_key("numLevel") {
                                merged.insert("numLevel".to_string(), val.clone());
                            }
                        }
                    }
                }
            }
            "pBdr" => {
                // Preserve existing border if not overwritten
                if !new_props.contains_key("border") {
                    merged.insert("border".to_string(), "preserve".to_string());
                }
            }
            "shd" => {
                if !new_props.contains_key("shading") && !new_props.contains_key("shd") {
                    let fill = child.attributes.get("fill").cloned().unwrap_or_default();
                    let pat = child
                        .attributes
                        .get("val")
                        .cloned()
                        .unwrap_or("clear".to_string());
                    let clr = child
                        .attributes
                        .get("color")
                        .cloned()
                        .unwrap_or("auto".to_string());
                    merged.insert("shd".to_string(), format!("{};{};{}", pat, fill, clr));
                }
            }
            _ => {}
        }
    }

    // Add all new properties (these override existing ones)
    for (k, v) in new_props {
        merged.insert(k.clone(), v.clone());
    }

    merged
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

/// Set run properties. Full vocabulary matching C# WordHandler.Set.Element:
/// text, bold/b, italic/i, underline/u, strike/strikeout, font/fontFamily, size/fontSize,
/// color/fontColor, bgColor/highlight/bg, shading/shd, caps, smallCaps, vanish/hidden,
/// kern, spacing, characterSpacing, border, emphasisMark, lang, rightToLeft,
/// font.fontName, font.bold, font.italic, font.size, font.color, font.underline, font.strike
fn set_run_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let run = navigate_to_element_mut(dom, path)?;

    // Check if text property is set
    if let Some(new_text) = properties.get("text") {
        let text_children: Vec<usize> = run
            .children
            .iter()
            .enumerate()
            .filter(|(_, c)| c.element_type == WordElementType::Text)
            .map(|(i, _)| i)
            .collect();

        if text_children.is_empty() {
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
        // Merge with existing rPr instead of replacing entirely
        let existing_rpr = run
            .children
            .iter()
            .find(|c| c.element_type == WordElementType::RunProperties);
        let merged_props = if let Some(rpr_node) = existing_rpr {
            merge_rpr_into_props(rpr_node, &run_props)
        } else {
            run_props.clone()
        };

        run.children
            .retain(|c| c.element_type != WordElementType::RunProperties);

        if let Some(new_rpr) = build_run_properties(&merged_props) {
            run.children.insert(0, new_rpr);
        }
    }

    let recognized = [
        "text",
        "bold",
        "b",
        "italic",
        "i",
        "underline",
        "u",
        "strike",
        "strikeout",
        "font",
        "fontFamily",
        "size",
        "fontSize",
        "color",
        "fontColor",
        "bgColor",
        "highlight",
        "bg",
        "shading",
        "shd",
        "caps",
        "smallCaps",
        "vanish",
        "hidden",
        "kern",
        "spacing",
        "characterSpacing",
        "border",
        "emphasisMark",
        "lang",
        "rightToLeft",
        "font.bold",
        "font.italic",
        "font.size",
        "font.color",
        "font.underline",
        "font.strike",
        "font.name",
    ];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Merge existing rPr node children into the new properties map.
fn merge_rpr_into_props(
    rpr_node: &WordNode,
    new_props: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = HashMap::new();

    for child in &rpr_node.children {
        let name = match &child.element_type {
            WordElementType::Unknown(n) => n.as_str(),
            _ => continue,
        };
        match name {
            "rFonts" => {
                if let Some(val) = child.attributes.get("ascii") {
                    if !new_props.contains_key("font") && !new_props.contains_key("fontFamily") {
                        merged.insert("font".to_string(), val.clone());
                    }
                }
            }
            "sz" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("size") && !new_props.contains_key("fontSize") {
                        merged.insert("fontSize".to_string(), val.clone());
                    }
                }
            }
            "color" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("color") && !new_props.contains_key("fontColor") {
                        merged.insert("color".to_string(), val.clone());
                    }
                }
            }
            "b" => {
                if !new_props.contains_key("bold") && !new_props.contains_key("b") {
                    merged.insert("bold".to_string(), "true".to_string());
                }
            }
            "i" => {
                if !new_props.contains_key("italic") && !new_props.contains_key("i") {
                    merged.insert("italic".to_string(), "true".to_string());
                }
            }
            "u" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("underline") && !new_props.contains_key("u") {
                        merged.insert("underline".to_string(), val.clone());
                    }
                }
            }
            "strike" => {
                if !new_props.contains_key("strike") && !new_props.contains_key("strikeout") {
                    merged.insert("strike".to_string(), "true".to_string());
                }
            }
            "highlight" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("highlight") && !new_props.contains_key("bgColor") {
                        merged.insert("highlight".to_string(), val.clone());
                    }
                }
            }
            "shd" => {
                if let Some(fill) = child.attributes.get("fill") {
                    if !new_props.contains_key("shading") && !new_props.contains_key("shd") {
                        let pat = child
                            .attributes
                            .get("val")
                            .cloned()
                            .unwrap_or("clear".to_string());
                        let clr = child
                            .attributes
                            .get("color")
                            .cloned()
                            .unwrap_or("auto".to_string());
                        merged.insert("shd".to_string(), format!("{};{};{}", pat, fill, clr));
                    }
                }
            }
            "caps" => {
                if !new_props.contains_key("caps") {
                    merged.insert("caps".to_string(), "true".to_string());
                }
            }
            "smallCaps" => {
                if !new_props.contains_key("smallCaps") {
                    merged.insert("smallCaps".to_string(), "true".to_string());
                }
            }
            "vanish" => {
                if !new_props.contains_key("vanish") && !new_props.contains_key("hidden") {
                    merged.insert("hidden".to_string(), "true".to_string());
                }
            }
            "kern" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("kern") {
                        merged.insert("kern".to_string(), val.clone());
                    }
                }
            }
            "spacing" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("characterSpacing") {
                        merged.insert("characterSpacing".to_string(), val.clone());
                    }
                }
            }
            "lang" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("lang") {
                        merged.insert("lang".to_string(), val.clone());
                    }
                }
            }
            "rtl" => {
                if !new_props.contains_key("rightToLeft") {
                    merged.insert("rightToLeft".to_string(), "true".to_string());
                }
            }
            "em" => {
                if let Some(val) = child.attributes.get("val") {
                    if !new_props.contains_key("emphasisMark") {
                        merged.insert("emphasisMark".to_string(), val.clone());
                    }
                }
            }
            _ => {}
        }
    }

    for (k, v) in new_props {
        merged.insert(k.clone(), v.clone());
    }

    merged
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
        let unsupported: Vec<String> = properties
            .keys()
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

/// Set table properties. Expanded vocabulary:
/// style/tblStyle, width, alignment/jc, indent, border, shading/shd,
/// firstRow, lastRow, firstCol, lastCol, rowBandSize, colBandSize, layout
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
            "alignment" | "jc" => {
                let jc = WordNode::new(WordElementType::Unknown("jc".to_string()))
                    .with_attribute("val", value.as_str());
                children.push(jc);
            }
            "indent" | "tblInd" => {
                let tbl_ind = WordNode::new(WordElementType::Unknown("tblInd".to_string()))
                    .with_attribute("w", value.as_str())
                    .with_attribute("type", "dxa");
                children.push(tbl_ind);
            }
            "layout" | "tblLayout" => {
                let layout = WordNode::new(WordElementType::Unknown("tblLayout".to_string()))
                    .with_attribute("type", value.as_str());
                children.push(layout);
            }
            "firstRow" => {
                let look = build_tbl_look_entry("firstRow", value);
                children.push(look);
            }
            "lastRow" => {
                let look = build_tbl_look_entry("lastRow", value);
                children.push(look);
            }
            "firstCol" => {
                let look = build_tbl_look_entry("firstCol", value);
                children.push(look);
            }
            "lastCol" => {
                let look = build_tbl_look_entry("lastCol", value);
                children.push(look);
            }
            "rowBandSize" | "bandSize" => {
                let band =
                    WordNode::new(WordElementType::Unknown("tblStyleRowBandSize".to_string()))
                        .with_attribute("val", value.as_str());
                children.push(band);
            }
            "colBandSize" => {
                let band =
                    WordNode::new(WordElementType::Unknown("tblStyleColBandSize".to_string()))
                        .with_attribute("val", value.as_str());
                children.push(band);
            }
            "shading" | "shd" => {
                let shd = build_shd_node(value);
                children.push(shd);
            }
            "border" | "borders" | "tblBorders" => {
                let borders = build_table_borders(value);
                children.push(borders);
            }
            _ => {}
        }
    }

    if !children.is_empty() {
        tbl_pr.children = children;
        table.children.insert(0, tbl_pr);
    }

    let recognized = [
        "style",
        "tblStyle",
        "width",
        "alignment",
        "jc",
        "indent",
        "tblInd",
        "layout",
        "tblLayout",
        "firstRow",
        "lastRow",
        "firstCol",
        "lastCol",
        "rowBandSize",
        "colBandSize",
        "bandSize",
        "shading",
        "shd",
        "border",
        "borders",
        "tblBorders",
    ];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

fn build_tbl_look_entry(attr: &str, value: &str) -> WordNode {
    let val = if value == "true" || value == "1" {
        "1"
    } else {
        "0"
    };
    WordNode::new(WordElementType::Unknown("tblLook".to_string())).with_attribute(attr, val)
}

pub fn build_shd_node(value: &str) -> WordNode {
    let value = value.strip_prefix('#').unwrap_or(value);
    let parts: Vec<&str> = value.split(';').collect();
    let (pat, fill, clr) = match parts.len() {
        3 => (parts[0], parts[1], parts[2]),
        2 => ("clear", parts[0], parts[1]),
        _ => ("clear", value, "auto"),
    };
    WordNode::new(WordElementType::Unknown("shd".to_string()))
        .with_attribute("val", pat)
        .with_attribute("color", clr)
        .with_attribute("fill", fill)
}

pub fn build_table_borders(value: &str) -> WordNode {
    let mut tbl_bdr = WordNode::new(WordElementType::Unknown("tblBorders".to_string()));
    let mut children = Vec::new();
    // Format: "top=single;bottom=single;left=none;right=none;insideH=single;insideV=single"
    // Or shorthand: "all=single" or "none"
    if value == "none" || value == "0" {
        for border_name in ["top", "bottom", "left", "right", "insideH", "insideV"] {
            children.push(
                WordNode::new(WordElementType::Unknown(border_name.to_string()))
                    .with_attribute("val", "none")
                    .with_attribute("sz", "0")
                    .with_attribute("space", "0")
                    .with_attribute("color", "auto"),
            );
        }
    } else if value.starts_with("all=") || value == "all" || value == "single" || value == "thin" {
        let style = value.strip_prefix("all=").unwrap_or("single");
        for border_name in ["top", "bottom", "left", "right", "insideH", "insideV"] {
            children.push(
                WordNode::new(WordElementType::Unknown(border_name.to_string()))
                    .with_attribute("val", style)
                    .with_attribute("sz", "4")
                    .with_attribute("space", "0")
                    .with_attribute("color", "auto"),
            );
        }
    } else {
        // Parse per-border format
        for pair in value.split(';') {
            if let Some(eq) = pair.find('=') {
                let name = &pair[..eq];
                let style = &pair[eq + 1..];
                let sz = match style {
                    "double" => "4",
                    "thick" => "12",
                    "dashed" => "4",
                    _ => "4",
                };
                children.push(
                    WordNode::new(WordElementType::Unknown(name.to_string()))
                        .with_attribute("val", style)
                        .with_attribute("sz", sz)
                        .with_attribute("space", "0")
                        .with_attribute("color", "auto"),
                );
            }
        }
    }
    tbl_bdr.children = children;
    tbl_bdr
}

/// Set row properties. Expanded: height, cantSplit, tableHeader, hidden
fn set_row_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let row = navigate_to_element_mut(dom, path)?;
    row.children
        .retain(|c| c.element_type != WordElementType::TableRowProperties);

    let mut tr_pr = WordNode::new(WordElementType::TableRowProperties);
    let mut children = Vec::new();

    if let Some(height) = properties.get("height") {
        let h_rule = properties
            .get("hRule")
            .cloned()
            .unwrap_or_else(|| "atLeast".to_string());
        let tr_height = WordNode::new(WordElementType::Unknown("trHeight".to_string()))
            .with_attribute("val", height.as_str())
            .with_attribute("hRule", h_rule.as_str());
        children.push(tr_height);
    }

    if let Some(val) = properties.get("cantSplit") {
        if val == "true" || val == "1" {
            children.push(WordNode::new(WordElementType::Unknown(
                "cantSplit".to_string(),
            )));
        }
    }

    if let Some(val) = properties.get("tableHeader") {
        let tf_val = if val == "true" || val == "1" {
            "true"
        } else {
            "false"
        };
        children.push(
            WordNode::new(WordElementType::Unknown("tblHeader".to_string()))
                .with_attribute("val", tf_val),
        );
    }

    if let Some(val) = properties.get("hidden") {
        let h_val = if val == "true" || val == "1" {
            "true"
        } else {
            "false"
        };
        children.push(
            WordNode::new(WordElementType::Unknown("hidden".to_string()))
                .with_attribute("val", h_val),
        );
    }

    if !children.is_empty() {
        tr_pr.children = children;
        row.children.insert(0, tr_pr);
    }

    let recognized = ["height", "hRule", "cantSplit", "tableHeader", "hidden"];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Set cell properties. Expanded vocabulary:
/// text, width, shading/shd, border, vAlign, vMerge, gridSpan, noWrap, textDirection
fn set_cell_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let cell = navigate_to_element_mut(dom, path)?;

    // If "text" property is set, replace paragraph text in the cell
    if let Some(new_text) = properties.get("text") {
        for child in &mut cell.children {
            if child.element_type == WordElementType::Paragraph {
                set_paragraph_text(child, new_text);
                break;
            }
        }
    }

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

        if let Some(val) = cell_props.get("shading") {
            let shd = build_shd_node(val);
            children.push(shd);
        }

        if let Some(val) = cell_props.get("shd") {
            let shd = build_shd_node(val);
            children.push(shd);
        }

        if let Some(val) = cell_props.get("border") {
            let borders = build_cell_borders(val);
            children.push(borders);
        }

        if let Some(val) = cell_props.get("vAlign") {
            let v_align = WordNode::new(WordElementType::Unknown("vAlign".to_string()))
                .with_attribute("val", val.as_str());
            children.push(v_align);
        }

        if let Some(val) = cell_props.get("vMerge") {
            let merge_val = if val == "true" || val == "1" || val == "continue" {
                "1"
            } else {
                "0"
            };
            let v_merge = WordNode::new(WordElementType::Unknown("vMerge".to_string()));
            if merge_val == "0" {
                // Restart vertical merge
                children.push(v_merge.with_attribute("val", "restart"));
            } else {
                children.push(v_merge);
            }
        }

        if let Some(val) = cell_props.get("gridSpan") {
            let gs = WordNode::new(WordElementType::Unknown("gridSpan".to_string()))
                .with_attribute("val", val.as_str());
            children.push(gs);
        }

        if let Some(val) = cell_props.get("noWrap") {
            if val == "true" || val == "1" {
                children.push(WordNode::new(WordElementType::Unknown(
                    "noWrap".to_string(),
                )));
            }
        }

        if let Some(val) = cell_props.get("textDirection") {
            let td = WordNode::new(WordElementType::Unknown("textDirection".to_string()))
                .with_attribute("val", val.as_str());
            children.push(td);
        }

        if !children.is_empty() {
            tc_pr.children = children;
            cell.children.insert(0, tc_pr);
        }
    }

    let recognized = [
        "text",
        "width",
        "shading",
        "shd",
        "border",
        "vAlign",
        "vMerge",
        "gridSpan",
        "noWrap",
        "textDirection",
    ];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

fn build_cell_borders(value: &str) -> WordNode {
    let mut tc_bdr = WordNode::new(WordElementType::Unknown("tcBorders".to_string()));
    let mut children = Vec::new();
    if value == "none" || value == "0" {
        for border_name in ["top", "bottom", "left", "right"] {
            children.push(
                WordNode::new(WordElementType::Unknown(border_name.to_string()))
                    .with_attribute("val", "none")
                    .with_attribute("sz", "0")
                    .with_attribute("space", "0")
                    .with_attribute("color", "auto"),
            );
        }
    } else if value.starts_with("all=") || value == "single" {
        let style = value.strip_prefix("all=").unwrap_or("single");
        for border_name in ["top", "bottom", "left", "right"] {
            children.push(
                WordNode::new(WordElementType::Unknown(border_name.to_string()))
                    .with_attribute("val", style)
                    .with_attribute("sz", "4")
                    .with_attribute("space", "0")
                    .with_attribute("color", "auto"),
            );
        }
    } else {
        for pair in value.split(';') {
            if let Some(eq) = pair.find('=') {
                let name = &pair[..eq];
                let style = &pair[eq + 1..];
                children.push(
                    WordNode::new(WordElementType::Unknown(name.to_string()))
                        .with_attribute("val", style)
                        .with_attribute("sz", "4")
                        .with_attribute("space", "0")
                        .with_attribute("color", "auto"),
                );
            }
        }
    }
    tc_bdr.children = children;
    tc_bdr
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

/// Swap two sibling elements in the Word DOM.
/// Both paths must share the same parent element.
pub fn swap_elements(
    dom: &mut WordDom,
    path1: &str,
    path2: &str,
) -> Result<(String, String), HandlerError> {
    let segs1 = parse_path(path1)?;
    let segs2 = parse_path(path2)?;
    if segs1.is_empty() || segs2.is_empty() {
        return Err(HandlerError::InvalidPath("empty path".to_string()));
    }

    // Both paths must share the same parent (all segments except the last)
    if segs1.len() != segs2.len() {
        return Err(HandlerError::InvalidArgument(
            "swap requires both elements at the same nesting depth".to_string(),
        ));
    }
    let parent_segs1 = &segs1[..segs1.len() - 1];
    let parent_segs2 = &segs2[..segs2.len() - 1];
    if !segments_eq(parent_segs1, parent_segs2) {
        return Err(HandlerError::InvalidArgument(
            "swap requires both elements to share the same parent".to_string(),
        ));
    }

    // Extract the indices of the two elements within their parent
    let idx1 = segs1
        .last()
        .and_then(|s| s.index)
        .ok_or_else(|| HandlerError::InvalidPath(format!("path has no index: {}", path1)))?;
    let idx2 = segs2
        .last()
        .and_then(|s| s.index)
        .ok_or_else(|| HandlerError::InvalidPath(format!("path has no index: {}", path2)))?;

    if idx1 == idx2 {
        return Err(HandlerError::InvalidArgument(format!(
            "swap requires two different elements, both were at index {}",
            idx1
        )));
    }

    // Navigate to the parent node
    let parent_path = if parent_segs1.is_empty() {
        "/body".to_string()
    } else {
        let mut p = String::new();
        for seg in parent_segs1 {
            p.push('/');
            p.push_str(&seg.name);
            if let Some(i) = seg.index {
                p.push_str(&format!("[{}]", i));
            }
        }
        p
    };

    let parent = navigate_to_element_mut(dom, &parent_path)?;

    // Convert 1-based to 0-based
    let i1 = idx1 - 1;
    let i2 = idx2 - 1;
    if i1 >= parent.children.len() || i2 >= parent.children.len() {
        return Err(HandlerError::PathNotFound(
            "swap index out of bounds".to_string(),
        ));
    }

    parent.children.swap(i1, i2);

    Ok((path1.to_string(), path2.to_string()))
}

/// Compare two PathSegment slices by name and index (since PathSegment doesn't derive PartialEq).
fn segments_eq(a: &[handler_common::PathSegment], b: &[handler_common::PathSegment]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (sa, sb) in a.iter().zip(b.iter()) {
        if sa.name != sb.name || sa.index != sb.index {
            return false;
        }
    }
    true
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
        let body = dom
            .root
            .children
            .iter()
            .find(|c| c.element_type == WordElementType::Body);
        if let Some(body) = body {
            if find_other_bookmark_by_name(body, new_name) {
                return Err(HandlerError::InvalidArgument(format!(
                    "bookmark name '{}' already exists; pick a unique name.",
                    new_name
                )));
            }
        }
    }

    // Validate id if present
    if let Some(new_id) = properties.get("id") {
        let id_val: i32 = new_id.parse().map_err(|_| {
            HandlerError::InvalidArgument(format!(
                "bookmark id must be a non-negative integer, got: {}",
                new_id
            ))
        })?;
        if id_val < 0 {
            return Err(HandlerError::InvalidArgument(
                "bookmark id must be non-negative".to_string(),
            ));
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

        let start_idx = parent
            .children
            .iter()
            .position(|c| {
                c.element_type == WordElementType::BookmarkStart
                    && c.attributes.get("id").map(|s| s.as_str()) == Some(&current_id)
            })
            .ok_or_else(|| {
                HandlerError::PathNotFound("bookmarkStart not found in parent".to_string())
            })?;

        let end_idx = parent
            .children
            .iter()
            .position(|c| {
                c.element_type == WordElementType::BookmarkEnd
                    && c.attributes.get("id").map(|s| s.as_str()) == Some(&current_id)
            })
            .ok_or_else(|| {
                HandlerError::PathNotFound("bookmarkEnd not found in parent".to_string())
            })?;

        // Collect indices of content to remove (between start and end)
        let remove_indices: Vec<usize> = (start_idx + 1..end_idx)
            .filter(|i| {
                let child = &parent.children[*i];
                matches!(
                    child.element_type,
                    WordElementType::Run | WordElementType::Text | WordElementType::Hyperlink
                )
            })
            .collect();

        // Remove in reverse to keep indices stable
        for idx in remove_indices.iter().rev() {
            parent.children.remove(*idx);
        }

        // Insert new run after BookmarkStart
        let run = crate::add::make_run_with_text(new_text, &HashMap::new());
        let new_start_idx = parent
            .children
            .iter()
            .position(|c| {
                c.element_type == WordElementType::BookmarkStart
                    && c.attributes.get("id").map(|s| s.as_str()) == Some(&current_id)
            })
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
    let unsupported: Vec<String> = properties
        .keys()
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
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Check if any BookmarkStart in the document has the given name.
fn find_other_bookmark_by_name(node: &WordNode, name: &str) -> bool {
    if node.element_type == WordElementType::BookmarkStart
        && node.attributes.get("name").map(|s| s.as_str()) == Some(name)
    {
        return true;
    }
    node.children
        .iter()
        .any(|c| find_other_bookmark_by_name(c, name))
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
        && node.attributes.get("id").map(|s| s.as_str()) == Some(old_id)
    {
        node.attributes.insert("id".to_string(), new_id.to_string());
    }
    for child in &mut node.children {
        update_bookmark_end_in_node(child, old_id, new_id);
    }
}

// ─── Document-level Set Properties ──────────────────────────────────

/// Set document-level properties (path "/" or "/body").
/// Vocabulary: protection, protectionEnforced, defaultTabStop
fn set_document_properties(
    dom: &mut WordDom,
    _path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // For document-level, the properties apply to the root/body element
    // We handle protection by modifying the sectPr (last section properties)
    // and defaultTabStop on the document settings

    for (key, value) in properties {
        match key.as_str() {
            "protection" | "protectionMode" => {
                // Document protection is stored in w:documentProtection inside sectPr
                let body = dom.body_mut();
                if let Some(body) = body {
                    // Find or create sectPr
                    let sect_pr_idx = body
                        .children
                        .iter()
                        .rposition(|c| c.element_type == WordElementType::SectionProperties);
                    if let Some(idx) = sect_pr_idx {
                        let sect_pr = &mut body.children[idx];
                        // Remove existing documentProtection
                        sect_pr.children.retain(|c| {
                            let name = match &c.element_type {
                                WordElementType::Unknown(n) => n.as_str(),
                                _ => "",
                            };
                            name != "documentProtection"
                        });
                        let prot = WordNode::new(WordElementType::Unknown(
                            "documentProtection".to_string(),
                        ))
                        .with_attribute("edit", value.as_str())
                        .with_attribute("enforcement", "1");
                        sect_pr.children.push(prot);
                    }
                }
            }
            "protectionEnforced" => {
                let body = dom.body_mut();
                if let Some(body) = body {
                    let sect_pr_idx = body
                        .children
                        .iter()
                        .rposition(|c| c.element_type == WordElementType::SectionProperties);
                    if let Some(idx) = sect_pr_idx {
                        let sect_pr = &mut body.children[idx];
                        // Find existing documentProtection and update enforcement
                        for child in &mut sect_pr.children {
                            if let WordElementType::Unknown(name) = &child.element_type {
                                if name == "documentProtection" {
                                    let enforcement_val = if value == "true" || value == "1" {
                                        "1"
                                    } else {
                                        "0"
                                    };
                                    child.attributes.insert(
                                        "enforcement".to_string(),
                                        enforcement_val.to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let recognized = ["protection", "protectionMode", "protectionEnforced"];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

// ─── Section Properties Set ──────────────────────────────────

/// Set section properties. Vocabulary:
/// pageWidth, pageHeight, orientation, marginLeft, marginRight, marginTop, marginBottom,
/// columns, headerDistance, footerDistance, gutter
fn set_section_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let sect_pr = navigate_to_element_mut(dom, path)?;

    // Handle orientation → page size changes
    if let Some(orient) = properties.get("orientation") {
        // Update pgSz with the correct dimensions for the orientation
        let existing_sz = sect_pr.children.iter_mut().find(|c| {
            let name = match &c.element_type {
                WordElementType::Unknown(n) => n.as_str(),
                _ => "",
            };
            name == "pgSz"
        });

        if let Some(sz) = existing_sz {
            sz.attributes
                .insert("orient".to_string(), orient.to_string());
        } else {
            let pg_sz = WordNode::new(WordElementType::Unknown("pgSz".to_string()))
                .with_attribute("orient", orient.as_str());
            sect_pr.children.insert(0, pg_sz);
        }
    }

    // Handle page dimensions
    for (key, value) in properties {
        match key.as_str() {
            "pageWidth" => {
                let pg_sz = find_or_create_child(sect_pr, "pgSz");
                pg_sz.attributes.insert("w".to_string(), value.clone());
            }
            "pageHeight" => {
                let pg_sz = find_or_create_child(sect_pr, "pgSz");
                pg_sz.attributes.insert("h".to_string(), value.clone());
            }
            "marginLeft" => {
                let pg_mar = find_or_create_child(sect_pr, "pgMar");
                pg_mar.attributes.insert("left".to_string(), value.clone());
            }
            "marginRight" => {
                let pg_mar = find_or_create_child(sect_pr, "pgMar");
                pg_mar.attributes.insert("right".to_string(), value.clone());
            }
            "marginTop" => {
                let pg_mar = find_or_create_child(sect_pr, "pgMar");
                pg_mar.attributes.insert("top".to_string(), value.clone());
            }
            "marginBottom" => {
                let pg_mar = find_or_create_child(sect_pr, "pgMar");
                pg_mar
                    .attributes
                    .insert("bottom".to_string(), value.clone());
            }
            "headerDistance" => {
                let pg_mar = find_or_create_child(sect_pr, "pgMar");
                pg_mar
                    .attributes
                    .insert("header".to_string(), value.clone());
            }
            "footerDistance" => {
                let pg_mar = find_or_create_child(sect_pr, "pgMar");
                pg_mar
                    .attributes
                    .insert("footer".to_string(), value.clone());
            }
            "gutter" => {
                let pg_mar = find_or_create_child(sect_pr, "pgMar");
                pg_mar
                    .attributes
                    .insert("gutter".to_string(), value.clone());
            }
            "columns" => {
                // Number of columns (integer)
                let cols = find_or_create_child(sect_pr, "cols");
                cols.attributes.insert("num".to_string(), value.clone());
                cols.attributes
                    .insert("space".to_string(), "720".to_string()); // Default column spacing
            }
            _ => {}
        }
    }

    let recognized = [
        "orientation",
        "pageWidth",
        "pageHeight",
        "marginLeft",
        "marginRight",
        "marginTop",
        "marginBottom",
        "headerDistance",
        "footerDistance",
        "gutter",
        "columns",
    ];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

/// Find a child element by name, or create it if it doesn't exist.
fn find_or_create_child<'a>(parent: &'a mut WordNode, name: &str) -> &'a mut WordNode {
    let idx = parent.children.iter().position(|c| {
        let c_name = match &c.element_type {
            WordElementType::Unknown(n) => n.as_str(),
            _ => "",
        };
        c_name == name
    });

    if let Some(idx) = idx {
        &mut parent.children[idx]
    } else {
        let node = WordNode::new(WordElementType::Unknown(name.to_string()));
        parent.children.push(node);
        let last = parent.children.len() - 1;
        &mut parent.children[last]
    }
}

// ─── SDT Set Properties ──────────────────────────────────

/// Set SDT (structured document tag / content control) properties.
/// Vocabulary: alias/name, tag, lock, text
fn set_sdt_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let sdt = navigate_to_element_mut(dom, path)?;

    // Handle SDT properties (sdtPr child)
    let sdt_pr_idx = sdt
        .children
        .iter()
        .position(|c| c.element_type == WordElementType::SdtPr);
    if let Some(idx) = sdt_pr_idx {
        let sdt_pr = &mut sdt.children[idx];

        for (key, value) in properties {
            match key.as_str() {
                "alias" | "name" => {
                    // Update or add w:alias in sdtPr
                    let alias_idx = sdt_pr.children.iter().position(|c| {
                        let name = match &c.element_type {
                            WordElementType::Unknown(n) => n.as_str(),
                            _ => "",
                        };
                        name == "alias"
                    });
                    if let Some(alias_idx) = alias_idx {
                        sdt_pr.children[alias_idx]
                            .attributes
                            .insert("val".to_string(), value.clone());
                    } else {
                        let alias = WordNode::new(WordElementType::Unknown("alias".to_string()))
                            .with_attribute("val", value.as_str());
                        sdt_pr.children.push(alias);
                    }
                }
                "tag" => {
                    let tag_idx = sdt_pr.children.iter().position(|c| {
                        let name = match &c.element_type {
                            WordElementType::Unknown(n) => n.as_str(),
                            _ => "",
                        };
                        name == "tag"
                    });
                    if let Some(tag_idx) = tag_idx {
                        sdt_pr.children[tag_idx]
                            .attributes
                            .insert("val".to_string(), value.clone());
                    } else {
                        let tag = WordNode::new(WordElementType::Unknown("tag".to_string()))
                            .with_attribute("val", value.as_str());
                        sdt_pr.children.push(tag);
                    }
                }
                "lock" => {
                    let lock_val = match value.as_str() {
                        "content" | "contentLocked" => "contentLocked",
                        "sdt" | "sdtLocked" => "sdtLocked",
                        "both" | "sdtContentLocked" => "sdtContentLocked",
                        "unlocked" | "none" => "unlocked",
                        other => other,
                    };
                    let lock_idx = sdt_pr.children.iter().position(|c| {
                        let name = match &c.element_type {
                            WordElementType::Unknown(n) => n.as_str(),
                            _ => "",
                        };
                        name == "lock"
                    });
                    if let Some(lock_idx) = lock_idx {
                        sdt_pr.children[lock_idx]
                            .attributes
                            .insert("val".to_string(), lock_val.to_string());
                    } else {
                        let lock = WordNode::new(WordElementType::Unknown("lock".to_string()))
                            .with_attribute("val", lock_val);
                        sdt_pr.children.push(lock);
                    }
                }
                _ => {}
            }
        }
    }

    // Handle text property: replace SDT content text
    if let Some(new_text) = properties.get("text") {
        let content_idx = sdt
            .children
            .iter()
            .position(|c| c.element_type == WordElementType::SdtContent);
        if let Some(idx) = content_idx {
            let content = &mut sdt.children[idx];
            // Find first paragraph and replace text
            for child in &mut content.children {
                if child.element_type == WordElementType::Paragraph {
                    set_paragraph_text(child, new_text);
                    break;
                }
            }
        }
    }

    let recognized = ["alias", "name", "tag", "lock", "text"];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

// ─── Hyperlink Set Properties ──────────────────────────────────

/// Set hyperlink properties. Vocabulary: url/target, tooltip
fn set_hyperlink_properties(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let link = navigate_to_element_mut(dom, path)?;

    for (key, value) in properties {
        match key.as_str() {
            "url" | "target" | "link" => {
                // Reject URI schemes that would survive OOXML round-trip but
                // execute script or exfiltrate data on click in the host
                // product (javascript:, data:, vbscript:). See
                // handler_common::hyperlink_validator for the allowlist.
                if let Err(msg) =
                    handler_common::hyperlink_validator::require_safe_scheme(value, "hyperlink")
                {
                    return Err(HandlerError::InvalidArgument(msg));
                }
                // Hyperlink target is stored as r:id attribute pointing to a relationship
                // For direct URLs, we can only update the attribute
                link.attributes.insert("r:id".to_string(), value.clone());
            }
            "tooltip" => {
                // Tooltips use w:tooltip attribute
                link.attributes.insert("tooltip".to_string(), value.clone());
            }
            "text" => {
                // Replace hyperlink text (runs inside the hyperlink)
                let runs: Vec<usize> = link
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.element_type == WordElementType::Run)
                    .map(|(i, _)| i)
                    .collect();
                // Remove existing runs
                for idx in runs.iter().rev() {
                    link.children.remove(*idx);
                }
                // Add new run with text
                let run = crate::add::make_run_with_text(value, &HashMap::new());
                link.children.push(run);
            }
            _ => {}
        }
    }

    let recognized = ["url", "target", "link", "tooltip", "text"];
    let unsupported: Vec<String> = properties
        .keys()
        .filter(|k| !recognized.contains(&k.as_str()))
        .cloned()
        .collect();

    Ok(unsupported)
}

// ─── Find & Replace ──────────────────────────────────────────────────

/// Apply find/replace to a single path or whole document body.
///
/// If `path` points to a Paragraph, only that paragraph's text is scanned.
/// If `path` is "/" or "/body", every paragraph in the body is scanned.
/// Returns a Vec containing either zero entries (silent success) or a single
/// summary entry "replaced=<n>" so callers can surface counts in view output.
fn apply_find_replace(
    dom: &mut WordDom,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // Coerce HashMap<String, String> to the shape extract_find_replace_props expects.
    let mut prop_map: HashMap<String, String> = HashMap::new();
    for (k, v) in properties {
        prop_map.insert(k.clone(), v.clone());
    }
    let (find, replace, opts) = extract_find_replace_props(&prop_map).ok_or_else(|| {
        HandlerError::InvalidArgument(
            "find/replace requires at least a 'find=<text>' property".to_string(),
        )
    })?;

    let path_lc = path.trim().to_lowercase();
    let total = if path_lc == "/" || path_lc == "/body" || path_lc.is_empty() {
        find_replace_in_body(dom, &find, &replace, &opts)?
    } else {
        // Resolve path → paragraph(s). Try navigation first to validate the path.
        let _segments = parse_path(path)?;
        let body = dom
            .body_mut()
            .ok_or_else(|| HandlerError::InvalidPath("document has no body".to_string()))?;
        // We accept paths of the form /body/p[i] — pull out the index from the raw path.
        find_replace_in_paragraph_path(body, path, &find, &replace, &opts)?
    };

    Ok(vec![format!("replaced={}", total)])
}

/// Run find/replace across all paragraphs in the body. Returns total replacements.
fn find_replace_in_body(
    dom: &mut WordDom,
    find: &str,
    replace: &str,
    opts: &FindReplaceOptions,
) -> Result<usize, HandlerError> {
    let body = dom
        .body_mut()
        .ok_or_else(|| HandlerError::InvalidPath("document has no body".to_string()))?;
    // Collect paragraph child indices first so we can borrow mutably without aliasing.
    let para_indices: Vec<usize> = body
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type == WordElementType::Paragraph)
        .map(|(i, _)| i)
        .collect();

    let mut total = 0usize;
    for idx in para_indices {
        total += find_replace_in_paragraph(&mut body.children[idx], find, replace, opts);
    }
    Ok(total)
}

/// Resolve /body/p[i] style paths to a paragraph and run find/replace on it.
fn find_replace_in_paragraph_path(
    body: &mut WordNode,
    path: &str,
    find: &str,
    replace: &str,
    opts: &FindReplaceOptions,
) -> Result<usize, HandlerError> {
    // Extract trailing [i] index from a path like /body/p[3]
    let lower = path.to_lowercase();
    let idx = match parse_paragraph_index(&lower) {
        Some(i) => i,
        None => {
            return Err(HandlerError::InvalidArgument(format!(
                "find/replace only supports paths of the form '/body/p[i]' or '/'. Got: '{}'",
                path
            )))
        }
    };

    let mut count = 0;
    let mut found_idx = 0;
    for child in &mut body.children {
        if child.element_type == WordElementType::Paragraph {
            found_idx += 1;
            if found_idx == idx {
                count = find_replace_in_paragraph(child, find, replace, opts);
                break;
            }
        }
    }
    if found_idx < idx {
        return Err(HandlerError::PathNotFound(format!(
            "paragraph index {} out of range (found {} paragraphs)",
            idx, found_idx
        )));
    }
    Ok(count)
}

/// Parse the trailing [n] index from a path ending in p[n].
fn parse_paragraph_index(path_lc: &str) -> Option<usize> {
    let open = path_lc.rfind('[')?;
    let close = path_lc.rfind(']')?;
    if close <= open {
        return None;
    }
    let inner = &path_lc[open + 1..close];
    inner.parse::<usize>().ok()
}

/// Apply find/replace to a single paragraph node. Returns count of replacements.
///
/// Walks every run's text content and replaces matches. The find/replace
/// operates per-run (cross-run matches are not spanned) which matches the
/// common case where users search literal strings.
fn find_replace_in_paragraph(
    para: &mut WordNode,
    find: &str,
    replace: &str,
    opts: &FindReplaceOptions,
) -> usize {
    let run_indices: Vec<usize> = para
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type == WordElementType::Run)
        .map(|(i, _)| i)
        .collect();

    let mut total = 0usize;
    for idx in run_indices {
        let run = &mut para.children[idx];
        // Walk run children to find Text nodes and replace in place.
        let text_indices: Vec<usize> = run
            .children
            .iter()
            .enumerate()
            .filter(|(_, c)| c.element_type == WordElementType::Text)
            .map(|(i, _)| i)
            .collect();

        for t_idx in text_indices {
            let t_node = &mut run.children[t_idx];
            let cur = t_node.text_content.clone().unwrap_or_default();
            let (new_text, n) = replace_in_string(&cur, find, replace, opts);
            if n > 0 {
                t_node.text_content = Some(new_text);
                total += n;
            }
        }
    }
    total
}

// ─── Part-aware Set: Styles, Comments, Footnotes, Endnotes ────────────

/// Set properties on a Word style. The path is `/styles/<styleId>` (or
/// `/styles/<styleId>/...` for future sub-targeting). Reads `word/styles.xml`,
/// finds the `<w:style w:styleId="...">` block, and modifies properties within it.
///
/// Supported properties:
///   - name            → set <w:name w:val="..."/>
///   - basedOn         → set <w:basedOn w:val="..."/>
///   - next            → set <w:next w:val="..."/>
///   - uiPriority      → set <w:uiPriority w:val="N"/>
///   - hidden          → toggle <w:hidden/>
///   - semiHidden      → toggle <w:semiHidden/>
///   - qFormat          → toggle <w:qFormat/>
///   - Run-level props (font/size/bold/...) → set within <w:rPr>
///   - Para-level props (alignment/spacing/...) → set within <w:pPr>
pub fn set_style_on_part(
    package: &mut OxmlPackage,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // Extract styleId from path: /styles/Heading1 → "Heading1"
    let style_id = path
        .trim_start_matches('/')
        .strip_prefix("styles/")
        .map(|s| s.trim_end_matches('/').to_string())
        .ok_or_else(|| {
            HandlerError::InvalidArgument(format!(
                "style set expects path '/styles/<styleId>', got '{}'",
                path
            ))
        })?;

    let xml = package
        .read_part_xml("word/styles.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    // Find the <w:style w:styleId="ID"> ... </w:style> block.
    let needle = format!("w:styleId=\"{}\"", style_id);
    let sid_offset = xml.find(&needle).ok_or_else(|| {
        HandlerError::PathNotFound(format!("style '{}' not found in word/styles.xml", style_id))
    })?;

    // Walk back to the <w:style opening tag.
    let style_open = xml[..sid_offset].rfind("<w:style").ok_or_else(|| {
        HandlerError::OperationFailed(format!(
            "could not locate <w:style> opening tag for '{}'",
            style_id
        ))
    })?;

    // Find matching </w:style> for this opening tag.
    let style_close = find_matching_close(&xml, style_open, "<w:style", "</w:style>")
        .ok_or_else(|| HandlerError::OperationFailed("malformed style block".to_string()))?;

    // Mutate the style block in place.
    let block = &xml[style_open..style_close];
    let mut new_block = block.to_string();
    let mut unsupported = Vec::new();

    // Style metadata vs. visual properties. Visual properties (font, size,
    // bold, color, alignment, spacing, etc.) belong inside <w:pPr> or <w:rPr>
    // children of the style, while metadata (name, basedOn, uiPriority, ...)
    // are direct children. Partition the property map by which family each
    // key belongs to.
    let mut meta_props: HashMap<String, String> = HashMap::new();
    let mut ppr_props: HashMap<String, String> = HashMap::new();
    let mut rpr_props: HashMap<String, String> = HashMap::new();

    for (k, v) in properties.iter() {
        if STYLE_META_KEYS.contains(&k.as_str()) {
            meta_props.insert(k.clone(), v.clone());
        } else if PARAGRAPH_STYLE_KEYS.contains(&k.as_str()) {
            ppr_props.insert(k.clone(), v.clone());
        } else if RUN_STYLE_KEYS.contains(&k.as_str()) {
            rpr_props.insert(k.clone(), v.clone());
        } else {
            unsupported.push(k.clone());
        }
    }

    for (key, value) in &meta_props {
        match key.as_str() {
            "name" => set_or_replace_attr_child(&mut new_block, "w:name", "w:val", value),
            "basedOn" => set_or_replace_attr_child(&mut new_block, "w:basedOn", "w:val", value),
            "next" => set_or_replace_attr_child(&mut new_block, "w:next", "w:val", value),
            "uiPriority" => {
                set_or_replace_attr_child(&mut new_block, "w:uiPriority", "w:val", value)
            }
            "hidden" => toggle_flag_child(&mut new_block, "w:hidden", value),
            "semiHidden" => toggle_flag_child(&mut new_block, "w:semiHidden", value),
            "qFormat" => toggle_flag_child(&mut new_block, "w:qFormat", value),
            "unhideWhenUsed" => toggle_flag_child(&mut new_block, "w:unhideWhenUsed", value),
            "link" => set_or_replace_attr_child(&mut new_block, "w:link", "w:val", value),
            _ => {}
        }
    }

    // Apply paragraph properties: ensure <w:pPr> exists, then apply each prop.
    if !ppr_props.is_empty() {
        ensure_style_child(&mut new_block, "w:pPr");
        let ppr_xml = build_ppr_fragment(&ppr_props);
        merge_into_child(&mut new_block, "w:pPr", &ppr_xml);
    }

    // Apply run properties: same pattern for <w:rPr>.
    if !rpr_props.is_empty() {
        ensure_style_child(&mut new_block, "w:rPr");
        let rpr_xml = build_rpr_fragment(&rpr_props);
        merge_into_child(&mut new_block, "w:rPr", &rpr_xml);
    }

    if new_block != block {
        let mut new_xml = String::with_capacity(xml.len() + new_block.len());
        new_xml.push_str(&xml[..style_open]);
        new_xml.push_str(&new_block);
        new_xml.push_str(&xml[style_close..]);
        package
            .write_part_xml("word/styles.xml", &new_xml)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    Ok(unsupported)
}

/// Style metadata keys — applied as direct children of <w:style>.
const STYLE_META_KEYS: &[&str] = &[
    "name",
    "basedOn",
    "next",
    "uiPriority",
    "hidden",
    "semiHidden",
    "qFormat",
    "unhideWhenUsed",
    "link",
];

/// Paragraph-style keys — applied inside <w:pPr>.
const PARAGRAPH_STYLE_KEYS: &[&str] = &[
    "alignment",
    "jc",
    "indentLeft",
    "indentRight",
    "firstLine",
    "hanging",
    "indent",
    "spacingBefore",
    "spacingAfter",
    "lineSpacing",
    "spacing",
    "keepLines",
    "keepNext",
    "outlineLevel",
    "numId",
    "numLevel",
    "listStyle",
    "pageBreakBefore",
    "widowControl",
];

/// Run-style keys — applied inside <w:rPr>.
const RUN_STYLE_KEYS: &[&str] = &[
    "bold",
    "b",
    "italic",
    "i",
    "underline",
    "u",
    "strike",
    "strikeout",
    "font",
    "fontFamily",
    "size",
    "fontSize",
    "color",
    "fontColor",
    "bgColor",
    "highlight",
    "bg",
    "shading",
    "shd",
    "caps",
    "smallCaps",
    "vanish",
    "kern",
    "spacing",
    "characterSpacing",
    "lang",
];

/// Build the <w:pPr> children XML for the given paragraph properties.
/// Mirrors the helper shape used by the DOM-based paragraph builder so the
/// resulting XML matches what Word expects.
fn build_ppr_fragment(props: &HashMap<String, String>) -> String {
    let mut s = String::new();
    if let Some(v) = props.get("alignment").or_else(|| props.get("jc")) {
        s.push_str(&format!("<w:jc w:val=\"{}\"/>", escape_attr(v)));
    }
    if let Some(v) = props.get("indentLeft").or_else(|| props.get("indent")) {
        s.push_str(&format!("<w:ind w:left=\"{}\"/>", to_twips(v)));
    }
    if let Some(v) = props.get("indentRight") {
        s.push_str(&format!("<w:ind w:right=\"{}\"/>", to_twips(v)));
    }
    if let Some(v) = props.get("firstLine") {
        s.push_str(&format!("<w:ind w:firstLine=\"{}\"/>", to_twips(v)));
    }
    if let Some(v) = props.get("hanging") {
        s.push_str(&format!("<w:ind w:hanging=\"{}\"/>", to_twips(v)));
    }
    if let Some(v) = props.get("spacingBefore") {
        s.push_str(&format!("<w:spacing w:before=\"{}\"/>", to_twips(v)));
    }
    if let Some(v) = props.get("spacingAfter") {
        s.push_str(&format!("<w:spacing w:after=\"{}\"/>", to_twips(v)));
    }
    if let Some(v) = props.get("lineSpacing") {
        s.push_str(&format!(
            "<w:spacing w:line=\"{}\" w:lineRule=\"auto\"/>",
            to_line(v)
        ));
    }
    if let Some(v) = props.get("outlineLevel") {
        s.push_str(&format!("<w:outlineLvl w:val=\"{}\"/>", escape_attr(v)));
    }
    if props.contains_key("keepLines") {
        s.push_str("<w:keepLines/>");
    }
    if props.contains_key("keepNext") {
        s.push_str("<w:keepNext/>");
    }
    if props.contains_key("pageBreakBefore") {
        s.push_str("<w:pageBreakBefore/>");
    }
    if props.contains_key("widowControl") {
        s.push_str("<w:widowControl/>");
    }
    s
}

/// Build the <w:rPr> children XML for the given run properties.
fn build_rpr_fragment(props: &HashMap<String, String>) -> String {
    let mut s = String::new();
    if let Some(v) = props.get("font").or_else(|| props.get("fontFamily")) {
        s.push_str(&format!(
            "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\" w:cs=\"{}\"/>",
            escape_attr(v),
            escape_attr(v),
            escape_attr(v)
        ));
    }
    if let Some(v) = props.get("size").or_else(|| props.get("fontSize")) {
        s.push_str(&format!(
            "<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>",
            to_half_points(v),
            to_half_points(v)
        ));
    }
    if let Some(v) = props.get("color").or_else(|| props.get("fontColor")) {
        s.push_str(&format!("<w:color w:val=\"{}\"/>", normalize_hex(v)));
    }
    if let Some(v) = props
        .get("highlight")
        .or_else(|| props.get("bgColor"))
        .or_else(|| props.get("bg"))
    {
        s.push_str(&format!("<w:highlight w:val=\"{}\"/>", escape_attr(v)));
    }
    if let Some(v) = props.get("bold").or_else(|| props.get("b")) {
        if is_on(v) {
            s.push_str("<w:b/><w:bCs/>");
        }
    }
    if let Some(v) = props.get("italic").or_else(|| props.get("i")) {
        if is_on(v) {
            s.push_str("<w:i/><w:iCs/>");
        }
    }
    if let Some(v) = props.get("underline").or_else(|| props.get("u")) {
        s.push_str(&format!(
            "<w:u w:val=\"{}\"/>",
            if is_on(v) { "single" } else { "none" }
        ));
    }
    if let Some(v) = props.get("strike").or_else(|| props.get("strikeout")) {
        if is_on(v) {
            s.push_str("<w:strike/>");
        }
    }
    if props.contains_key("caps") {
        s.push_str("<w:caps/>");
    }
    if props.contains_key("smallCaps") {
        s.push_str("<w:smallCaps/>");
    }
    s
}

/// Ensure the style block contains a `<w:TAG>...</w:TAG>` child. Inserts it
/// immediately after the opening `<w:style ...>` tag if missing.
fn ensure_style_child(block: &mut String, tag: &str) {
    let open = format!("<{}", tag);
    if block.contains(&open) {
        return;
    }
    // Insert right after the opening <w:style ...> tag (after the first '>').
    let Some(gt) = block.find('>') else { return };
    let child = format!("<{}></{}>", tag, tag);
    block.insert_str(gt + 1, &child);
}

/// Merge `fragment_xml` (containing child elements) into the named child block.
/// Removes existing matching leaf children first to avoid duplicates, then
/// appends the new fragment before the closing tag of the child.
fn merge_into_child(block: &mut String, child_tag: &str, fragment_xml: &str) {
    let open = format!("<{}", child_tag);
    let close = format!("</{}>", child_tag);
    let Some(open_idx) = block.find(&open) else {
        return;
    };
    let Some(close_idx) = block.find(&close) else {
        return;
    };

    // Splice out the existing child block content.
    let inner_start = block[open_idx..]
        .find('>')
        .map(|i| open_idx + i + 1)
        .unwrap_or(open_idx + open.len());
    let inner_end = close_idx;
    let inner = &block[inner_start..inner_end];

    // For each leaf element the fragment will introduce (e.g. "<w:b/>" or
    // "<w:color w:val=\"...\"/>"), remove any existing sibling with the same
    // tag from the inner content. This keeps the property update idempotent.
    let mut new_inner = inner.to_string();
    // Extract top-level child tags from the fragment.
    for frag_tag in extract_top_level_tags(fragment_xml) {
        let tag_open = format!("<{}", frag_tag);
        let mut cursor = 0;
        while let Some(p) = new_inner[cursor..].find(&tag_open) {
            let abs = cursor + p;
            // Find end of this element (either '/>' or the matching close).
            let after = &new_inner[abs..];
            let end = if let Some(sc) = after.find("/>") {
                abs + sc + 2
            } else if let Some(oc) = after.find('>') {
                // open-close form: find matching close
                let close_tag = format!("</{}>", frag_tag);
                if let Some(ct) = new_inner[abs + oc..].find(&close_tag) {
                    abs + oc + ct + close_tag.len()
                } else {
                    abs + oc + 1
                }
            } else {
                break;
            };
            new_inner.replace_range(abs..end, "");
            cursor = abs;
        }
    }

    new_inner.push_str(fragment_xml);

    block.replace_range(inner_start..inner_end, &new_inner);
}

/// Extract the unique top-level tag names from an XML fragment.
fn extract_top_level_tags(fragment: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let bytes = fragment.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b':') {
                j += 1;
            }
            let name = fragment[start..j].to_string();
            if !tags.contains(&name) {
                tags.push(name);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    tags
}

fn is_on(v: &str) -> bool {
    matches!(v.to_lowercase().as_str(), "true" | "1" | "on" | "yes" | "")
}

fn to_twips(v: &str) -> String {
    // Accept already-twips numbers or unit suffixed values.
    if let Ok(n) = v.parse::<i64>() {
        return n.to_string();
    }
    if let Some(rest) = v.strip_suffix("in") {
        if let Ok(n) = rest.parse::<f64>() {
            return ((n * 1440.0) as i64).to_string();
        }
    }
    if let Some(rest) = v.strip_suffix("cm") {
        if let Ok(n) = rest.parse::<f64>() {
            return ((n * 567.0) as i64).to_string();
        }
    }
    if let Some(rest) = v.strip_suffix("pt") {
        if let Ok(n) = rest.parse::<f64>() {
            return ((n * 20.0) as i64).to_string();
        }
    }
    "0".to_string()
}

fn to_half_points(v: &str) -> String {
    if let Ok(n) = v.parse::<f64>() {
        return (n * 2.0).round().to_string();
    }
    "24".to_string()
}

fn to_line(v: &str) -> String {
    // lineSpacing in lines (e.g. "1.5") → 240×N
    if let Ok(n) = v.parse::<f64>() {
        return ((n * 240.0).round() as i64).to_string();
    }
    "240".to_string()
}

fn normalize_hex(v: &str) -> String {
    let trimmed = v.trim().trim_start_matches('#');
    trimmed.to_string()
}

/// Set properties on a comment. Path is `/comments/<id>` or `/comments/comment[N]`.
/// Reads the given part (word/comments.xml) and modifies the targeted
/// `<w:comment w:id="N">` block.
///
/// Supported properties: text, author, initials, date
pub fn set_comment_on_part(
    package: &mut OxmlPackage,
    part: &str,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let comment_id = extract_id_from_path(path, "comments")?;
    let xml = package
        .read_part_xml(part)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let needle = format!("w:id=\"{}\"", comment_id);
    let id_offset = xml.find(&needle).ok_or_else(|| {
        HandlerError::PathNotFound(format!("comment id '{}' not found in {}", comment_id, part))
    })?;

    let open = xml[..id_offset]
        .rfind("<w:comment")
        .ok_or_else(|| HandlerError::OperationFailed("malformed comment element".to_string()))?;
    let close = find_matching_close(&xml, open, "<w:comment", "</w:comment>")
        .ok_or_else(|| HandlerError::OperationFailed("unterminated comment".to_string()))?;

    let block = &xml[open..close];
    let mut new_block = block.to_string();
    let mut unsupported = Vec::new();

    for (key, value) in properties {
        match key.as_str() {
            "author" => set_attr_on_open_tag(&mut new_block, "w:author", value),
            "initials" => set_attr_on_open_tag(&mut new_block, "w:initials", value),
            "date" => set_attr_on_open_tag(&mut new_block, "w:date", value),
            "text" => {
                // Replace the inner <w:t>...</w:t> text.
                let opts = FindReplaceOptions::default();
                let (replaced, n) = replace_first_text_node(&new_block, value, &opts);
                new_block = replaced;
                if n == 0 {
                    unsupported.push("text".to_string());
                }
            }
            _ => unsupported.push(key.clone()),
        }
    }

    if new_block != block {
        let mut new_xml = String::with_capacity(xml.len() + new_block.len());
        new_xml.push_str(&xml[..open]);
        new_xml.push_str(&new_block);
        new_xml.push_str(&xml[close..]);
        package
            .write_part_xml(part, &new_xml)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    Ok(unsupported)
}

/// Set properties on a footnote or endnote. The same XML shape applies to both.
/// Path is `/footnotes/<id>` or `/endnotes/<id>`.
///
/// Supported properties: text
pub fn set_footnote_endnote_on_part(
    package: &mut OxmlPackage,
    part: &str,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let (kind, prefix) = if part.ends_with("footnotes.xml") {
        ("footnote", "footnotes")
    } else {
        ("endnote", "endnotes")
    };
    let note_id = extract_id_from_path(path, prefix)?;
    let xml = package
        .read_part_xml(part)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let needle = format!("w:id=\"{}\"", note_id);
    let id_offset = xml.find(&needle).ok_or_else(|| {
        HandlerError::PathNotFound(format!("{} id '{}' not found in {}", kind, note_id, part))
    })?;

    let open_tag = format!("<w:{}", kind);
    let close_tag = format!("</w:{}>", kind);
    let open = xml[..id_offset]
        .rfind(&open_tag)
        .ok_or_else(|| HandlerError::OperationFailed(format!("malformed {} element", kind)))?;
    let close = find_matching_close(&xml, open, &open_tag, &close_tag)
        .ok_or_else(|| HandlerError::OperationFailed(format!("unterminated {}", kind)))?;

    let block = &xml[open..close];
    let mut new_block = block.to_string();
    let mut unsupported = Vec::new();

    for (key, value) in properties {
        match key.as_str() {
            "text" => {
                let opts = FindReplaceOptions::default();
                let (replaced, n) = replace_first_text_node(&new_block, value, &opts);
                new_block = replaced;
                if n == 0 {
                    unsupported.push("text".to_string());
                }
            }
            _ => unsupported.push(key.clone()),
        }
    }

    if new_block != block {
        let mut new_xml = String::with_capacity(xml.len() + new_block.len());
        new_xml.push_str(&xml[..open]);
        new_xml.push_str(&new_block);
        new_xml.push_str(&xml[close..]);
        package
            .write_part_xml(part, &new_xml)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    Ok(unsupported)
}

// ─── XML helper utilities ─────────────────────────────────────────────

/// Find the matching close tag for `open_tag` starting at `open_start`.
/// Simple depth-counting: each nested `<w:style ...` increments, each
/// `</w:style>` decrements; the close that brings the count to zero is the match.
fn find_matching_close(
    xml: &str,
    open_start: usize,
    open_prefix: &str,
    close_tag: &str,
) -> Option<usize> {
    let mut depth = 0;
    let mut cursor = open_start;
    while let Some(pos) = xml[cursor..].find(open_prefix) {
        let abs = cursor + pos;
        // Skip self-closing tags `<w:tag .../>`.
        let next_close = xml[abs..].find('>')?;
        let tag_end = abs + next_close;
        let is_self_closing = xml[..tag_end].ends_with('/');
        // Only count as an open if it's not self-closing.
        if !is_self_closing {
            depth += 1;
        }
        cursor = tag_end + 1;
        // From here, search for the next close_tag.
        if let Some(c_rel) = xml[cursor..].find(close_tag) {
            let c_abs = cursor + c_rel;
            depth -= 1;
            if depth == 0 {
                return Some(c_abs + close_tag.len());
            }
            cursor = c_abs + close_tag.len();
        } else {
            break;
        }
        // Continue searching for more opens.
    }
    None
}

/// Set or replace a child element of the form `<w:name w:val="VALUE"/>`.
/// Removes any existing child first, then inserts a new one right after
/// the opening `<w:style ...>` tag.
fn set_or_replace_attr_child(block: &mut String, child_tag: &str, attr: &str, value: &str) {
    let open = format!("<{}", child_tag);
    let open_close = format!("</{}>", child_tag);
    // Remove existing child of the same tag in either self-closing or
    // open/close form.
    if let Some(start) = block.find(&open) {
        // Try self-closing form first.
        if let Some(end_rel) = block[start..].find("/>") {
            let end_abs = start + end_rel + 2;
            block.replace_range(start..end_abs, "");
        } else if let Some(close_rel) = block[start..].find(&open_close) {
            let end_abs = start + close_rel + open_close.len();
            block.replace_range(start..end_abs, "");
        }
    }
    // Insert the new child immediately after the opening <w:style ...> tag.
    if let Some(gt) = block.find('>') {
        let new_child = format!("<{} {}=\"{}\"/>", child_tag, attr, escape_attr(value));
        block.insert_str(gt + 1, &new_child);
    }
    let _ = attr; // currently fixed to w:val
}

/// Toggle a flag-style child element (`<w:hidden/>` present = true).
fn toggle_flag_child(block: &mut String, child_tag: &str, value: &str) {
    let on = value == "true" || value == "1" || value.is_empty();
    let open = format!("<{}", child_tag);
    let exists = block.contains(&open);
    if on && !exists {
        if let Some(gt) = block.find('>') {
            let new_child = format!("<{}/>", child_tag);
            block.insert_str(gt + 1, &new_child);
        }
    } else if !on && exists {
        if let Some(start) = block.find(&open) {
            if let Some(end_rel) = block[start..].find("/>") {
                let end_abs = start + end_rel + 2;
                block.replace_range(start..end_abs, "");
            }
        }
    }
}

/// Set or replace an attribute on the first opening tag in `block`.
fn set_attr_on_open_tag(block: &mut String, attr: &str, value: &str) {
    let open_end = match block.find('>') {
        Some(p) => p,
        None => return,
    };
    let open_tag = &block[..open_end];
    let attr_pattern = format!("{}=\"", attr);
    if let Some(attr_start) = open_tag.find(&attr_pattern) {
        let val_start = attr_start + attr_pattern.len();
        let val_end = block[val_start..]
            .find('"')
            .map(|e| val_start + e)
            .unwrap_or(val_start);
        block.replace_range(val_start..val_end, &escape_attr(value));
    } else {
        // Insert before the closing > of the opening tag.
        let insert_at = if block.as_bytes()[open_end - 1] == b'/' {
            open_end - 1
        } else {
            open_end
        };
        let insertion = format!(" {}=\"{}\"", attr, escape_attr(value));
        block.insert_str(insert_at, &insertion);
    }
}

/// Replace the first `<w:t>...</w:t>` content with `new_text`.
fn replace_first_text_node(
    block: &str,
    new_text: &str,
    opts: &FindReplaceOptions,
) -> (String, usize) {
    let Some(t_start) = block.find("<w:t") else {
        return (block.to_string(), 0);
    };
    let Some(close_after_open) = block[t_start..].find('>') else {
        return (block.to_string(), 0);
    };
    let open_end = t_start + close_after_open + 1;
    let Some(t_close_rel) = block[open_end..].find("</w:t>") else {
        return (block.to_string(), 0);
    };
    let t_close = open_end + t_close_rel;
    let mut out = String::with_capacity(block.len() + new_text.len());
    out.push_str(&block[..open_end]);
    out.push_str(new_text);
    out.push_str(&block[t_close..]);
    // We don't actually do find/replace here — we replace the whole run text.
    // Return count = 1 to signal one substitution made.
    let _ = opts;
    (out, 1)
}

fn extract_id_from_path(path: &str, prefix: &str) -> Result<String, HandlerError> {
    let rest = path
        .trim_start_matches('/')
        .strip_prefix(prefix)
        .ok_or_else(|| {
            HandlerError::InvalidArgument(format!("expected '/{}/<id>', got '{}'", prefix, path))
        })?;
    let rest = rest.trim_start_matches('/');
    // Accept either "Heading1" or "comment[1]" or "5".
    if let Some(bracket) = rest.find('[') {
        let inner = &rest[bracket + 1..rest.find(']').unwrap_or(rest.len())];
        return Ok(inner.to_string());
    }
    Ok(rest.to_string())
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

/// Escape XML text content (not attribute values): minimal but enough for
/// chart titles / category labels we generate locally.
fn xml_escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Add an image to a paragraph in word/document.xml, embedding it as a
/// full OOXML picture: writes `word/media/imageN.<ext>`, wires
/// `word/_rels/document.xml.rels` to the image, updates `[Content_Types].xml`
/// with the extension's MIME type, and inserts a `<w:drawing>` with an inline
/// `<wp:inline>` anchor referencing the relationship.
///
/// Supported properties: src (path on disk, optional), payloadBase64 /
/// payloadHex (alternative binary source), format/ext, width/height (EMU,
/// "4in", "10cm", "200px"), alt/description, name.
pub fn add_image_part_aware(
    package: &mut OxmlPackage,
    parent: &str,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    use std::path::Path;

    // Resolve extension and content type.
    let ext = properties
        .get("format")
        .or_else(|| properties.get("ext"))
        .map(|s| s.as_str())
        .or_else(|| {
            // Try to derive from src filename extension.
            properties
                .get("src")
                .or_else(|| properties.get("path"))
                .or_else(|| properties.get("file"))
                .and_then(|p| Path::new(p).extension())
                .and_then(|e| e.to_str())
        })
        .unwrap_or("png");
    let (ext_norm, content_type) = match ext.to_lowercase().as_str() {
        "png" => ("png", "image/png"),
        "jpg" | "jpeg" => ("jpeg", "image/jpeg"),
        "gif" => ("gif", "image/gif"),
        "bmp" => ("bmp", "image/bmp"),
        "tiff" | "tif" => ("tiff", "image/tiff"),
        "webp" => ("webp", "image/webp"),
        "svg" => ("svg", "image/svg+xml"),
        "ico" => ("ico", "image/x-icon"),
        "emf" => ("emf", "image/x-emf"),
        "wmf" => ("wmf", "image/x-wmf"),
        _ => ("png", "image/png"),
    };

    // Dimensions in EMU. Default 4in × 3in.
    let (width_emu, height_emu) = parse_image_dimensions_emu(properties);
    let alt = properties
        .get("alt")
        .or_else(|| properties.get("description"))
        .map(|s| s.as_str())
        .unwrap_or("");
    let name = properties
        .get("name")
        .cloned()
        .unwrap_or_else(|| format!("Image {}", ext_norm));

    // Probe for next free image index.
    let image_idx = next_docx_image_index(package, ext_norm);
    let media_path = format!("word/media/image{}.{}", image_idx, ext_norm);

    // Write image binary — priority: src file > payloadBase64 > payloadHex > empty stub.
    let bytes_written = if let Some(src) = properties.get("src").or_else(|| properties.get("path")).or_else(|| properties.get("file"))
    {
        std::fs::read(src).ok()
    } else if let Some(b64) = properties.get("payloadBase64") {
        docx_base64_decode(b64).ok()
    } else if let Some(hex) = properties.get("payloadHex") {
        docx_hex_decode(hex).ok()
    } else {
        Some(Vec::new())
    };
    if let Some(bytes) = bytes_written {
        let _ = package.write_part(&media_path, bytes);
    }

    // Wire document.xml.rels → image relationship.
    let doc_rels_path = "word/_rels/document.xml.rels";
    let image_rel_id = next_docx_rel_id(package, doc_rels_path);
    let rel_target = format!("media/image{}.{}", image_idx, ext_norm);
    let rel_xml = format!(
        "<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"{}\"/>",
        image_rel_id, rel_target
    );
    inject_docx_relationship(package, doc_rels_path, &rel_xml)?;

    // Update [Content_Types].xml with the image extension's Default entry.
    update_docx_content_types_for_image(package, ext_norm, content_type)?;

    // Insert <w:drawing> into the target paragraph (or body) of word/document.xml.
    let doc_xml = package
        .read_part_xml("word/document.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    // docPr id — use the image index so it stays unique across the document.
    let doc_pr_id = image_idx;
    let drawing_xml = format!(
        r#"<w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{w}" cy="{h}"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:docPr id="{id}" name="{name}" descr="{alt}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="{id}" name="{name}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{w}" cy="{h}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#,
        w = width_emu,
        h = height_emu,
        id = doc_pr_id,
        name = escape_attr(&name),
        alt = escape_attr(alt),
        rid = image_rel_id,
    );

    let new_doc_xml = insert_drawing_in_paragraph(&doc_xml, parent, &drawing_xml)?;
    package
        .write_part_xml("word/document.xml", &new_doc_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    Ok(format!("{}/drawing[{}]", parent, image_idx))
}

/// Add a chart to a Word document. Mirrors `WordHandler.Add.Misc.cs /
/// WordHandler.Helpers.Chart.cs` from the C# upstream: writes
/// `word/charts/chartN.xml` (ChartSpace with inline literal data so the
/// chart is self-contained), wires `word/_rels/document.xml.rels`, adds the
/// chart's Override entry to `[Content_Types].xml`, and injects a
/// `<w:drawing>` containing `<wp:inline>` + `<a:graphic>` + `<c:chart>`
/// reference into the target paragraph.
///
/// Supported properties:
///   type=bar|column|line|pie     (default: column)
///   title=<chart title>          (default: "Chart")
///   categories=A,B,C             (CSV literal; default Cat A/B/C)
///   values=1,2,3                 (CSV literal of numbers — required)
///   width, height                (EMU or "1in"/"2cm"; default 4in × 3in)
pub fn add_chart_part_aware(
    package: &mut OxmlPackage,
    parent: &str,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let chart_type = properties
        .get("type")
        .map(|s| s.as_str())
        .unwrap_or("column")
        .to_lowercase();
    let title = properties
        .get("title")
        .cloned()
        .unwrap_or_else(|| "Chart".to_string());
    let categories = properties
        .get("categories")
        .or_else(|| properties.get("cat"))
        .cloned()
        .unwrap_or_else(|| "Cat A,Cat B,Cat C".to_string());
    let values = properties
        .get("values")
        .or_else(|| properties.get("val"))
        .cloned()
        .unwrap_or_else(|| "1,2,3".to_string());

    let cats: Vec<&str> = categories
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let vals: Vec<f64> = values
        .split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect();
    if vals.is_empty() {
        return Err(HandlerError::InvalidArgument(
            "chart requires 'values' as CSV of numbers (e.g. values=1,2,3)".to_string(),
        ));
    }

    let chart_idx = next_docx_chart_index(package);
    let chart_path = format!("word/charts/chart{}.xml", chart_idx);

    let chart_xml = build_docx_chart_xml(&chart_type, &title, &cats, &vals)?;
    package
        .write_part_xml(&chart_path, &chart_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    // document.xml.rels → chart relationship.
    let doc_rels_path = "word/_rels/document.xml.rels";
    let chart_rel_id = next_docx_rel_id(package, doc_rels_path);
    let chart_target = format!("charts/chart{}.xml", chart_idx);
    let rel_xml = format!(
        "<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"{}\"/>",
        chart_rel_id, chart_target
    );
    inject_docx_relationship(package, doc_rels_path, &rel_xml)?;

    // [Content_Types].xml Override for the chart part.
    update_docx_content_types_for_chart(package, &chart_path)?;

    // Inject <w:drawing> referencing the chart into the target paragraph.
    let doc_xml = package
        .read_part_xml("word/document.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let (width_emu, height_emu) = parse_image_dimensions_emu(properties);
    let doc_pr_id = chart_idx;
    let drawing_xml = format!(
        r#"<w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{w}" cy="{h}"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:docPr id="{id}" name="Chart {idx}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="{rid}"/></a:graphicData></a:graphic></wp:inline></w:drawing>"#,
        w = width_emu,
        h = height_emu,
        id = doc_pr_id,
        idx = chart_idx,
        rid = chart_rel_id,
    );
    let new_doc_xml = insert_drawing_in_paragraph(&doc_xml, parent, &drawing_xml)?;
    package
        .write_part_xml("word/document.xml", &new_doc_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    Ok(format!("{}/drawing[{}]", parent, chart_idx))
}

/// Find next free chart index in word/charts/chartN.xml.
fn next_docx_chart_index(package: &OxmlPackage) -> usize {
    let mut i = 1;
    loop {
        let path = format!("word/charts/chart{}.xml", i);
        if package.read_part_xml(&path).is_err() {
            return i;
        }
        i += 1;
        // Sanity ceiling — no document should legitimately reach this.
        if i > 9999 {
            return i;
        }
    }
}

/// Build a self-contained ChartSpace XML for a docx chart. The chart embeds
/// its categories and values via `strCache` / `numCache` so viewers don't
/// need a backing spreadsheet.
fn build_docx_chart_xml(
    chart_type: &str,
    title: &str,
    cats: &[&str],
    vals: &[f64],
) -> Result<String, HandlerError> {
    let (bar_dir, chart_kind) = match chart_type {
        "bar" => ("bar", "barChart"),
        "column" => ("col", "barChart"),
        "line" => ("", "lineChart"),
        "pie" => ("", "pieChart"),
        other => {
            return Err(HandlerError::InvalidArgument(format!(
                "unsupported chart type '{}': expected bar/column/line/pie",
                other
            )))
        }
    };

    let mut xml = String::with_capacity(2048);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    xml.push_str(
        "<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" ",
    );
    xml.push_str("xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" ");
    xml.push_str(
        "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
    );
    xml.push_str("<c:chart>");
    if !title.is_empty() {
        xml.push_str(&format!(
            "<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\"/><a:t>{}</a:t></a:r></a:p></c:rich></c:tx></c:title>",
            xml_escape_text(title)
        ));
    }
    xml.push_str("<c:autoTitleDeleted val=\"0\"/>");
    xml.push_str("<c:plotArea><c:layout/>");
    xml.push_str(&format!("<c:{}>", chart_kind));
    if chart_kind == "barChart" {
        xml.push_str(&format!("<c:barDir val=\"{}\"/>", bar_dir));
        xml.push_str("<c:grouping val=\"clustered\"/>");
        xml.push_str("<c:varyColors val=\"1\"/>");
    } else if chart_kind == "lineChart" {
        xml.push_str("<c:grouping val=\"standard\"/>");
        xml.push_str("<c:varyColors val=\"1\"/>");
        xml.push_str("<c:smooth val=\"0\"/>");
    } else {
        xml.push_str("<c:varyColors val=\"1\"/>");
    }
    xml.push_str("<c:ser>");
    xml.push_str("<c:idx val=\"0\"/>");
    xml.push_str("<c:order val=\"0\"/>");
    if !cats.is_empty() {
        xml.push_str("<c:cat><c:strRef><c:f>categories</c:f><c:strCache>");
        xml.push_str(&format!("<c:ptCount val=\"{}\"/>", cats.len()));
        for (i, c) in cats.iter().enumerate() {
            xml.push_str(&format!(
                "<c:pt idx=\"{}\"><c:v>{}</c:v></c:pt>",
                i,
                xml_escape_text(c)
            ));
        }
        xml.push_str("</c:strCache></c:strRef></c:cat>");
    }
    xml.push_str("<c:val><c:numRef><c:f>values</c:f><c:numCache>");
    xml.push_str("<c:formatCode>General</c:formatCode>");
    xml.push_str(&format!("<c:ptCount val=\"{}\"/>", vals.len()));
    for (i, v) in vals.iter().enumerate() {
        xml.push_str(&format!("<c:pt idx=\"{}\"><c:v>{}</c:v></c:pt>", i, v));
    }
    xml.push_str("</c:numCache></c:numRef></c:val>");
    xml.push_str("</c:ser>");
    if chart_kind == "barChart" {
        xml.push_str("<c:axId val=\"1\"/><c:axId val=\"2\"/>");
        xml.push_str("</c:barChart>");
        xml.push_str("<c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"bottom\"/></c:catAx>");
        xml.push_str("<c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"left\"/></c:valAx>");
    } else if chart_kind == "lineChart" {
        xml.push_str("<c:axId val=\"1\"/><c:axId val=\"2\"/>");
        xml.push_str("</c:lineChart>");
        xml.push_str("<c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"bottom\"/></c:catAx>");
        xml.push_str("<c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"left\"/></c:valAx>");
    } else {
        xml.push_str("</c:pieChart>");
    }
    xml.push_str("</c:plotArea>");
    xml.push_str("</c:chart>");
    xml.push_str("</c:chartSpace>");
    Ok(xml)
}

/// Add a chart part Override entry to [Content_Types].xml. Chart parts
/// are unique per part-path so we use Override rather than Default.
fn update_docx_content_types_for_chart(
    package: &mut OxmlPackage,
    chart_path: &str,
) -> Result<(), HandlerError> {
    let xml = package
        .read_part_xml("[Content_Types].xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let part_name_attr = format!("PartName=\"/{}\"", chart_path);
    if xml.contains(&part_name_attr) {
        return Ok(());
    }
    let override_xml = format!(
        "<Override PartName=\"/{}\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>",
        chart_path
    );
    let new_xml = if let Some(close) = xml.find('>') {
        let mut out = String::with_capacity(xml.len() + override_xml.len());
        out.push_str(&xml[..close + 1]);
        out.push_str(&override_xml);
        out.push_str(&xml[close + 1..]);
        out
    } else {
        format!("<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">{}</Types>", override_xml)
    };
    package
        .write_part_xml("[Content_Types].xml", &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}
fn next_docx_image_index(package: &OxmlPackage, ext: &str) -> usize {
    let mut i = 1;
    loop {
        let path = format!("word/media/image{}.{}", i, ext);
        if package.read_part_xml(&path).is_err() {
            return i;
        }
        i += 1;
    }
}

/// Find the next free rIdN in a relationships part. Defaults to rId1 if absent.
fn next_docx_rel_id(package: &OxmlPackage, rels_path: &str) -> String {
    let xml = package.read_part_xml(rels_path).unwrap_or_default();
    let mut max_id = 0usize;
    for part in xml.split("Id=\"rId") {
        if let Some(end) = part.find('"') {
            if let Ok(id) = part[..end].parse::<usize>() {
                if id > max_id {
                    max_id = id;
                }
            }
        }
    }
    format!("rId{}", max_id + 1)
}

/// Inject a Relationship element into a .rels part, creating the wrapper if missing.
fn inject_docx_relationship(
    package: &mut OxmlPackage,
    rels_path: &str,
    rel_xml: &str,
) -> Result<(), HandlerError> {
    let xml = package.read_part_xml(rels_path).unwrap_or_else(|_| {
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>".to_string()
    });

    let new_xml = if let Some(pos) = xml.find("</Relationships>") {
        let mut r = xml.clone();
        r.insert_str(pos, rel_xml);
        r
    } else if xml.trim().is_empty()
        || xml.trim() == "<Relationships/>"
        || xml.trim()
            == "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>"
    {
        let mut r = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">".to_string();
        r.push_str(rel_xml);
        r.push_str("</Relationships>");
        r
    } else {
        let mut r = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships>".to_string();
        r.push_str(rel_xml);
        r.push_str("</Relationships>");
        r
    };

    package
        .write_part_xml(rels_path, &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}

/// Add Default extension entry to [Content_Types].xml if the extension isn't registered.
fn update_docx_content_types_for_image(
    package: &mut OxmlPackage,
    ext: &str,
    content_type: &str,
) -> Result<(), HandlerError> {
    let xml = package
        .read_part_xml("[Content_Types].xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let ext_attr = format!("Extension=\"{}\"", ext);
    if xml.contains(&ext_attr) {
        return Ok(());
    }
    let default_xml = format!(
        "<Default Extension=\"{}\" ContentType=\"{}\"/>",
        ext, content_type
    );
    let new_xml = if let Some(close) = xml.find('>') {
        // Insert Default right after <Types ...>.
        let mut out = String::with_capacity(xml.len() + default_xml.len());
        out.push_str(&xml[..close + 1]);
        out.push_str(&default_xml);
        out.push_str(&xml[close + 1..]);
        out
    } else {
        xml.replace("</Types>", &format!("{}{}</Types>", default_xml, ""))
    };
    package
        .write_part_xml("[Content_Types].xml", &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}

/// Insert a `<w:drawing>` element inside the paragraph at `parent` path. If the
/// path is "/" or "/body", appends to the last paragraph in the body (or
/// creates one if none exists).
fn insert_drawing_in_paragraph(
    doc_xml: &str,
    parent: &str,
    drawing_xml: &str,
) -> Result<String, HandlerError> {
    let target = if parent == "/" || parent == "/body" || parent.is_empty() {
        // Append to last <w:p>...</w:p>, creating one if none exists.
        if let Some(close_idx) = doc_xml.rfind("</w:p>") {
            let mut out = String::with_capacity(doc_xml.len() + drawing_xml.len());
            out.push_str(&doc_xml[..close_idx]);
            out.push_str(drawing_xml);
            out.push_str(&doc_xml[close_idx..]);
            return Ok(out);
        }
        // No paragraphs: inject one right before </w:body>.
        let wrap = format!("<w:p>{}{}</w:p>", drawing_xml, "");
        if let Some(body_end) = doc_xml.find("</w:body>") {
            let mut out = String::with_capacity(doc_xml.len() + wrap.len());
            out.push_str(&doc_xml[..body_end]);
            out.push_str(&wrap);
            out.push_str(&doc_xml[body_end..]);
            return Ok(out);
        }
        return Err(HandlerError::OperationFailed(
            "could not locate body for image insertion".to_string(),
        ));
    } else {
        // Parent is like /body/p[N] — find the Nth <w:p>.
        let p_idx = parse_paragraph_index_from_parent(parent).ok_or_else(|| {
            HandlerError::InvalidPath(format!(
                "image add expects '/body/p[N]' parent, got '{}'",
                parent
            ))
        })?;
        locate_nth_w_p(doc_xml, p_idx).ok_or_else(|| {
            HandlerError::PathNotFound(format!("paragraph index {} not found", p_idx))
        })?
    };

    // target points at the position right before the matching </w:p>.
    let mut out = String::with_capacity(doc_xml.len() + drawing_xml.len());
    out.push_str(&doc_xml[..target]);
    out.push_str(drawing_xml);
    out.push_str(&doc_xml[target..]);
    Ok(out)
}

/// Parse "/body/p[3]" → Some(3).
fn parse_paragraph_index_from_parent(parent: &str) -> Option<usize> {
    let lower = parent.to_lowercase();
    let pos = lower.find("/p[")?;
    let rest = &parent[pos + 3..];
    let end = rest.find(']')?;
    rest[..end].parse::<usize>().ok()
}

/// Return the byte offset of `</w:p>` for the Nth <w:p> element (1-based).
/// Matches both bare `<w:p>` and attributed `<w:p paraId="..." ...>`.
fn locate_nth_w_p(xml: &str, n: usize) -> Option<usize> {
    let bytes = xml.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + 3 < bytes.len() {
        // Look for `<w:p` followed by `>` or ` `.
        if bytes[i] == b'<' && bytes[i + 1] == b'w' && bytes[i + 2] == b':' && bytes[i + 3] == b'p'
        {
            let next = bytes.get(i + 4).copied().unwrap_or(0);
            if next == b'>' || next == b' ' || next == b'\t' || next == b'\n' {
                count += 1;
                if count == n {
                    // Find the corresponding </w:p> after this opening tag.
                    return xml[i..].find("</w:p>").map(|p| i + p);
                }
            }
        }
        i += 1;
    }
    None
}

/// Parse dimension properties (width / height) in EMU. Accepts numeric EMU
/// or unit suffixes like "4in", "10cm", "200px", "300pt". Default 4in × 3in.
fn parse_image_dimensions_emu(props: &HashMap<String, String>) -> (i64, i64) {
    let width = props
        .get("width")
        .or_else(|| props.get("w"))
        .map(|s| parse_emu(s))
        .unwrap_or(3_657_600); // 4 inches
    let height = props
        .get("height")
        .or_else(|| props.get("h"))
        .map(|s| parse_emu(s))
        .unwrap_or(2_743_200); // 3 inches
    (width, height)
}

/// Convert a measurement string into EMU (English Metric Units: 914400/inch).
fn parse_emu(s: &str) -> i64 {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("in") {
        v.trim()
            .parse::<f64>()
            .map(|n| (n * 914400.0) as i64)
            .unwrap_or(3_657_600)
    } else if let Some(v) = s.strip_suffix("cm") {
        v.trim()
            .parse::<f64>()
            .map(|n| (n * 360000.0) as i64)
            .unwrap_or(3_657_600)
    } else if let Some(v) = s.strip_suffix("mm") {
        v.trim()
            .parse::<f64>()
            .map(|n| (n * 36000.0) as i64)
            .unwrap_or(3_657_600)
    } else if let Some(v) = s.strip_suffix("pt") {
        v.trim()
            .parse::<f64>()
            .map(|n| (n * 12700.0) as i64)
            .unwrap_or(3_657_600)
    } else if let Some(v) = s.strip_suffix("px") {
        v.trim()
            .parse::<f64>()
            .map(|n| (n * 9525.0) as i64)
            .unwrap_or(3_657_600)
    } else {
        s.trim().parse::<f64>()
            .map(|n| (n * 12700.0) as i64)
            .unwrap_or(3_657_600)
    }
}

fn docx_base64_decode(s: &str) -> Result<Vec<u8>, ()> {
    let mut bits: u32 = 0;
    let mut nbits: u32 = 0;
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for c in s.chars().filter(|c| !c.is_whitespace()) {
        let v: u32 = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            'a'..='z' => (c as u32) - ('a' as u32) + 26,
            '0'..='9' => (c as u32) - ('0' as u32) + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            '=' => break,
            _ => return Err(()),
        };
        bits = (bits << 6) | v;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
        }
    }
    Ok(out)
}

fn docx_hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err(());
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let byte = u8::from_str_radix(&format!("{}{}", bytes[i] as char, bytes[i + 1] as char), 16)
            .map_err(|_| ())?;
        out.push(byte);
        i += 2;
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────
// Document Defaults / Settings / Compatibility
//
// These targets live in `word/styles.xml` (`<w:docDefaults>`) and
// `word/settings.xml` (`<w:defaultTabStop>`, `<w:compat>`, etc.) — not in
// the document body, so the body DOM is the wrong layer. They mirror the C#
// `WordHandler.Set.DocDefaults.cs` / `Set.DocSettings.cs` / `Set.Compatibility.cs`
// partial classes but with a flat key namespace (no `rPr.` / `pPr.` nesting)
// since callers are humans/agents, not OOXML authors.

/// Keys accepted by `set /docDefaults`. The `r.` / `run.` prefix is optional;
/// `p.` / `para.` likewise.
const DOC_DEFAULTS_RUN_KEYS: &[&str] = &[
    "r.font",
    "run.font",
    "r.size",
    "run.size",
    "r.color",
    "run.color",
    "r.bold",
    "run.bold",
    "r.italic",
    "run.italic",
    "r.lang",
    "run.lang",
];
const DOC_DEFAULTS_PARA_KEYS: &[&str] = &[
    "p.spacing",
    "para.spacing",
    "p.align",
    "para.align",
    "p.ind",
    "para.ind",
];

pub fn set_doc_defaults_on_part(
    package: &mut OxmlPackage,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let xml = package
        .read_part_xml("word/styles.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    // Locate or synthesize <w:docDefaults>...</w:docDefaults>.
    let dd_open_marker = "<w:docDefaults";
    let dd_close_marker = "</w:docDefaults>";
    let doc_defaults_block = match xml.find(dd_open_marker) {
        Some(open) => {
            let close = match find_matching_close(&xml, open, dd_open_marker, dd_close_marker) {
                Some(c) => c,
                None => {
                    return Err(HandlerError::OperationFailed(
                        "malformed <w:docDefaults> block in styles.xml".into(),
                    ))
                }
            };
            xml[open..close].to_string()
        }
        None => {
            r#"<w:docDefaults><w:rPrDefault><w:rPr/></w:rPrDefault><w:pPrDefault><w:pPr/></w:pPrDefault></w:docDefaults>"#
                .to_string()
        }
    };

    let mut new_block = doc_defaults_block.clone();
    let mut unsupported = Vec::new();

    // Route each property into the rPr (inside <w:rPrDefault>) or pPr
    // (inside <w:pPrDefault>) sub-block of docDefaults. The wrapped block
    // includes `<w:rPr>...</w:rPr>` or `<w:pPr>...</w:pPr>` tags so that
    // set_or_replace_attr_child / toggle_flag_child still anchor on the
    // opening tag's `>`.
    for (k, v) in properties {
        let key = k.as_str();
        if DOC_DEFAULTS_RUN_KEYS.contains(&key) {
            let mut rpr = extract_or_synthesize_wrapped(&mut new_block, "w:rPrDefault", "w:rPr");
            apply_run_property(&mut rpr, key_to_run_attr(key), v);
            splice_wrapped_back(&mut new_block, "w:rPrDefault", "w:rPr", &rpr);
        } else if DOC_DEFAULTS_PARA_KEYS.contains(&key) {
            let mut ppr = extract_or_synthesize_wrapped(&mut new_block, "w:pPrDefault", "w:pPr");
            apply_para_property(&mut ppr, key_to_para_attr(key), v);
            splice_wrapped_back(&mut new_block, "w:pPrDefault", "w:pPr", &ppr);
        } else {
            unsupported.push(k.clone());
        }
    }

    if new_block != doc_defaults_block {
        let new_xml = if xml.contains(dd_open_marker) {
            let open = xml.find(dd_open_marker).unwrap();
            let close = find_matching_close(&xml, open, dd_open_marker, dd_close_marker).unwrap();
            let mut out = String::with_capacity(xml.len() + new_block.len());
            out.push_str(&xml[..open]);
            out.push_str(&new_block);
            out.push_str(&xml[close..]);
            out
        } else {
            // Insert docDefaults right after the root <w:styles ...> opening tag.
            let open = match xml
                .find("<w:styles")
                .and_then(|p| xml[p..].find('>').map(|q| p + q + 1))
            {
                Some(p) => p,
                None => {
                    return Err(HandlerError::OperationFailed(
                        "styles.xml missing <w:styles> root".into(),
                    ))
                }
            };
            let mut out = String::with_capacity(xml.len() + new_block.len() + 2);
            out.push_str(&xml[..open]);
            out.push_str(&new_block);
            out.push_str(&xml[open..]);
            out
        };

        package
            .write_part_xml("word/styles.xml", &new_xml)
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    }

    Ok(unsupported)
}

/// Extract the wrapped `<w:{inner}>…</w:{inner}>` block from inside the
/// `<w:{wrapper}>…</w:{wrapper}>` element of `block`. The returned string
/// contains the wrapper's child element with its open+close tags so that
/// helpers (which anchor on an opening tag's `>`) work unchanged.
/// If either wrapper or inner is missing, synthesize an empty
/// `<w:{wrapper}><w:{inner}></w:{inner}></w:{wrapper}>` fragment.
fn extract_or_synthesize_wrapped(block: &mut String, wrapper: &str, inner: &str) -> String {
    let wrapper_open_marker = format!("<{}", wrapper);
    let wrapper_open = match block.find(wrapper_open_marker.as_str()) {
        Some(p) => p,
        None => {
            let fragment = format!("<{}><{}></{}></{}>", wrapper, inner, inner, wrapper);
            let insert_at = find_inner_insertion_point(block);
            block.insert_str(insert_at, &fragment);
            return format!("<{}></{}>", inner, inner);
        }
    };
    let wrapper_tag_close = match block[wrapper_open..].find('>') {
        Some(q) => wrapper_open + q,
        None => return format!("<{}></{}>", inner, inner),
    };
    if block.as_bytes().get(wrapper_tag_close.saturating_sub(1)) == Some(&b'/') {
        // `<w:{wrapper}/>` self-closed — replace with explicit open/close
        // containing an empty inner element.
        let replacement = format!("<{}><{}></{}></{}>", wrapper, inner, inner, wrapper);
        block.replace_range(wrapper_open..wrapper_tag_close + 1, &replacement);
        return format!("<{}></{}>", inner, inner);
    }
    let after_open = wrapper_tag_close + 1;
    let inner_open_marker = format!("<{}", inner);
    let inner_open = match block[after_open..].find(inner_open_marker.as_str()) {
        Some(p) => after_open + p,
        None => {
            // Insert an empty inner element right after wrapper open.
            let empty_inner = format!("<{}></{}>", inner, inner);
            block.insert_str(after_open, &empty_inner);
            return empty_inner;
        }
    };
    let inner_tag_close = match block[inner_open..].find('>') {
        Some(q) => inner_open + q,
        None => return format!("<{}></{}>", inner, inner),
    };
    if block.as_bytes().get(inner_tag_close.saturating_sub(1)) == Some(&b'/') {
        // Self-closed `<w:{inner}/>`. Replace with `<w:{inner}></w:{inner}>`.
        let replacement = format!("<{}></{}>", inner, inner);
        block.replace_range(inner_open..inner_tag_close + 1, &replacement);
        return replacement;
    }
    // Inner has explicit open/close. Find the close.
    let inner_close_marker = format!("</{}>", inner);
    let close_from = inner_tag_close + 1;
    let inner_close = match block[close_from..].find(inner_close_marker.as_str()) {
        Some(p) => close_from + p,
        None => return format!("<{}></{}>", inner, inner),
    };
    let end_after_close = inner_close + inner_close_marker.len();
    block[inner_open..end_after_close].to_string()
}

/// Splice the (possibly modified) wrapped inner block back into `block`.
/// Inverse of `extract_or_synthesize_wrapped`.
fn splice_wrapped_back(block: &mut String, wrapper: &str, inner: &str, new_wrapped: &str) {
    let wrapper_open_marker = format!("<{}", wrapper);
    let inner_open_marker = format!("<{}", inner);
    let wrapper_open = match block.find(wrapper_open_marker.as_str()) {
        Some(p) => p,
        None => return,
    };
    let inner_search_from = match block[wrapper_open..].find('>') {
        Some(q) => wrapper_open + q + 1,
        None => return,
    };
    let inner_open = match block[inner_search_from..].find(inner_open_marker.as_str()) {
        Some(p) => inner_search_from + p,
        None => return,
    };
    let inner_tag_close = match block[inner_open..].find('>') {
        Some(q) => inner_open + q,
        None => return,
    };
    if block.as_bytes().get(inner_tag_close.saturating_sub(1)) == Some(&b'/') {
        block.replace_range(inner_open..inner_tag_close + 1, new_wrapped);
        return;
    }
    let inner_close_marker = format!("</{}>", inner);
    let close_from = inner_tag_close + 1;
    let inner_close = match block[close_from..].find(inner_close_marker.as_str()) {
        Some(p) => close_from + p,
        None => return,
    };
    let end_after_close = inner_close + inner_close_marker.len();
    block.replace_range(inner_open..end_after_close, new_wrapped);
}

/// Insertion point inside the outermost <w:docDefaults>...</w:docDefaults>
/// block — right after `<w:docDefaults...>` opening tag.
fn find_inner_insertion_point(block: &str) -> usize {
    let open_marker = "<w:docDefaults";
    if let Some(p) = block.find(open_marker) {
        if let Some(q) = block[p..].find('>') {
            return p + q + 1;
        }
    }
    0
}

/// Translate a CLI key (`r.font`, `run.size`, …) to its OOXML child-element
/// local name (`w:rFonts`, `w:sz`, …).
fn key_to_run_attr(key: &str) -> &'static str {
    let bare = key.split('.').next_back().unwrap_or(key);
    match bare {
        "font" => "w:rFonts",
        "size" => "w:sz",
        "color" => "w:color",
        "bold" => "w:b",
        "italic" => "w:i",
        "lang" => "w:lang",
        _ => "",
    }
}

fn key_to_para_attr(key: &str) -> &'static str {
    let bare = key.split('.').next_back().unwrap_or(key);
    match bare {
        "spacing" => "w:spacing",
        "align" => "w:jc",
        "ind" => "w:ind",
        _ => "",
    }
}

/// Apply a property by rewriting the OOXML child inside the rPr/rPrDefault
/// parent. Mutates `block` in place.
fn apply_run_property(block: &mut String, child_tag: &str, value: &str) {
    if child_tag.is_empty() {
        return;
    }
    match child_tag {
        "w:rFonts" => set_or_replace_attr_child(block, "w:rFonts", "w:ascii", value),
        "w:sz" => set_or_replace_attr_child(block, "w:sz", "w:val", value),
        "w:color" => set_or_replace_attr_child(block, "w:color", "w:val", value),
        "w:b" | "w:i" => toggle_flag_child(block, child_tag, value),
        "w:lang" => set_or_replace_attr_child(block, "w:lang", "w:val", value),
        _ => {}
    }
}

fn apply_para_property(block: &mut String, child_tag: &str, value: &str) {
    if child_tag.is_empty() {
        return;
    }
    match child_tag {
        "w:spacing" => set_or_replace_attr_child(block, "w:spacing", "w:after", value),
        "w:jc" => set_or_replace_attr_child(block, "w:jc", "w:val", value),
        "w:ind" => set_or_replace_attr_child(block, "w:ind", "w:left", value),
        _ => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────
// word/settings.xml — `<w:defaultTabStop>`, `<w:compat>` flags, and other
// top-level settings elements. This is a thin, op-aware wrapper that mirrors
// the C# WordHandler.Set.DocSettings.cs / Set.Compatibility.cs.

/// Keys that toggle a `<w:compat>` flag (present = true, absent = false).
const COMPAT_FLAGS: &[&str] = &[
    "useFELayout",
    "doNotExpandShiftReturn",
    "noLineBreaksAfter",
    "noLineBreaksBefore",
    "saveIfXMLInvalid",
    "doNotUseEastAsianBreakRules",
    "useWord2013",
    "compatExp",
];

/// Keys that map to a single self-closing settings element.
const SETTINGS_ELEMENT_KEYS: &[(&str, &str, &str)] = &[
    // (cli key, child element name, attribute name for value)
    ("defaultTabStop", "w:defaultTabStop", "w:val"),
    (
        "characterSpacingControl",
        "w:characterSpacingControl",
        "w:val",
    ),
    ("trackChanges", "w:trackChanges", ""),
    ("defaultDateFormat", "w:date", "w:val"),
    ("linkStyles", "w:linkStyles", ""),
    ("alignBordersAndEdges", "w:alignBordersAndEdges", ""),
    ("autoFormatOverride", "w:autoFormatOverride", ""),
    ("displayBackgroundShape", "w:displayBackgroundShape", ""),
    (
        "doNotDisplayPageBoundaries",
        "w:doNotDisplayPageBoundaries",
        "",
    ),
    ("embedSystemFonts", "w:embedSystemFonts", ""),
    ("zoomPercent", "w:zoom", "w:percent"),
    ("evenAndOddHeaders", "w:evenAndOddHeaders", ""),
];

pub fn set_settings_on_part(
    package: &mut OxmlPackage,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let mut xml = package
        .read_part_xml("word/settings.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let mut unsupported = Vec::new();

    for (key, value) in properties {
        // A CLI key may use plain form (`compat.useFELayout`) or a bare form
        // (`useFELayout`). The `compat.` / `settings.` prefix is stripped
        // before lookup.
        let bare = key
            .strip_prefix("compat.")
            .or_else(|| key.strip_prefix("settings."))
            .unwrap_or(key);

        if COMPAT_FLAGS.contains(&bare) {
            xml = update_compat_flag(&xml, bare, value);
            continue;
        }

        if let Some((_, elem, attr)) = SETTINGS_ELEMENT_KEYS
            .iter()
            .find(|(k, _, _)| k.eq_ignore_ascii_case(bare))
        {
            xml = update_settings_child(&xml, elem, attr, value);
            continue;
        }

        unsupported.push(key.clone());
    }

    package
        .write_part_xml("word/settings.xml", &xml)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    Ok(unsupported)
}

/// Insert, replace, or remove a `<w:compat>` flag child of `<w:compat>`.
fn update_compat_flag(xml: &str, flag: &str, value: &str) -> String {
    let tag = format!("w:{}", flag);
    let truthy = matches!(
        value.to_ascii_lowercase().as_str(),
        "true" | "1" | "on" | "yes" | ""
    );
    let current = strip_named_element_pub(xml, &tag);
    if !truthy {
        return current;
    }
    // Insert the new flag inside <w:compat>…</w:compat>. If the wrapper is
    // missing, synthesize it just before </w:settings>.
    let compat_marker = "<w:compat>";
    if let Some(open) = current.find(compat_marker) {
        let insert_pos = open + compat_marker.len();
        let mut out = String::with_capacity(current.len() + tag.len() + 4);
        out.push_str(&current[..insert_pos]);
        out.push_str(&format!("<{}/>", tag));
        out.push_str(&current[insert_pos..]);
        return out;
    }
    let settings_close = "</w:settings>";
    if let Some(p) = current.rfind(settings_close) {
        let insertion = format!("<w:compat><{}/></w:compat>", tag);
        let mut out = String::with_capacity(current.len() + insertion.len());
        out.push_str(&current[..p]);
        out.push_str(&insertion);
        out.push_str(&current[p..]);
        return out;
    }
    current
}

/// Insert or replace a single self-closing `<w:NAME w:ATTR="VAL"/>` (or just
/// `<w:NAME/>` when ATTR is empty) at the start of `<w:settings>`. Removes any
/// existing same-named child first.
fn update_settings_child(xml: &str, elem: &str, attr: &str, value: &str) -> String {
    let current = strip_named_element_pub(xml, elem);
    // Build the new element fragment.
    let fragment = if attr.is_empty() {
        format!("<{}/>", elem)
    } else {
        format!("<{} {}=\"{}\"/>", elem, attr, escape_attr(value))
    };
    // Inject right after the opening <w:settings ...> tag.
    let open_close = match current
        .find("<w:settings")
        .and_then(|p| find_tag_close_after(&current, p).map(|q| q + 1))
    {
        Some(p) => p,
        None => return current, // malformed; give up gracefully
    };
    let mut out = String::with_capacity(current.len() + fragment.len() + 2);
    out.push_str(&current[..open_close]);
    out.push_str(&fragment);
    out.push_str(&current[open_close..]);
    out
}

/// Strip every `<prefix:name …>…</prefix:name>` or `<prefix:name …/>` from `xml`.
/// Walks the opening tag char-by-char to find its real close `>`, then either
/// consumes a self-closing form or scans to the matching close tag.
fn strip_named_element_pub(xml: &str, qualified_name: &str) -> String {
    let mut out = xml.to_string();
    let open_tag_pat = format!("<{}", qualified_name);
    let close_tag_pat = format!("</{}>", qualified_name);
    loop {
        let Some(open) = out.find(&open_tag_pat) else {
            break;
        };
        // Ensure the match is the start of a real tag (not a longer name).
        let next = out.as_bytes().get(open + open_tag_pat.len()).copied();
        if !matches!(
            next,
            Some(b' ') | Some(b'/') | Some(b'>') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            break;
        }
        let opening_close = match find_tag_close_after(&out, open) {
            Some(p) => p,
            None => break,
        };
        let opening_close_end = opening_close + 1;
        let self_closing = out.as_bytes().get(opening_close).copied() == Some(b'/');
        if self_closing {
            out.replace_range(open..opening_close_end, "");
            continue;
        }
        let Some(close_rel) = out[opening_close_end..].find(&close_tag_pat) else {
            break;
        };
        let close_start = opening_close_end + close_rel;
        let close_end = close_start + close_tag_pat.len();
        out.replace_range(open..close_end, "");
    }
    out
}

/// Find the byte index of the `>` (or `/` for self-close) that closes the
/// opening tag starting at `tag_open`. Walks through attribute values
/// respecting single/double quotes.
fn find_tag_close_after(s: &str, tag_open: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = tag_open;
    let mut in_single = false;
    let mut in_double = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\'' {
                in_single = false;
            }
        } else if in_double {
            if b == b'"' {
                in_double = false;
            }
        } else {
            match b {
                b'\'' => in_single = true,
                b'"' => in_double = true,
                b'/' => {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                        return Some(i);
                    }
                }
                b'>' => return Some(i),
                _ => {}
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod doc_settings_tests {
    use super::*;

    #[test]
    fn settings_key_lookup() {
        let table = SETTINGS_ELEMENT_KEYS;
        assert!(table.iter().any(|(k, _, _)| *k == "defaultTabStop"));
        assert!(table
            .iter()
            .any(|(k, _, _)| *k == "characterSpacingControl"));
    }

    #[test]
    fn compat_flag_toggle_inserts_into_compat_block() {
        let xml = r#"<w:settings xmlns:w="w"><w:compat/></w:settings>"#;
        let out = update_compat_flag(xml, "useFELayout", "true");
        assert!(out.contains("<w:useFELayout/>"));
        assert!(out.contains("<w:compat><w:useFELayout/>"));
    }

    #[test]
    fn compat_flag_false_strips_existing() {
        let xml = r#"<w:settings><w:compat><w:useFELayout/></w:compat></w:settings>"#;
        let out = update_compat_flag(xml, "useFELayout", "false");
        assert!(!out.contains("useFELayout"));
    }

    #[test]
    fn compat_flag_synthesizes_compat_block_when_missing() {
        let xml = r#"<w:settings></w:settings>"#;
        let out = update_compat_flag(xml, "useFELayout", "true");
        assert!(out.contains("<w:compat><w:useFELayout/></w:compat>"));
    }

    #[test]
    fn settings_inserts_default_tab_stop() {
        let xml = r#"<w:settings xmlns:w="w"></w:settings>"#;
        let out = update_settings_child(xml, "w:defaultTabStop", "w:val", "720");
        assert!(out.contains("<w:defaultTabStop w:val=\"720\"/>"));
    }

    #[test]
    fn settings_replaces_existing() {
        let xml = r#"<w:settings><w:defaultTabStop w:val="360"/></w:settings>"#;
        let out = update_settings_child(xml, "w:defaultTabStop", "w:val", "720");
        assert!(out.contains("720"));
        assert!(!out.contains("360"));
    }

    #[test]
    fn key_to_run_attr_maps_known_keys() {
        assert_eq!(key_to_run_attr("r.font"), "w:rFonts");
        assert_eq!(key_to_run_attr("run.size"), "w:sz");
        assert_eq!(key_to_run_attr("r.color"), "w:color");
        assert_eq!(key_to_run_attr("r.bold"), "w:b");
    }

    #[test]
    fn key_to_para_attr_maps_known_keys() {
        assert_eq!(key_to_para_attr("p.spacing"), "w:spacing");
        assert_eq!(key_to_para_attr("para.align"), "w:jc");
        assert_eq!(key_to_para_attr("p.ind"), "w:ind");
    }
}

#[cfg(test)]
mod chart_tests {
    use super::*;

    #[test]
    fn column_chart_has_axes_and_categories() {
        let cats = vec!["Q1", "Q2", "Q3"];
        let vals = vec![10.0, 20.0, 30.0];
        let xml = build_docx_chart_xml("column", "Revenue", &cats, &vals).unwrap();
        assert!(xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:barDir val=\"col\"/>"));
        assert!(xml.contains("<c:title>"));
        assert!(xml.contains("Revenue</a:t>"));
        assert!(xml.contains("Q1</c:v>"));
        assert!(xml.contains("30</c:v>"));
        assert!(xml.contains("<c:catAx>"));
        assert!(xml.contains("<c:valAx>"));
    }

    #[test]
    fn bar_chart_uses_horizontal_dir() {
        let empty: [&str; 0] = [];
        let xml = build_docx_chart_xml("bar", "", &empty, &[1.0]).unwrap_or_default();
        // No title when empty.
        assert!(!xml.contains("<c:title>"));
        assert!(xml.contains("<c:barDir val=\"bar\"/>"));
    }

    #[test]
    fn line_chart_has_two_axes_no_bar_dir() {
        let xml = build_docx_chart_xml("line", "Trend", &["A", "B"], &[5.0, 9.0]).unwrap();
        assert!(xml.contains("<c:lineChart>"));
        assert!(!xml.contains("<c:barDir"));
        assert!(xml.contains("<c:grouping val=\"standard\"/>"));
    }

    #[test]
    fn pie_chart_has_no_axes() {
        let xml = build_docx_chart_xml("pie", "Share", &["A", "B"], &[1.0, 2.0]).unwrap();
        assert!(xml.contains("<c:pieChart>"));
        assert!(!xml.contains("<c:catAx>"));
        assert!(!xml.contains("<c:valAx>"));
    }

    #[test]
    fn unknown_chart_type_rejected() {
        let err = build_docx_chart_xml("radar", "x", &["a"], &[1.0]).unwrap_err();
        match err {
            HandlerError::InvalidArgument(msg) => assert!(msg.contains("radar")),
            other => panic!("expected InvalidArgument, got {:?}", other),
        }
    }

    #[test]
    fn text_escaping_in_titles() {
        let xml = build_docx_chart_xml("pie", "A & B < C >", &["A"], &[1.0]).unwrap();
        assert!(xml.contains("A &amp; B &lt; C &gt;</a:t>"));
    }
}
