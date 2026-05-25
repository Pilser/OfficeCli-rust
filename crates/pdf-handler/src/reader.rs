use handler_common::HandlerError;
use lopdf::Document as LopdfDocument;

/// PDF document reader using lopdf.
pub struct PdfReader {
    doc: LopdfDocument,
    page_count: usize,
    file_path: String,
}

impl PdfReader {
    /// Open a PDF document.
    pub fn open(path: &str) -> Result<Self, HandlerError> {
        let doc = LopdfDocument::load(path)
            .map_err(|e| HandlerError::OpenError(format!("failed to open PDF: {}", e)))?;
        let page_count = Self::count_pages(&doc);
        Ok(Self { doc, page_count, file_path: path.to_string() })
    }

    pub fn page_count(&self) -> usize { self.page_count }
    pub fn document(&self) -> &LopdfDocument { &self.doc }
    pub fn document_mut(&mut self) -> &mut LopdfDocument { &mut self.doc }
    pub fn file_path(&self) -> &str { &self.file_path }

    /// Recount pages from the document (e.g. after deleting a page).
    pub fn recount_pages(&mut self) {
        self.page_count = Self::count_pages(&self.doc);
    }

    /// Create a fallback reader with an empty document (used when re-loading fails).
    pub fn fallback(page_count: usize, file_path: &str) -> Self {
        Self { doc: LopdfDocument::new(), page_count, file_path: file_path.to_string() }
    }

    /// Extract text from all pages.
    pub fn extract_all_text(&self) -> String {
        let mut full_text = String::new();
        for i in 1..=self.page_count {
            if let Some(page_text) = self.extract_page_text(i) {
                if !full_text.is_empty() { full_text.push('\n'); }
                full_text.push_str(&page_text);
            }
        }
        full_text
    }

    /// Extract text from a specific page.
    pub fn extract_page_text(&self, page_num: usize) -> Option<String> {
        let pages = self.doc.get_pages();
        let page_id = pages.get(&(page_num as u32))?;
        if let Ok(content) = self.doc.get_page_content(*page_id) {
            Some(Self::parse_content_stream(&content))
        } else {
            None
        }
    }

    /// Parse a PDF content stream into text.
    fn parse_content_stream(stream: &[u8]) -> String {
        let mut text = String::new();
        let mut in_text_object = false;

        let content_str = String::from_utf8_lossy(stream);
        for line in content_str.lines() {
            let line = line.trim();
            if line == "BT" { in_text_object = true; continue; }
            if line == "ET" { in_text_object = false; continue; }
            if !in_text_object { continue; }

            if line.ends_with("Tj") {
                let string_part = line.trim_end_matches("Tj").trim();
                if let Some(extracted) = extract_pdf_string(string_part) {
                    text.push_str(&extracted);
                }
            }
            if line.ends_with("TJ") {
                let array_part = line.trim_end_matches("TJ").trim();
                if let Some(extracted) = extract_pdf_array_text(array_part) {
                    text.push_str(&extracted);
                }
            }
            if line.ends_with("Td") || line.ends_with("TD") { text.push('\n'); }
            if line == "T*" { text.push('\n'); }
        }
        text
    }

    fn count_pages(doc: &LopdfDocument) -> usize {
        doc.get_pages().len()
    }
}

/// Extract a PDF string literal: (Hello World) -> "Hello World"
fn extract_pdf_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len()-1];
        let mut result = String::new();
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('b') => result.push('\u{08}'),
                    Some('f') => result.push('\u{0C}'),
                    Some('(') => result.push('('),
                    Some(')') => result.push(')'),
                    Some('\\') => result.push('\\'),
                    Some(d) if d.is_ascii_digit() => {
                        let mut octal = String::from(d);
                        for _ in 0..2 {
                            if let Some(&next) = chars.peek() {
                                if next.is_ascii_digit() { octal.push(chars.next().unwrap()); }
                                else { break; }
                            }
                        }
                        if let Ok(code) = u8::from_str_radix(&octal, 8) { result.push(code as char); }
                    }
                    Some(other) => result.push(other),
                    None => result.push('\\'),
                }
            } else { result.push(c); }
        }
        Some(result)
    } else if s.starts_with('<') && s.ends_with('>') {
        Some(decode_hex_string(&s[1..s.len()-1]))
    } else {
        None
    }
}

/// Extract text from a PDF array: [(Hello)-5(World)] TJ
fn extract_pdf_array_text(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') { return None; }

    let inner = &s[1..s.len()-1];
    let bytes = inner.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '(' {
            let mut depth = 1;
            let start = i + 1;
            i += 1;
            while i < bytes.len() && depth > 0 {
                let bc = bytes[i] as char;
                if bc == '(' && (i == 0 || bytes[i-1] as char != '\\') { depth += 1; }
                else if bc == ')' && (i == 0 || bytes[i-1] as char != '\\') { depth -= 1; }
                i += 1;
            }
            let string_content = std::str::from_utf8(&bytes[start..i-1]).unwrap_or("");
            if let Some(extracted) = extract_pdf_string(&format!("({})", string_content)) {
                result.push_str(&extracted);
            }
        } else if c == '<' {
            let start = i + 1;
            i += 1;
            while i < bytes.len() && bytes[i] as char != '>' { i += 1; }
            let hex_content = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
            result.push_str(&decode_hex_string(hex_content));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '.' {
            i += 1;
            while i < bytes.len() {
                let bc = bytes[i] as char;
                if bc.is_ascii_digit() || bc == '.' || bc == '-' { i += 1; }
                else { break; }
            }
        } else { i += 1; }
    }
    Some(result)
}

fn decode_hex_string(hex: &str) -> String {
    let hex = hex.trim();
    let mut result = String::new();
    let mut i = 0;
    while i + 2 <= hex.len() {
        if let Ok(byte) = u8::from_str_radix(&hex[i..i+2], 16) { result.push(byte as char); }
        i += 2;
    }
    result
}