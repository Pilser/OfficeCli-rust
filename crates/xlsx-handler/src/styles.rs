use handler_common::HandlerError;
use std::collections::HashMap;

pub struct StylesModel {
    pub fonts: Vec<StyleFont>,
    pub fills: Vec<StyleFill>,
    pub borders: Vec<StyleBorder>,
    pub cell_style_xfs: Vec<StyleXf>,
    pub cell_xfs: Vec<StyleXf>,
    pub num_fmts: Vec<StyleNumFmt>,
    pub dxfs: Vec<StyleDxf>,
}

pub struct StyleFont {
    pub bold: bool,
    pub italic: bool,
    pub underline: Option<String>,
    pub strike: bool,
    pub size: Option<f64>,
    pub color: Option<String>,
    pub name: Option<String>,
    pub family: Option<u32>,
    pub charset: Option<u32>,
    pub scheme: Option<String>,
}

pub struct StyleFill {
    pub pattern_type: String,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
}

pub struct StyleBorder {
    pub left: Option<BorderEdge>,
    pub right: Option<BorderEdge>,
    pub top: Option<BorderEdge>,
    pub bottom: Option<BorderEdge>,
    pub diagonal: Option<BorderEdge>,
    pub diagonal_up: bool,
    pub diagonal_down: bool,
}

#[derive(Clone)]
pub struct BorderEdge {
    pub style: String,
    pub color: String,
}

pub struct StyleXf {
    pub font_id: u32,
    pub fill_id: u32,
    pub border_id: u32,
    pub num_fmt_id: u32,
    pub alignment: Option<StyleAlignment>,
    pub apply_font: bool,
    pub apply_fill: bool,
    pub apply_border: bool,
    pub apply_alignment: bool,
    pub apply_number_format: bool,
}

#[derive(Clone, PartialEq)]
pub struct StyleAlignment {
    pub horizontal: Option<String>,
    pub vertical: Option<String>,
    pub wrap_text: bool,
    pub indent: Option<u32>,
    pub text_rotation: Option<u32>,
}

pub struct StyleNumFmt {
    pub id: u32,
    pub format_code: String,
}

pub struct StyleDxf {
    pub font: Option<StyleFont>,
    pub fill: Option<StyleFill>,
    pub border: Option<StyleBorder>,
    pub num_fmt: Option<StyleNumFmt>,
    pub alignment: Option<StyleAlignment>,
}

#[allow(dead_code)]
const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

fn detect_prefix(xml: &str) -> String {
    if let Some(pos) = xml.find("styleSheet") {
        if let Some(lt_pos) = xml[..pos].rfind('<') {
            let prefix = &xml[lt_pos + 1..pos];
            if !prefix.is_empty() && prefix.ends_with(':') {
                return prefix.to_string();
            }
        }
    }
    String::new()
}

pub fn parse_styles_xml(xml: &str) -> Result<StylesModel, String> {
    let p = detect_prefix(xml);

    let fonts = parse_fonts(xml, &p);
    let fills = parse_fills(xml, &p);
    let borders = parse_borders(xml, &p);
    let cell_style_xfs = parse_xf_list(xml, &p, "cellStyleXfs");
    let cell_xfs = parse_xf_list(xml, &p, "cellXfs");
    let num_fmts = parse_num_fmts(xml, &p);
    let dxfs = parse_dxfs(xml, &p);

    Ok(StylesModel {
        fonts,
        fills,
        borders,
        cell_style_xfs,
        cell_xfs,
        num_fmts,
        dxfs,
    })
}

fn parse_fonts(xml: &str, p: &str) -> Vec<StyleFont> {
    let open = format!("<{}font", p);
    let close = format!("</{}font>", p);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find(&open) {
        let abs_start = cursor + start;
        let after_gt = xml[abs_start..].find('>').map(|i| abs_start + i + 1).unwrap_or(xml.len());
        let abs_close = xml[after_gt..].find(&close).map(|i| after_gt + i + close.len()).unwrap_or(xml.len());
        let block = &xml[abs_start..abs_close];

        let mut font = StyleFont {
            bold: false,
            italic: false,
            underline: None,
            strike: false,
            size: None,
            color: None,
            name: None,
            family: None,
            charset: None,
            scheme: None,
        };

        if block.contains(&format!("<{}b/>", p)) || block.contains(&format!("<{}b>", p)) {
            font.bold = true;
        }
        if block.contains(&format!("<{}italic/>", p)) || block.contains(&format!("<{}i>", p)) || block.contains(&format!("<{}i/>", p)) {
            font.italic = true;
        }
        if block.contains(&format!("<{}strike/>", p)) || block.contains(&format!("<{}strike>", p)) {
            font.strike = true;
        }

        if let Some(v) = extract_val_attr(block, "sz", p) {
            font.size = v.parse::<f64>().ok();
        }
        if let Some(v) = extract_val_attr(block, "name", p) {
            font.name = Some(v);
        }
        if let Some(v) = extract_val_attr(block, "family", p) {
            font.family = v.parse::<u32>().ok();
        }
        if let Some(v) = extract_val_attr(block, "charset", p) {
            font.charset = v.parse::<u32>().ok();
        }
        if let Some(v) = extract_val_attr(block, "scheme", p) {
            font.scheme = Some(v);
        }
        if block.contains(&format!("<{}u", p)) {
            font.underline = extract_val_attr(block, "u", p).or_else(|| Some("single".to_string()));
        }
        if let Some(c) = extract_color(block, p) {
            font.color = Some(c);
        }

        out.push(font);
        cursor = abs_close;
    }
    out
}

fn parse_fills(xml: &str, p: &str) -> Vec<StyleFill> {
    let open = format!("<{}fill", p);
    let close = format!("</{}fill>", p);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find(&open) {
        let abs_start = cursor + start;
        let after_gt = xml[abs_start..].find('>').map(|i| abs_start + i + 1).unwrap_or(xml.len());
        let abs_close = xml[after_gt..].find(&close).map(|i| after_gt + i + close.len()).unwrap_or(xml.len());
        let block = &xml[abs_start..abs_close];

        let pattern_type = extract_val_attr(block, "patternType", p).unwrap_or_else(|| "none".to_string());
        let fg_color = extract_color_from_child(block, "fgColor", p);
        let bg_color = extract_color_from_child(block, "bgColor", p);

        out.push(StyleFill { pattern_type, fg_color, bg_color });
        cursor = abs_close;
    }
    out
}

fn parse_borders(xml: &str, p: &str) -> Vec<StyleBorder> {
    let open = format!("<{}border", p);
    let close = format!("</{}border>", p);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find(&open) {
        let abs_start = cursor + start;
        let after_gt = xml[abs_start..].find('>').map(|i| abs_start + i + 1).unwrap_or(xml.len());
        let abs_close = xml[after_gt..].find(&close).map(|i| after_gt + i + close.len()).unwrap_or(xml.len());
        let block = &xml[abs_start..abs_close];

        let left = parse_border_edge(block, "left", p);
        let right = parse_border_edge(block, "right", p);
        let top = parse_border_edge(block, "top", p);
        let bottom = parse_border_edge(block, "bottom", p);
        let diagonal = parse_border_edge(block, "diagonal", p);
        let diagonal_up = block.contains("diagonalUp");
        let diagonal_down = block.contains("diagonalDown");

        out.push(StyleBorder { left, right, top, bottom, diagonal, diagonal_up, diagonal_down });
        cursor = abs_close;
    }
    out
}

fn parse_border_edge(block: &str, side: &str, p: &str) -> Option<BorderEdge> {
    let pattern = format!("<{}{}", p, side);
    let start = block.find(&pattern)?;
    let after_gt = block[start..].find('>').map(|i| start + i + 1)?;
    let close = format!("</{}{}>", p, side);
    let end = block[after_gt..].find(&close).map(|i| after_gt + i + close.len()).unwrap_or(after_gt);
    let edge_block = &block[start..end];

    let mut style = String::new();
    if let Some(s) = extract_val_attr(edge_block, "style", p) {
        style = s;
    }
    let color = extract_color(edge_block, p).unwrap_or_default();

    if style.is_empty() && color.is_empty() {
        return None;
    }
    Some(BorderEdge { style, color })
}

fn parse_xf_list(xml: &str, p: &str, parent_tag: &str) -> Vec<StyleXf> {
    let parent_open = format!("<{}{}", p, parent_tag);
    let parent_close = format!("</{}{}>", p, parent_tag);
    let parent_start = match xml.find(&parent_open) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let after_gt = xml[parent_start..].find('>').map(|i| parent_start + i + 1).unwrap_or(xml.len());
    let parent_end = xml[after_gt..].find(&parent_close).map(|i| after_gt + i).unwrap_or(xml.len());
    let parent_block = &xml[parent_start..parent_end];

    let open = format!("<{}xf", p);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = parent_block[cursor..].find(&open) {
        let abs_start = cursor + start;
        let gt = parent_block[abs_start..].find('>').map(|i| abs_start + i + 1).unwrap_or(parent_block.len());
        let tag_end = if parent_block.as_bytes().get(gt - 2) == Some(&b'/') { gt } else {
            let xf_close = format!("</{}xf>", p);
            parent_block[gt..].find(&xf_close).map(|i| gt + i + xf_close.len()).unwrap_or(parent_block.len())
        };
        let block = &parent_block[abs_start..tag_end];
        let xf = parse_single_xf(block, p);
        out.push(xf);
        cursor = tag_end;
    }
    out
}

fn parse_single_xf(block: &str, p: &str) -> StyleXf {
    let font_id = extract_attr(block, "fontId").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let fill_id = extract_attr(block, "fillId").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let border_id = extract_attr(block, "borderId").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let num_fmt_id = extract_attr(block, "numFmtId").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let apply_font = extract_attr(block, "applyFont").map(|v| v == "1").unwrap_or(false);
    let apply_fill = extract_attr(block, "applyFill").map(|v| v == "1").unwrap_or(false);
    let apply_border = extract_attr(block, "applyBorder").map(|v| v == "1").unwrap_or(false);
    let apply_alignment = extract_attr(block, "applyAlignment").map(|v| v == "1").unwrap_or(false);
    let apply_number_format = extract_attr(block, "applyNumberFormat").map(|v| v == "1").unwrap_or(false);
    let alignment = parse_alignment(block, p);

    StyleXf {
        font_id, fill_id, border_id, num_fmt_id,
        alignment, apply_font, apply_fill, apply_border,
        apply_alignment, apply_number_format,
    }
}

fn parse_alignment(block: &str, p: &str) -> Option<StyleAlignment> {
    let pattern = format!("<{}alignment", p);
    let start = block.find(&pattern)?;
    let tag_end = block[start..].find('>').map(|i| start + i + 1)?;
    let close = format!("</{}alignment>", p);
    let end = block[tag_end..].find(&close).map(|i| tag_end + i + close.len()).unwrap_or(tag_end);
    let al_block = &block[start..end];

    let horizontal = extract_attr(al_block, "horizontal");
    let vertical = extract_attr(al_block, "vertical");
    let wrap_text = extract_attr(al_block, "wrapText").map(|v| v == "1").unwrap_or(false);
    let indent = extract_attr(al_block, "indent").and_then(|v| v.parse::<u32>().ok());
    let text_rotation = extract_attr(al_block, "textRotation").and_then(|v| v.parse::<u32>().ok());

    Some(StyleAlignment { horizontal, vertical, wrap_text, indent, text_rotation })
}

fn parse_num_fmts(xml: &str, p: &str) -> Vec<StyleNumFmt> {
    let parent_open = format!("<{}numFmt", p);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find(&parent_open) {
        let abs_start = cursor + start;
        let gt = xml[abs_start..].find('>').map(|i| abs_start + i + 1).unwrap_or(xml.len());
        let tag_end = if xml.as_bytes().get(gt - 2) == Some(&b'/') { gt } else {
            let close = format!("</{}numFmt>", p);
            xml[gt..].find(&close).map(|i| gt + i + close.len()).unwrap_or(xml.len())
        };
        let block = &xml[abs_start..tag_end];

        let id = extract_attr(block, "numFmtId").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let format_code = extract_attr(block, "formatCode").unwrap_or_default();
        if id > 0 && !format_code.is_empty() {
            out.push(StyleNumFmt { id, format_code });
        }
        cursor = tag_end;
    }
    out
}

fn parse_dxfs(xml: &str, p: &str) -> Vec<StyleDxf> {
    let parent_open = format!("<{}dxf", p);
    let parent_close = format!("</{}dxf>", p);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find(&parent_open) {
        let abs_start = cursor + start;
        let after_gt = xml[abs_start..].find('>').map(|i| abs_start + i + 1).unwrap_or(xml.len());
        let abs_close = xml[after_gt..].find(&parent_close).map(|i| after_gt + i + parent_close.len()).unwrap_or(xml.len());
        let block = &xml[abs_start..abs_close];
        out.push(parse_single_dxf(block, p));
        cursor = abs_close;
    }
    out
}

fn parse_single_dxf(block: &str, p: &str) -> StyleDxf {
    let font = if block.contains(&format!("<{}font", p)) {
        Some(parse_single_font(block, p))
    } else {
        None
    };
    let fill = if block.contains(&format!("<{}fill", p)) {
        Some(parse_single_fill(block, p))
    } else {
        None
    };
    let border = if block.contains(&format!("<{}border", p)) {
        Some(parse_single_border(block, p))
    } else {
        None
    };
    let num_fmt = parse_single_numfmt_in_block(block, p);
    let alignment = parse_alignment(block, p);
    StyleDxf { font, fill, border, num_fmt, alignment }
}

fn parse_single_font(block: &str, p: &str) -> StyleFont {
    let open = format!("<{}font", p);
    let close = format!("</{}font>", p);
    let start = block.find(&open).unwrap_or(0);
    let after_gt = block[start..].find('>').map(|i| start + i + 1).unwrap_or(block.len());
    let end = block[after_gt..].find(&close).map(|i| after_gt + i + close.len()).unwrap_or(block.len());
    let inner = &block[start..end];
    let mut font = StyleFont {
        bold: false, italic: false, underline: None, strike: false,
        size: None, color: None, name: None, family: None,
        charset: None, scheme: None,
    };
    if inner.contains(&format!("<{}b/>", p)) || inner.contains(&format!("<{}b>", p)) { font.bold = true; }
    if let Some(v) = extract_val_attr(inner, "sz", p) { font.size = v.parse::<f64>().ok(); }
    if let Some(v) = extract_val_attr(inner, "name", p) { font.name = Some(v); }
    if let Some(c) = extract_color(inner, p) { font.color = Some(c); }
    font
}

fn parse_single_fill(block: &str, p: &str) -> StyleFill {
    let open = format!("<{}fill", p);
    let close = format!("</{}fill>", p);
    let start = block.find(&open).unwrap_or(0);
    let after_gt = block[start..].find('>').map(|i| start + i + 1).unwrap_or(block.len());
    let end = block[after_gt..].find(&close).map(|i| after_gt + i + close.len()).unwrap_or(block.len());
    let inner = &block[start..end];
    let pattern_type = extract_val_attr(inner, "patternType", p).unwrap_or_else(|| "none".to_string());
    let fg_color = extract_color_from_child(inner, "fgColor", p);
    let bg_color = extract_color_from_child(inner, "bgColor", p);
    StyleFill { pattern_type, fg_color, bg_color }
}

fn parse_single_border(block: &str, p: &str) -> StyleBorder {
    let open = format!("<{}border", p);
    let close = format!("</{}border>", p);
    let start = block.find(&open).unwrap_or(0);
    let after_gt = block[start..].find('>').map(|i| start + i + 1).unwrap_or(block.len());
    let end = block[after_gt..].find(&close).map(|i| after_gt + i + close.len()).unwrap_or(block.len());
    let inner = &block[start..end];
    StyleBorder {
        left: parse_border_edge(inner, "left", p),
        right: parse_border_edge(inner, "right", p),
        top: parse_border_edge(inner, "top", p),
        bottom: parse_border_edge(inner, "bottom", p),
        diagonal: parse_border_edge(inner, "diagonal", p),
        diagonal_up: inner.contains("diagonalUp"),
        diagonal_down: inner.contains("diagonalDown"),
    }
}

fn parse_single_numfmt_in_block(block: &str, p: &str) -> Option<StyleNumFmt> {
    let open = format!("<{}numFmt", p);
    let start = block.find(&open)?;
    let gt = block[start..].find('>').map(|i| start + i + 1)?;
    let end = if block.as_bytes().get(gt - 2) == Some(&b'/') { gt } else {
        let close = format!("</{}numFmt>", p);
        block[gt..].find(&close).map(|i| gt + i + close.len()).unwrap_or(block.len())
    };
    let inner = &block[start..end];
    let id = extract_attr(inner, "numFmtId").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let format_code = extract_attr(inner, "formatCode").unwrap_or_default();
    if id > 0 { Some(StyleNumFmt { id, format_code }) } else { None }
}

fn extract_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = xml.find(&pattern)?;
    let val_start = start + pattern.len();
    let end = xml[val_start..].find('"')?;
    Some(xml[val_start..val_start + end].to_string())
}

fn extract_val_attr(xml: &str, tag: &str, p: &str) -> Option<String> {
    let pattern = format!("<{}{}", p, tag);
    let start = xml.find(&pattern)?;
    let after = &xml[start..];
    let val_pattern = "val=\"";
    let v_start = after.find(val_pattern)?;
    let actual_start = v_start + val_pattern.len();
    let v_end = after[actual_start..].find('"')?;
    Some(after[actual_start..actual_start + v_end].to_string())
}

fn extract_color(block: &str, p: &str) -> Option<String> {
    let pattern = format!("<{}color ", p);
    let start = block.find(&pattern)?;
    let after = &block[start..];
    if let Some(rgb) = extract_attr(after, "rgb") {
        return Some(rgb);
    }
    if let Some(indexed) = extract_attr(after, "indexed") {
        return Some(indexed);
    }
    if let Some(themed) = extract_attr(after, "theme") {
        return Some(themed);
    }
    None
}

fn extract_color_from_child(block: &str, child_tag: &str, p: &str) -> Option<String> {
    let pattern = format!("<{}{}", p, child_tag);
    let start = block.find(&pattern)?;
    let after = &block[start..];
    if let Some(rgb) = extract_attr(after, "rgb") {
        return Some(rgb);
    }
    if let Some(themed) = extract_attr(after, "theme") {
        return Some(themed);
    }
    None
}

pub fn serialize_styles_xml(model: &StylesModel) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\n"
    );

    xml.push_str(&format!("<fonts count=\"{}\">\n", model.fonts.len()));
    for font in &model.fonts {
        xml.push_str("  <font>\n");
        if font.bold { xml.push_str("    <b/>\n"); }
        if font.italic { xml.push_str("    <i/>\n"); }
        if font.strike { xml.push_str("    <strike/>\n"); }
        if let Some(ref u) = font.underline {
            xml.push_str(&format!("    <u val=\"{}\"/>\n", u));
        }
        if let Some(sz) = font.size {
            xml.push_str(&format!("    <sz val=\"{}\"/>\n", sz));
        }
        if let Some(ref name) = font.name {
            xml.push_str(&format!("    <name val=\"{}\"/>\n", name));
        }
        if let Some(family) = font.family {
            xml.push_str(&format!("    <family val=\"{}\"/>\n", family));
        }
        if let Some(charset) = font.charset {
            xml.push_str(&format!("    <charset val=\"{}\"/>\n", charset));
        }
        if let Some(ref color) = font.color {
            xml.push_str(&format!("    <color rgb=\"{}\"/>\n", color));
        }
        if let Some(ref scheme) = font.scheme {
            xml.push_str(&format!("    <scheme val=\"{}\"/>\n", scheme));
        }
        xml.push_str("  </font>\n");
    }
    xml.push_str("</fonts>\n");

    xml.push_str(&format!("<fills count=\"{}\">\n", model.fills.len()));
    for fill in &model.fills {
        xml.push_str("  <fill>\n");
        xml.push_str(&format!("    <patternFill patternType=\"{}\"", fill.pattern_type));
        if fill.fg_color.is_some() || fill.bg_color.is_some() {
            xml.push_str(">\n");
            if let Some(ref c) = fill.fg_color {
                xml.push_str(&format!("      <fgColor rgb=\"{}\"/>\n", c));
            }
            if let Some(ref c) = fill.bg_color {
                xml.push_str(&format!("      <bgColor rgb=\"{}\"/>\n", c));
            }
            xml.push_str("    </patternFill>\n");
        } else {
            xml.push_str("/>\n");
        }
        xml.push_str("  </fill>\n");
    }
    xml.push_str("</fills>\n");

    xml.push_str(&format!("<borders count=\"{}\">\n", model.borders.len()));
    for border in &model.borders {
        xml.push_str("  <border>\n");
        xml.push_str(&border_edge_xml("left", &border.left));
        xml.push_str(&border_edge_xml("right", &border.right));
        xml.push_str(&border_edge_xml("top", &border.top));
        xml.push_str(&border_edge_xml("bottom", &border.bottom));
        xml.push_str(&border_edge_xml("diagonal", &border.diagonal));
        if border.diagonal_up { xml.push_str("    <diagonalUp/>\n"); }
        if border.diagonal_down { xml.push_str("    <diagonalDown/>\n"); }
        xml.push_str("  </border>\n");
    }
    xml.push_str("</borders>\n");

    xml.push_str(&format!("<cellStyleXfs count=\"{}\">\n", model.cell_style_xfs.len()));
    for xf in &model.cell_style_xfs {
        xml.push_str(&serialize_xf(xf, false));
    }
    xml.push_str("</cellStyleXfs>\n");

    xml.push_str(&format!("<cellXfs count=\"{}\">\n", model.cell_xfs.len()));
    for xf in &model.cell_xfs {
        xml.push_str(&serialize_xf(xf, true));
    }
    xml.push_str("</cellXfs>\n");

    if !model.num_fmts.is_empty() {
        xml.push_str(&format!("<numFmts count=\"{}\">\n", model.num_fmts.len()));
        for nf in &model.num_fmts {
            xml.push_str(&format!("  <numFmt numFmtId=\"{}\" formatCode=\"{}\"/>\n", nf.id, nf.format_code));
        }
        xml.push_str("</numFmts>\n");
    }

    if !model.dxfs.is_empty() {
        xml.push_str(&format!("<dxfs count=\"{}\">\n", model.dxfs.len()));
        for dxf in &model.dxfs {
            xml.push_str("  <dxf>\n");
            if let Some(ref f) = dxf.font {
                xml.push_str("    <font>\n");
                if f.bold { xml.push_str("      <b/>\n"); }
                if f.italic { xml.push_str("      <i/>\n"); }
                if let Some(sz) = f.size { xml.push_str(&format!("      <sz val=\"{}\"/>\n", sz)); }
                if let Some(ref n) = f.name { xml.push_str(&format!("      <name val=\"{}\"/>\n", n)); }
                if let Some(ref c) = f.color { xml.push_str(&format!("      <color rgb=\"{}\"/>\n", c)); }
                xml.push_str("    </font>\n");
            }
            if let Some(ref f) = dxf.fill {
                xml.push_str("    <fill>\n");
                xml.push_str(&format!("      <patternFill patternType=\"{}\"", f.pattern_type));
                if let Some(ref c) = f.fg_color {
                    xml.push_str(&format!("><fgColor rgb=\"{}\"/></patternFill>\n", c));
                } else {
                    xml.push_str("/>\n");
                }
                xml.push_str("    </fill>\n");
            }
            if let Some(ref b) = dxf.border {
                xml.push_str("    <border>\n");
                xml.push_str(&border_edge_xml("left", &b.left));
                xml.push_str(&border_edge_xml("right", &b.right));
                xml.push_str(&border_edge_xml("top", &b.top));
                xml.push_str(&border_edge_xml("bottom", &b.bottom));
                xml.push_str(&border_edge_xml("diagonal", &b.diagonal));
                xml.push_str("    </border>\n");
            }
            if let Some(ref nf) = dxf.num_fmt {
                xml.push_str(&format!("    <numFmt numFmtId=\"{}\" formatCode=\"{}\"/>\n", nf.id, nf.format_code));
            }
            if let Some(ref a) = dxf.alignment {
                xml.push_str(&alignment_xml(a, "    "));
            }
            xml.push_str("  </dxf>\n");
        }
        xml.push_str("</dxfs>\n");
    }

    xml.push_str("<cellStyles count=\"1\">\n  <cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/>\n</cellStyles>\n");
    xml.push_str("</styleSheet>\n");
    xml
}

fn border_edge_xml(side: &str, edge: &Option<BorderEdge>) -> String {
    match edge {
        Some(e) => {
            if e.style.is_empty() && e.color.is_empty() {
                return format!("    <{}/>\n", side);
            }
            format!("    <{} style=\"{}\"><color rgb=\"{}\"/></{}>\n", side, e.style, e.color, side)
        }
        None => format!("    <{}/>\n", side),
    }
}

fn serialize_xf(xf: &StyleXf, add_xf_id: bool) -> String {
    let mut attrs = format!(
        "numFmtId=\"{}\" fontId=\"{}\" fillId=\"{}\" borderId=\"{}\"",
        xf.num_fmt_id, xf.font_id, xf.fill_id, xf.border_id,
    );
    if add_xf_id {
        attrs.push_str(" xfId=\"0\"");
    }
    if xf.apply_font { attrs.push_str(" applyFont=\"1\""); }
    if xf.apply_fill { attrs.push_str(" applyFill=\"1\""); }
    if xf.apply_border { attrs.push_str(" applyBorder=\"1\""); }
    if xf.apply_alignment { attrs.push_str(" applyAlignment=\"1\""); }
    if xf.apply_number_format { attrs.push_str(" applyNumberFormat=\"1\""); }

    if let Some(ref a) = xf.alignment {
        let al_xml = alignment_xml(a, "    ");
        format!("  <xf {}>{}</xf>\n", attrs, al_xml)
    } else {
        format!("  <xf {}/>\n", attrs)
    }
}

fn alignment_xml(a: &StyleAlignment, indent: &str) -> String {
    let mut al = format!("{}<alignment", indent);
    if let Some(ref h) = a.horizontal { al.push_str(&format!(" horizontal=\"{}\"", h)); }
    if let Some(ref v) = a.vertical { al.push_str(&format!(" vertical=\"{}\"", v)); }
    if a.wrap_text { al.push_str(" wrapText=\"1\""); }
    if let Some(ind) = a.indent { al.push_str(&format!(" indent=\"{}\"", ind)); }
    if let Some(rot) = a.text_rotation { al.push_str(&format!(" textRotation=\"{}\"", rot)); }
    al.push_str("/>\n");
    al
}

fn find_or_create_font(model: &mut StylesModel, props: &HashMap<String, String>) -> u32 {
    let font_size = props.get("fontSize").and_then(|v| v.parse::<f64>().ok());
    let font_name = props.get("font").or_else(|| props.get("fontName")).cloned();
    let font_color = props.get("fontColor").or_else(|| props.get("color")).map(|s| strip_hash(s));
    let bold = props.get("bold").map(|v| v == "true" || v == "1").unwrap_or(false);
    let italic = props.get("italic").map(|v| v == "true" || v == "1").unwrap_or(false);
    let underline = props.get("underline").cloned();
    let strike = false;

    for (i, f) in model.fonts.iter().enumerate() {
        if f.bold == bold && f.italic == italic && f.underline == underline && f.strike == strike
            && f.size == font_size && f.name == font_name && f.color == font_color
        {
            return i as u32;
        }
    }

    let new_font = StyleFont {
        bold, italic, underline, strike, size: font_size,
        color: font_color, name: font_name, family: None, charset: None, scheme: None,
    };
    model.fonts.push(new_font);
    (model.fonts.len() - 1) as u32
}

fn find_or_create_fill(model: &mut StylesModel, props: &HashMap<String, String>) -> u32 {
    let fill = props.get("fill").or_else(|| props.get("bgColor")).or_else(|| props.get("bg"));
    let fg_color = fill.map(|s| strip_hash(s));
    let pattern_type = if fg_color.is_some() { "solid".to_string() } else { "none".to_string() };

    for (i, f) in model.fills.iter().enumerate() {
        if f.pattern_type == pattern_type && f.fg_color == fg_color {
            return i as u32;
        }
    }

    let new_fill = StyleFill { pattern_type, fg_color, bg_color: None };
    model.fills.push(new_fill);
    (model.fills.len() - 1) as u32
}

fn find_or_create_border(model: &mut StylesModel, props: &HashMap<String, String>) -> u32 {
    let border_style = props.get("border").cloned();
    let border_color = props.get("borderColor").map(|s| strip_hash(s));

    for (i, b) in model.borders.iter().enumerate() {
        let match_left = match (&b.left, &border_style, &border_color) {
            (Some(edge), Some(style), Some(color)) => edge.style == *style && edge.color == *color,
            (None, None, None) => true,
            (Some(edge), Some(style), None) => edge.style == *style && edge.color.is_empty(),
            (None, Some(_), _) => false,
            (Some(_), None, _) => false,
            (_, _, Some(_)) => false,
        };
        let match_right = match (&b.right, &border_style, &border_color) {
            (Some(edge), Some(style), Some(color)) => edge.style == *style && edge.color == *color,
            (None, None, None) => true,
            (Some(edge), Some(style), None) => edge.style == *style && edge.color.is_empty(),
            (None, Some(_), _) => false,
            (Some(_), None, _) => false,
            (_, _, Some(_)) => false,
        };
        if match_left && match_right {
            return i as u32;
        }
    }

    let edge = border_style.map(|style| BorderEdge {
        style,
        color: border_color.unwrap_or_default(),
    });

    let new_border = StyleBorder {
        left: edge.clone(),
        right: edge.clone(),
        top: edge.clone(),
        bottom: edge.clone(),
        diagonal: None,
        diagonal_up: false,
        diagonal_down: false,
    };
    model.borders.push(new_border);
    (model.borders.len() - 1) as u32
}

fn find_or_create_numfmt(model: &mut StylesModel, props: &HashMap<String, String>) -> u32 {
    let code = props.get("numberformat").or_else(|| props.get("numberFormat"))
        .or_else(|| props.get("numFmt")).cloned();
    let Some(ref code) = code else {
        return 0;
    };

    for nf in &model.num_fmts {
        if nf.format_code == *code {
            return nf.id;
        }
    }

    let new_id = model.num_fmts.iter().map(|n| n.id).max().unwrap_or(164) + 1;
    model.num_fmts.push(StyleNumFmt { id: new_id, format_code: code.clone() });
    new_id
}

fn find_or_create_xf(model: &mut StylesModel, font_id: u32, fill_id: u32, border_id: u32,
    num_fmt_id: u32, alignment: Option<StyleAlignment>, props: &HashMap<String, String>) -> u32
{
    let apply_font = font_id > 0 || props.contains_key("bold") || props.contains_key("italic")
        || props.contains_key("fontName") || props.contains_key("fontSize")
        || props.contains_key("fontColor") || props.contains_key("color") || props.contains_key("underline");
    let apply_fill = fill_id > 0 || props.contains_key("fill") || props.contains_key("bgColor")
        || props.contains_key("bg");
    let apply_border = border_id > 0 || props.contains_key("border") || props.contains_key("borderColor");
    let apply_alignment = alignment.is_some();
    let apply_number_format = num_fmt_id > 0;

    for (i, xf) in model.cell_xfs.iter().enumerate() {
        if xf.font_id == font_id && xf.fill_id == fill_id && xf.border_id == border_id
            && xf.num_fmt_id == num_fmt_id && xf.alignment == alignment
        {
            return i as u32;
        }
    }

    let new_xf = StyleXf {
        font_id, fill_id, border_id, num_fmt_id, alignment,
        apply_font, apply_fill, apply_border, apply_alignment, apply_number_format,
    };
    model.cell_xfs.push(new_xf);
    (model.cell_xfs.len() - 1) as u32
}

fn strip_hash(s: &str) -> String {
    s.trim_start_matches('#').to_string()
}

fn parse_alignment_from_props(props: &HashMap<String, String>) -> Option<StyleAlignment> {
    let horizontal = props.get("alignment").or_else(|| props.get("align")).cloned();
    let vertical = props.get("valign").or_else(|| props.get("vertical")).cloned();
    let wrap_text = props.get("wrap").or_else(|| props.get("wrapText"))
        .map(|v| v == "true" || v == "1").unwrap_or(false);
    let indent = props.get("indent").and_then(|v| v.parse::<u32>().ok());
    let text_rotation = props.get("rotation").or_else(|| props.get("textRotation"))
        .and_then(|v| v.parse::<u32>().ok());

    if horizontal.is_none() && vertical.is_none() && !wrap_text && indent.is_none() && text_rotation.is_none() {
        return None;
    }
    Some(StyleAlignment { horizontal, vertical, wrap_text, indent, text_rotation })
}

pub fn register_style(model: &mut StylesModel, props: &HashMap<String, String>) -> u32 {
    let font_id = find_or_create_font(model, props);
    let fill_id = find_or_create_fill(model, props);
    let border_id = find_or_create_border(model, props);
    let num_fmt_id = find_or_create_numfmt(model, props);
    let alignment = parse_alignment_from_props(props);
    find_or_create_xf(model, font_id, fill_id, border_id, num_fmt_id, alignment, props)
}

pub fn ensure_styles_part(package: &mut oxml::OxmlPackage) -> Result<StylesModel, HandlerError> {
    let xml = if package.has_part("xl/styles.xml") {
        package.read_part_xml("xl/styles.xml")
            .map_err(|e| HandlerError::OperationFailed(format!("failed to read styles.xml: {}", e)))?
    } else {
        String::new()
    };

    let model = if xml.is_empty() {
        create_default_styles()
    } else {
        parse_styles_xml(&xml).map_err(|e| HandlerError::OperationFailed(e))?
    };

    Ok(model)
}

fn create_default_styles() -> StylesModel {
    StylesModel {
        fonts: vec![
            StyleFont { bold: false, italic: false, underline: None, strike: false,
                size: Some(11.0), name: Some("Calibri".to_string()), color: None,
                family: Some(2), charset: None, scheme: Some("minor".to_string()) },
        ],
        fills: vec![
            StyleFill { pattern_type: "none".to_string(), fg_color: None, bg_color: None },
            StyleFill { pattern_type: "gray125".to_string(), fg_color: None, bg_color: None },
        ],
        borders: vec![StyleBorder {
            left: None, right: None, top: None, bottom: None,
            diagonal: None, diagonal_up: false, diagonal_down: false,
        }],
        cell_style_xfs: vec![StyleXf {
            font_id: 0, fill_id: 0, border_id: 0, num_fmt_id: 0,
            alignment: None, apply_font: false, apply_fill: false,
            apply_border: false, apply_alignment: false, apply_number_format: false,
        }],
        cell_xfs: vec![StyleXf {
            font_id: 0, fill_id: 0, border_id: 0, num_fmt_id: 0,
            alignment: None, apply_font: false, apply_fill: false,
            apply_border: false, apply_alignment: false, apply_number_format: false,
        }],
        num_fmts: Vec::new(),
        dxfs: Vec::new(),
    }
}
