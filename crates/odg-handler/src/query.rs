use handler_common::DocumentNode;

use crate::navigation::{get_attr, normalize_tag_to_type};
use crate::navigation::OdgNavigator;

pub fn query_by_type(content_xml: &str, element_type: &str) -> Vec<DocumentNode> {
    let doc = match roxmltree::Document::parse(content_xml) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let nodes = OdgNavigator::find_all_by_type(&doc, element_type);

    nodes
        .into_iter()
        .map(|resolved| {
            let text = resolved
                .node
                .text()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty());
            let display_type = normalize_tag_to_type(&resolved.element_type);
            let mut node = DocumentNode::new(&resolved.tag_path, display_type);

            for attr in [
                "svg:x", "svg:y", "svg:width", "svg:height", "svg:cx", "svg:cy", "svg:r",
                "svg:rx", "svg:ry", "svg:x1", "svg:y1", "svg:x2", "svg:y2", "svg:d",
                "svg:stroke-width", "draw:fill", "draw:stroke", "draw:opacity",
                "draw:style-name", "draw:name", "draw:transform", "draw:opacity",
                "fo:font-size", "fo:font-family", "fo:color",
            ] {
                if let Some(val) = get_attr(&resolved.node, attr) {
                    node = node.with_format(attr, serde_json::json!(val));
                }
            }

            if let Some(t) = text {
                node = node.with_text(t);
            }

            node
        })
        .collect()
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
    xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
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
    fn test_query_by_type_rect() {
        let results = query_by_type(sample_xml(), "rect");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_type, "rect");
    }

    #[test]
    fn test_query_by_type_circle() {
        let results = query_by_type(sample_xml(), "circle");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_type, "circle");
    }

    #[test]
    fn test_query_by_type_page() {
        let results = query_by_type(sample_xml(), "page");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_type, "page");
    }
}
