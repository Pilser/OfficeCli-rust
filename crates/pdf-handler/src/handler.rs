use crate::reader::PdfReader;
use crate::navigation::PdfNavigator;
use crate::view::PdfViewer;
use crate::text_extract::PdfTextExtractor;
use handler_common::*;
use handler_common::output_format::BinaryInfo;
use std::cell::RefCell;
use std::collections::HashMap;

/// PDF document handler implementing DocumentHandler trait.
pub struct PdfHandler {
    reader: RefCell<PdfReader>,
    editable: bool,
}

impl PdfHandler {
    /// Open a PDF document.
    pub fn open(path: &str, editable: bool) -> Result<Self, HandlerError> {
        let reader = PdfReader::open(path)?;
        Ok(Self { reader: RefCell::new(reader), editable })
    }
}

impl DocumentHandler for PdfHandler {
    fn format_name(&self) -> &str { "pdf" }

    fn view_as_text(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let reader = self.reader.borrow();
        PdfViewer::new(PdfReader::open(reader.file_path())?).view_as_text(&opts)
    }

    fn view_as_annotated(&self, opts: ViewOptions) -> Result<String, HandlerError> {
        let reader = self.reader.borrow();
        PdfViewer::new(PdfReader::open(reader.file_path())?).view_as_annotated(&opts)
    }

    fn view_as_outline(&self) -> Result<String, HandlerError> {
        let reader = self.reader.borrow();
        PdfViewer::new(PdfReader::open(reader.file_path())?).view_as_outline()
    }

    fn view_as_stats(&self) -> Result<String, HandlerError> {
        let reader = self.reader.borrow();
        PdfViewer::new(PdfReader::open(reader.file_path())?).view_as_stats()
    }

    fn view_as_issues(&self, issue_type: Option<&str>, limit: Option<usize>) -> Result<Vec<DocumentIssue>, HandlerError> {
        let reader = self.reader.borrow();
        PdfViewer::new(PdfReader::open(reader.file_path())?).view_as_issues(issue_type, limit)
    }

    fn view_as_html(&self) -> Result<String, HandlerError> {
        crate::html_preview::view_as_html(&self.reader.borrow())
    }

    fn view_as_text_json(&self, opts: ViewOptions) -> Result<serde_json::Value, HandlerError> {
        let text = self.view_as_text(opts)?;
        Ok(serde_json::json!({
            "format": "pdf",
            "text": text,
            "pageCount": self.reader.borrow().page_count()
        }))
    }

    fn view_as_outline_json(&self) -> Result<serde_json::Value, HandlerError> {
        let outline = self.view_as_outline()?;
        Ok(serde_json::json!({
            "format": "pdf",
            "outline": outline,
            "pageCount": self.reader.borrow().page_count()
        }))
    }

    fn view_as_stats_json(&self) -> Result<serde_json::Value, HandlerError> {
        let stats = self.view_as_stats()?;
        Ok(serde_json::json!({
            "format": "pdf",
            "stats": stats,
            "pageCount": self.reader.borrow().page_count()
        }))
    }

    fn get(&self, path: &str, depth: usize) -> Result<DocumentNode, HandlerError> {
        let reader = self.reader.borrow();

        if path == "/" {
            let mut root_node = DocumentNode::new("/", "pdf-document");
            if depth > 0 {
                let page_count = reader.page_count();
                let mut children = Vec::new();
                for i in 1..=page_count {
                    let page_text = reader.extract_page_text(i).unwrap_or_default();
                    let preview = if page_text.chars().count() > 80 {
                        format!("{}...", page_text.chars().take(80).collect::<String>())
                    } else {
                        page_text.clone()
                    };
                    children.push(
                        DocumentNode::new(&format!("/page[{}]", i), "page")
                            .with_text(&page_text)
                            .with_preview(&preview)
                    );
                }
                root_node = root_node.with_children(children);
            }
            return Ok(root_node);
        }

        let nav = PdfNavigator::new(reader.page_count());
        nav.validate_path(path).map_err(|e| HandlerError::InvalidPath(e))?;

        let page_num = PdfNavigator::page_number_from_path(path)
            .map_err(|e| HandlerError::InvalidPath(e))?;

        let node = DocumentNode::new(path, "page")
            .with_text(reader.extract_page_text(page_num).unwrap_or_default());
        Ok(node)
    }

    fn query(&self, selector: &str) -> Result<Vec<DocumentNode>, HandlerError> {
        let parsed = Selector::parse(selector).map_err(|e| HandlerError::InvalidArgument(e.to_string()))?;
        let reader = self.reader.borrow();
        let mut results = Vec::new();

        if let Some(element_type) = &parsed.element_type {
            if element_type == "page" {
                for i in 1..=reader.page_count() {
                    let path = format!("/page[{}]", i);
                    let node = DocumentNode::new(&path, "page")
                        .with_text(reader.extract_page_text(i).unwrap_or_default());
                    results.push(node);
                }
            } else if element_type == "text" {
                for i in 1..=reader.page_count() {
                    let path = format!("/page[{}]/text[1]", i);
                    let node = DocumentNode::new(&path, "text-block")
                        .with_text(reader.extract_page_text(i).unwrap_or_default());
                    results.push(node);
                }
            }
        }
        Ok(results)
    }

    fn set(&self, path: &str, properties: &HashMap<String, String>) -> Result<Vec<String>, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError("PDF opened in read-only mode".to_string()));
        }

        let mut unsupported = Vec::new();

        // Parse path to determine page
        let page_num = if path == "/" {
            None
        } else {
            let nav = PdfNavigator::new(self.reader.borrow().page_count());
            nav.validate_path(path).map_err(|e| HandlerError::InvalidPath(e))?;
            Some(PdfNavigator::page_number_from_path(path).map_err(|e| HandlerError::InvalidPath(e))?)
        };

        for (key, value) in properties {
            match key.as_str() {
                "text" | "content" => {
                    // Replace text on the specified page (or all pages)
                    let mut reader = self.reader.borrow_mut();
                    if let Some(page) = page_num {
                        crate::modifier::replace_text_on_page(
                            reader.document_mut(), page, value,
                        )?;
                    } else {
                        // Replace on all pages
                        let page_count = reader.page_count();
                        for page in 1..=page_count {
                            crate::modifier::replace_text_on_page(
                                reader.document_mut(), page, value,
                            ).ok(); // may fail on pages without text
                        }
                    }
                }
                other => unsupported.push(other.to_string()),
            }
        }

        Ok(unsupported)
    }

    fn add(&self, _parent: &str, element_type: &str, _position: InsertPosition, _properties: &HashMap<String, String>) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedType(format!("PDF does not support adding {}", element_type)))
    }

    fn remove(&self, path: &str) -> Result<Option<String>, HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError("PDF opened in read-only mode".to_string()));
        }

        let nav = PdfNavigator::new(self.reader.borrow().page_count());
        nav.validate_path(path).map_err(|e| HandlerError::InvalidPath(e))?;

        let page_num = PdfNavigator::page_number_from_path(path)
            .map_err(|e| HandlerError::InvalidPath(e))?;

        let mut reader = self.reader.borrow_mut();
        crate::modifier::delete_page(reader.document_mut(), page_num)?;
        reader.recount_pages();

        Ok(Some(format!("removed page {}", page_num)))
    }

    fn move_element(&self, _source: &str, _target_parent: Option<&str>, _position: InsertPosition) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedMode("PDF does not support moving elements".to_string()))
    }

    fn copy_from(&self, _source: &str, _target_parent: &str, _position: InsertPosition) -> Result<String, HandlerError> {
        Err(HandlerError::UnsupportedMode("PDF does not support copying elements".to_string()))
    }

    fn raw(&self, part_path: &str, _opts: RawOptions) -> Result<String, HandlerError> {
        let reader = self.reader.borrow();
        let page_num = part_path.strip_prefix("/page[")
            .and_then(|s| s.strip_suffix("]"))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| HandlerError::InvalidPath(part_path.to_string()))?;

        let pages = reader.document().get_pages();
        let page_id = pages.get(&(page_num as u32))
            .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

        reader.document().get_page_content(*page_id)
            .map(|content| String::from_utf8_lossy(&content).to_string())
            .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))
    }

    fn raw_set(&self, part_path: &str, _xpath: &str, action: &str, content: Option<&str>) -> Result<(), HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError("PDF opened in read-only mode".to_string()));
        }

        let page_num = part_path.strip_prefix("/page[")
            .and_then(|s| s.strip_suffix("]"))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| HandlerError::InvalidPath(part_path.to_string()))?;

        let mut reader = self.reader.borrow_mut();
        let pages = reader.document().get_pages();
        let page_id = pages.get(&(page_num as u32))
            .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

        match action {
            "replace_content" => {
                let new_content = content.ok_or_else(|| HandlerError::InvalidArgument("content required for replace_content".to_string()))?;
                let new_bytes = new_content.as_bytes();
                crate::modifier::replace_page_content(reader.document_mut(), *page_id, new_bytes)?;
                Ok(())
            }
            _ => Err(HandlerError::UnsupportedMode(format!("PDF raw_set action '{}' not supported", action))),
        }
    }

    fn add_part(&self, _parent: &str, _part_type: &str, _properties: Option<&HashMap<String, String>>) -> Result<(String, String), HandlerError> {
        Err(HandlerError::UnsupportedMode("PDF does not support adding parts".to_string()))
    }

    fn validate(&self) -> Result<Vec<ValidationError>, HandlerError> {
        let reader = self.reader.borrow();
        let viewer = PdfViewer::new(PdfReader::open(reader.file_path())?);
        viewer.validate()
    }

    fn try_extract_binary(&self, path: &str, dest: &str) -> Result<Option<BinaryInfo>, HandlerError> {
        // PDF binary extraction: extract embedded images from a page
        let page_num = if path.starts_with("/page[") {
            let nav = PdfNavigator::new(self.reader.borrow().page_count());
            nav.validate_path(path).map_err(|e| HandlerError::InvalidPath(e))?;
            PdfNavigator::page_number_from_path(path).map_err(|e| HandlerError::InvalidPath(e))?
        } else {
            return Err(HandlerError::InvalidPath(path.to_string()));
        };

        let reader = self.reader.borrow();
        let pages = reader.document().get_pages();
        let page_id = pages.get(&(page_num as u32))
            .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

        let doc = reader.document();

        // Look for image streams in the document objects associated with this page
        let content_ids = doc.get_page_contents(*page_id);
        for content_id in content_ids {
            if let Ok(lopdf::Object::Stream(stream)) = doc.get_object(content_id) {
                // Check if this is an image stream
                if let Ok(subtype_obj) = stream.dict.get(b"Subtype") {
                    if let Ok(name) = subtype_obj.as_name_str() {
                        if name == "Image" {
                            std::fs::write(dest, &stream.content)
                                .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
                            return Ok(Some(BinaryInfo {
                                content_type: "image/raw".to_string(),
                                byte_count: stream.content.len(),
                            }));
                        }
                    }
                }
            }
        }

        // Search all objects for image streams referenced by this page
        for (_, obj) in doc.objects.iter() {
            if let lopdf::Object::Stream(stream) = obj {
                if let Ok(subtype_obj) = stream.dict.get(b"Subtype") {
                    if let Ok(name) = subtype_obj.as_name_str() {
                        if name == "Image" {
                            std::fs::write(dest, &stream.content)
                                .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
                            return Ok(Some(BinaryInfo {
                                content_type: "image/raw".to_string(),
                                byte_count: stream.content.len(),
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn save(&self) -> Result<(), HandlerError> {
        if !self.editable {
            return Err(HandlerError::SaveError("PDF opened in read-only mode".to_string()));
        }

        let file_path = self.reader.borrow().file_path().to_string();
        self.reader.borrow_mut().document_mut().save(&file_path)
            .map_err(|e| HandlerError::SaveError(format!("failed to save PDF: {}", e)))?;
        Ok(())
    }

    fn extract_text_with_offsets(&self) -> Result<TextOffsetMap, HandlerError> {
        let reader = self.reader.borrow();
        let extractor = PdfTextExtractor::new(PdfReader::open(reader.file_path())?);
        Ok(extractor.extract_with_offsets())
    }
}