use layout_engine::{LayoutEngine, PositionedPage, PositionedElement};

pub struct PngRenderer {
    layout: LayoutEngine,
}

impl PngRenderer {
    pub fn new(layout: LayoutEngine) -> Self {
        Self { layout }
    }

    /// Render a single page to PNG bytes.
    pub fn render_page(&self, page: &PositionedPage, scale: f32) -> Result<Vec<u8>, String> {
        use tiny_skia::*;

        let w = (page.width * scale) as u32;
        let h = (page.height * scale) as u32;

        let mut pixmap = Pixmap::new(w, h)
            .ok_or("failed to create pixmap")?;

        pixmap.fill(Color::WHITE);

        let mut paint = Paint::default();
        paint.set_color_rgba8(0, 0, 0, 255);

        for element in &page.elements {
            let x = (element.bbox.x * scale) as i32;
            let y = (element.bbox.y * scale) as i32;
            let ew = (element.bbox.width * scale) as i32;
            let eh = (element.bbox.height * scale) as i32;

            let rect = Rect::from_xywh(
                x as f32, y as f32,
                ew as f32, eh as f32,
            ).ok_or("invalid rect")?;

            // Draw element border
            let path = PathBuilder::from_rect(rect);
            let stroke = Stroke::default();
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

            // Fill element background (placeholder)
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }

        pixmap.encode_png()
            .map_err(|e| format!("encode png: {}", e))
    }

    /// Render a single element with zoom.
    pub fn render_element(&self, element: &PositionedElement, zoom: f32) -> Result<Vec<u8>, String> {
        use tiny_skia::*;

        let w = (element.bbox.width * zoom) as u32;
        let h = (element.bbox.height * zoom) as u32;

        let mut pixmap = Pixmap::new(w.max(1), h.max(1))
            .ok_or("failed to create pixmap")?;

        pixmap.fill(Color::WHITE);

        let mut paint = Paint::default();
        paint.set_color_rgba8(0, 0, 0, 255);

        let rect = Rect::from_xywh(0.0, 0.0, w as f32, h as f32)
            .ok_or("invalid rect")?;

        let path = PathBuilder::from_rect(rect);
        let stroke = Stroke::default();
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

        pixmap.encode_png()
            .map_err(|e| format!("encode png: {}", e))
    }

    /// Render multiple pages in a grid.
    pub fn render_grid(&self, pages: &[PositionedPage], cols: u32) -> Result<Vec<u8>, String> {
        if pages.is_empty() {
            return Err("no pages to render".to_string());
        }

        let scale = 0.5;
        let thumb_w = (pages[0].width * scale) as u32;
        let thumb_h = (pages[0].height * scale) as u32;
        let gap = 10u32;
        let cols = cols.max(1);
        let rows = ((pages.len() as u32 + cols - 1) / cols).max(1);

        let total_w = cols * (thumb_w + gap) + gap;
        let total_h = rows * (thumb_h + gap) + gap;

        use tiny_skia::*;
        let mut pixmap = Pixmap::new(total_w, total_h)
            .ok_or("failed to create grid pixmap")?;
        pixmap.fill(Color::WHITE);

        for (i, page) in pages.iter().enumerate() {
            let col = i as u32 % cols;
            let row = i as u32 / cols;
            let ox = gap + col * (thumb_w + gap);
            let oy = gap + row * (thumb_h + gap);

            let thumbnail = self.render_page(page, scale)?;
            if let Ok(thumb_img) = image::load_from_memory(&thumbnail) {
                let thumb_rgba = thumb_img.to_rgba8();
                for py in 0..thumb_h {
                    for px in 0..thumb_w {
                        let src_pixel = thumb_rgba.get_pixel(
                            px.min(thumb_img.width() - 1),
                            py.min(thumb_img.height() - 1),
                        );
                        let dx = ox + px;
                        let dy = oy + py;
                        if dx < total_w && dy < total_h {
                            let idx = (dy * pixmap.width() + dx) as usize * 4;
                            let data = pixmap.data_mut();
                            data[idx] = src_pixel[0];
                            data[idx + 1] = src_pixel[1];
                            data[idx + 2] = src_pixel[2];
                            data[idx + 3] = src_pixel[3];
                        }
                    }
                }
            }
        }

        pixmap.encode_png()
            .map_err(|e| format!("encode png: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout_engine::{LayoutEngine, PositionedPage, PositionedElement};
    use handler_common::BBoxSpan;

    fn make_test_page() -> PositionedPage {
        PositionedPage {
            width: 612.0,
            height: 792.0,
            elements: vec![
                PositionedElement {
                    path: "/page[1]/text[1]".to_string(),
                    bbox: BBoxSpan { x: 50.0, y: 50.0, width: 200.0, height: 20.0 },
                    text: "Hello".to_string(),
                    style: handler_common::StyleSpan { font: None, size: None, color: None },
                    element_type: "text".to_string(),
                    children: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_render_page_returns_bytes() {
        let engine = LayoutEngine::new(612.0, 792.0, 72.0);
        let renderer = PngRenderer::new(engine);
        let page = make_test_page();
        let result = renderer.render_page(&page, 1.0).unwrap();
        assert!(!result.is_empty());
        assert_eq!(&result[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn test_render_element() {
        let engine = LayoutEngine::new(612.0, 792.0, 72.0);
        let renderer = PngRenderer::new(engine);
        let element = &make_test_page().elements[0];
        let result = renderer.render_element(element, 2.0).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_render_grid_empty_fails() {
        let engine = LayoutEngine::new(612.0, 792.0, 72.0);
        let renderer = PngRenderer::new(engine);
        let result = renderer.render_grid(&[], 2);
        assert!(result.is_err());
    }
}
