use serde::{Deserialize, Serialize};

/// PPTX namespace constants.
pub const NS_P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Slide ID entry from presentation.xml <p:sldIdLst>.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideIdEntry {
    /// Slide ID attribute (e.g. "256")
    pub id: String,
    /// Relationship ID pointing to the slide part (e.g. "rId2")
    pub r_id: String,
}

/// A table node in the presentation DOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableNode {
    pub rows: Vec<TableRow>,
    pub grid_col_count: u32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub height: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub text: String,
    pub col_span: Option<u32>,
    pub row_span: Option<u32>,
    pub col_idx: u32,
    pub row_idx: u32,
}

/// A parsed shape on a slide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shape {
    /// Shape name from <p:nvSpPr>/<p:cNvPr name="...">
    pub name: String,
    /// Shape ID from <p:nvSpPr>/<p:cNvPr id="...">
    pub id: String,
    /// Placeholder type if this is a placeholder shape (e.g. "title", "ctrTitle", "subTitle", "body")
    pub placeholder_type: Option<String>,
    /// All text content concatenated from <p:txBody>
    pub text: String,
    /// Individual paragraphs in the shape's text body
    pub paragraphs: Vec<Paragraph>,
    /// Bounding box from <a:xfrm> (position/size in EMU, converted to points)
    pub bbox: Option<handler_common::BBoxSpan>,
    /// Whether this shape is a table (<p:graphicFrame> containing <a:tbl>)
    pub is_table: bool,
    /// Table data if this shape is a table
    pub table: Option<TableNode>,
}

/// A paragraph within a shape's text body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    /// All runs concatenated
    pub text: String,
    /// Individual runs
    pub runs: Vec<Run>,
}

/// A run (<a:r>) within a paragraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    /// Text content from <a:t>
    pub text: String,
}

/// A fully parsed slide with its shapes.
#[derive(Debug, Clone)]
pub struct Slide {
    /// Slide index (1-based)
    pub index: usize,
    /// Part path in the ZIP (e.g. "ppt/slides/slide1.xml")
    pub part_path: String,
    /// Slide ID from presentation.xml
    pub slide_id: String,
    /// Shapes on this slide (in document order)
    pub shapes: Vec<Shape>,
    /// Whether this slide has a morph transition
    pub has_morph: bool,
    /// Number of morph candidate shapes (shapes with matching names on adjacent slides)
    pub morph_candidates: usize,
}

/// The parsed presentation model.
#[derive(Debug, Clone)]
pub struct Presentation {
    /// Ordered list of slides
    pub slides: Vec<Slide>,
}
