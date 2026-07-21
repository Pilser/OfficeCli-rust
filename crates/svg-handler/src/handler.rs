use crate::add;
use crate::dom_types::SvgNodeType;
use crate::mutations;
use crate::navigation::SvgNavigator;
use crate::query;
use crate::raw;
use crate::view;
use handler_common::output_format::BinaryInfo;
use handler_common::*;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct SvgDom {
    pub xml_string: String,
}

pub struct SvgHandler {
    dom: RefCell<SvgDom>,
    editable: bool,
    file_path: String,
}

impl SvgHandler {
    pub fn open(path: &str, editable: bool) -> Result<Self, HandlerError> {
        let xml = std::fs::read_to_string(path)
            .map_err(|e| HandlerError::OpenError(e.to_string()))?;
        let _ = usvg::Tree::from_str(&xml, &usvg::Options::default())
            .map_err(|e| HandlerError::OpenError(format!("invalid SVG: {}", e)))?;
        Ok(Self {
            dom: RefCell::new(SvgDom { xml_string: xml }),
            editable,
            file_path: path.to_string(),
        })
    }

    pub fn create(path: &str, _props: Option<&HashMap<String, String>>) -> Result<Self, HandlerError> {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600" width="800" height="600"/>"#.to_string();
        Ok(Self {
            dom: RefCell::new(SvgDom { xml_string: xml }),
            editable: true,
            file_path: path.to_string(),
        })
    }
}

impl DocumentHandler for SvgHandler {
    fn format_name(&self) -> &str {
        "svg"
    }

    fn view_as_text(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_text(&dom.xml_string, &opts)
    }

    fn view_as_annotated(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_annotated(&dom.xml_string, &opts)
    }

    fn view_as_outline(&self) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_outline(&dom.xml_string)
    }

    fn view_as_stats(&self) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_stats(&dom.xml_string)
    }

    fn view_as_issues(
        &self,
        issue_type: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<DocumentIssue>, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_issues(&dom.xml_string, issue_type, limit)
    }

    fn view_as_html(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_html(&dom.xml_string, &opts)
    }

    fn view_as_svg(&self) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        Ok(dom.xml_string.clone())
    }

    fn view_as_text_json(&self, opts: ViewOptions) -> Result<serde_json::Value, HandlerError> {
        let text = self.view_as_text(opts)?;
        Ok(serde_json::json!({
            "format": "svg",
            "text": text,
        }))
    }

    fn view_as_outline_json(&self) -> Result<serde_json::Value, HandlerError> {
        let outline = self.view_as_outline()?;
        Ok(serde_json::json!({
            "format": "svg",
            "outline": outline,
        }))
    }

    fn view_as_stats_json(&self) -> Result<serde_json::Value, HandlerError> {
        let stats = self.view_as_stats()?;
        Ok(serde_json::json!({
            "format": "svg",
            "stats": stats,
        }))
    }

    fn get(&self, path: &str, depth: usize) -> Result<DocumentNode, HandlerError> {
        let dom = self.dom.borrow();
        let doc = roxmltree::Document::parse(&dom.xml_string)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

        if path == "/" || path == "/svg" {
            let root = doc.root_element();
            let mut node = DocumentNode::new("/svg", "svg");
            if let Some(vb) = root.attribute("viewBox") {
                node = node.with_format("viewBox", serde_json::json!(vb));
            }
            if let Some(w) = root.attribute("width") {
                node = node.with_format("width", serde_json::json!(w));
            }
            if let Some(h) = root.attribute("height") {
                node = node.with_format("height", serde_json::json!(h));
            }

            if depth > 0 {
                let children = build_children(&root, depth - 1);
                node = node.with_children(children);
            }
            return Ok(node);
        }

        let resolved = SvgNavigator::resolve(&doc, path)
            .map_err(|e| HandlerError::PathNotFound(format!("{}: {}", path, e)))?;

        build_document_node(&resolved, depth)
    }

    fn query(&self, selector: &str) -> Result<Vec<DocumentNode>, HandlerError> {
        let dom = self.dom.borrow();
        let parsed = Selector::parse(selector)
            .map_err(|e| HandlerError::InvalidArgument(e.to_string()))?;

        if let Some(element_type) = &parsed.element_type {
            let results = query::query_by_type(&dom.xml_string, element_type)
                .map_err(|e| HandlerError::OperationFailed(e))?;

            let mut filtered = results;
            for (attr_key, attr_val) in &parsed.attributes {
                filtered.retain(|node| {
                    node.format.get(attr_key).and_then(|v| v.as_ref()).map_or(false, |v| {
                        v.as_str() == Some(attr_val.as_str())
                    })
                });
            }

            return Ok(filtered);
        }

        Ok(Vec::new())
    }

    fn set(
        &self,
        path: &str,
        properties: &HashMap<String, String>,
    ) -> Result<Vec<String>, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "SVG opened in read-only mode".to_string(),
            ));
        }

        let mut dom = self.dom.borrow_mut();
        let mut new_xml = dom.xml_string.clone();

        for (key, value) in properties {
            new_xml = mutations::set_attribute(&new_xml, path, key, value)
                .map_err(|e| HandlerError::OperationFailed(e))?;
        }

        dom.xml_string = new_xml;
        Ok(Vec::new())
    }

    fn add(
        &self,
        parent: &str,
        element_type: &str,
        position: InsertPosition,
        properties: &HashMap<String, String>,
        wrap: Option<&str>,
    ) -> Result<String, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "SVG opened in read-only mode".to_string(),
            ));
        }

        let dom = self.dom.borrow();
        let xml = dom.xml_string.clone();

        let insert_index = resolve_insert_index(&xml, parent, &position)?;
        let (new_path, result_xml) = add::add_element(
            &xml, parent, element_type, insert_index, properties, wrap,
        )
        .map_err(|e| HandlerError::OperationFailed(e))?;

        drop(dom);
        let mut dom = self.dom.borrow_mut();
        dom.xml_string = result_xml;

        Ok(new_path)
    }

    fn remove(&self, path: &str) -> Result<Option<String>, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "SVG opened in read-only mode".to_string(),
            ));
        }

        let mut dom = self.dom.borrow_mut();
        let new_xml = mutations::remove_element(&dom.xml_string, path)
            .map_err(|e| HandlerError::OperationFailed(e))?;
        dom.xml_string = new_xml;
        Ok(Some(format!("removed {}", path)))
    }

    fn move_element(
        &self,
        source: &str,
        target_parent: Option<&str>,
        position: InsertPosition,
    ) -> Result<String, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "SVG opened in read-only mode".to_string(),
            ));
        }

        let dom = self.dom.borrow();
        let xml = dom.xml_string.clone();
        drop(dom);

        let parent_path = match target_parent {
            Some(p) => p.to_string(),
            None => {
                let doc = roxmltree::Document::parse(&xml)
                    .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
                let resolved = SvgNavigator::resolve(&doc, source)
                    .map_err(|e| HandlerError::PathNotFound(e))?;
                let parent = resolved.node.parent().and_then(|p| {
                    if p.is_element() { Some(p) } else { None }
                }).ok_or_else(|| HandlerError::OperationFailed("cannot determine parent".to_string()))?;
                let parent_path = resolve_tag_path_for_node(&parent);
                parent_path
            }
        };

        let insert_index = resolve_insert_index(&xml, &parent_path, &position)?;

        let new_xml = mutations::move_element(&xml, source, &parent_path, insert_index)
            .map_err(|e| HandlerError::OperationFailed(e))?;

        let mut dom = self.dom.borrow_mut();
        dom.xml_string = new_xml;

        Ok(format!("moved {} to {}", source, parent_path))
    }

    fn copy_from(
        &self,
        source: &str,
        target_parent: &str,
        position: InsertPosition,
    ) -> Result<String, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "SVG opened in read-only mode".to_string(),
            ));
        }

        let dom = self.dom.borrow();
        let xml = dom.xml_string.clone();

        let doc = roxmltree::Document::parse(&xml)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;
        let src_resolved = SvgNavigator::resolve(&doc, source)
            .map_err(|e| HandlerError::PathNotFound(e))?;

        let element_type = src_resolved.element_type.clone();
        let insert_index = resolve_insert_index(&xml, target_parent, &position)?;

        let mut props = HashMap::new();
        for attr in src_resolved.node.attributes() {
            props.insert(attr.name().to_string(), attr.value().to_string());
        }
        if element_type == "text" || element_type == "tspan" {
            if let Some(text) = src_resolved.node.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    props.insert("text".to_string(), trimmed.to_string());
                }
            }
        }

        let (new_path, result_xml) =
            add::add_element(&xml, target_parent, &element_type, insert_index, &props, None)
                .map_err(|e| HandlerError::OperationFailed(e))?;

        drop(dom);
        let mut dom = self.dom.borrow_mut();
        dom.xml_string = result_xml;
        Ok(new_path)
    }

    fn swap(&self, path1: &str, path2: &str) -> Result<(String, String), HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "SVG opened in read-only mode".to_string(),
            ));
        }

        let mut dom = self.dom.borrow_mut();
        let new_xml = mutations::swap_elements(&dom.xml_string, path1, path2)
            .map_err(|e| HandlerError::OperationFailed(e))?;

        let doc = roxmltree::Document::parse(&new_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("re-parse error: {}", e)))?;
        let r1 = SvgNavigator::resolve(&doc, path1)
            .map_err(|e| HandlerError::PathNotFound(format!("{} after swap: {}", path1, e)))?;
        let r2 = SvgNavigator::resolve(&doc, path2)
            .map_err(|e| HandlerError::PathNotFound(format!("{} after swap: {}", path2, e)))?;
        let p1 = r1.tag_path.clone();
        let p2 = r2.tag_path.clone();

        dom.xml_string = new_xml;
        Ok((p1, p2))
    }

    fn raw(&self, part_path: &str, opts: RawOptions) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        raw::get_raw(&dom.xml_string, part_path, &opts)
            .map_err(|e| HandlerError::OperationFailed(e))
    }

    fn raw_set(
        &self,
        part_path: &str,
        xpath: &str,
        action: &str,
        content: Option<&str>,
    ) -> Result<(), HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "SVG opened in read-only mode".to_string(),
            ));
        }
        let dom = self.dom.borrow_mut();
        let new_xml = raw::set_raw(&dom.xml_string, part_path, xpath, action, content)
            .map_err(|e| HandlerError::OperationFailed(e))?;
        drop(dom);
        let mut dom = self.dom.borrow_mut();
        dom.xml_string = new_xml;
        Ok(())
    }

    fn add_part(
        &self,
        _parent: &str,
        _part_type: &str,
        _properties: Option<&HashMap<String, String>>,
    ) -> Result<(String, String), HandlerError> {
        Err(HandlerError::UnsupportedMode("add_part".to_string()))
    }

    fn validate(&self) -> Result<Vec<ValidationError>, HandlerError> {
        let dom = self.dom.borrow();
        Ok(raw::validate(&dom.xml_string))
    }

    fn try_extract_binary(
        &self,
        path: &str,
        dest: &str,
    ) -> Result<Option<BinaryInfo>, HandlerError> {
        let dom = self.dom.borrow();
        let doc = roxmltree::Document::parse(&dom.xml_string)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

        let resolved = SvgNavigator::resolve(&doc, path)
            .map_err(|e| HandlerError::PathNotFound(e))?;
        let node = resolved.node;

        let href = node
            .attribute("href")
            .or_else(|| node.attribute("xlink:href"))
            .ok_or_else(|| {
                HandlerError::OperationFailed("no href attribute on element".to_string())
            })?;

        if let Some(data_uri) = href.strip_prefix("data:") {
            let (meta_part, encoded) = data_uri
                .split_once(',')
                .ok_or_else(|| HandlerError::OperationFailed("invalid data URI".to_string()))?;
            let content_type = meta_part
                .split(';')
                .next()
                .unwrap_or("application/octet-stream")
                .to_string();
            let is_base64 = meta_part.contains(";base64");

            let decoded = if is_base64 {
                base64_decode(encoded)
                    .map_err(|e| HandlerError::OperationFailed(format!("base64 decode error: {}", e)))?
            } else {
                encoded.as_bytes().to_vec()
            };

            let byte_count = decoded.len();
            std::fs::write(dest, &decoded)?;
            Ok(Some(BinaryInfo {
                content_type,
                byte_count,
            }))
        } else {
            let src_path = if href.starts_with('/') {
                href.to_string()
            } else {
                let base = std::path::Path::new(&self.file_path).parent().unwrap_or(std::path::Path::new("."));
                base.join(href).to_string_lossy().to_string()
            };

            let data = std::fs::read(&src_path)?;
            let byte_count = data.len();
            std::fs::write(dest, &data)?;

            let ext = std::path::Path::new(href)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin");
            let content_type = match ext {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            };

            Ok(Some(BinaryInfo {
                content_type: content_type.to_string(),
                byte_count,
            }))
        }
    }

    fn save(&self) -> Result<(), HandlerError> {
        let xml = self.dom.borrow().xml_string.clone();
        std::fs::write(&self.file_path, &xml)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
        Ok(())
    }

    fn extract_text_with_offsets(&self) -> Result<TextOffsetMap, HandlerError> {
        let dom = self.dom.borrow();
        let doc = roxmltree::Document::parse(&dom.xml_string)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

        let mut map = TextOffsetMap::empty("svg");

        let root = doc.root_element();
        collect_text_offsets(&root, "/svg", &mut map);

        Ok(map)
    }
}

fn collect_text_offsets(node: &roxmltree::Node, path: &str, map: &mut TextOffsetMap) {
    if !node.is_element() {
        return;
    }
    let tag = node.tag_name().name();

    if tag == "text" || tag == "tspan" {
        if let Some(text) = node.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let node_type = SvgNodeType::from_tag(tag);
                map.push_span(trimmed, path, node_type.as_str());
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
            collect_text_offsets(&child, &child_path, map);
        }
    }
}

fn build_document_node(resolved: &crate::navigation::ResolvedNode, depth: usize) -> Result<DocumentNode, HandlerError> {
    let mut node = DocumentNode::new(&resolved.tag_path, &resolved.element_type);

    if let Some(text) = resolved.node.text() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            node = node.with_text(trimmed);
        }
    }

    for attr in [
        "fill", "stroke", "stroke-width", "opacity", "x", "y", "width", "height",
        "cx", "cy", "r", "rx", "ry", "font-size", "font-family", "id", "class",
        "transform", "d", "viewBox",
    ] {
        if let Some(val) = resolved.node.attribute(attr) {
            node = node.with_format(attr, serde_json::json!(val));
        }
    }

    if depth > 0 {
        let children = build_children(&resolved.node, depth - 1);
        node = node.with_children(children);
    }

    Ok(node)
}

fn build_children(parent: &roxmltree::Node, depth: usize) -> Vec<DocumentNode> {
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut children = Vec::new();

    for child in parent.children() {
        if !child.is_element() {
            continue;
        }
        let tag = child.tag_name().name();
        let count = tag_counts.entry(tag.to_string()).or_insert(0);
        *count += 1;
        let child_path = format!(
            "{}/{}[{}]",
            parent_path_string(parent),
            tag,
            count
        );

        let mut child_node = DocumentNode::new(&child_path, tag);
        if let Some(text) = child.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                child_node = child_node.with_text(trimmed);
            }
        }

        if let Some(id) = child.attribute("id") {
            child_node = child_node.with_format("id", serde_json::json!(id));
        }

        if depth > 0 {
            let grandchildren = build_children(&child, depth - 1);
            child_node = child_node.with_children(grandchildren);
        }

        children.push(child_node);
    }

    children
}

fn parent_path_string(node: &roxmltree::Node) -> String {
    if let Some(parent) = node.parent() {
        if parent.is_element() {
            let tag = parent.tag_name().name();
            let idx = compute_child_index(&parent);
            let parent_path = parent_path_string(&parent);
            return format!("{}/{}[{}]", parent_path, tag, idx);
        }
    }
    String::new()
}

fn compute_child_index(node: &roxmltree::Node) -> usize {
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

fn resolve_insert_index(xml: &str, parent_path: &str, position: &InsertPosition) -> Result<Option<usize>, HandlerError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;
    let resolved = SvgNavigator::resolve(&doc, parent_path)
        .map_err(|e| HandlerError::PathNotFound(e))?;
    let child_count = resolved.node.children().filter(|c| c.is_element()).count();

    let anchor_finder = |anchor: &str| -> usize {
        match SvgNavigator::resolve(&doc, anchor) {
            Ok(r) => {
                let mut global_idx = 0usize;
                if let Some(parent) = r.node.parent() {
                    for child in parent.children() {
                        if child == r.node {
                            break;
                        }
                        if child.is_element() {
                            global_idx += 1;
                        }
                    }
                }
                global_idx
            }
            Err(_) => 0,
        }
    };

    Ok(position.resolve(&anchor_finder, child_count))
}

fn resolve_tag_path_for_node(node: &roxmltree::Node) -> String {
    if !node.is_element() {
        return String::new();
    }
    let tag = node.tag_name().name();
    let idx = compute_child_index(node);
    let parent_path = if let Some(parent) = node.parent() {
        if parent.is_element() {
            resolve_tag_path_for_node(&parent)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    if parent_path.is_empty() {
        format!("/{}[{}]", tag, idx)
    } else {
        format!("{}/{}[{}]", parent_path, tag, idx)
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let len = clean.len();
    if len == 0 {
        return Ok(Vec::new());
    }
    let padding = 4 - (len % 4) % 4;
    let padded = if padding != 4 {
        format!("{}{}", clean, "=".repeat(padding))
    } else {
        clean
    };

    fn decode_char(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("invalid base64 character: {}", c as char)),
        }
    }

    let bytes = padded.as_bytes();
    let mut result = Vec::with_capacity(len / 4 * 3);
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = decode_char(bytes[i])?;
        let b = decode_char(bytes[i + 1])?;
        let c = decode_char(bytes[i + 2])?;
        let d = decode_char(bytes[i + 3])?;
        result.push((a << 2) | (b >> 4));
        result.push((b << 4) | (c >> 2));
        result.push((c << 6) | d);
        i += 4;
    }
    if i + 2 < bytes.len() && bytes[i + 2] != b'=' {
        let a = decode_char(bytes[i])?;
        let b = decode_char(bytes[i + 1])?;
        let c = decode_char(bytes[i + 2])?;
        result.push((a << 2) | (b >> 4));
        result.push((b << 4) | (c >> 2));
    } else if i + 1 < bytes.len() && bytes[i + 1] != b'=' {
        let a = decode_char(bytes[i])?;
        let b = decode_char(bytes[i + 1])?;
        result.push((a << 2) | (b >> 4));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_svg() -> &'static str {
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">
  <rect x="10" y="10" width="80" height="80" fill="red"/>
  <text x="10" y="50" font-size="16">Hello World</text>
</svg>"#
    }

    #[test]
    fn test_open_and_view_text() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let text = handler.view_as_text(ViewOptions::default()).unwrap();
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_view_outline() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let outline = handler.view_as_outline().unwrap();
        assert!(outline.contains("/svg"));
        assert!(outline.contains("rect"));
        assert!(outline.contains("text"));
    }

    #[test]
    fn test_view_stats() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let stats = handler.view_as_stats().unwrap();
        assert!(stats.contains("Elements"));
    }

    #[test]
    fn test_view_svg() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let svg = handler.view_as_svg().unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_format_name() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: String::new() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        assert_eq!(handler.format_name(), "svg");
    }

    #[test]
    fn test_extract_text_with_offsets() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let offset_map = handler.extract_text_with_offsets().unwrap();
        assert!(offset_map.full_text.contains("Hello World"));
        assert!(!offset_map.spans.is_empty());
    }

    #[test]
    fn test_get_root() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let node = handler.get("/svg", 0).unwrap();
        assert_eq!(node.path, "/svg");
        assert_eq!(node.element_type, "svg");
    }

    #[test]
    fn test_get_element() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let node = handler.get("/svg/rect[1]", 0).unwrap();
        assert_eq!(node.element_type, "rect");
        assert!(node.format.contains_key("fill"));
    }

    #[test]
    fn test_set_attribute() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: true,
            file_path: "/tmp/test.svg".to_string(),
        };
        let mut props = HashMap::new();
        props.insert("fill".to_string(), "blue".to_string());
        let result = handler.set("/svg/rect[1]", &props).unwrap();
        assert!(result.is_empty());
        let updated = handler.dom.borrow().xml_string.clone();
        assert!(updated.contains("fill=\"blue\""));
    }

    #[test]
    fn test_add_rect() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: true,
            file_path: "/tmp/test.svg".to_string(),
        };
        let mut props = HashMap::new();
        props.insert("x".to_string(), "20".to_string());
        props.insert("y".to_string(), "20".to_string());
        props.insert("width".to_string(), "50".to_string());
        props.insert("height".to_string(), "50".to_string());
        props.insert("fill".to_string(), "green".to_string());
        let path = handler
            .add("/svg", "rect", InsertPosition::Append, &props, None)
            .unwrap();
        assert!(path.starts_with("/svg/rect"));
    }

    #[test]
    fn test_remove_element() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: true,
            file_path: "/tmp/test.svg".to_string(),
        };
        let result = handler.remove("/svg/rect[1]").unwrap();
        assert!(result.is_some());
        let updated = handler.dom.borrow().xml_string.clone();
        assert!(!updated.contains("fill=\"red\""));
    }

    #[test]
    fn test_query_by_type() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let results = handler.query("rect").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_type, "rect");
    }

    #[test]
    fn test_editable_check() {
        let handler = SvgHandler {
            dom: RefCell::new(SvgDom { xml_string: sample_svg().to_string() }),
            editable: false,
            file_path: "/tmp/test.svg".to_string(),
        };
        let mut props = HashMap::new();
        props.insert("fill".to_string(), "blue".to_string());
        assert!(handler.set("/svg/rect[1]", &props).is_err());
    }
}
