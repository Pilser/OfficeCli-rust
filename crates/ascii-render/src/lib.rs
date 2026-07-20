use handler_common::{DocumentHandler, HandlerError, OffsetSpan, TextOffsetMap};

pub struct AsciiRenderer {
    char_width: u32,
    char_height: u32,
    scale_x: f32,
    scale_y: f32,
    buffer: Vec<Vec<char>>,
    compact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotateMode {
    None,
    BBox,
    Text,
}

impl AsciiRenderer {
    pub fn new(term_width: u32, term_height: u32) -> Self {
        let buffer = vec![vec![' '; term_width as usize]; term_height as usize];
        Self {
            char_width: term_width,
            char_height: term_height,
            scale_x: 1.0,
            scale_y: 1.0,
            buffer,
            compact: false,
        }
    }

    pub fn set_compact(&mut self, compact: bool) {
        self.compact = compact;
    }

    pub fn render(
        &mut self,
        doc: &dyn DocumentHandler,
        mode: AnnotateMode,
    ) -> Result<String, HandlerError> {
        let map = doc.extract_text_with_offsets()?;
        self.reset_buffer();

        let bbox_spans: Vec<&OffsetSpan> = map.spans.iter().filter(|s| s.bbox.is_some()).collect();

        if bbox_spans.is_empty() {
            return self.render_fallback(&map);
        }

        let (doc_width, doc_height) = self.calculate_bounds(&bbox_spans);

        self.scale_x = self.char_width as f32 / doc_width;
        self.scale_y = self.char_height as f32 / doc_height;

        for span in &bbox_spans {
            if let Some(ref bbox) = span.bbox {
                let x = (bbox.x * self.scale_x) as usize;
                let y = (bbox.y * self.scale_y) as usize;
                let w = ((bbox.width * self.scale_x).ceil() as usize).max(2);
                let h = ((bbox.height * self.scale_y).ceil() as usize).max(2);

                self.draw_box(x, y, w, h);

                match mode {
                    AnnotateMode::None => {}
                    AnnotateMode::BBox => {
                        let label = format!("{:.0},{:.0} {}x{}", bbox.x, bbox.y, bbox.width, bbox.height);
                        self.write_inside(x, y, w, h, &label);
                    }
                    AnnotateMode::Text => {
                        self.write_inside(x, y, w, h, &span.text);
                    }
                }
            }
        }

        let ascii = self.buffer_to_string();
        let ratio = self.empty_ratio();

        Ok(format!(
            "{}\nEmpty/white space ratio: {:.1}%",
            ascii,
            ratio * 100.0
        ))
    }

    fn reset_buffer(&mut self) {
        for row in self.buffer.iter_mut() {
            row.fill(' ');
        }
    }

    fn calculate_bounds(&self, spans: &[&OffsetSpan]) -> (f32, f32) {
        let mut max_w = 0.0f32;
        let mut max_h = 0.0f32;
        for span in spans {
            if let Some(ref bbox) = span.bbox {
                let right = bbox.x + bbox.width;
                let bottom = bbox.y + bbox.height;
                if right > max_w {
                    max_w = right;
                }
                if bottom > max_h {
                    max_h = bottom;
                }
            }
        }
        if max_w == 0.0 {
            max_w = 612.0;
        }
        if max_h == 0.0 {
            max_h = 792.0;
        }
        (max_w, max_h)
    }

    fn draw_box(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w < 2 || h < 2 {
            return;
        }
        let x_max = self.char_width as usize;
        let y_max = self.char_height as usize;

        let x_end = (x + w).min(x_max);
        let y_end = (y + h).min(y_max);

        if x >= x_max || y >= y_max {
            return;
        }

        // top edge: ┌───┐
        if y < y_max {
            self.put(x, y, '┌');
            for cx in (x + 1)..x_end.saturating_sub(1) {
                if cx < x_max {
                    self.put(cx, y, '─');
                }
            }
            if x_end > x {
                self.put(x_end - 1, y, '┐');
            }
        }

        // middle edges: │   │
        for cy in (y + 1)..y_end.saturating_sub(1) {
            if cy < y_max {
                self.put(x, cy, '│');
                if x_end > x {
                    self.put(x_end - 1, cy, '│');
                }
            }
        }

        // bottom edge: └───┘
        if y_end > y {
            let by = y_end - 1;
            if by < y_max {
                self.put(x, by, '└');
                for cx in (x + 1)..x_end.saturating_sub(1) {
                    if cx < x_max {
                        self.put(cx, by, '─');
                    }
                }
                if x_end > x {
                    self.put(x_end - 1, by, '┘');
                }
            }
        }
    }

    fn put(&mut self, x: usize, y: usize, c: char) {
        if x < self.char_width as usize && y < self.char_height as usize {
            self.buffer[y][x] = c;
        }
    }

    fn write_inside(&mut self, x: usize, y: usize, w: usize, h: usize, text: &str) {
        if w < 3 || h < 3 {
            return;
        }
        let inner_x = x + 1;
        let inner_y = y + 1;
        let max_w = (w - 2).min(self.char_width as usize - inner_x);

        let display: String = text.chars().take(max_w).collect();
        for (i, c) in display.chars().enumerate() {
            self.put(inner_x + i, inner_y, c);
        }
    }

    fn empty_ratio(&self) -> f32 {
        let total = (self.char_width * self.char_height) as f32;
        if total == 0.0 {
            return 1.0;
        }
        let filled: usize = self
            .buffer
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&&c| c != ' ')
            .count();
        1.0 - (filled as f32 / total)
    }

    fn buffer_to_string(&self) -> String {
        let rows: Vec<String> = self
            .buffer
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect();

        if self.compact {
            let trimmed = trim_empty_rows(&rows);
            trimmed.join("\n")
        } else {
            rows.join("\n")
        }
    }

    fn render_fallback(&self, map: &TextOffsetMap) -> Result<String, HandlerError> {
        if map.spans.is_empty() {
            return Ok("(empty document - no bbox data)".to_string());
        }
        let mut result = String::from("(no bbox data - showing text spans):\n");
        for span in &map.spans {
            result.push_str(&format!("  {}: {}\n", span.path, span.text));
        }
        Ok(result)
    }
}

fn trim_empty_rows(rows: &[String]) -> Vec<String> {
    let start = rows
        .iter()
        .position(|r| r.chars().any(|c| c != ' '))
        .unwrap_or(rows.len());
    let end = rows
        .iter()
        .rposition(|r| r.chars().any(|c| c != ' '))
        .map(|i| i + 1)
        .unwrap_or(start);
    rows[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use handler_common::{
        BBoxSpan, DocumentHandler, HandlerError, InsertPosition, OffsetSpan,
        RawOptions, TextOffsetMap, ViewOptions,
    };
    use std::collections::HashMap;

    struct MockDoc {
        format: String,
        map: TextOffsetMap,
    }

    impl DocumentHandler for MockDoc {
        fn format_name(&self) -> &str {
            &self.format
        }
        fn view_as_text(&self, _opts: ViewOptions) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn view_as_annotated(&self, _opts: ViewOptions) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn view_as_outline(&self) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn view_as_stats(&self) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn view_as_issues(
            &self,
            _issue_type: Option<&str>,
            _limit: Option<usize>,
        ) -> Result<Vec<handler_common::DocumentIssue>, HandlerError> {
            unreachable!()
        }
        fn view_as_text_json(
            &self,
            _opts: ViewOptions,
        ) -> Result<serde_json::Value, HandlerError> {
            unreachable!()
        }
        fn view_as_outline_json(&self) -> Result<serde_json::Value, HandlerError> {
            unreachable!()
        }
        fn view_as_stats_json(&self) -> Result<serde_json::Value, HandlerError> {
            unreachable!()
        }
        fn get(
            &self,
            _path: &str,
            _depth: usize,
        ) -> Result<handler_common::DocumentNode, HandlerError> {
            unreachable!()
        }
        fn query(
            &self,
            _selector: &str,
        ) -> Result<Vec<handler_common::DocumentNode>, HandlerError> {
            unreachable!()
        }
        fn set(
            &self,
            _path: &str,
            _properties: &HashMap<String, String>,
        ) -> Result<Vec<String>, HandlerError> {
            unreachable!()
        }
        fn add(
            &self,
            _parent: &str,
            _element_type: &str,
            _position: InsertPosition,
            _properties: &HashMap<String, String>,
            _wrap: Option<&str>,
        ) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn remove(&self, _path: &str) -> Result<Option<String>, HandlerError> {
            unreachable!()
        }
        fn move_element(
            &self,
            _source: &str,
            _target_parent: Option<&str>,
            _position: InsertPosition,
        ) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn copy_from(
            &self,
            _source: &str,
            _target_parent: &str,
            _position: InsertPosition,
        ) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn raw(
            &self,
            _part_path: &str,
            _opts: RawOptions,
        ) -> Result<String, HandlerError> {
            unreachable!()
        }
        fn raw_set(
            &self,
            _part_path: &str,
            _xpath: &str,
            _action: &str,
            _xml: Option<&str>,
        ) -> Result<(), HandlerError> {
            unreachable!()
        }
        fn add_part(
            &self,
            _parent: &str,
            _part_type: &str,
            _properties: Option<&HashMap<String, String>>,
        ) -> Result<(String, String), HandlerError> {
            unreachable!()
        }
        fn validate(&self) -> Result<Vec<handler_common::ValidationError>, HandlerError> {
            unreachable!()
        }
        fn try_extract_binary(
            &self,
            _path: &str,
            _dest: &str,
        ) -> Result<Option<handler_common::BinaryInfo>, HandlerError> {
            unreachable!()
        }
        fn save(&self) -> Result<(), HandlerError> {
            unreachable!()
        }
        fn extract_text_with_offsets(&self) -> Result<TextOffsetMap, HandlerError> {
            Ok(self.map.clone())
        }
    }

    fn make_span(text: &str, path: &str, bbox: BBoxSpan) -> OffsetSpan {
        let chars: Vec<char> = text.chars().collect();
        OffsetSpan {
            start: 0,
            end: chars.len(),
            path: path.to_string(),
            text: text.to_string(),
            element_type: "text".to_string(),
            id: None,
            bbox: Some(bbox),
            style: None,
        }
    }

    #[test]
    fn test_empty_document() {
        let mut renderer = AsciiRenderer::new(80, 24);
        let doc = MockDoc {
            format: "pdf".to_string(),
            map: TextOffsetMap::empty("pdf"),
        };
        let result = renderer.render(&doc, AnnotateMode::None).unwrap();
        assert!(result.contains("empty document"));
    }

    #[test]
    fn test_single_bbox_element() {
        let mut renderer = AsciiRenderer::new(40, 20);
        let bbox = BBoxSpan {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };
        let span = make_span("Hello", "/page[1]/text[1]", bbox);
        let mut map = TextOffsetMap::empty("pdf");
        map.full_text = "Hello".to_string();
        map.spans.push(span);
        map.meta.total_chars = 5;
        map.meta.total_spans = 1;

        let doc = MockDoc {
            format: "pdf".to_string(),
            map,
        };
        let result = renderer.render(&doc, AnnotateMode::None).unwrap();
        assert!(result.contains('┌'));
        assert!(result.contains('┐'));
        assert!(result.contains('└'));
        assert!(result.contains('┘'));
        assert!(result.contains("Empty/white space ratio"));
    }

    #[test]
    fn test_multiple_elements() {
        let mut renderer = AsciiRenderer::new(60, 30);
        let bbox1 = BBoxSpan {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 50.0,
        };
        let bbox2 = BBoxSpan {
            x: 0.0,
            y: 60.0,
            width: 150.0,
            height: 40.0,
        };
        let span1 = make_span("First", "/page[1]/text[1]", bbox1);
        let span2 = make_span("Second", "/page[1]/text[2]", bbox2);
        let mut map = TextOffsetMap::empty("pdf");
        map.full_text = "FirstSecond".to_string();
        map.spans.push(span1);
        map.spans.push(span2);
        map.meta.total_chars = 10;
        map.meta.total_spans = 2;

        let doc = MockDoc {
            format: "pdf".to_string(),
            map,
        };
        let result = renderer.render(&doc, AnnotateMode::None).unwrap();
        assert!(result.contains('┌'));
        assert!(result.contains("Empty/white space ratio"));

        let lines: Vec<&str> = result.lines().collect();
        let ratio_line = lines.last().unwrap();
        assert!(ratio_line.contains("Empty/white space ratio"));
    }

    #[test]
    fn test_compact_mode() {
        let mut renderer = AsciiRenderer::new(40, 20);
        renderer.set_compact(true);
        let bbox = BBoxSpan {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };
        let span = make_span("Hello", "/page[1]/text[1]", bbox);
        let mut map = TextOffsetMap::empty("pdf");
        map.full_text = "Hello".to_string();
        map.spans.push(span);
        map.meta.total_chars = 5;
        map.meta.total_spans = 1;

        let doc = MockDoc {
            format: "pdf".to_string(),
            map,
        };
        let result = renderer.render(&doc, AnnotateMode::None).unwrap();
        assert!(result.contains("Empty/white space ratio"));
    }

    #[test]
    fn test_annotate_modes() {
        let bbox = BBoxSpan {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 50.0,
        };
        let span = make_span("Sample text here", "/page[1]/text[1]", bbox);
        let mut map = TextOffsetMap::empty("pdf");
        map.full_text = "Sample text here".to_string();
        map.spans.push(span);
        map.meta.total_chars = 15;
        map.meta.total_spans = 1;

        let doc = MockDoc {
            format: "pdf".to_string(),
            map,
        };

        // AnnotateMode::None
        let mut r1 = AsciiRenderer::new(40, 20);
        let result_none = r1.render(&doc, AnnotateMode::None).unwrap();
        assert!(result_none.contains('┌'));

        // AnnotateMode::BBox
        let mut r2 = AsciiRenderer::new(40, 20);
        let result_bbox = r2.render(&doc, AnnotateMode::BBox).unwrap();
        assert!(result_bbox.contains("10,20"));

        // AnnotateMode::Text
        let mut r3 = AsciiRenderer::new(40, 20);
        let result_text = r3.render(&doc, AnnotateMode::Text).unwrap();
        assert!(result_text.contains("Sample"));
    }

    #[test]
    fn test_trim_empty_rows() {
        let rows = vec![
            "   ".to_string(),
            " a ".to_string(),
            "   ".to_string(),
        ];
        let trimmed = trim_empty_rows(&rows);
        assert_eq!(trimmed.len(), 1);
        assert_eq!(trimmed[0], " a ");
    }

    #[test]
    fn test_draw_box_minimum_size() {
        let mut renderer = AsciiRenderer::new(10, 10);
        // 2x2 box should draw corners
        renderer.draw_box(0, 0, 2, 2);
        assert_eq!(renderer.buffer[0][0], '┌');
        assert_eq!(renderer.buffer[0][1], '┐');
        assert_eq!(renderer.buffer[1][0], '└');
        assert_eq!(renderer.buffer[1][1], '┘');
    }

    #[test]
    fn test_render_with_bbox_spans_no_fallback() {
        // Ensure that when bbox data exists, we do NOT get "no bbox data" in output
        let mut renderer = AsciiRenderer::new(40, 20);
        let bbox = BBoxSpan {
            x: 0.0,
            y: 0.0,
            width: 72.0,
            height: 36.0,
        };
        let span = make_span("Hi", "/p[1]", bbox);
        let mut map = TextOffsetMap::empty("pdf");
        map.full_text = "Hi".to_string();
        map.spans.push(span);
        map.meta.total_chars = 2;
        map.meta.total_spans = 1;

        let doc = MockDoc {
            format: "pdf".to_string(),
            map,
        };
        let result = renderer.render(&doc, AnnotateMode::None).unwrap();
        assert!(!result.contains("no bbox data"));
        assert!(result.contains('┌'));
    }

    #[test]
    fn test_empty_ratio_calculation() {
        let mut renderer = AsciiRenderer::new(10, 10);
        let ratio = renderer.empty_ratio();
        // All spaces = ratio 1.0
        assert!((ratio - 1.0).abs() < f32::EPSILON);

        renderer.put(0, 0, 'X');
        let ratio2 = renderer.empty_ratio();
        assert!(ratio2 < 1.0);
        assert!(ratio2 > 0.98);
    }

    #[test]
    fn test_reset_buffer() {
        let mut renderer = AsciiRenderer::new(5, 5);
        renderer.put(2, 2, 'X');
        assert_eq!(renderer.buffer[2][2], 'X');
        renderer.reset_buffer();
        for row in &renderer.buffer {
            for &c in row {
                assert_eq!(c, ' ');
            }
        }
    }
}
