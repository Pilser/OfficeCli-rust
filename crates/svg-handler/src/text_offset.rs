use handler_common::TextOffsetMap;
use std::collections::HashMap;

pub fn extract_text_with_offsets(xml: &str) -> TextOffsetMap {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut map = TextOffsetMap::empty("svg");
    let root = doc.root_element();
    collect_text_spans(&root, "/svg", &mut map);
    map
}

fn collect_text_spans(node: &roxmltree::Node, path: &str, map: &mut TextOffsetMap) {
    if !node.is_element() {
        return;
    }
    let tag = node.tag_name().name();

    if tag == "text" || tag == "tspan" {
        if let Some(text) = node.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                map.push_span(trimmed, path, tag);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_svg() -> &'static str {
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <rect x="10" y="10" width="80" height="80" fill="red"/>
  <text x="10" y="50" font-size="16">Hello World</text>
</svg>"#
    }

    #[test]
    fn test_extract_text_with_offsets() {
        let map = extract_text_with_offsets(sample_svg());
        assert!(map.full_text.contains("Hello World"));
        assert!(!map.spans.is_empty());
        assert_eq!(map.meta.format, "svg");
    }

    #[test]
    fn test_spans_have_correct_paths() {
        let map = extract_text_with_offsets(sample_svg());
        let text_span = map.spans.iter().find(|s| s.element_type == "text");
        assert!(text_span.is_some());
        assert_eq!(text_span.unwrap().path, "/svg/text[1]");
    }

    #[test]
    fn test_tspan_in_text() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <text x="10" y="50">
    <tspan>First</tspan>
    <tspan>Second</tspan>
  </text>
</svg>"#;
        let map = extract_text_with_offsets(svg);
        assert!(map.full_text.contains("First"));
        assert!(map.full_text.contains("Second"));
        assert_eq!(map.spans.len(), 2);
    }
}
