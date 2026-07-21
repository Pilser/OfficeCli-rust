use std::collections::HashMap;

use crate::navigation::SvgNavigator;

pub fn add_element(
    xml: &str,
    parent_path: &str,
    element_type: &str,
    position: Option<usize>,
    props: &HashMap<String, String>,
    wrap: Option<&str>,
) -> Result<(String, String), String> {
    let element_xml = generate_element_xml(element_type, props)?;
    let final_xml = if let Some(w) = wrap {
        if w == "g" {
            format!("<g>{}</g>", element_xml)
        } else {
            return Err(format!("unsupported wrapper type: {}", w));
        }
    } else {
        element_xml
    };

    let doc =
        roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = SvgNavigator::resolve(&doc, parent_path)?;
    let parent_node = resolved.node;
    let parent_range = parent_node.range();

    let children: Vec<_> = parent_node.children().filter(|c| c.is_element()).collect();
    let insert_index = position.unwrap_or(children.len());

    let tag = element_type;
    let index_before = children
        .iter()
        .take(insert_index)
        .filter(|c| c.tag_name().name() == tag)
        .count();
    let new_index = index_before + 1;
    let new_path = format!("{}/{}[{}]", parent_path, tag, new_index);

    let mut result = String::with_capacity(xml.len() + final_xml.len());

    if children.is_empty() {
        let parent_xml = &xml[parent_range.start..parent_range.end];
        let trimmed = parent_xml.trim_end();
        if trimmed.ends_with("/>") {
            let slash_byte_offset = trimmed.len() - 1;
            let leading_ws = parent_xml.len() - trimmed.len();
            let real_slash = parent_range.start + leading_ws + slash_byte_offset;
            result.push_str(&xml[..real_slash]);
            result.push('>');
            result.push_str(&final_xml);
            result.push_str(&format!("</{}>", parent_node.tag_name().name()));
            result.push_str(&xml[parent_range.end..]);
        } else {
            let closing = format!("</{}>", parent_node.tag_name().name());
            let close_pos = xml[parent_range.start..]
                .rfind(&closing)
                .map(|p| parent_range.start + p)
                .unwrap_or(parent_range.end - closing.len());
            result.push_str(&xml[..close_pos]);
            result.push_str(&final_xml);
            result.push_str(&xml[close_pos..]);
        }
    } else if insert_index >= children.len() {
        let last = children.last().unwrap();
        let last_end = last.range().end;
        result.push_str(&xml[..last_end]);
        result.push_str(&final_xml);
        result.push_str(&xml[last_end..]);
    } else if insert_index == 0 {
        let first = children[0];
        let first_start = first.range().start;
        result.push_str(&xml[..first_start]);
        result.push_str(&final_xml);
        result.push_str(&xml[first_start..]);
    } else {
        let before = children[insert_index - 1];
        let before_end = before.range().end;
        result.push_str(&xml[..before_end]);
        result.push_str(&final_xml);
        result.push_str(&xml[before_end..]);
    }

    Ok((new_path, final_xml))
}

fn generate_element_xml(element_type: &str, props: &HashMap<String, String>) -> Result<String, String> {
    let mut attrs = Vec::new();
    let mut has_text = false;
    let mut text_content = String::new();

    for (key, value) in props {
        match key.as_str() {
            "text" | "content" => {
                has_text = true;
                text_content = value.clone();
            }
            _ => {
                attrs.push(format!("{}=\"{}\"", key, value));
            }
        }
    }

    let attr_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    };

    match element_type {
        "rect" | "circle" | "ellipse" | "line" | "path" | "image" | "use" => {
            Ok(format!("<{}{}/>", element_type, attr_str))
        }
        "text" | "tspan" => {
            if has_text {
                Ok(format!("<{}{}>{}</{}>", element_type, attr_str, text_content, element_type))
            } else {
                Ok(format!("<{}{}/>", element_type, attr_str))
            }
        }
        "g" | "a" | "defs" | "clipPath" | "mask" | "symbol" | "marker" | "linearGradient" | "radialGradient" | "pattern" | "filter" => {
            Ok(format!("<{}{}/>", element_type, attr_str))
        }
        _ => {
            Ok(format!("<{}{}/>", element_type, attr_str))
        }
    }
}
