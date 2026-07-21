pub fn resolve_path(content_xml: &str, path: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(content_xml)
        .map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = OdgNavigator::resolve(&doc, path)?;
    Ok(resolved.tag_path)
}

pub fn get_element_xml(content_xml: &str, path: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(content_xml)
        .map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = OdgNavigator::resolve(&doc, path)?;
    let range = resolved.node.range();
    Ok(content_xml[range.start..range.end].to_string())
}

pub struct ResolvedNode<'doc> {
    pub node: roxmltree::Node<'doc, 'doc>,
    pub tag_path: String,
    pub element_type: String,
}

struct Seg {
    name: String,
    index: Option<usize>,
    attribute: Option<(String, String)>,
}

fn local_name<'a>(name: &'a str) -> &'a str {
    if let Some(pos) = name.find(':') {
        &name[pos + 1..]
    } else {
        name
    }
}

pub struct OdgNavigator;

impl OdgNavigator {
    pub fn resolve<'doc>(
        doc: &'doc roxmltree::Document,
        path: &str,
    ) -> Result<ResolvedNode<'doc>, String> {
        let segments = Self::parse_path(path)?;
        if segments.is_empty() {
            return Err("empty path".to_string());
        }

        let root = doc.root_element();
        let first_name = root.tag_name().name();
        let first_seg_local = local_name(&segments[0].name);

        let mut current = root;
        let mut tag_path = format!("/{}", first_name);

        if first_seg_local == first_name || segments[0].name == "*" {
            for segment in &segments[1..] {
                let seg_local = local_name(&segment.name);
                let candidates: Vec<_> = current
                    .children()
                    .filter(|c| c.is_element())
                    .filter(|c| {
                        segment.name == "*" || seg_local == "*" || c.tag_name().name() == seg_local
                    })
                    .filter(|c| {
                        if let Some((key, val)) = &segment.attribute {
                            get_attr(c, key) == Some(val.as_str())
                        } else {
                            true
                        }
                    })
                    .collect();

                if candidates.is_empty() {
                    return Err(format!(
                        "no element matching '{}' found at {}",
                        seg_description(segment),
                        tag_path
                    ));
                }

                let idx = segment.index.unwrap_or(1).saturating_sub(1);
                if idx >= candidates.len() {
                    return Err(format!(
                        "index {} out of range for '{}' at {}",
                        segment.index.unwrap_or(1),
                        segment.name,
                        tag_path
                    ));
                }

                current = candidates[idx];
                let my_index = compute_index(&current);
                tag_path = format!("{}/{}[{}]", tag_path, current.tag_name().name(), my_index);
            }
        } else {
            for segment in &segments {
                let seg_local = local_name(&segment.name);
                let candidates: Vec<_> = current
                    .children()
                    .filter(|c| c.is_element())
                    .filter(|c| {
                        segment.name == "*" || seg_local == "*" || c.tag_name().name() == seg_local
                    })
                    .filter(|c| {
                        if let Some((key, val)) = &segment.attribute {
                            get_attr(c, key) == Some(val.as_str())
                        } else {
                            true
                        }
                    })
                    .collect();

                if candidates.is_empty() {
                    return Err(format!(
                        "no element matching '{}' found at {}",
                        seg_description(segment),
                        tag_path
                    ));
                }

                let idx = segment.index.unwrap_or(1).saturating_sub(1);
                if idx >= candidates.len() {
                    return Err(format!(
                        "index {} out of range for '{}' at {}",
                        segment.index.unwrap_or(1),
                        segment.name,
                        tag_path
                    ));
                }

                current = candidates[idx];
                let my_index = compute_index(&current);
                tag_path = format!("{}/{}[{}]", tag_path, current.tag_name().name(), my_index);
            }
        }

        let element_type = current.tag_name().name().to_string();
        Ok(ResolvedNode {
            node: current,
            tag_path,
            element_type,
        })
    }

    pub fn find_all_by_type<'doc>(
        doc: &'doc roxmltree::Document,
        element_type: &str,
    ) -> Vec<ResolvedNode<'doc>> {
        let local_et = local_name(element_type);
        let mut results = Vec::new();
        for node in doc.descendants() {
            if !node.is_element() {
                continue;
            }
            let tag = node.tag_name().name();
            let matches = match local_et {
                "page" => tag == "page",
                "rect" | "rectangle" => tag == "rect",
                "circle" => tag == "circle",
                "ellipse" => tag == "ellipse",
                "line" => tag == "line",
                "polyline" => tag == "polyline",
                "polygon" => tag == "polygon",
                "path" => tag == "path",
                "text-box" | "textbox" => tag == "text-box",
                "image" => tag == "image",
                "connector" => tag == "connector",
                "paragraph" => tag == "p" || tag == "h",
                "span" => tag == "span",
                "page-thumbnail" => tag == "page-thumbnail",
                _ => tag == local_et,
            };
            if matches {
                let tag_path = resolve_tag_path(&node);
                results.push(ResolvedNode {
                    node,
                    tag_path,
                    element_type: tag.to_string(),
                });
            }
        }
        results
    }

    fn parse_path(path: &str) -> Result<Vec<Seg>, String> {
        if !path.starts_with('/') {
            return Err(format!("path must start with /: {}", path));
        }
        let raw_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut segments = Vec::new();
        for raw in raw_segments {
            segments.push(parse_seg(raw)?);
        }
        Ok(segments)
    }
}

fn parse_seg(s: &str) -> Result<Seg, String> {
    let mut name = s.to_string();
    let mut index = None;
    let mut attribute = None;

    let mut bracket_start = 0;
    let remaining = s;

    while let Some(open) = remaining[bracket_start..].find('[') {
        let open_pos = bracket_start + open;
        if let Some(close) = remaining[open_pos..].find(']') {
            let close_pos = open_pos + close;
            let content = &remaining[open_pos + 1..close_pos];

            if name.len() > open_pos || name == s {
                name = remaining[..open_pos].to_string();
            }

            if let Some(attr_content) = content.strip_prefix('@') {
                if let Some(eq_pos) = attr_content.find('=') {
                    let attr_key = attr_content[..eq_pos].to_string();
                    let attr_val = attr_content[eq_pos + 1..].to_string();
                    attribute = Some((attr_key, attr_val));
                }
            } else if let Ok(idx) = content.parse::<usize>() {
                index = Some(idx);
            }

            bracket_start = close_pos + 1;
        } else {
            break;
        }
    }

    Ok(Seg {
        name,
        index,
        attribute,
    })
}

fn seg_description(seg: &Seg) -> String {
    let mut s = seg.name.clone();
    if let Some((k, v)) = &seg.attribute {
        s.push_str(&format!("[@{}={}]", k, v));
    }
    if let Some(idx) = seg.index {
        s.push_str(&format!("[{}]", idx));
    }
    s
}

fn compute_index(node: &roxmltree::Node) -> usize {
    let tag = node.tag_name().name();
    if let Some(parent) = node.parent() {
        let mut count = 0;
        for child in parent.children() {
            if child.is_element() && child.tag_name().name() == tag {
                count += 1;
                if child == *node {
                    return count;
                }
            }
        }
    }
    1
}

fn resolve_tag_path(node: &roxmltree::Node) -> String {
    let mut segments: Vec<String> = Vec::new();
    let mut current = *node;
    loop {
        let tag = current.tag_name().name();
        let idx = compute_index(&current);
        segments.push(format!("{}[{}]", tag, idx));
        if let Some(parent) = current.parent() {
            if parent.is_element() {
                current = parent;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    segments.reverse();
    let mut path = String::new();
    for s in &segments {
        path.push('/');
        path.push_str(s);
    }
    path
}

pub fn get_attr<'a>(node: &'a roxmltree::Node, qualified_name: &str) -> Option<&'a str> {
    let local = if let Some(pos) = qualified_name.find(':') {
        &qualified_name[pos + 1..]
    } else {
        qualified_name
    };
    for attr in node.attributes() {
        if attr.name() == local {
            return Some(attr.value());
        }
    }
    None
}

pub fn normalize_tag_to_type(tag: &str) -> &str {
    match tag {
        "draw:page" => "page",
        "draw:rect" => "rect",
        "draw:circle" => "circle",
        "draw:ellipse" => "ellipse",
        "draw:line" => "line",
        "draw:polyline" => "polyline",
        "draw:polygon" => "polygon",
        "draw:path" => "path",
        "draw:text-box" => "text-box",
        "draw:image" => "image",
        "draw:connector" => "connector",
        "draw:g" => "group",
        "text:p" => "paragraph",
        "text:h" => "paragraph",
        "text:span" => "span",
        "draw:page-thumbnail" => "page-thumbnail",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_content_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
    xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
    office:version="1.2">
  <office:body>
    <office:drawing>
      <draw:page draw:name="page1">
        <draw:rect draw:style-name="gr1" svg:x="1cm" svg:y="1cm" svg:width="10cm" svg:height="5cm"/>
        <draw:text-box draw:style-name="gr2" svg:x="2cm" svg:y="2cm" svg:width="8cm" svg:height="3cm">
          <text:p>Hello World</text:p>
        </draw:text-box>
        <draw:circle draw:style-name="gr3" svg:cx="5cm" svg:cy="5cm" svg:r="2cm"/>
      </draw:page>
    </office:drawing>
  </office:body>
</office:document-content>"#
    }

    #[test]
    fn test_resolve_path_page() {
        let result = resolve_path(sample_content_xml(), "/document-content/body/drawing/page[1]");
        assert!(result.is_ok(), "resolve failed: {:?}", result);
    }

    #[test]
    fn test_resolve_path_rect() {
        let result = resolve_path(
            sample_content_xml(),
            "/document-content/body/drawing/page[1]/rect[1]",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_path_text_box() {
        let result = resolve_path(
            sample_content_xml(),
            "/document-content/body/drawing/page[1]/text-box[1]",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_element_xml() {
        let xml = get_element_xml(
            sample_content_xml(),
            "/document-content/body/drawing/page[1]/rect[1]",
        )
        .unwrap();
        assert!(xml.contains("draw:rect"));
        assert!(xml.contains("svg:width=\"10cm\""));
    }

    #[test]
    fn test_invalid_path() {
        let result = resolve_path(sample_content_xml(), "/document-content/body/nonexistent");
        assert!(result.is_err());
    }
}
