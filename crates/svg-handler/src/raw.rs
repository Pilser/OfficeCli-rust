use crate::navigation::SvgNavigator;
use handler_common::ValidationError;

pub fn get_raw(xml: &str, part_path: &str, _opts: &handler_common::RawOptions) -> Result<String, String> {
    if part_path == "/" {
        return Ok(xml.to_string());
    }

    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = SvgNavigator::resolve(&doc, part_path)?;
    let range = resolved.node.range();
    Ok(xml[range.start..range.end].to_string())
}

pub fn set_raw(
    xml: &str,
    part_path: &str,
    _xpath: &str,
    action: &str,
    content: Option<&str>,
) -> Result<String, String> {
    if action != "replace" {
        return Err(format!("unsupported raw_set action: {}", action));
    }
    let content = content.ok_or("content required for replace action")?;

    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = SvgNavigator::resolve(&doc, part_path)?;
    let range = resolved.node.range();

    let mut result = String::with_capacity(xml.len() + content.len());
    result.push_str(&xml[..range.start]);
    result.push_str(content);
    result.push_str(&xml[range.end..]);
    Ok(result)
}

pub fn validate(xml: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(e) => {
            errors.push(ValidationError {
                error_type: "xml-well-formed".to_string(),
                description: format!("XML is not well-formed: {}", e),
                path: None,
                part: None,
            });
            return errors;
        }
    };

    if let Err(e) = usvg::Tree::from_str(xml, &usvg::Options::default()) {
        errors.push(ValidationError {
            error_type: "invalid-svg".to_string(),
            description: format!("SVG validation failed: {}", e),
            path: None,
            part: None,
        });
    }

    let root = doc.root_element();

    let has_xmlns = root
        .namespaces()
        .iter()
        .any(|ns| ns.name().is_none() && ns.uri() == "http://www.w3.org/2000/svg");
    if !has_xmlns {
        errors.push(ValidationError {
            error_type: "missing-xmlns".to_string(),
            description: "SVG element is missing the required xmlns attribute".to_string(),
            path: Some("/svg".to_string()),
            part: None,
        });
    }

    if root.attribute("viewBox").is_none() {
        errors.push(ValidationError {
            error_type: "missing-viewBox".to_string(),
            description: "SVG element is missing viewBox attribute".to_string(),
            path: Some("/svg".to_string()),
            part: None,
        });
    }

    find_empty_groups(&root, "/svg", &mut errors);

    let element_count = doc.descendants().filter(|n| n.is_element()).count();
    if element_count > 10000 {
        errors.push(ValidationError {
            error_type: "oversized".to_string(),
            description: format!("SVG has {} elements (max recommended: 10000)", element_count),
            path: None,
            part: None,
        });
    }

    errors
}

fn find_empty_groups(node: &roxmltree::Node, path: &str, errors: &mut Vec<ValidationError>) {
    if !node.is_element() {
        return;
    }
    let tag = node.tag_name().name();
    if tag == "g" {
        let has_element_child = node.children().any(|c| c.is_element());
        if !has_element_child {
            errors.push(ValidationError {
                error_type: "empty-group".to_string(),
                description: "Empty <g> element serves no purpose".to_string(),
                path: Some(path.to_string()),
                part: None,
            });
        }
    }
    let mut tag_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            let count = tag_counts.entry(child_tag.to_string()).or_insert(0);
            *count += 1;
            let child_path = format!("{}/{}[{}]", path, child_tag, count);
            find_empty_groups(&child, &child_path, errors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handler_common::RawOptions;

    fn sample_svg() -> &'static str {
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">
  <rect x="10" y="10" width="80" height="80" fill="red"/>
  <g>
    <circle cx="50" cy="50" r="30" fill="blue"/>
  </g>
  <text x="10" y="50" font-size="16">Hello World</text>
</svg>"#
    }

    #[test]
    fn test_get_raw_full() {
        let result = get_raw(sample_svg(), "/", &RawOptions::default()).unwrap();
        assert!(result.starts_with("<svg"));
        assert!(result.ends_with("</svg>"));
    }

    #[test]
    fn test_get_raw_element() {
        let result = get_raw(sample_svg(), "/svg/rect[1]", &RawOptions::default()).unwrap();
        assert_eq!(result, r#"<rect x="10" y="10" width="80" height="80" fill="red"/>"#);
    }

    #[test]
    fn test_get_raw_group() {
        let result = get_raw(sample_svg(), "/svg/g[1]", &RawOptions::default()).unwrap();
        assert!(result.starts_with("<g>"));
        assert!(result.ends_with("</g>"));
    }

    #[test]
    fn test_set_raw_replace() {
        let result = set_raw(
            sample_svg(),
            "/svg/rect[1]",
            "",
            "replace",
            Some("<rect x='10' y='10' width='50' height='50'/>"),
        )
        .unwrap();
        assert!(result.contains("x='10'"));
        assert!(result.contains("width='50'"));
    }

    #[test]
    fn test_validate_valid() {
        let errors = validate(sample_svg());
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_missing_xmlns() {
        let svg = r#"<svg viewBox="0 0 100 100"><rect x="10" y="10" width="80" height="80"/></svg>"#;
        let errors = validate(svg);
        assert!(errors.iter().any(|e| e.error_type == "missing-xmlns"));
    }

    #[test]
    fn test_validate_missing_viewbox() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect x="10" y="10" width="80" height="80"/></svg>"#;
        let errors = validate(svg);
        assert!(errors.iter().any(|e| e.error_type == "missing-viewBox"));
    }

    #[test]
    fn test_validate_empty_group() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><g></g></svg>"#;
        let errors = validate(svg);
        assert!(errors.iter().any(|e| e.error_type == "empty-group"));
    }

    #[test]
    fn test_validate_malformed_xml() {
        let errors = validate("not xml");
        assert!(errors.iter().any(|e| e.error_type == "xml-well-formed"));
    }
}
