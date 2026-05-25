use crate::reader::PdfReader;
use handler_common::{DocumentIssue, IssueSeverity, HandlerError, ValidationError, ViewOptions};

/// PDF view modes implementation.
pub struct PdfViewer {
    reader: PdfReader,
}

impl PdfViewer {
    pub fn new(reader: PdfReader) -> Self { Self { reader } }

    pub fn view_as_text(&self, opts: &ViewOptions) -> Result<String, HandlerError> {
        let full_text = self.reader.extract_all_text();
        let lines: Vec<&str> = full_text.lines().collect();
        let start = opts.start_line.unwrap_or(0);
        let end = opts.end_line.unwrap_or(lines.len());
        let max = opts.max_lines.unwrap_or(lines.len());
        let end = end.min(lines.len()).min(start + max);
        if start >= lines.len() { return Ok(String::new()); }
        Ok(lines[start..end].join("\n"))
    }

    pub fn view_as_annotated(&self, opts: &ViewOptions) -> Result<String, HandlerError> {
        let mut result = String::new();
        let mut line_num = 0;
        let start = opts.start_line.unwrap_or(0);
        let end = opts.end_line.unwrap_or(usize::MAX);
        let max = opts.max_lines.unwrap_or(usize::MAX);

        for page_num in 1..=self.reader.page_count() {
            result.push_str(&format!("=== Page {} ===\n", page_num));
            if let Some(page_text) = self.reader.extract_page_text(page_num) {
                for line in page_text.lines() {
                    line_num += 1;
                    if line_num < start { continue; }
                    if line_num > end || line_num >= start + max { break; }
                    result.push_str(&format!("  {} | {}\n", line_num, line));
                }
            }
        }
        Ok(result)
    }

    pub fn view_as_outline(&self) -> Result<String, HandlerError> {
        let mut result = String::new();
        result.push_str("PDF Document\n");
        result.push_str(&format!("  Pages: {}\n", self.reader.page_count()));
        for page_num in 1..=self.reader.page_count() {
            if let Some(page_text) = self.reader.extract_page_text(page_num) {
                let char_count = page_text.chars().count();
                let first_line = page_text.lines().next().unwrap_or("");
                let preview = if first_line.chars().count() > 60 { format!("{}...", first_line.chars().take(60).collect::<String>()) } else { first_line.to_string() };
                result.push_str(&format!("  page[{}]: {} chars, \"{}\"\n", page_num, char_count, preview));
            } else {
                result.push_str(&format!("  page[{}]: (empty)\n", page_num));
            }
        }
        Ok(result)
    }

    pub fn view_as_stats(&self) -> Result<String, HandlerError> {
        let mut total_chars = 0;
        let mut total_lines = 0;
        for page_num in 1..=self.reader.page_count() {
            if let Some(page_text) = self.reader.extract_page_text(page_num) {
                total_chars += page_text.chars().count();
                total_lines += page_text.lines().count();
            }
        }
        Ok(format!("PDF Statistics\n  Pages: {}\n  Total chars: {}\n  Total lines: {}\n",
            self.reader.page_count(), total_chars, total_lines))
    }

    pub fn view_as_issues(&self, issue_type: Option<&str>, limit: Option<usize>) -> Result<Vec<DocumentIssue>, HandlerError> {
        let mut issues = Vec::new();
        let limit = limit.unwrap_or(50);
        for page_num in 1..=self.reader.page_count() {
            if let Some(page_text) = self.reader.extract_page_text(page_num) {
                if page_text.trim().is_empty() && issues.len() < limit {
                    issues.push(DocumentIssue {
                        severity: IssueSeverity::Info,
                        issue_type: "EmptyPage".to_string(),
                        description: format!("page {} contains no extractable text", page_num),
                        path: Some(format!("/page[{}]", page_num)),
                    });
                }
            }
        }
        if let Some(filter) = issue_type { issues.retain(|i| i.issue_type == filter); }
        Ok(issues)
    }

    /// Validate the PDF document structure.
    pub fn validate(&self) -> Result<Vec<ValidationError>, HandlerError> {
        let mut errors = Vec::new();

        // Check that the document has pages
        if self.reader.page_count() == 0 {
            errors.push(ValidationError {
                error_type: "structure".to_string(),
                description: "PDF has no pages".to_string(),
                path: Some("/".to_string()),
                part: None,
            });
        }

        // Check that each page has content
        for page_num in 1..=self.reader.page_count() {
            let pages = self.reader.document().get_pages();
            if !pages.contains_key(&(page_num as u32)) {
                errors.push(ValidationError {
                    error_type: "structure".to_string(),
                    description: format!("page {} referenced but not found in page tree", page_num),
                    path: Some(format!("/page[{}]", page_num)),
                    part: None,
                });
            }
        }

        Ok(errors)
    }
}