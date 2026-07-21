use crate::content_stream::{parse_page_content_stream, ParsedContentStream};
use handler_common::HandlerError;
use lopdf::Document as LopdfDocument;
use std::cell::RefCell;

/// PDF document reader using pdf_oxide for text extraction and lopdf for editing.
pub struct PdfReader {
    doc: LopdfDocument,
    oxide_doc: RefCell<pdf_oxide::PdfDocument>,
    page_count: usize,
    file_path: String,
}

impl PdfReader {
    /// Open a PDF document.
    pub fn open(path: &str) -> Result<Self, HandlerError> {
        let mut doc = LopdfDocument::load(path)
            .map_err(|e| HandlerError::OpenError(format!("failed to open PDF: {}", e)))?;
        doc.decompress();
        let page_count = Self::count_pages(&doc);
        let oxide_doc = pdf_oxide::PdfDocument::open(path)
            .map_err(|e| HandlerError::OpenError(format!("failed to open PDF with pdf_oxide: {}", e)))?;
        Ok(Self {
            doc,
            oxide_doc: RefCell::new(oxide_doc),
            page_count,
            file_path: path.to_string(),
        })
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }
    pub fn document(&self) -> &LopdfDocument {
        &self.doc
    }
    pub fn document_mut(&mut self) -> &mut LopdfDocument {
        &mut self.doc
    }
    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    /// Recount pages from the document (e.g. after deleting a page).
    pub fn recount_pages(&mut self) {
        self.page_count = Self::count_pages(&self.doc);
    }

    /// Create a fallback reader with an empty document (used when re-loading fails).
    pub fn fallback(page_count: usize, file_path: &str) -> Self {
        let doc = LopdfDocument::new();
        // Create a minimal pdf_oxide document from bytes
        let oxide_doc = pdf_oxide::PdfDocument::from_bytes(Vec::new())
            .unwrap_or_else(|_| {
                // If we can't create an empty doc, use a minimal valid PDF
                pdf_oxide::PdfDocument::from_bytes(
                    b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[]/Count 0>>endobj\nxref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \ntrailer<</Size 3/Root 1 0 R>>\nstartxref\n117\n%%EOF".to_vec()
                ).expect("fallback PDF creation failed")
            });
        Self {
            doc,
            oxide_doc: RefCell::new(oxide_doc),
            page_count,
            file_path: file_path.to_string(),
        }
    }

    /// Extract text from all pages using pdf_oxide.
    pub fn extract_all_text(&self) -> String {
        let doc = self.oxide_doc.borrow();
        doc.extract_all_text().unwrap_or_default()
    }

    /// Extract text from a specific page using pdf_oxide.
    pub fn extract_page_text(&self, page_num: usize) -> Option<String> {
        let doc = self.oxide_doc.borrow();
        doc.extract_text(page_num - 1).ok()
    }

    /// Extract spans (text with font/bbox metadata) from a page using pdf_oxide.
    pub fn extract_page_spans(&self, page_num: usize) -> Option<Vec<PdfOxideSpan>> {
        let doc = self.oxide_doc.borrow();
        let spans = doc.extract_spans(page_num - 1).ok()?;
        Some(spans.into_iter().map(|s| PdfOxideSpan {
            text: s.text,
            font_name: s.font_name,
            font_size: s.font_size,
            bbox_x: s.bbox.x,
            bbox_y: s.bbox.y,
            bbox_width: s.bbox.width,
            bbox_height: s.bbox.height,
        }).collect())
    }

    /// Parse a page's content stream into structured text blocks with bbox info.
    /// Keeps lopdf-based parsing for backward compat with content stream editing.
    pub fn parse_page_text_blocks(&self, page_num: usize) -> Option<ParsedContentStream> {
        let pages = self.doc.get_pages();
        let page_id = pages.get(&(page_num as u32))?;
        let content = self.doc.get_page_content(*page_id).ok()?;
        parse_page_content_stream(&content, *page_id, &self.doc).ok()
    }

    fn count_pages(doc: &LopdfDocument) -> usize {
        doc.get_pages().len()
    }
}

/// A simplified span from pdf_oxide text extraction.
#[derive(Debug, Clone)]
pub struct PdfOxideSpan {
    pub text: String,
    pub font_name: String,
    pub font_size: f32,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_width: f32,
    pub bbox_height: f32,
}
