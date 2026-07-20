use handler_common::{BBoxSpan, DocumentHandler, StyleSpan};

pub struct LayoutEngine {
    page_width: f32,
    page_height: f32,
    margin_top: f32,
    margin_bottom: f32,
    margin_left: f32,
    margin_right: f32,
}

pub struct PositionedPage {
    pub width: f32,
    pub height: f32,
    pub elements: Vec<PositionedElement>,
}

pub struct PositionedElement {
    pub path: String,
    pub bbox: BBoxSpan,
    pub text: String,
    pub style: StyleSpan,
    pub element_type: String,
    pub children: Vec<PositionedElement>,
}

impl LayoutEngine {
    pub fn new(page_width: f32, page_height: f32, margin: f32) -> Self {
        Self {
            page_width,
            page_height,
            margin_top: margin,
            margin_bottom: margin,
            margin_left: margin,
            margin_right: margin,
        }
    }

    /// Layout a full document into pages.
    pub fn layout(&self, dom: &dyn DocumentHandler) -> Result<Vec<PositionedPage>, String> {
        let map = dom
            .extract_text_with_offsets()
            .map_err(|e| format!("extract offsets: {}", e))?;

        let content_width = self.page_width - self.margin_left - self.margin_right;
        let content_height = self.page_height - self.margin_top - self.margin_bottom;

        let mut elements = Vec::new();
        let mut y = self.margin_top;

        for span in &map.spans {
            let bbox = span.bbox.clone().unwrap_or(BBoxSpan {
                x: self.margin_left,
                y,
                width: content_width,
                height: 16.0,
            });

            let element = PositionedElement {
                path: span.path.clone(),
                bbox: BBoxSpan {
                    x: bbox.x,
                    y,
                    width: content_width,
                    height: bbox.height.max(12.0),
                },
                text: span.text.clone(),
                style: span.style.clone().unwrap_or(StyleSpan {
                    font: None,
                    size: None,
                    color: None,
                }),
                element_type: span.element_type.clone(),
                children: Vec::new(),
            };
            y += bbox.height.max(12.0) + 2.0;
            elements.push(element);

            // Simple pagination
            if y > self.margin_top + content_height {
                break;
            }
        }

        Ok(vec![PositionedPage {
            width: self.page_width,
            height: self.page_height,
            elements,
        }])
    }

    /// Layout a single element by path.
    pub fn layout_element(
        &self,
        path: &str,
        dom: &dyn DocumentHandler,
    ) -> Result<PositionedElement, String> {
        let map = dom
            .extract_text_with_offsets()
            .map_err(|e| format!("extract offsets: {}", e))?;

        let span = map
            .spans_for_path(path)
            .into_iter()
            .next()
            .ok_or_else(|| format!("path not found: {}", path))?;

        let bbox = span.bbox.clone().unwrap_or(BBoxSpan {
            x: self.margin_left,
            y: self.margin_top,
            width: self.page_width - self.margin_left - self.margin_right,
            height: 16.0,
        });

        Ok(PositionedElement {
            path: span.path.clone(),
            bbox,
            text: span.text.clone(),
            style: span.style.clone().unwrap_or(StyleSpan {
                font: None,
                size: None,
                color: None,
            }),
            element_type: span.element_type.clone(),
            children: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handler_common::{HandlerError, InsertPosition, TextOffsetMap};
    use std::collections::HashMap;

    struct MockDoc {
        map: TextOffsetMap,
    }

    impl MockDoc {
        fn new() -> Self {
            let mut map = TextOffsetMap::empty("pdf");
            map.push_span_with_metadata(
                "Hello",
                "/page[1]/text[1]",
                "text-block",
                Some(BBoxSpan {
                    x: 50.0,
                    y: 50.0,
                    width: 200.0,
                    height: 20.0,
                }),
                Some(StyleSpan {
                    font: Some("Arial".into()),
                    size: Some(12.0),
                    color: Some("#000000".into()),
                }),
            );
            map.push_span_with_metadata(
                "World",
                "/page[1]/text[2]",
                "text-block",
                Some(BBoxSpan {
                    x: 50.0,
                    y: 80.0,
                    width: 200.0,
                    height: 20.0,
                }),
                Some(StyleSpan {
                    font: Some("Arial".into()),
                    size: Some(14.0),
                    color: Some("#FF0000".into()),
                }),
            );
            Self { map }
        }
    }

    impl DocumentHandler for MockDoc {
        fn format_name(&self) -> &str {
            "pdf"
        }

        fn view_as_text(
            &self,
            _opts: handler_common::ViewOptions,
        ) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn view_as_annotated(
            &self,
            _opts: handler_common::ViewOptions,
        ) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn view_as_outline(&self) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn view_as_stats(&self) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn view_as_issues(
            &self,
            _issue_type: Option<&str>,
            _limit: Option<usize>,
        ) -> Result<Vec<handler_common::DocumentIssue>, HandlerError> {
            unimplemented!()
        }

        fn view_as_html(
            &self,
            _opts: handler_common::ViewOptions,
        ) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn view_as_svg(&self) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn view_as_forms(&self) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn view_as_text_json(
            &self,
            _opts: handler_common::ViewOptions,
        ) -> Result<serde_json::Value, HandlerError> {
            unimplemented!()
        }

        fn view_as_outline_json(&self) -> Result<serde_json::Value, HandlerError> {
            unimplemented!()
        }

        fn view_as_stats_json(&self) -> Result<serde_json::Value, HandlerError> {
            unimplemented!()
        }

        fn get(
            &self,
            _path: &str,
            _depth: usize,
        ) -> Result<handler_common::DocumentNode, HandlerError> {
            unimplemented!()
        }

        fn query(
            &self,
            _selector: &str,
        ) -> Result<Vec<handler_common::DocumentNode>, HandlerError> {
            unimplemented!()
        }

        fn set(
            &self,
            _path: &str,
            _properties: &HashMap<String, String>,
        ) -> Result<Vec<String>, HandlerError> {
            unimplemented!()
        }

        fn add(
            &self,
            _parent: &str,
            _element_type: &str,
            _position: InsertPosition,
            _properties: &HashMap<String, String>,
            _wrap: Option<&str>,
        ) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn remove(&self, _path: &str) -> Result<Option<String>, HandlerError> {
            unimplemented!()
        }

        fn move_element(
            &self,
            _source: &str,
            _target_parent: Option<&str>,
            _position: InsertPosition,
        ) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn copy_from(
            &self,
            _source: &str,
            _target_parent: &str,
            _position: InsertPosition,
        ) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn swap(
            &self,
            _path1: &str,
            _path2: &str,
        ) -> Result<(String, String), HandlerError> {
            unimplemented!()
        }

        fn merge(
            &self,
            _data: &HashMap<String, String>,
        ) -> Result<handler_common::MergeResult, HandlerError> {
            unimplemented!()
        }

        fn raw(
            &self,
            _part_path: &str,
            _opts: handler_common::RawOptions,
        ) -> Result<String, HandlerError> {
            unimplemented!()
        }

        fn raw_set(
            &self,
            _part_path: &str,
            _xpath: &str,
            _action: &str,
            _xml: Option<&str>,
        ) -> Result<(), HandlerError> {
            unimplemented!()
        }

        fn add_part(
            &self,
            _parent: &str,
            _part_type: &str,
            _properties: Option<&HashMap<String, String>>,
        ) -> Result<(String, String), HandlerError> {
            unimplemented!()
        }

        fn validate(&self) -> Result<Vec<handler_common::ValidationError>, HandlerError> {
            unimplemented!()
        }

        fn try_extract_binary(
            &self,
            _path: &str,
            _dest: &str,
        ) -> Result<Option<handler_common::BinaryInfo>, HandlerError> {
            unimplemented!()
        }

        fn save(&self) -> Result<(), HandlerError> {
            Ok(())
        }

        fn extract_text_with_offsets(&self) -> Result<TextOffsetMap, HandlerError> {
            Ok(self.map.clone())
        }
    }

    #[test]
    fn test_layout_creates_pages() {
        let engine = LayoutEngine::new(612.0, 792.0, 72.0);
        let doc = MockDoc::new();
        let pages = engine.layout(&doc).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].elements.len(), 2);
    }

    #[test]
    fn test_layout_element_by_path() {
        let engine = LayoutEngine::new(612.0, 792.0, 72.0);
        let doc = MockDoc::new();
        let element = engine.layout_element("/page[1]/text[1]", &doc).unwrap();
        assert_eq!(element.text, "Hello");
    }

    #[test]
    fn test_layout_missing_path() {
        let engine = LayoutEngine::new(612.0, 792.0, 72.0);
        let doc = MockDoc::new();
        let result = engine.layout_element("/page[1]/text[99]", &doc);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_dimensions() {
        let engine = LayoutEngine::new(800.0, 600.0, 50.0);
        assert!((engine.page_width - 800.0).abs() < 0.001);
        assert!((engine.page_height - 600.0).abs() < 0.001);
    }
}
