use crate::navigation::OdgNavigator;

pub fn set_attribute(content_xml: &str, path: &str, key: &str, value: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(content_xml)
        .map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = OdgNavigator::resolve(&doc, path)?;
    let node = resolved.node;

    let node_range = node.range();
    let node_xml = &content_xml[node_range.start..node_range.end];
    let tag_end = node_xml.find('>').ok_or("malformed element: no '>'")?;

    let search = format!("{}=\"", key);

    let mut result = String::with_capacity(content_xml.len());
    result.push_str(&content_xml[..node_range.start]);

    if let Some(attr_start) = node_xml[..tag_end].find(&search) {
        let val_start = attr_start + search.len();
        let val_end = node_xml[val_start..]
            .find('"')
            .map(|p| val_start + p)
            .unwrap_or(node_xml.len());
        result.push_str(&node_xml[..attr_start + search.len()]);
        result.push_str(value);
        result.push_str(&node_xml[val_end..]);
    } else {
        let before_close = if node_xml[..tag_end].trim_end().ends_with('/') {
            let trimmed = node_xml[..tag_end].trim_end();
            trimmed.len() - 1
        } else {
            tag_end
        };
        result.push_str(&node_xml[..before_close]);
        result.push_str(&format!(" {}=\"{}\"", key, value));
        result.push_str(&node_xml[before_close..]);
    }

    result.push_str(&content_xml[node_range.end..]);
    Ok(result)
}

pub fn move_element_raw(
    content_xml: &str,
    source_path: &str,
    target_parent_path: &str,
    target_index: Option<usize>,
) -> Result<String, String> {
    let doc = roxmltree::Document::parse(content_xml)
        .map_err(|e| format!("XML parse error: {}", e))?;

    let src = OdgNavigator::resolve(&doc, source_path)?;
    let src_range = src.node.range();
    let src_xml = content_xml[src_range.start..src_range.end].to_string();

    OdgNavigator::resolve(&doc, target_parent_path)?;

    let mut result = String::with_capacity(content_xml.len());
    result.push_str(&content_xml[..src_range.start]);
    result.push_str(&content_xml[src_range.end..]);

    let modified = result;
    let doc2 = roxmltree::Document::parse(&modified).map_err(|e| format!("re-parse error: {}", e))?;
    let parent2 = OdgNavigator::resolve(&doc2, target_parent_path)?;
    let parent2_range = parent2.node.range();
    let children2: Vec<_> = parent2.node.children().filter(|c| c.is_element()).collect();
    let insert_idx = target_index.unwrap_or(children2.len());

    let mut final_result = String::with_capacity(modified.len() + src_xml.len());

    if insert_idx >= children2.len() {
        let closing = format!("</{}>", parent2.node.tag_name().name());
        let close_pos = modified[parent2_range.start..]
            .rfind(&closing)
            .map(|p| parent2_range.start + p)
            .unwrap_or(parent2_range.end - closing.len());
        final_result.push_str(&modified[..close_pos]);
        final_result.push_str(&src_xml);
        final_result.push_str(&modified[close_pos..]);
    } else if insert_idx == 0 {
        let first = children2[0];
        let first_range = first.range();
        final_result.push_str(&modified[..first_range.start]);
        final_result.push_str(&src_xml);
        final_result.push_str(&modified[first_range.start..]);
    } else {
        let before_child = children2[insert_idx - 1];
        let before_end = before_child.range().end;
        final_result.push_str(&modified[..before_end]);
        final_result.push_str(&src_xml);
        final_result.push_str(&modified[before_end..]);
    }

    Ok(final_result)
}

pub fn remove_element(content_xml: &str, path: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(content_xml)
        .map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = OdgNavigator::resolve(&doc, path)?;
    let node = resolved.node;
    let range = node.range();

    let mut result = String::with_capacity(content_xml.len());
    result.push_str(&content_xml[..range.start]);
    result.push_str(&content_xml[range.end..]);
    Ok(result)
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
    fn test_set_attribute_existing() {
        let result = set_attribute(
            sample_xml(),
            "/document-content/body/drawing/page[1]/rect[1]",
            "svg:width",
            "15cm",
        )
        .unwrap();
        assert!(result.contains("svg:width=\"15cm\""));
    }

    #[test]
    fn test_set_attribute_new() {
        let result = set_attribute(
            sample_xml(),
            "/document-content/body/drawing/page[1]/rect[1]",
            "draw:fill",
            "blue",
        )
        .unwrap();
        assert!(result.contains("draw:fill=\"blue\""));
    }

    #[test]
    fn test_remove_element() {
        let result = remove_element(
            sample_xml(),
            "/document-content/body/drawing/page[1]/rect[1]",
        )
        .unwrap();
        assert!(!result.contains("svg:width=\"10cm\""));
    }
}
