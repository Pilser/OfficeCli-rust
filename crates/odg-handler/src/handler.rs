use crate::add;
use crate::mutations;
use crate::navigation::{get_attr, normalize_tag_to_type, OdgNavigator};
use crate::query;
use crate::raw;
use crate::text_offset;
use crate::view;
use handler_common::output_format::BinaryInfo;
use handler_common::*;
use lo_core::draw::Drawing;
use lo_zip::archive::ZipArchive;
use lo_zip::ZipEntry;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct OdgDom {
    pub drawing: Drawing,
}

pub struct OdgHandler {
    dom: RefCell<OdgDom>,
    editable: bool,
    file_path: String,
}

impl OdgHandler {
    pub fn open(path: &str, editable: bool) -> Result<Self, HandlerError> {
        let bytes = std::fs::read(path)
            .map_err(|e| HandlerError::OpenError(e.to_string()))?;
        let title = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        let drawing = lo_draw::import::from_odg_bytes(title, &bytes)
            .map_err(|e| HandlerError::OpenError(e.to_string()))?;
        Ok(Self {
            dom: RefCell::new(OdgDom { drawing }),
            editable,
            file_path: path.to_string(),
        })
    }

    pub fn create(path: &str, _props: Option<&HashMap<String, String>>) -> Result<Self, HandlerError> {
        let mut drawing = Drawing::new("Untitled");
        drawing.pages.clear();
        drawing.pages.push(lo_core::draw::DrawPage {
            name: "page1".to_string(),
            elements: Vec::new(),
        });
        Ok(Self {
            dom: RefCell::new(OdgDom { drawing }),
            editable: true,
            file_path: path.to_string(),
        })
    }

    fn export_content_xml(&self) -> Result<String, HandlerError> {
        let drawing = self.dom.borrow();
        let odg_bytes = lo_draw::save_as(&drawing.drawing, "odg")
            .map_err(|e| HandlerError::OperationFailed(format!("export failed: {}", e)))?;
        let zip = ZipArchive::new(&odg_bytes)
            .map_err(|e| HandlerError::OperationFailed(format!("ZIP error: {}", e)))?;
        zip.read_string("content.xml")
            .map_err(|e| HandlerError::OperationFailed(format!("read content.xml: {}", e)))
    }

    fn reload_from_content_xml(&self, content_xml: &str) -> Result<(), HandlerError> {
        let odg_bytes = lo_draw::save_as(&self.dom.borrow().drawing, "odg")
            .map_err(|e| HandlerError::OperationFailed(format!("export failed: {}", e)))?;
        let zip = ZipArchive::new(&odg_bytes)
            .map_err(|e| HandlerError::OperationFailed(format!("ZIP error: {}", e)))?;

        let mut entries: Vec<ZipEntry> = zip
            .entries()
            .into_iter()
            .map(|name| {
                let data = zip.read(name).unwrap_or_default();
                if name == "content.xml" {
                    ZipEntry::new(name, content_xml.as_bytes().to_vec())
                } else {
                    ZipEntry::new(name, data)
                }
            })
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let new_odg = lo_zip::write_zip_to_vec(&entries)
            .map_err(|e| HandlerError::OperationFailed(format!("write ZIP: {}", e)))?;

        let title = std::path::Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        let new_drawing = lo_draw::import::from_odg_bytes(title, &new_odg)
            .map_err(|e| HandlerError::OperationFailed(format!("reload: {}", e)))?;
        self.dom.replace(OdgDom { drawing: new_drawing });
        Ok(())
    }

}

impl DocumentHandler for OdgHandler {
    fn format_name(&self) -> &str {
        "odg"
    }

    fn view_as_text(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_text(&dom.drawing, &opts)
    }

    fn view_as_annotated(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_annotated(&dom.drawing, &opts)
    }

    fn view_as_outline(&self) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_outline(&dom.drawing)
    }

    fn view_as_stats(&self) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_stats(&dom.drawing)
    }

    fn view_as_issues(
        &self,
        issue_type: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<DocumentIssue>, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_issues(&dom.drawing, issue_type, limit)
    }

    fn view_as_html(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_html(&dom.drawing, &opts)
    }

    fn view_as_svg(&self) -> Result<String, HandlerError> {
        let dom = self.dom.borrow();
        view::view_as_svg(&dom.drawing)
    }

    fn view_as_text_json(&self, opts: ViewOptions) -> Result<serde_json::Value, HandlerError> {
        let text = self.view_as_text(opts)?;
        Ok(serde_json::json!({
            "format": "odg",
            "text": text,
            "pageCount": self.dom.borrow().drawing.pages.len(),
        }))
    }

    fn view_as_outline_json(&self) -> Result<serde_json::Value, HandlerError> {
        let outline = self.view_as_outline()?;
        Ok(serde_json::json!({
            "format": "odg",
            "outline": outline,
            "pageCount": self.dom.borrow().drawing.pages.len(),
        }))
    }

    fn view_as_stats_json(&self) -> Result<serde_json::Value, HandlerError> {
        let stats = self.view_as_stats()?;
        Ok(serde_json::json!({
            "format": "odg",
            "stats": stats,
            "pageCount": self.dom.borrow().drawing.pages.len(),
        }))
    }

    fn get(&self, path: &str, depth: usize) -> Result<DocumentNode, HandlerError> {
        let content_xml = self.export_content_xml()?;
        let doc = roxmltree::Document::parse(&content_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

        let resolved = OdgNavigator::resolve(&doc, path)
            .map_err(|e| HandlerError::PathNotFound(format!("{}: {}", path, e)))?;

        build_document_node(&doc, &resolved, depth, &content_xml)
    }

    fn query(&self, selector: &str) -> Result<Vec<DocumentNode>, HandlerError> {
        let parsed = Selector::parse(selector)
            .map_err(|e| HandlerError::InvalidArgument(e.to_string()))?;

        let content_xml = self.export_content_xml()?;

        if let Some(element_type) = &parsed.element_type {
            let results = query::query_by_type(&content_xml, element_type);

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
                "ODG opened in read-only mode".to_string(),
            ));
        }

        let content_xml = self.export_content_xml()?;
        let mut new_xml = content_xml;

        for (key, value) in properties {
            new_xml = mutations::set_attribute(&new_xml, path, key, value)
                .map_err(|e| HandlerError::OperationFailed(e))?;
        }

        self.reload_from_content_xml(&new_xml)?;
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
                "ODG opened in read-only mode".to_string(),
            ));
        }

        let content_xml = self.export_content_xml()?;
        let doc = roxmltree::Document::parse(&content_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

        let resolved = OdgNavigator::resolve(&doc, parent)
            .map_err(|e| HandlerError::PathNotFound(e))?;
        let child_count = resolved.node.children().filter(|c| c.is_element()).count();

        let anchor_finder = |_anchor: &str| -> usize {
            0
        };
        let insert_index = position.resolve(&anchor_finder, child_count);

        let (new_path, new_xml) = add::add_element(
            &content_xml, parent, element_type, insert_index, properties, wrap,
        )
        .map_err(|e| HandlerError::OperationFailed(e))?;

        let doc2 = roxmltree::Document::parse(&new_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("re-parse: {}", e)))?;
        let real_path = OdgNavigator::resolve(
            &doc2,
            &new_xml_into_path(&new_xml, parent, element_type, insert_index)
                .map_err(|e| HandlerError::OperationFailed(e))?,
        )
        .map(|r| r.tag_path)
        .unwrap_or(new_path.clone());

        self.reload_from_content_xml(&new_xml)?;
        Ok(real_path)
    }

    fn remove(&self, path: &str) -> Result<Option<String>, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "ODG opened in read-only mode".to_string(),
            ));
        }

        let content_xml = self.export_content_xml()?;
        let new_xml = mutations::remove_element(&content_xml, path)
            .map_err(|e| HandlerError::OperationFailed(e))?;
        self.reload_from_content_xml(&new_xml)?;
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
                "ODG opened in read-only mode".to_string(),
            ));
        }

        let content_xml = self.export_content_xml()?;
        let doc = roxmltree::Document::parse(&content_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

        let parent_path = match target_parent {
            Some(p) => p.to_string(),
            None => {
                let src_resolved = OdgNavigator::resolve(&doc, source)
                    .map_err(|e| HandlerError::PathNotFound(e))?;
                let parent = src_resolved.node.parent().and_then(|p| {
                    if p.is_element() { Some(p) } else { None }
                }).ok_or_else(|| HandlerError::OperationFailed("cannot determine parent".to_string()))?;
                resolve_tag_path_for_node(&parent)
            }
        };

        let doc_for_index = roxmltree::Document::parse(&content_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;
        let parent_resolved = OdgNavigator::resolve(&doc_for_index, &parent_path)
            .map_err(|e| HandlerError::PathNotFound(e))?;
        let child_count = parent_resolved.node.children().filter(|c| c.is_element()).count();
        let anchor_finder = |_anchor: &str| -> usize { 0 };
        let insert_index = position.resolve(&anchor_finder, child_count);

        let new_xml = crate::mutations::move_element_raw(
            &content_xml, source, &parent_path, insert_index,
        ).map_err(|e| HandlerError::OperationFailed(e))?;

        self.reload_from_content_xml(&new_xml)?;
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
                "ODG opened in read-only mode".to_string(),
            ));
        }

        let content_xml = self.export_content_xml()?;
        let doc = roxmltree::Document::parse(&content_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("XML parse error: {}", e)))?;

        let src_resolved = OdgNavigator::resolve(&doc, source)
            .map_err(|e| HandlerError::PathNotFound(e))?;

        let element_type = src_resolved.element_type.clone();

        let parent_resolved = OdgNavigator::resolve(&doc, target_parent)
            .map_err(|e| HandlerError::PathNotFound(e))?;
        let child_count = parent_resolved.node.children().filter(|c| c.is_element()).count();
        let anchor_finder = |_anchor: &str| -> usize { 0 };
        let insert_index = position.resolve(&anchor_finder, child_count);

        let mut props = HashMap::new();
        for attr in src_resolved.node.attributes() {
            props.insert(attr.name().to_string(), attr.value().to_string());
        }

        let display_type = normalize_tag_to_type(&element_type);

        let (new_path, new_xml) = add::add_element(
            &content_xml, target_parent, display_type, insert_index, &props, None,
        )
        .map_err(|e| HandlerError::OperationFailed(e))?;

        self.reload_from_content_xml(&new_xml)?;
        Ok(new_path)
    }

    fn raw(&self, part_path: &str, _opts: RawOptions) -> Result<String, HandlerError> {
        let odg_bytes = lo_draw::save_as(&self.dom.borrow().drawing, "odg")
            .map_err(|e| HandlerError::OperationFailed(format!("export failed: {}", e)))?;

        if part_path == "/" || part_path == "" || part_path == "content.xml" {
            let zip = ZipArchive::new(&odg_bytes)
                .map_err(|e| HandlerError::OperationFailed(format!("ZIP error: {}", e)))?;
            return zip.read_string("content.xml")
                .map_err(|e| HandlerError::OperationFailed(format!("read content.xml: {}", e)));
        }

        raw::get_raw_xml(&odg_bytes, part_path)
            .map_err(|e| HandlerError::OperationFailed(e))
    }

    fn raw_set(
        &self,
        part_path: &str,
        _xpath: &str,
        action: &str,
        content: Option<&str>,
    ) -> Result<(), HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "ODG opened in read-only mode".to_string(),
            ));
        }

        if action != "replace" {
            return Err(HandlerError::InvalidArgument(
                format!("unsupported raw_set action: {}", action),
            ));
        }

        let content = content.ok_or_else(|| HandlerError::InvalidArgument("content required".to_string()))?;

        let entry_path = match part_path {
            "/" | "" | "content.xml" => "content.xml",
            p => p.trim_start_matches('/'),
        };

        let odg_bytes = lo_draw::save_as(&self.dom.borrow().drawing, "odg")
            .map_err(|e| HandlerError::OperationFailed(format!("export failed: {}", e)))?;
        let zip = ZipArchive::new(&odg_bytes)
            .map_err(|e| HandlerError::OperationFailed(format!("ZIP error: {}", e)))?;

        let mut entries: Vec<ZipEntry> = zip
            .entries()
            .into_iter()
            .map(|name| {
                let data = zip.read(name).unwrap_or_default();
                ZipEntry::new(name, data)
            })
            .collect();

        if let Some(entry) = entries.iter_mut().find(|e| e.name == entry_path) {
            entry.data = content.as_bytes().to_vec();
        } else {
            entries.push(ZipEntry::new(entry_path, content.as_bytes().to_vec()));
        }

        let new_odg = lo_zip::write_zip_to_vec(&entries)
            .map_err(|e| HandlerError::OperationFailed(format!("write ZIP: {}", e)))?;

        if entry_path == "content.xml" {
            let title = std::path::Path::new(&self.file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled");
            let new_drawing = lo_draw::import::from_odg_bytes(title, &new_odg)
                .map_err(|e| HandlerError::OperationFailed(format!("reload: {}", e)))?;
            self.dom.replace(OdgDom { drawing: new_drawing });
        }

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
        let content_xml = self.export_content_xml()?;
        Ok(raw::validate(&content_xml))
    }

    fn try_extract_binary(
        &self,
        _path: &str,
        _dest: &str,
    ) -> Result<Option<BinaryInfo>, HandlerError> {
        Err(HandlerError::UnsupportedMode("try_extract_binary".to_string()))
    }

    fn save(&self) -> Result<(), HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError(
                "ODG opened in read-only mode".to_string(),
            ));
        }
        let drawing = &self.dom.borrow().drawing;
        lo_draw::save_odg(&self.file_path, drawing)
            .map_err(|e| HandlerError::SaveError(e.to_string()))
    }

    fn extract_text_with_offsets(&self) -> Result<TextOffsetMap, HandlerError> {
        let content_xml = self.export_content_xml()?;
        Ok(text_offset::extract_text_with_offsets(&content_xml))
    }
}

fn build_document_node(
    doc: &roxmltree::Document,
    resolved: &crate::navigation::ResolvedNode,
    depth: usize,
    _xml: &str,
) -> Result<DocumentNode, HandlerError> {
    let display_type = normalize_tag_to_type(&resolved.element_type);
    let mut node = DocumentNode::new(&resolved.tag_path, display_type);

    if let Some(text) = resolved.node.text() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            node = node.with_text(trimmed);
        }
    }

    for attr in [
        "svg:x", "svg:y", "svg:width", "svg:height", "svg:cx", "svg:cy", "svg:r",
        "svg:rx", "svg:ry", "svg:x1", "svg:y1", "svg:x2", "svg:y2", "svg:d",
        "svg:stroke-width", "draw:fill", "draw:stroke", "draw:opacity",
        "draw:style-name", "draw:name", "draw:transform",
        "fo:font-size", "fo:font-family", "fo:color",
    ] {
        if let Some(val) = get_attr(&resolved.node, attr) {
            node = node.with_format(attr, serde_json::json!(val));
        }
    }

    if depth > 0 {
        let children = build_children(doc, &resolved.node, depth - 1);
        node = node.with_children(children);
    }

    Ok(node)
}

fn build_children(
    _doc: &roxmltree::Document,
    parent: &roxmltree::Node,
    depth: usize,
) -> Vec<DocumentNode> {
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut children = Vec::new();

    for child in parent.children() {
        if !child.is_element() {
            continue;
        }
        let tag = child.tag_name().name();
        let count = tag_counts.entry(tag.to_string()).or_insert(0);
        *count += 1;

        let parent_path = parent_path_string(parent);
        let child_path = format!("{}/{}[{}]", parent_path, tag, count);

        let display_type = normalize_tag_to_type(tag);
        let mut child_node = DocumentNode::new(&child_path, display_type);

        if let Some(text) = child.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                child_node = child_node.with_text(trimmed);
            }
        }

        if let Some(id) = get_attr(&child, "draw:name") {
            child_node = child_node.with_format("draw:name", serde_json::json!(id));
        }

        if depth > 0 {
            let grandchildren = build_children(_doc, &child, depth - 1);
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
            let idx = compute_sibling_index(&parent);
            let parent_path = parent_path_string(&parent);
            return format!("{}/{}[{}]", parent_path, tag, idx);
        }
    }
    String::new()
}

fn compute_sibling_index(node: &roxmltree::Node) -> usize {
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

fn resolve_tag_path_for_node(node: &roxmltree::Node) -> String {
    if !node.is_element() {
        return String::new();
    }
    let tag = node.tag_name().name();
    let idx = compute_sibling_index(node);
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

fn new_xml_into_path(
    _new_xml: &str,
    parent_path: &str,
    element_type: &str,
    insert_index: Option<usize>,
) -> Result<String, String> {
    let tag = match element_type {
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
    let idx = insert_index.unwrap_or(1) + 1;
    Ok(format!("{}/{}[{}]", parent_path, tag, idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_drawing() -> Drawing {
        use lo_core::draw::{DrawElement, DrawPage};
        use lo_core::geometry::Rect;
        use lo_core::impress::{Shape, ShapeKind, TextBox};
        use lo_core::style::{ShapeStyle, TextBoxStyle};
        use lo_core::units::Length;

        let mut drawing = Drawing::new("test");
        let page = DrawPage {
            name: "page1".to_string(),
            elements: vec![
                DrawElement::TextBox(TextBox {
                    frame: Rect::new(
                        Length::mm(10.0),
                        Length::mm(10.0),
                        Length::mm(80.0),
                        Length::mm(30.0),
                    ),
                    text: "Hello ODG".to_string(),
                    style: TextBoxStyle::default(),
                }),
                DrawElement::Shape(Shape {
                    frame: Rect::new(
                        Length::mm(50.0),
                        Length::mm(50.0),
                        Length::mm(100.0),
                        Length::mm(80.0),
                    ),
                    style: ShapeStyle::default(),
                    kind: ShapeKind::Rectangle,
                }),
            ],
        };
        drawing.pages.push(page);
        drawing
    }

    #[test]
    fn test_format_name() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: Drawing::new("test") }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        assert_eq!(handler.format_name(), "odg");
    }

    #[test]
    fn test_view_as_text() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: sample_drawing() }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let text = handler.view_as_text(ViewOptions::default()).unwrap();
        assert!(text.contains("Hello ODG"));
    }

    #[test]
    fn test_view_as_outline() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: sample_drawing() }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let outline = handler.view_as_outline().unwrap();
        assert!(outline.contains("page1"));
        assert!(outline.contains("Hello ODG"));
    }

    #[test]
    fn test_view_as_stats() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: sample_drawing() }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let stats = handler.view_as_stats().unwrap();
        assert!(stats.contains("Elements:"));
        assert!(stats.contains("Pages:"));
    }

    #[test]
    fn test_view_as_svg() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: sample_drawing() }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let svg = handler.view_as_svg().unwrap();
        assert!(svg.contains("svg") || svg.contains("<svg"));
    }

    #[test]
    fn test_view_as_issues_empty_page() {
        let mut drawing = Drawing::new("empty");
        drawing.pages.push(lo_core::draw::DrawPage {
            name: "empty_page".to_string(),
            elements: Vec::new(),
        });
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let issues = handler.view_as_issues(None, None).unwrap();
        assert!(issues.iter().any(|i| i.issue_type == "EmptyPage"));
    }

    #[test]
    fn test_validate() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: sample_drawing() }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let errors = handler.validate().unwrap();
        assert!(errors.is_empty() || errors.iter().any(|e| e.error_type != "no-pages"));
    }

    #[test]
    fn test_extract_text_with_offsets() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: sample_drawing() }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let map = handler.extract_text_with_offsets().unwrap();
        assert!(map.full_text.contains("Hello ODG"));
        assert_eq!(map.meta.format, "odg");
    }

    #[test]
    fn test_editable_check() {
        let handler = OdgHandler {
            dom: RefCell::new(OdgDom { drawing: Drawing::new("test") }),
            editable: false,
            file_path: "/tmp/test.odg".to_string(),
        };
        let mut props = HashMap::new();
        props.insert("svg:width".to_string(), "15cm".to_string());
        assert!(handler.set("/nonexistent", &props).is_err());
    }
}
