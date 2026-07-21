use handler_common::{BBoxSpan, TextOffsetMap};
use std::collections::HashMap;

pub fn extract_text_with_offsets(content_xml: &str) -> TextOffsetMap {
    let doc = match roxmltree::Document::parse(content_xml) {
        Ok(d) => d,
        Err(_) => return TextOffsetMap::empty("odg"),
    };
    let mut map = TextOffsetMap::empty("odg");
    let root = doc.root_element();
    collect_text_spans(&root, &format!("/{}", root.tag_name().name()), &mut map);
    map
}

fn collect_text_spans(node: &roxmltree::Node, path: &str, map: &mut TextOffsetMap) {
    if !node.is_element() {
        return;
    }
    let tag = node.tag_name().name();

    if tag == "text-box" {
        let bbox = extract_bbox(node);
        let mut para_counts: HashMap<String, usize> = HashMap::new();
        for child in node.children() {
            if child.is_element() {
                let child_tag = child.tag_name().name();
                if child_tag == "p" || child_tag == "h" {
                    let count = para_counts.entry(child_tag.to_string()).or_insert(0);
                    *count += 1;
                    let para_path = format!("{}/paragraph[{}]", path, count);
                    if let Some(text) = child.text() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                                map.push_span_with_metadata(
                                    trimmed,
                                    &para_path,
                                    "paragraph",
                                    bbox.clone(),
                                    None,
                                );
                        }
                    }
                }
            }
        }
        return;
    }

    if tag == "text:p" || tag == "text:h" {
        if let Some(text) = node.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                map.push_span(trimmed, path, "paragraph");
                return;
            }
        }
    }

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            let count = tag_counts.entry(child_tag.to_string()).or_insert(0);
            *count += 1;
            let child_path = format!("{}/{}[{}]", path, child_tag, count);
            collect_text_spans(&child, &child_path, map);
        }
    }
}

fn get_attr_value<'a>(node: &'a roxmltree::Node, local_name: &str) -> Option<&'a str> {
    for attr in node.attributes() {
        if attr.name() == local_name {
            return Some(attr.value());
        }
    }
    None
}

fn extract_bbox(node: &roxmltree::Node) -> Option<BBoxSpan> {
    let x = get_attr_value(node, "x").and_then(parse_length_mm)?;
    let y = get_attr_value(node, "y").and_then(parse_length_mm)?;
    let w = get_attr_value(node, "width").and_then(parse_length_mm)?;
    let h = get_attr_value(node, "height").and_then(parse_length_mm)?;
    Some(BBoxSpan {
        x: x as f32,
        y: y as f32,
        width: w as f32,
        height: h as f32,
    })
}

fn parse_length_mm(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(val) = s.strip_suffix("mm") {
        val.trim().parse::<f64>().ok()
    } else if let Some(val) = s.strip_suffix("cm") {
        val.trim().parse::<f64>().ok().map(|v| v * 10.0)
    } else if let Some(val) = s.strip_suffix("in") {
        val.trim().parse::<f64>().ok().map(|v| v * 25.4)
    } else if let Some(val) = s.strip_suffix("pt") {
        val.trim().parse::<f64>().ok().map(|v| v * 0.352778)
    } else if let Some(val) = s.strip_suffix("px") {
        val.trim().parse::<f64>().ok().map(|v| v * 0.264583)
    } else {
        s.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
    xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
    office:version="1.2">
  <office:body>
    <office:drawing>
      <draw:page draw:name="page1">
        <draw:text-box draw:style-name="gr1" svg:x="1cm" svg:y="1cm" svg:width="10cm" svg:height="5cm">
          <text:p>Hello World</text:p>
          <text:p>Second paragraph</text:p>
        </draw:text-box>
      </draw:page>
    </office:drawing>
  </office:body>
</office:document-content>"#
    }

    #[test]
    fn test_extract_text_with_offsets() {
        let map = extract_text_with_offsets(sample_xml());
        assert!(map.full_text.contains("Hello World"));
        assert!(map.full_text.contains("Second paragraph"));
        assert!(!map.spans.is_empty());
        assert_eq!(map.meta.format, "odg");
    }

    #[test]
    fn test_spans_have_correct_paths() {
        let map = extract_text_with_offsets(sample_xml());
        let p1 = map.spans.iter().find(|s| s.text == "Hello World");
        assert!(p1.is_some());
        assert!(p1.unwrap().path.contains("paragraph[1]"));

        let p2 = map.spans.iter().find(|s| s.text == "Second paragraph");
        assert!(p2.is_some());
        assert!(p2.unwrap().path.contains("paragraph[2]"));
    }

    #[test]
    fn test_spans_have_bounding_box() {
        let map = extract_text_with_offsets(sample_xml());
        eprintln!("spans: {:?}", map.spans);
        for span in &map.spans {
            if span.text == "Hello World" {
                assert!(span.bbox.is_some(), "bbox should be Some for '{}' at path '{}'", span.text, span.path);
                let bbox = span.bbox.as_ref().unwrap();
                assert!((bbox.width - 100.0).abs() < 0.1);
                break;
            }
        }
    }
}
