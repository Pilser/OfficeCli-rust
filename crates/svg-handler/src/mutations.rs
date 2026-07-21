use crate::navigation::SvgNavigator;

pub fn set_attribute(xml: &str, path: &str, key: &str, value: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = SvgNavigator::resolve(&doc, path)?;
    let node = resolved.node;

    let node_range = node.range();
    let node_xml = &xml[node_range.start..node_range.end];
    let tag_end = node_xml.find('>').ok_or("malformed element: no '>'")?;

    let search = format!("{}=\"", key);

    let mut result = String::with_capacity(xml.len());
    result.push_str(&xml[..node_range.start]);

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

    result.push_str(&xml[node_range.end..]);
    Ok(result)
}

pub fn remove_element(xml: &str, path: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;
    let resolved = SvgNavigator::resolve(&doc, path)?;
    let node = resolved.node;
    let range = node.range();

    let mut result = String::with_capacity(xml.len());
    result.push_str(&xml[..range.start]);
    result.push_str(&xml[range.end..]);
    Ok(result)
}

pub fn move_element(
    xml: &str,
    source_path: &str,
    target_parent_path: &str,
    target_index: Option<usize>,
) -> Result<String, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;

    let src = SvgNavigator::resolve(&doc, source_path)?;
    let src_range = src.node.range();
    let src_xml = xml[src_range.start..src_range.end].to_string();

    let _parent = SvgNavigator::resolve(&doc, target_parent_path)?;

    let mut result = String::with_capacity(xml.len());
    result.push_str(&xml[..src_range.start]);
    result.push_str(&xml[src_range.end..]);

    let modified = result;
    let doc2 = roxmltree::Document::parse(&modified).map_err(|e| format!("re-parse error: {}", e))?;
    let parent2 = SvgNavigator::resolve(&doc2, target_parent_path)?;
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

pub fn swap_elements(xml: &str, path1: &str, path2: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;

    let r1 = SvgNavigator::resolve(&doc, path1)?;
    let r2 = SvgNavigator::resolve(&doc, path2)?;

    let range1 = r1.node.range();
    let range2 = r2.node.range();

    let mut result = String::with_capacity(xml.len());

    if range1.start < range2.start {
        result.push_str(&xml[..range1.start]);
        result.push_str(&xml[range2.start..range2.end]);
        result.push_str(&xml[range1.end..range2.start]);
        result.push_str(&xml[range1.start..range1.end]);
        result.push_str(&xml[range2.end..]);
    } else {
        result.push_str(&xml[..range2.start]);
        result.push_str(&xml[range1.start..range1.end]);
        result.push_str(&xml[range2.end..range1.start]);
        result.push_str(&xml[range2.start..range2.end]);
        result.push_str(&xml[range1.end..]);
    }

    Ok(result)
}
