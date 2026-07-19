use crate::dom_types::{WordElementType, WordNode};

const DEFAULT_PAGE_WIDTH: f32 = 612.0;
const DEFAULT_PAGE_HEIGHT: f32 = 792.0;
const DEFAULT_MARGIN: f32 = 72.0;
const DEFAULT_LINE_HEIGHT: f32 = 12.0;
const TWIP_PER_PT: f32 = 20.0;
const EMU_PER_PT: f32 = 12700.0;

pub fn twip_to_pt(twips: &str) -> f32 {
    twips.parse::<f32>().unwrap_or(0.0) / TWIP_PER_PT
}

pub fn emu_to_pt(emu: &str) -> f32 {
    emu.parse::<f32>().unwrap_or(0.0) / EMU_PER_PT
}

pub struct DocxLayout {
    pub page_width: f32,
    pub page_height: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub margin_top: f32,
    pub current_y: f32,
}

impl DocxLayout {
    pub fn new() -> Self {
        Self {
            page_width: DEFAULT_PAGE_WIDTH,
            page_height: DEFAULT_PAGE_HEIGHT,
            margin_left: DEFAULT_MARGIN,
            margin_right: DEFAULT_MARGIN,
            margin_top: DEFAULT_MARGIN,
            current_y: DEFAULT_MARGIN,
        }
    }

    pub fn content_width(&self) -> f32 {
        self.page_width - self.margin_left - self.margin_right
    }

    pub fn read_section_properties(&mut self, node: &WordNode) {
        for child in &node.children {
            if child.element_type != WordElementType::SectionProperties {
                continue;
            }
            for prop in &child.children {
                match prop.element_type {
                    WordElementType::Unknown(ref n) if n == "pgSz" => {
                        if let Some(w) = prop.attributes.get("w") {
                            let v = twip_to_pt(w);
                            if v > 0.0 {
                                self.page_width = v;
                            }
                        }
                        if let Some(h) = prop.attributes.get("h") {
                            let v = twip_to_pt(h);
                            if v > 0.0 {
                                self.page_height = v;
                            }
                        }
                    }
                    WordElementType::Unknown(ref n) if n == "pgMar" => {
                        if let Some(left) = prop.attributes.get("left") {
                            self.margin_left = twip_to_pt(left);
                        }
                        if let Some(right) = prop.attributes.get("right") {
                            self.margin_right = twip_to_pt(right);
                        }
                        if let Some(top) = prop.attributes.get("top") {
                            self.margin_top = twip_to_pt(top);
                            self.current_y = self.margin_top;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParaLayoutInfo {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct TableLayoutInfo {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rows: Vec<RowLayoutInfo>,
}

#[derive(Debug, Clone)]
pub struct RowLayoutInfo {
    pub y: f32,
    pub height: f32,
    pub cells: Vec<CellLayoutInfo>,
}

#[derive(Debug, Clone)]
pub struct CellLayoutInfo {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn get_alignment(ppr: &WordNode) -> Option<&str> {
    ppr.children
        .iter()
        .find(|c| matches!(&c.element_type, WordElementType::Unknown(n) if n == "jc"))
        .and_then(|c| c.attributes.get("val"))
        .map(|s| s.as_str())
}

pub fn get_indent_pt(ppr: &WordNode) -> (f32, f32, f32, f32) {
    if let Some(ind) = ppr
        .children
        .iter()
        .find(|c| matches!(&c.element_type, WordElementType::Unknown(n) if n == "ind"))
    {
        let left = ind.attributes.get("left").map(|v| twip_to_pt(v)).unwrap_or(0.0);
        let right = ind.attributes.get("right").map(|v| twip_to_pt(v)).unwrap_or(0.0);
        let first_line = ind.attributes.get("firstLine").map(|v| twip_to_pt(v)).unwrap_or(0.0);
        let hanging = ind.attributes.get("hanging").map(|v| twip_to_pt(v)).unwrap_or(0.0);
        (left, right, first_line, hanging)
    } else {
        (0.0, 0.0, 0.0, 0.0)
    }
}

pub fn get_spacing_pt(ppr: &WordNode) -> (f32, f32, f32) {
    if let Some(spacing) = ppr
        .children
        .iter()
        .find(|c| matches!(&c.element_type, WordElementType::Unknown(n) if n == "spacing"))
    {
        let before = spacing.attributes.get("before").map(|v| twip_to_pt(v)).unwrap_or(0.0);
        let after = spacing.attributes.get("after").map(|v| twip_to_pt(v)).unwrap_or(0.0);
        let line = spacing
            .attributes
            .get("line")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| v / 240.0 * DEFAULT_LINE_HEIGHT)
            .unwrap_or(DEFAULT_LINE_HEIGHT);
        (before, after, line)
    } else {
        (0.0, 0.0, DEFAULT_LINE_HEIGHT)
    }
}

pub fn estimate_para_height(text: &str, width: f32, line_height: f32) -> f32 {
    if width <= 0.0 || text.is_empty() {
        return line_height;
    }
    let avg_char_width = line_height * 0.5;
    let chars_per_line = (width / avg_char_width).max(1.0) as usize;
    let line_count = {
        let from_newlines = text.chars().filter(|&c| c == '\n').count() + 1;
        let from_wrapping = (text.chars().count() + chars_per_line - 1) / chars_per_line.max(1);
        from_newlines.max(from_wrapping)
    };
    (line_count as f32) * line_height
}

pub fn calc_para_layout(para: &WordNode, layout: &mut DocxLayout) -> ParaLayoutInfo {
    let ppr = para.paragraph_properties();
    let (left_indent, right_indent, _first_line, _hanging) =
        ppr.map(|p| get_indent_pt(p)).unwrap_or_default();
    let (spacing_before, spacing_after, line_height) =
        ppr.map(|p| get_spacing_pt(p)).unwrap_or((0.0, 0.0, DEFAULT_LINE_HEIGHT));

    let x = layout.margin_left + left_indent;
    let para_width = (layout.page_width - layout.margin_left - layout.margin_right - left_indent - right_indent).max(0.0);
    let text = para.paragraph_text();
    let para_height = estimate_para_height(&text, para_width, line_height);

    let y = layout.current_y + spacing_before;
    let height = para_height.max(line_height);
    layout.current_y = y + para_height + spacing_after;

    ParaLayoutInfo {
        x,
        y,
        width: para_width,
        height,
    }
}

pub fn calc_table_layout(tbl: &WordNode, layout: &mut DocxLayout) -> TableLayoutInfo {
    let mut tbl_width = layout.content_width();
    if let Some(tbl_pr) = tbl
        .children
        .iter()
        .find(|c| c.element_type == WordElementType::TableProperties)
    {
        if let Some(tbl_w) = tbl_pr
            .children
            .iter()
            .find(|c| matches!(&c.element_type, WordElementType::Unknown(n) if n == "tblW"))
        {
            if let Some(w) = tbl_w.attributes.get("w") {
                let parsed = twip_to_pt(w);
                if parsed > 0.0 {
                    tbl_width = parsed;
                }
            }
        }
    }

    let x = layout.margin_left;
    let mut y = layout.current_y;
    let mut total_height = 0.0;
    let mut rows = Vec::new();

    for row_child in &tbl.children {
        if row_child.element_type != WordElementType::TableRow {
            continue;
        }
        let mut row_height = 20.0;
        if let Some(tr_pr) = row_child
            .children
            .iter()
            .find(|c| c.element_type == WordElementType::TableRowProperties)
        {
            if let Some(tr_h) = tr_pr
                .children
                .iter()
                .find(|c| matches!(&c.element_type, WordElementType::Unknown(n) if n == "trHeight"))
            {
                if let Some(val) = tr_h.attributes.get("val") {
                    row_height = twip_to_pt(val).max(10.0);
                }
            }
        }

        let cell_count = row_child
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::TableCell)
            .count() as f32;
        let default_cell_width = if cell_count > 0.0 { tbl_width / cell_count } else { tbl_width };

        let mut cells = Vec::new();
        let mut cell_x = x;
        for tc_child in &row_child.children {
            if tc_child.element_type != WordElementType::TableCell {
                continue;
            }
            let mut cell_width = default_cell_width;
            if let Some(tc_pr) = tc_child
                .children
                .iter()
                .find(|c| c.element_type == WordElementType::TableCellProperties)
            {
                if let Some(tc_w) = tc_pr
                    .children
                    .iter()
                    .find(|c| matches!(&c.element_type, WordElementType::Unknown(n) if n == "tcW"))
                {
                    if let Some(w) = tc_w.attributes.get("w") {
                        let parsed = twip_to_pt(w);
                        if parsed > 0.0 {
                            cell_width = parsed;
                        }
                    }
                }
            }
            cells.push(CellLayoutInfo {
                x: cell_x,
                y,
                width: cell_width,
                height: row_height,
            });
            cell_x += cell_width;
        }

        rows.push(RowLayoutInfo {
            y,
            height: row_height,
            cells,
        });
        y += row_height;
        total_height += row_height;
    }

    layout.current_y = y;
    TableLayoutInfo {
        x,
        y: y - total_height,
        width: tbl_width,
        height: total_height,
        rows,
    }
}

pub fn drawing_extent_in_para(para: &WordNode) -> Option<(f32, f32)> {
    for child in &para.children {
        if child.element_type == WordElementType::Run {
            if let Some(d) = find_extent_in_drawing(child) {
                return Some(d);
            }
        }
    }
    None
}

fn find_extent_in_drawing(node: &WordNode) -> Option<(f32, f32)> {
    if node.element_type == WordElementType::Drawing {
        for anchor_child in &node.children {
            for grandchild in &anchor_child.children {
                if matches!(&grandchild.element_type, WordElementType::Unknown(n) if n == "extent") {
                    let cx = grandchild.attributes.get("cx").map(|v| emu_to_pt(v)).unwrap_or(0.0);
                    let cy = grandchild.attributes.get("cy").map(|v| emu_to_pt(v)).unwrap_or(0.0);
                    if cx > 0.0 && cy > 0.0 {
                        return Some((cx, cy));
                    }
                }
            }
        }
        return None;
    }
    for child in &node.children {
        if let Some(d) = find_extent_in_drawing(child) {
            return Some(d);
        }
    }
    None
}
