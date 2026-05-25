use crate::reader::PdfReader;
use handler_common::TextOffsetMap;

/// Extract text from PDF with offset→path mapping.
pub struct PdfTextExtractor {
    reader: PdfReader,
}

impl PdfTextExtractor {
    pub fn new(reader: PdfReader) -> Self { Self { reader } }

    pub fn extract_with_offsets(&self) -> TextOffsetMap {
        let mut map = TextOffsetMap::empty("pdf");
        let mut text_block_idx = 0;

        for page_num in 1..=self.reader.page_count() {
            let page_path = format!("/page[{}]", page_num);
            if let Some(page_text) = self.reader.extract_page_text(page_num) {
                if page_text.is_empty() { continue; }
                for line in page_text.lines() {
                    if line.is_empty() {
                        map.push_span("\n", &page_path, "paragraph-break");
                        continue;
                    }
                    text_block_idx += 1;
                    let text_path = format!("{}/text[{}]", page_path, text_block_idx);
                    map.push_span(line, &text_path, "text-block");
                    map.push_span("\n", &page_path, "line-break");
                }
            }
            if page_num < self.reader.page_count() {
                map.push_span("\n\n", &format!("/page[{}]", page_num), "page-break");
                text_block_idx = 0;
            }
        }
        map.meta.total_chars = map.full_text.len();
        map.meta.total_spans = map.spans.len();
        map
    }

    pub fn extract_text(&self) -> String { self.reader.extract_all_text() }
}