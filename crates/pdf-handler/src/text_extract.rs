use crate::reader::PdfReader;
use handler_common::{BBoxSpan, StyleSpan, TextOffsetMap};

/// Extract text from PDF with offset→path mapping, including bbox and style info.
/// Uses pdf_oxide for text extraction while keeping the same output format.
pub struct PdfTextExtractor {
    reader: PdfReader,
}

impl PdfTextExtractor {
    pub fn new(reader: PdfReader) -> Self {
        Self { reader }
    }

    pub fn extract_with_offsets(&self) -> TextOffsetMap {
        let mut map = TextOffsetMap::empty("pdf");

        for page_num in 1..=self.reader.page_count() {
            let page_path = format!("/page[{}]", page_num);

            if let Some(spans) = self.reader.extract_page_spans(page_num) {
                for (i, span) in spans.iter().enumerate() {
                    let text_path = format!("{}/text[{}]", page_path, i + 1);

                    let bbox = Some(BBoxSpan {
                        x: span.bbox_x,
                        y: span.bbox_y,
                        width: span.bbox_width,
                        height: span.bbox_height,
                    });

                    let style = Some(StyleSpan {
                        font: Some(span.font_name.clone()),
                        size: Some(span.font_size),
                        color: None,
                    });

                    map.push_span_with_metadata(&span.text, &text_path, "text-block", bbox, style);
                    map.push_span_with_metadata("\n", &page_path, "line-break", None, None);
                }
            }

            if page_num < self.reader.page_count() {
                map.push_span_with_metadata(
                    "\n\n",
                    &format!("/page[{}]", page_num),
                    "page-break",
                    None,
                    None,
                );
            }
        }

        map.meta.total_chars = map.full_text.chars().count();
        map.meta.total_spans = map.spans.len();
        map
    }

    pub fn extract_text(&self) -> String {
        self.reader.extract_all_text()
    }
}
