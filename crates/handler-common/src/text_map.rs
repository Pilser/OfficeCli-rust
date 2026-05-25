use serde::{Deserialize, Serialize};

/// Single offset mapping entry: a text range maps to a document path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetSpan {
    /// Start offset in full text (UTF-8 byte offset)
    pub start: usize,
    /// End offset in full text (exclusive)
    pub end: usize,
    /// Document path ID (e.g., "/body/p[3]/r[1]", "/page[1]/text[2]")
    pub path: String,
    /// Original text content in this span
    pub text: String,
    /// Element type: "run", "paragraph-separator", "paragraph-break",
    /// "cell", "shape", "text-block", etc.
    pub element_type: String,
}

/// Full document text + offset→path mapping.
/// Each character's offset maps to the real document path ID,
/// enabling AI agents to precisely locate and modify elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOffsetMap {
    /// Complete concatenated text of the document
    pub full_text: String,
    /// Ordered list of offset spans covering every character in full_text
    pub spans: Vec<OffsetSpan>,
    /// Metadata about the document
    pub meta: TextMapMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMapMeta {
    /// Document format: "docx" | "xlsx" | "pptx" | "pdf"
    pub format: String,
    /// Total character count in full_text
    pub total_chars: usize,
    /// Total number of offset spans
    pub total_spans: usize,
}

impl TextOffsetMap {
    /// Create an empty TextOffsetMap for a given format
    pub fn empty(format: &str) -> Self {
        Self {
            full_text: String::new(),
            spans: Vec::new(),
            meta: TextMapMeta {
                format: format.to_string(),
                total_chars: 0,
                total_spans: 0,
            },
        }
    }

    /// Find the path ID for a given character offset.
    /// Returns the OffsetSpan that contains the character at `offset`.
    pub fn find_span_at_offset(&self, offset: usize) -> Option<&OffsetSpan> {
        if offset >= self.full_text.len() {
            return None;
        }
        self.spans.iter().find(|span| offset >= span.start && offset < span.end)
    }

    /// Find all spans whose path matches the given path prefix.
    pub fn spans_for_path(&self, path_prefix: &str) -> Vec<&OffsetSpan> {
        self.spans
            .iter()
            .filter(|span| span.path.starts_with(path_prefix))
            .collect()
    }

    /// Get the text content for a given path ID.
    pub fn text_for_path(&self, path: &str) -> Option<&str> {
        self.spans
            .iter()
            .find(|span| span.path == path)
            .map(|span| span.text.as_str())
    }

    /// Add a span to the map, extending full_text.
    pub fn push_span(&mut self, text: &str, path: &str, element_type: &str) {
        let start = self.full_text.len();
        self.full_text.push_str(text);
        let end = self.full_text.len();
        self.spans.push(OffsetSpan {
            start,
            end,
            path: path.to_string(),
            text: text.to_string(),
            element_type: element_type.to_string(),
        });
        self.meta.total_chars = self.full_text.len();
        self.meta.total_spans = self.spans.len();
    }
}