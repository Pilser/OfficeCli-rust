use layout_engine::PositionedPage;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};

pub struct PdfRenderer;

pub struct PdfOptions {
    pub paper_size: String,
    pub margin: String,
    pub embed_fonts: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            paper_size: "A4".to_string(),
            margin: "72".to_string(),
            embed_fonts: false,
        }
    }
}

impl PdfRenderer {
    pub fn render_document(
        &self,
        pages: &[PositionedPage],
        opts: &PdfOptions,
    ) -> Result<Vec<u8>, String> {
        let (page_w, page_h) = parse_paper_size(&opts.paper_size);

        let mut pdf = Pdf::new();

        let catalog_id = Ref::new(1);
        let page_tree_id = Ref::new(2);

    let mut next_ref = 3i32;
    let page_ids: Vec<Ref> = pages
        .iter()
        .map(|_| {
            let id = Ref::new(next_ref);
            next_ref += 1;
            id
        })
        .collect();

        pdf.catalog(catalog_id).pages(page_tree_id);
        pdf.pages(page_tree_id)
            .kids(page_ids.iter().copied())
            .count(page_ids.len() as i32);

        for (i, page_ref) in page_ids.iter().enumerate() {
            let content_id = Ref::new(next_ref);
            next_ref += 1;
            let font_id = Ref::new(next_ref);
            next_ref += 1;
            let font_name = Name(b"F1");

            {
                let mut page = pdf.page(*page_ref);
                page.parent(page_tree_id);
                page.media_box(Rect::new(0.0, 0.0, page_w, page_h));
                page.contents(content_id);
                page.resources().fonts().pair(font_name, font_id);
                page.finish();
            }

            pdf.type1_font(font_id).base_font(Name(b"Helvetica"));

            let content_bytes = build_content_stream(&pages[i], page_h);
            pdf.stream(content_id, &content_bytes);
        }

        Ok(pdf.finish())
    }
}

fn parse_paper_size(size: &str) -> (f32, f32) {
    match size.to_lowercase().as_str() {
        "a4" => (595.0, 842.0),
        "letter" => (612.0, 792.0),
        "legal" => (612.0, 1008.0),
        _ => (595.0, 842.0),
    }
}

fn build_content_stream(page: &PositionedPage, page_h: f32) -> Vec<u8> {
    let mut content = Content::new();
    let font_name = Name(b"F1");

    for element in &page.elements {
        let x = element.bbox.x;
        let y = page_h - element.bbox.y - element.bbox.height;
        let size = element.style.size.unwrap_or(12.0);

        content.begin_text();
        content.set_font(font_name, size);
        content.next_line(x, y);
        content.show(Str(element.text.as_bytes()));
        content.end_text();
    }

    content.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use handler_common::BBoxSpan;
    use layout_engine::PositionedElement;

    fn make_test_pages() -> Vec<PositionedPage> {
        vec![PositionedPage {
            width: 612.0,
            height: 792.0,
            elements: vec![PositionedElement {
                path: "/page[1]/text[1]".to_string(),
                bbox: BBoxSpan {
                    x: 50.0,
                    y: 50.0,
                    width: 200.0,
                    height: 20.0,
                },
                text: "Hello PDF".to_string(),
                style: handler_common::StyleSpan {
                    font: None,
                    size: Some(14.0),
                    color: None,
                },
                element_type: "text".to_string(),
                children: vec![],
            }],
        }]
    }

    #[test]
    fn test_render_pdf_returns_bytes() {
        let pages = make_test_pages();
        let opts = PdfOptions::default();
        let result = PdfRenderer.render_document(&pages, &opts).unwrap();
        assert!(!result.is_empty());
        assert_eq!(&result[..5], b"%PDF-");
    }

    #[test]
    fn test_parse_paper_size_default() {
        let (w, h) = parse_paper_size("unknown");
        assert!((w - 595.0).abs() < 0.1);
        assert!((h - 842.0).abs() < 0.1);
    }

    #[test]
    fn test_render_multiple_pages() {
        let pages = vec![
            PositionedPage {
                width: 595.0,
                height: 842.0,
                elements: vec![],
            },
            PositionedPage {
                width: 595.0,
                height: 842.0,
                elements: vec![],
            },
        ];
        let opts = PdfOptions::default();
        let result = PdfRenderer.render_document(&pages, &opts).unwrap();
        assert!(&result[..5] == b"%PDF-");
    }
}
