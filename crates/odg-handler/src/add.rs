use std::collections::HashMap;

use crate::navigation::OdgNavigator;

pub fn add_element(
    content_xml: &str,
    parent_path: &str,
    element_type: &str,
    position: Option<usize>,
    props: &HashMap<String, String>,
    _wrap: Option<&str>,
) -> Result<(String, String), String> {
    let element_xml = generate_element_xml(element_type, props)?;

    let doc = roxmltree::Document::parse(content_xml)
        .map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = OdgNavigator::resolve(&doc, parent_path)?;
    let parent_node = resolved.node;
    let parent_range = parent_node.range();

    let children: Vec<_> = parent_node.children().filter(|c| c.is_element()).collect();
    let insert_index = position.unwrap_or(children.len());

    let draw_tag_local = match element_type {
        "rect" => "rect",
        "circle" => "circle",
        "ellipse" => "ellipse",
        "line" => "line",
        "polyline" => "polyline",
        "polygon" => "polygon",
        "path" => "path",
        "text-box" => "text-box",
        "image" => "image",
        "connector" => "connector",
        "group" => "g",
        "page" => "page",
        "paragraph" => "p",
        "span" => "span",
        _ => element_type,
    };

    let index_before = children
        .iter()
        .take(insert_index)
        .filter(|c| c.tag_name().name() == draw_tag_local)
        .count();
    let new_index = index_before + 1;
    let new_path = format!("{}/{}[{}]", parent_path, draw_tag_local, new_index);

    let mut result = String::with_capacity(content_xml.len() + element_xml.len());

    if children.is_empty() {
        let parent_xml = &content_xml[parent_range.start..parent_range.end];
        let trimmed = parent_xml.trim_end();
        if trimmed.ends_with("/>") {
            let slash_byte_offset = trimmed.len() - 1;
            let leading_ws = parent_xml.len() - trimmed.len();
            let real_slash = parent_range.start + leading_ws + slash_byte_offset;
            result.push_str(&content_xml[..real_slash]);
            result.push('>');
            result.push_str(&element_xml);
            result.push_str(&format!("</{}>", parent_node.tag_name().name()));
            result.push_str(&content_xml[parent_range.end..]);
        } else {
            let closing = format!("</{}>", parent_node.tag_name().name());
            let close_pos = content_xml[parent_range.start..]
                .rfind(&closing)
                .map(|p| parent_range.start + p)
                .unwrap_or(parent_range.end - closing.len());
            result.push_str(&content_xml[..close_pos]);
            result.push_str(&element_xml);
            result.push_str(&content_xml[close_pos..]);
        }
    } else if insert_index >= children.len() {
        let last = children.last().unwrap();
        let last_end = last.range().end;
        result.push_str(&content_xml[..last_end]);
        result.push_str(&element_xml);
        result.push_str(&content_xml[last_end..]);
    } else if insert_index == 0 {
        let first = children[0];
        let first_start = first.range().start;
        result.push_str(&content_xml[..first_start]);
        result.push_str(&element_xml);
        result.push_str(&content_xml[first_start..]);
    } else {
        let before = children[insert_index - 1];
        let before_end = before.range().end;
        result.push_str(&content_xml[..before_end]);
        result.push_str(&element_xml);
        result.push_str(&content_xml[before_end..]);
    }

    Ok((new_path, result))
}

fn generate_element_xml(element_type: &str, props: &HashMap<String, String>) -> Result<String, String> {
    let mut attrs = Vec::new();
    let mut text_content = String::new();
    let mut has_text = false;

    for (key, value) in props {
        match key.as_str() {
            "text" | "content" => {
                has_text = true;
                text_content = value.clone();
            }
            _ => {
                let draw_key = match key.as_str() {
                    "x" => "svg:x",
                    "y" => "svg:y",
                    "width" => "svg:width",
                    "height" => "svg:height",
                    "cx" => "svg:cx",
                    "cy" => "svg:cy",
                    "r" => "svg:r",
                    "rx" => "svg:rx",
                    "ry" => "svg:ry",
                    "x1" => "svg:x1",
                    "y1" => "svg:y1",
                    "x2" => "svg:x2",
                    "y2" => "svg:y2",
                    "d" => "svg:d",
                    "fill" => "draw:fill",
                    "stroke" => "draw:stroke",
                    "stroke-width" => "svg:stroke-width",
                    "font-size" => "fo:font-size",
                    "font-family" => "fo:font-family",
                    "color" => "fo:color",
                    "name" => "draw:name",
                    "style-name" => "draw:style-name",
                    "opacity" => "draw:opacity",
                    "transform" => "draw:transform",
                    _ => key,
                };
                attrs.push(format!("{}=\"{}\"", draw_key, value));
            }
        }
    }

    let attr_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    };

    let draw_tag = match element_type {
        "rect" => "draw:rect",
        "circle" => "draw:circle",
        "ellipse" => "draw:ellipse",
        "line" => "draw:line",
        "polyline" => "draw:polyline",
        "polygon" => "draw:polygon",
        "path" => "draw:path",
        "text-box" => "draw:text-box",
        "image" => "draw:image",
        "connector" => "draw:connector",
        "group" => "draw:g",
        "page" => "draw:page",
        "paragraph" => "text:p",
        "span" => "text:span",
        _ => element_type,
    };

    match element_type {
        "text-box" => {
            if has_text {
                Ok(format!(
                    "<{}{}><text:p>{}</text:p></{}>",
                    draw_tag, attr_str, text_content, draw_tag
                ))
            } else {
                Ok(format!("<{}{}/>", draw_tag, attr_str))
            }
        }
        "page" => {
            Ok(format!("<{}{}/>", draw_tag, attr_str))
        }
        _ => {
            if has_text {
                Ok(format!(
                    "<{}{}>{}</{}>",
                    draw_tag, attr_str, text_content, draw_tag
                ))
            } else {
                Ok(format!("<{}{}/>", draw_tag, attr_str))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
    office:version="1.2">
  <office:body>
    <office:drawing>
      <draw:page draw:name="page1">
        <draw:rect draw:style-name="gr1" svg:x="1cm" svg:y="1cm" svg:width="10cm" svg:height="5cm"/>
      </draw:page>
    </office:drawing>
  </office:body>
</office:document-content>"#
    }

    #[test]
    fn test_add_circle() {
        let mut props = HashMap::new();
        props.insert("cx".to_string(), "3cm".to_string());
        props.insert("cy".to_string(), "3cm".to_string());
        props.insert("r".to_string(), "1cm".to_string());
        props.insert("fill".to_string(), "blue".to_string());

        let (path, _) = add_element(
            sample_xml(),
            "/document-content/body/drawing/page[1]",
            "circle",
            None,
            &props,
            None,
        )
        .unwrap();
        assert!(path.contains("circle"));
    }

    #[test]
    fn test_add_text_box() {
        let mut props = HashMap::new();
        props.insert("x".to_string(), "1cm".to_string());
        props.insert("y".to_string(), "1cm".to_string());
        props.insert("width".to_string(), "5cm".to_string());
        props.insert("height".to_string(), "2cm".to_string());
        props.insert("text".to_string(), "Hello".to_string());

        let (path, xml) = add_element(
            sample_xml(),
            "/document-content/body/drawing/page[1]",
            "text-box",
            None,
            &props,
            None,
        )
        .unwrap();
        assert!(path.contains("text-box"));
        assert!(xml.contains("Hello"));
    }
}
