use handler_common::*;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct AiHandlerAdapter {
    inner: RefCell<illustrator_rs::handler::AiHandler>,
    file_path: String,
}

impl AiHandlerAdapter {
    pub fn open(path: &str, _editable: bool) -> Result<Self, HandlerError> {
        let handler = illustrator_rs::handler::AiHandler::open(path)
            .map_err(|e| HandlerError::OpenError(e.to_string()))?;
        Ok(Self {
            inner: RefCell::new(handler),
            file_path: path.to_string(),
        })
    }
}

impl DocumentHandler for AiHandlerAdapter {
    fn format_name(&self) -> &str {
        "ai"
    }

    fn view_as_text(&self, _opts: ViewOptions) -> Result<String, HandlerError> {
        Ok(self.inner.borrow().view_text())
    }

    fn view_as_annotated(&self, _opts: ViewOptions) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedMode("annotated".to_string()))
    }

    fn view_as_outline(&self) -> Result<String, HandlerError> {
        Ok(self.inner.borrow().view_outline())
    }

    fn view_as_svg(&self) -> Result<String, HandlerError> {
        self.inner
            .borrow()
            .view_svg()
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))
    }

    fn view_as_stats(&self) -> Result<String, HandlerError> {
        Ok(self.inner.borrow().stats())
    }

    fn view_as_issues(
        &self,
        _issue_type: Option<&str>,
        _limit: Option<usize>,
    ) -> Result<Vec<DocumentIssue>, HandlerError> {
        let issues = self.inner.borrow().issues();
        Ok(issues
            .into_iter()
            .map(|desc| DocumentIssue {
                severity: IssueSeverity::Warning,
                issue_type: "structure".to_string(),
                description: desc,
                path: None,
            })
            .collect())
    }

    fn view_as_text_json(&self, _opts: ViewOptions) -> Result<serde_json::Value, HandlerError> {
        Err(HandlerError::UnsupportedMode("text-json".to_string()))
    }

    fn view_as_outline_json(&self) -> Result<serde_json::Value, HandlerError> {
        Err(HandlerError::UnsupportedMode("outline-json".to_string()))
    }

    fn view_as_stats_json(&self) -> Result<serde_json::Value, HandlerError> {
        Err(HandlerError::UnsupportedMode("stats-json".to_string()))
    }

    fn get(&self, path: &str, _depth: usize) -> Result<DocumentNode, HandlerError> {
        let mut inner = self.inner.borrow_mut();
        let obj = inner
            .get_mut(path)
            .map_err(|e| HandlerError::PathNotFound(format!("{}: {}", path, e)))?;
        Ok(ai_object_to_node(path, obj))
    }

    fn query(&self, _selector: &str) -> Result<Vec<DocumentNode>, HandlerError> {
        Err(HandlerError::UnsupportedMode("query".to_string()))
    }

    fn set(
        &self,
        _path: &str,
        _properties: &HashMap<String, String>,
    ) -> Result<Vec<String>, HandlerError> {
        Err(HandlerError::UnsupportedMode("set".to_string()))
    }

    fn add(
        &self,
        _parent: &str,
        _element_type: &str,
        _position: InsertPosition,
        _properties: &HashMap<String, String>,
        _wrap: Option<&str>,
    ) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedMode("add".to_string()))
    }

    fn remove(&self, _path: &str) -> Result<Option<String>, HandlerError> {
        Err(HandlerError::UnsupportedMode("remove".to_string()))
    }

    fn move_element(
        &self,
        _source: &str,
        _target_parent: Option<&str>,
        _position: InsertPosition,
    ) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedMode("move".to_string()))
    }

    fn copy_from(
        &self,
        _source: &str,
        _target_parent: &str,
        _position: InsertPosition,
    ) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedMode("copy".to_string()))
    }

    fn raw(&self, _part_path: &str, _opts: RawOptions) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedMode("raw".to_string()))
    }

    fn raw_set(
        &self,
        _part_path: &str,
        _xpath: &str,
        _action: &str,
        _xml: Option<&str>,
    ) -> Result<(), HandlerError> {
        Err(HandlerError::UnsupportedMode("raw-set".to_string()))
    }

    fn add_part(
        &self,
        _parent: &str,
        _part_type: &str,
        _properties: Option<&HashMap<String, String>>,
    ) -> Result<(String, String), HandlerError> {
        Err(HandlerError::UnsupportedMode("add-part".to_string()))
    }

    fn validate(&self) -> Result<Vec<ValidationError>, HandlerError> {
        Err(HandlerError::UnsupportedMode("validate".to_string()))
    }

    fn try_extract_binary(
        &self,
        _path: &str,
        _dest: &str,
    ) -> Result<Option<BinaryInfo>, HandlerError> {
        Err(HandlerError::UnsupportedMode("extract-binary".to_string()))
    }

    fn save(&self) -> Result<(), HandlerError> {
        self.inner
            .borrow_mut()
            .save(&self.file_path)
            .map_err(|e| HandlerError::SaveError(e.to_string()))
    }

    fn extract_text_with_offsets(&self) -> Result<TextOffsetMap, HandlerError> {
        let entries = self.inner.borrow().extract_text_with_offsets();
        let mut map = TextOffsetMap::empty("ai");
        for (text, x, y) in &entries {
            map.push_span_with_metadata(
                text,
                "",
                "text",
                Some(BBoxSpan {
                    x: *x as f32,
                    y: *y as f32,
                    width: 0.0,
                    height: 0.0,
                }),
                None,
            );
        }
        Ok(map)
    }
}

fn ai_object_to_node(path: &str, obj: &illustrator_rs::AiObject) -> DocumentNode {
    match obj {
        illustrator_rs::AiObject::Path(p) => {
            let mut node = DocumentNode::new(path, "path");
            if let Some(name) = &p.name {
                node = node.with_text(name);
            }
            node.child_count = p.segments.len();
            node
        }
        illustrator_rs::AiObject::Group(g) => {
            let children: Vec<DocumentNode> = g
                .children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let child_path = format!("{}/child[{}]", path, i + 1);
                    ai_object_to_node(&child_path, child)
                })
                .collect();
            let mut node = DocumentNode::new(path, "group");
            if let Some(name) = &g.name {
                node = node.with_text(name);
            }
            node.with_children(children)
        }
        illustrator_rs::AiObject::CompoundPath(c) => {
            let mut node = DocumentNode::new(path, "compound");
            if let Some(name) = &c.name {
                node = node.with_text(name);
            }
            node.child_count = c.subpaths.len();
            node
        }
        illustrator_rs::AiObject::Text(t) => {
            let mut node = DocumentNode::new(path, "text").with_text(&t.content);
            node = node.with_format("font", serde_json::Value::String(t.font.clone()));
            if let Some(size) = serde_json::Number::from_f64(t.size) {
                node = node.with_format("size", serde_json::Value::Number(size));
            }
            node
        }
        illustrator_rs::AiObject::Image(img) => {
            let mut node = DocumentNode::new(path, "image");
            node = node.with_format(
                "width",
                serde_json::Value::Number(serde_json::Number::from(img.width)),
            );
            node = node.with_format(
                "height",
                serde_json::Value::Number(serde_json::Number::from(img.height)),
            );
            if let Some(name) = &img.name {
                node = node.with_text(name);
            }
            node
        }
    }
}
