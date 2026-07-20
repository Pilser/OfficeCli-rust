use crate::dom_types::*;
use handler_common::HandlerError;
use oxml::OxmlPackage;

/// Parse a `<a:tbl>` element XML string into a TableNode structure.
pub fn parse_table(tbl_xml: &str) -> TableNode {
    let wrapped = format!(
        "<dummy xmlns:a=\"{}\" xmlns:p=\"{}\">{}</dummy>",
        NS_A, NS_P, tbl_xml
    );
    let doc = match roxmltree::Document::parse(&wrapped) {
        Ok(d) => d,
        Err(_) => {
            return TableNode {
                rows: Vec::new(),
                grid_col_count: 0,
                name: None,
            }
        }
    };
    let dummy = doc.root_element();
    let tbl_node = match dummy.children().find(|n| n.has_tag_name("tbl")) {
        Some(n) => n,
        None => {
            return TableNode {
                rows: Vec::new(),
                grid_col_count: 0,
                name: None,
            }
        }
    };
    parse_table_from_node(&tbl_node)
}

fn parse_table_from_node(tbl: &roxmltree::Node) -> TableNode {
    let tbl_grid = tbl.children().find(|n| n.has_tag_name("tblGrid"));
    let grid_col_count = tbl_grid
        .map(|g| g.children().filter(|c| c.has_tag_name("gridCol")).count() as u32)
        .unwrap_or(0);

    let mut rows = Vec::new();
    for (ri, tr) in tbl.children().filter(|n| n.has_tag_name("tr")).enumerate() {
        let height = tr.attribute("h").and_then(|s| s.parse::<i64>().ok());
        let mut cells = Vec::new();
        for (ci, tc) in tr.children().filter(|n| n.has_tag_name("tc")).enumerate() {
            let mut cell_text = String::new();
            for t_node in tc.descendants().filter(|n| n.has_tag_name("t")) {
                if let Some(t) = t_node.text() {
                    cell_text.push_str(t);
                }
            }
            let col_span = tc.attribute("gridSpan").and_then(|s| s.parse::<u32>().ok());
            let row_span = tc.attribute("rowSpan").and_then(|s| s.parse::<u32>().ok());
            cells.push(TableCell {
                text: cell_text,
                col_span,
                row_span,
                col_idx: ci as u32,
                row_idx: ri as u32,
            });
        }
        rows.push(TableRow { cells, height });
    }

    TableNode {
        rows,
        grid_col_count,
        name: None,
    }
}

/// Serialize a TableNode back to `<a:tbl>` XML string with proper namespace.
pub fn serialize_table(table: &TableNode) -> String {
    let mut xml = String::from("<a:tbl>");

    // tblPr
    xml.push_str("<a:tblPr firstRow=\"1\" bandRow=\"1\"><a:tableStyleId>{5940675A-B579-4CD6-9FD5-AB1180B14A42}</a:tableStyleId></a:tblPr>");

    // tblGrid
    xml.push_str("<a:tblGrid>");
    for _ in 0..table.grid_col_count {
        xml.push_str("<a:gridCol w=\"914400\"/>");
    }
    xml.push_str("</a:tblGrid>");

    // rows
    for row in &table.rows {
        xml.push_str("<a:tr");
        if let Some(h) = row.height {
            xml.push_str(&format!(" h=\"{}\"", h));
        }
        xml.push('>');
        for cell in &row.cells {
            xml.push_str("<a:tc");
            if let Some(cs) = cell.col_span {
                xml.push_str(&format!(" gridSpan=\"{}\"", cs));
            }
            if let Some(rs) = cell.row_span {
                xml.push_str(&format!(" rowSpan=\"{}\"", rs));
            }
            xml.push_str("><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" dirty=\"0\"/>");
            xml.push_str(&format!("<a:t>{}</a:t>", xml_escape(&cell.text)));
            xml.push_str("</a:r></a:p></a:txBody></a:tc>");
        }
        xml.push_str("</a:tr>");
    }

    xml.push_str("</a:tbl>");
    xml
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Get the table XML for a specific slide at a specific shape index.
/// shape_idx is 1-based within the slide's spTree (counting all element types).
/// Returns None if the shape doesn't exist or isn't a table.
pub fn get_table_xml(
    package: &OxmlPackage,
    slide_idx: usize,
    shape_idx: usize,
) -> Result<Option<String>, HandlerError> {
    let slide_path = format!("ppt/slides/slide{}.xml", slide_idx);
    let slide_xml = package
        .read_part_xml(&slide_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let tbl_xml = extract_table_xml_from_slide(&slide_xml, shape_idx)?;
    Ok(tbl_xml)
}

/// Find the Nth shape element in the spTree and extract its table XML.
fn extract_table_xml_from_slide(
    slide_xml: &str,
    shape_idx: usize,
) -> Result<Option<String>, HandlerError> {
    let doc = roxmltree::Document::parse(slide_xml).map_err(|e| {
        HandlerError::OperationFailed(format!("roxmltree parse error: {}", e))
    })?;

    let sp_tree = doc
        .descendants()
        .find(|n| n.has_tag_name((NS_P, "spTree")))
        .or_else(|| doc.descendants().find(|n| n.has_tag_name("spTree")));

    let tree = match sp_tree {
        Some(t) => t,
        None => return Ok(None),
    };

    let mut count = 0;
    for child in tree.children() {
        let tag = child.tag_name().name();
        if is_shape_element(tag) {
            count += 1;
            if count == shape_idx && tag == "graphicFrame" {
                if let Some(tbl) = child.descendants().find(|n| n.has_tag_name("tbl")) {
                    let range = tbl.range();
                    let tbl_str = &slide_xml[range];
                    return Ok(Some(tbl_str.to_string()));
                }
            }
        }
    }
    Ok(None)
}

fn is_shape_element(tag: &str) -> bool {
    matches!(tag, "sp" | "graphicFrame" | "pic" | "grpSp" | "cxnSp" | "AlternateContent")
}

/// Set cell text in a table at a specific position.
/// slide_idx: 1-based slide index
/// shape_idx: 1-based shape index within the slide
/// row_idx: 1-based row index
/// col_idx: 1-based column index
/// text: the new text content
pub fn set_cell_text(
    package: &mut OxmlPackage,
    slide_idx: usize,
    shape_idx: usize,
    row_idx: usize,
    col_idx: usize,
    text: &str,
) -> Result<(), HandlerError> {
    let slide_path = format!("ppt/slides/slide{}.xml", slide_idx);
    let slide_xml = package
        .read_part_xml(&slide_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let modified = set_cell_text_in_xml(&slide_xml, shape_idx, row_idx, col_idx, text)?;

    package
        .write_part_xml(&slide_path, &modified)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))
}

fn set_cell_text_in_xml(
    xml: &str,
    shape_idx: usize,
    row_idx: usize,
    col_idx: usize,
    text: &str,
) -> Result<String, HandlerError> {
    let mut result = xml.to_string();
    let mut shape_count = 0;
    let mut search_start = 0;

    while let Some(sp_open) = find_shape_open(&result, search_start) {
        shape_count += 1;
        let sp_close = find_shape_close(&result, sp_open)?;

        if shape_count == shape_idx {
            let shape_xml = &result[sp_open..sp_close];

            // Find the table in this shape
            if let Some(tbl_start) = shape_xml.find("<a:tbl") {
                let tbl_segment = &shape_xml[tbl_start..];
                let tbl_tag_end = find_tag_end(tbl_segment, "<a:tbl").map_err(|_| {
                    HandlerError::OperationFailed("malformed <a:tbl>".to_string())
                })?;
                let tbl_body_start = tbl_tag_end;
                let tbl_close = tbl_segment.rfind("</a:tbl>").ok_or_else(|| {
                    HandlerError::OperationFailed("no </a:tbl> found".to_string())
                })? + "</a:tbl>".len();

                let tbl_content = &tbl_segment[tbl_body_start..tbl_close];
                let abs_tbl_start = sp_open + tbl_start + tbl_tag_end;

                // Find the Nth row
                let mut row_count = 0;
                let mut row_pos = 0;
                while let Some(tr_open) = tbl_content[row_pos..].find("<a:tr") {
                    let abs_tr = row_pos + tr_open;
                    let tr_tag_end = find_tag_end(&tbl_content[abs_tr..], "<a:tr").map_err(|_| {
                        HandlerError::OperationFailed("malformed <a:tr>".to_string())
                    })?;
                    let tr_body_start = abs_tr + tr_tag_end;
                    let tr_close_rel = tbl_content[tr_body_start..].find("</a:tr>").ok_or_else(|| {
                        HandlerError::OperationFailed("no </a:tr> found".to_string())
                    })?;
                    let tr_close = tr_body_start + tr_close_rel + "</a:tr>".len();

                    row_count += 1;
                    if row_count == row_idx {
                        let tr_xml = &tbl_content[abs_tr..tr_close];
                        let new_tr_xml = set_cell_in_row(tr_xml, col_idx, text)?;

                        let abs_tr_abs = abs_tbl_start + abs_tr;
                        result.replace_range(abs_tr_abs..abs_tr_abs + tr_xml.len(), &new_tr_xml);
                        return Ok(result);
                    }

                    row_pos = tr_close;
                }
                return Err(HandlerError::PathNotFound(format!(
                    "row[{}] not found in table",
                    row_idx
                )));
            }
            return Err(HandlerError::PathNotFound(format!(
                "shape[{}] is not a table",
                shape_idx
            )));
        }
        search_start = sp_close;
    }

    Err(HandlerError::PathNotFound(format!(
        "shape[{}] not found",
        shape_idx
    )))
}

fn set_cell_in_row(tr_xml: &str, col_idx: usize, text: &str) -> Result<String, HandlerError> {
    let mut col_count = 0;
    let mut pos = 0;
    let escaped = xml_escape(text);

    while let Some(tc_open) = tr_xml[pos..].find("<a:tc") {
        let abs_tc = pos + tc_open;
        let tc_tag_end = find_tag_end(&tr_xml[abs_tc..], "<a:tc").map_err(|_| {
            HandlerError::OperationFailed("malformed <a:tc>".to_string())
        })?;
        let tc_body_start = abs_tc + tc_tag_end;
        let tc_close_rel = tr_xml[tc_body_start..].find("</a:tc>").ok_or_else(|| {
            HandlerError::OperationFailed("no </a:tc> found".to_string())
        })?;
        let tc_close = tc_body_start + tc_close_rel + "</a:tc>".len();

        col_count += 1;
        if col_count == col_idx {
            let tc_xml = &tr_xml[abs_tc..tc_close];
            let new_tc_xml = replace_tc_text(tc_xml, &escaped);
            let mut result = tr_xml.to_string();
            result.replace_range(abs_tc..tc_close, &new_tc_xml);
            return Ok(result);
        }

        pos = tc_close;
    }

    Err(HandlerError::PathNotFound(format!(
        "cell[{}] not found in row",
        col_idx
    )))
}

fn replace_tc_text(tc_xml: &str, new_text: &str) -> String {
    // Find <a:t>...</a:t> and replace content
    let mut result = tc_xml.to_string();
    if let Some(t_start) = result.find("<a:t>") {
        let content_start = t_start + "<a:t>".len();
        if let Some(t_end) = result[content_start..].find("</a:t>") {
            let abs_end = content_start + t_end;
            result.replace_range(content_start..abs_end, new_text);
        } else if let Some(t_end) = result[content_start..].find("/>") {
            let abs_end = content_start + t_end;
            result.replace_range(content_start..abs_end, new_text);
        }
    } else if let Some(t_start) = find_tag_with_attrs(&result, "a:t") {
        let content_start = t_start + tag_name_len_with_attrs(&result[t_start..], "a:t");
        if let Some(t_end) = result[content_start..].find("</a:t>") {
            let abs_end = content_start + t_end;
            result.replace_range(content_start..abs_end, new_text);
        }
    }
    result
}

/// Get cell text from a table.
pub fn get_cell_text(
    package: &OxmlPackage,
    slide_idx: usize,
    shape_idx: usize,
    row_idx: usize,
    col_idx: usize,
) -> Result<String, HandlerError> {
    let slide_path = format!("ppt/slides/slide{}.xml", slide_idx);
    let slide_xml = package
        .read_part_xml(&slide_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let doc = roxmltree::Document::parse(&slide_xml).map_err(|e| {
        HandlerError::OperationFailed(format!("roxmltree parse error: {}", e))
    })?;

    let sp_tree = doc
        .descendants()
        .find(|n| n.has_tag_name((NS_P, "spTree")))
        .or_else(|| doc.descendants().find(|n| n.has_tag_name("spTree")));

    let tree = sp_tree.ok_or_else(|| {
        HandlerError::OperationFailed("no spTree found".to_string())
    })?;

    let mut count = 0;
    for child in tree.children() {
        if is_shape_element(child.tag_name().name()) {
            count += 1;
            if count == shape_idx {
                if let Some(tbl) = child.descendants().find(|n| n.has_tag_name("tbl")) {
                    let mut row_i = 0;
                    for tr in tbl.children().filter(|n| n.has_tag_name("tr")) {
                        row_i += 1;
                        if row_i == row_idx {
                            let mut col_i = 0;
                            for tc in tr.children().filter(|n| n.has_tag_name("tc")) {
                                col_i += 1;
                                if col_i == col_idx {
                                    let mut text = String::new();
                                    for t_node in tc.descendants().filter(|n| n.has_tag_name("t")) {
                                        if let Some(t) = t_node.text() {
                                            text.push_str(t);
                                        }
                                    }
                                    return Ok(text);
                                }
                            }
                            return Err(HandlerError::PathNotFound(format!(
                                "cell[{}] not found in row[{}]",
                                col_idx, row_idx
                            )));
                        }
                    }
                    return Err(HandlerError::PathNotFound(format!(
                        "row[{}] not found in table",
                        row_idx
                    )));
                }
                return Err(HandlerError::OperationFailed("shape is not a table".to_string()));
            }
        }
    }

    Err(HandlerError::PathNotFound(format!(
        "shape[{}] not found",
        shape_idx
    )))
}

/// Add a row to a table.
pub fn add_row(
    package: &mut OxmlPackage,
    slide_idx: usize,
    shape_idx: usize,
    after_row: Option<usize>,
) -> Result<(), HandlerError> {
    let slide_path = format!("ppt/slides/slide{}.xml", slide_idx);
    let slide_xml = package
        .read_part_xml(&slide_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let modified = add_row_to_xml(&slide_xml, shape_idx, after_row)?;

    package
        .write_part_xml(&slide_path, &modified)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))
}

fn add_row_to_xml(
    xml: &str,
    shape_idx: usize,
    after_row: Option<usize>,
) -> Result<String, HandlerError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| {
        HandlerError::OperationFailed(format!("roxmltree parse error: {}", e))
    })?;

    let sp_tree = doc
        .descendants()
        .find(|n| n.has_tag_name((NS_P, "spTree")))
        .or_else(|| doc.descendants().find(|n| n.has_tag_name("spTree")));

    let tree = sp_tree.ok_or_else(|| {
        HandlerError::OperationFailed("no spTree found".to_string())
    })?;

    let mut count = 0;
    let mut tbl_range = None;
    let mut target_tr_range = None;
    let mut grid_col_count = 0;

    for child in tree.children() {
        if is_shape_element(child.tag_name().name()) {
            count += 1;
            if count == shape_idx {
                if let Some(tbl) = child.descendants().find(|n| n.has_tag_name("tbl")) {
                    tbl_range = Some(tbl.range());
                    // Count grid columns
                    if let Some(grid) = tbl.children().find(|n| n.has_tag_name("tblGrid")) {
                        grid_col_count = grid.children().filter(|c| c.has_tag_name("gridCol")).count();
                    }
                    if let Some(after) = after_row {
                        let mut ri = 0;
                        for tr in tbl.children().filter(|n| n.has_tag_name("tr")) {
                            ri += 1;
                            if ri == after {
                                target_tr_range = Some(tr.range());
                            }
                        }
                    }
                }
            }
        }
    }

    let tbl = tbl_range.ok_or_else(|| {
        HandlerError::PathNotFound(format!("table shape[{}] not found", shape_idx))
    })?;
    let tbl_start = tbl.start;
    let tbl_end = tbl.end;

    let new_row_xml = new_empty_row_xml(grid_col_count.max(1));

    if let Some(tr) = target_tr_range {
        if tr.end == 0 {
            let mut result = xml.to_string();
            result.insert_str(tr.start, &new_row_xml);
            Ok(result)
        } else {
            let mut result = xml.to_string();
            result.insert_str(tr.end, &new_row_xml);
            Ok(result)
        }
    } else {
        // Insert before </a:tbl>
        let close_tag = "</a:tbl>";
        let abs = tbl_start + tbl_end - close_tag.len();
        let mut result = xml.to_string();
        result.insert_str(abs, &new_row_xml);
        Ok(result)
    }
}

fn new_empty_row_xml(col_count: usize) -> String {
    let mut cells = String::new();
    for _ in 0..col_count {
        cells.push_str(
            "<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" dirty=\"0\"/><a:t></a:t></a:r></a:p></a:txBody></a:tc>",
        );
    }
    format!("<a:tr h=\"457200\">{}</a:tr>", cells)
}

/// Add a column to a table.
pub fn add_column(
    package: &mut OxmlPackage,
    slide_idx: usize,
    shape_idx: usize,
    after_col: Option<usize>,
) -> Result<(), HandlerError> {
    let slide_path = format!("ppt/slides/slide{}.xml", slide_idx);
    let slide_xml = package
        .read_part_xml(&slide_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let modified = add_column_to_xml(&slide_xml, shape_idx, after_col)?;

    package
        .write_part_xml(&slide_path, &modified)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))
}

fn add_column_to_xml(
    xml: &str,
    shape_idx: usize,
    after_col: Option<usize>,
) -> Result<String, HandlerError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| {
        HandlerError::OperationFailed(format!("roxmltree parse error: {}", e))
    })?;

    let sp_tree = doc
        .descendants()
        .find(|n| n.has_tag_name((NS_P, "spTree")))
        .or_else(|| doc.descendants().find(|n| n.has_tag_name("spTree")));

    let tree = sp_tree.ok_or_else(|| {
        HandlerError::OperationFailed("no spTree found".to_string())
    })?;

    let mut count = 0;
    for child in tree.children() {
        if is_shape_element(child.tag_name().name()) {
            count += 1;
            if count == shape_idx {
                if let Some(tbl) = child.descendants().find(|n| n.has_tag_name("tbl")) {
                    let tbl_range = tbl.range();
                    let _tbl_xml = &xml[tbl_range.clone()];

                    // Add gridCol to tblGrid
                    let mut result = xml.to_string();

                    // Find tblGrid and add a gridCol
                    if let Some(grid) = tbl.children().find(|n| n.has_tag_name("tblGrid")) {
                        let grid_range = grid.range();
                        let grid_xml = &xml[grid_range.clone()];
                        if let Some(after) = after_col {
                            let mut ci = 0;
                            for gc in grid.children().filter(|n| n.has_tag_name("gridCol")) {
                                ci += 1;
                                if ci == after {
                                    let gc_range = gc.range();
                                    let insert_pos = gc_range.end - grid_range.start;
                                    let abs_pos = grid_range.start + insert_pos;
                                    result.insert_str(abs_pos, "<a:gridCol w=\"914400\"/>");
                                    break;
                                }
                            }
                        } else {
                            // Insert at end
                            let insert_pos = grid_range.start + grid_xml.len() - "</a:tblGrid>".len();
                            result.insert_str(insert_pos, "<a:gridCol w=\"914400\"/>");
                        }
                    }

                    // Add cells to each row
                    let mut pending_offset = 0;
                    for tr in tbl.children().filter(|n| n.has_tag_name("tr")) {
                        let tr_range = tr.range();
                        let tr_xml = &xml[tr_range.clone()];
                        let insert_pos = if let Some(after) = after_col {
                            let mut ci = 0;
                            let mut found = false;
                            let mut pos: usize = 0;
                            for tc in tr.children().filter(|n| n.has_tag_name("tc")) {
                                ci += 1;
                                if ci == after {
                                    let tc_range = tc.range();
                                    pos = tc_range.end - tr_range.start;
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                tr_range.start + pos
                            } else {
                                tr_range.start + tr_xml.len() - "</a:tr>".len()
                            }
                        } else {
                            tr_range.start + tr_xml.len() - "</a:tr>".len()
                        };
                        let adj_pos = insert_pos + pending_offset;
                        result.insert_str(
                            adj_pos,
                            "<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" dirty=\"0\"/><a:t></a:t></a:r></a:p></a:txBody></a:tc>",
                        );
                        pending_offset += "<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" dirty=\"0\"/><a:t></a:t></a:r></a:p></a:txBody></a:tc>".len();
                    }

                    return Ok(result);
                }
                return Err(HandlerError::OperationFailed("shape is not a table".to_string()));
            }
        }
    }

    Err(HandlerError::PathNotFound(format!(
        "shape[{}] not found",
        shape_idx
    )))
}

/// Remove a row from a table.
pub fn remove_row(
    package: &mut OxmlPackage,
    slide_idx: usize,
    shape_idx: usize,
    row_idx: usize,
) -> Result<(), HandlerError> {
    let slide_path = format!("ppt/slides/slide{}.xml", slide_idx);
    let slide_xml = package
        .read_part_xml(&slide_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let modified = remove_row_from_xml(&slide_xml, shape_idx, row_idx)?;

    package
        .write_part_xml(&slide_path, &modified)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))
}

fn remove_row_from_xml(
    xml: &str,
    shape_idx: usize,
    row_idx: usize,
) -> Result<String, HandlerError> {
    let mut result = xml.to_string();
    let doc = roxmltree::Document::parse(xml).map_err(|e| {
        HandlerError::OperationFailed(format!("roxmltree parse error: {}", e))
    })?;

    let sp_tree = doc
        .descendants()
        .find(|n| n.has_tag_name((NS_P, "spTree")))
        .or_else(|| doc.descendants().find(|n| n.has_tag_name("spTree")));

    let tree = sp_tree.ok_or_else(|| {
        HandlerError::OperationFailed("no spTree found".to_string())
    })?;

    let mut count = 0;
    for child in tree.children() {
        if is_shape_element(child.tag_name().name()) {
            count += 1;
            if count == shape_idx {
                if let Some(tbl) = child.descendants().find(|n| n.has_tag_name("tbl")) {
                    let mut ri = 0;
                    let tr_ranges: Vec<std::ops::Range<usize>> = tbl
                        .children()
                        .filter(|n| n.has_tag_name("tr"))
                        .filter_map(|tr| {
                            ri += 1;
                            if ri == row_idx {
                                Some(tr.range())
                            } else {
                                None
                            }
                        })
                        .collect();
                    if let Some(range) = tr_ranges.first() {
                        result.replace_range(range.clone(), "");
                        return Ok(result);
                    }
                    return Err(HandlerError::PathNotFound(format!(
                        "row[{}] not found",
                        row_idx
                    )));
                }
                return Err(HandlerError::OperationFailed("shape is not a table".to_string()));
            }
        }
    }

    Err(HandlerError::PathNotFound(format!(
        "shape[{}] not found",
        shape_idx
    )))
}

/// Remove a column from a table.
pub fn remove_column(
    package: &mut OxmlPackage,
    slide_idx: usize,
    shape_idx: usize,
    col_idx: usize,
) -> Result<(), HandlerError> {
    let slide_path = format!("ppt/slides/slide{}.xml", slide_idx);
    let slide_xml = package
        .read_part_xml(&slide_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let modified = remove_column_from_xml(&slide_xml, shape_idx, col_idx)?;

    package
        .write_part_xml(&slide_path, &modified)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))
}

fn remove_column_from_xml(
    xml: &str,
    shape_idx: usize,
    col_idx: usize,
) -> Result<String, HandlerError> {
    let mut result = xml.to_string();
    let doc = roxmltree::Document::parse(xml).map_err(|e| {
        HandlerError::OperationFailed(format!("roxmltree parse error: {}", e))
    })?;

    let sp_tree = doc
        .descendants()
        .find(|n| n.has_tag_name((NS_P, "spTree")))
        .or_else(|| doc.descendants().find(|n| n.has_tag_name("spTree")));

    let tree = sp_tree.ok_or_else(|| {
        HandlerError::OperationFailed("no spTree found".to_string())
    })?;

    let mut count = 0;
    for child in tree.children() {
        if is_shape_element(child.tag_name().name()) {
            count += 1;
            if count == shape_idx {
                if let Some(tbl) = child.descendants().find(|n| n.has_tag_name("tbl")) {
                    let _tbl_range = tbl.range();

                    // Remove gridCol from tblGrid
                    if let Some(grid) = tbl.children().find(|n| n.has_tag_name("tblGrid")) {
                        let mut ci = 0;
                        for gc in grid.children().filter(|n| n.has_tag_name("gridCol")) {
                            ci += 1;
                            if ci == col_idx {
                                let gc_range = gc.range();
                                result.replace_range(gc_range.clone(), "");
                                break;
                            }
                        }
                    }

                    // Remove cells from each row
                    let mut pending_offset: isize = 0;
                    for tr in tbl.children().filter(|n| n.has_tag_name("tr")) {
                        let mut ci = 0;
                        let _tr_range = tr.range();
                        for tc in tr.children().filter(|n| n.has_tag_name("tc")) {
                            ci += 1;
                            if ci == col_idx {
                                let tc_range = tc.range();
                                let start = (tc_range.start as isize + pending_offset) as usize;
                                let end = (tc_range.end as isize + pending_offset) as usize;
                                let removed_len = end - start;
                                result.replace_range(start..end, "");
                                pending_offset -= removed_len as isize;
                                break;
                            }
                        }
                    }

                    return Ok(result);
                }
                return Err(HandlerError::OperationFailed("shape is not a table".to_string()));
            }
        }
    }

    Err(HandlerError::PathNotFound(format!(
        "shape[{}] not found",
        shape_idx
    )))
}

/// Find the start index of an opening tag like <a:tr, <a:tc, etc.
fn find_shape_open(xml: &str, start: usize) -> Option<usize> {
    let candidates = ["<p:sp", "<p:graphicFrame", "<p:pic", "<p:grpSp", "<p:cxnSp", "<mc:AlternateContent"];
    let mut first_pos = None;
    for pat in &candidates {
        if let Some(pos) = xml[start..].find(pat) {
            let abs = start + pos;
            if first_pos.map_or(true, |p| abs < p) {
                first_pos = Some(abs);
            }
        }
    }
    first_pos
}

/// Find the closing position of a shape element.
fn find_shape_close(xml: &str, open: usize) -> Result<usize, HandlerError> {
    // Check what type of element this is
    let snippet = &xml[open..];
    if snippet.starts_with("<mc:AlternateContent") {
        // Need to find matching </mc:AlternateContent>
        let close = snippet.find("</mc:AlternateContent>").ok_or_else(|| {
            HandlerError::OperationFailed("no </mc:AlternateContent> found".to_string())
        })?;
        Ok(open + close + "</mc:AlternateContent>".len())
    } else {
        // p:sp, p:graphicFrame, p:pic, p:grpSp, p:cxnSp all end with </p:tag>
        for tag in &["sp", "graphicFrame", "pic", "grpSp", "cxnSp"] {
            let close_tag = format!("</p:{}>", tag);
            if let Some(pos) = snippet.rfind(&close_tag) {
                return Ok(open + pos + close_tag.len());
            }
        }
        Err(HandlerError::OperationFailed("cannot find element close".to_string()))
    }
}

/// Find the end of an opening tag (the '>' character).
fn find_tag_end(xml_snippet: &str, _tag_prefix: &str) -> Result<usize, HandlerError> {
    // Find the '>' that ends the opening tag
    let mut in_quote = false;
    let mut quote_char = '"';
    for (i, c) in xml_snippet.char_indices() {
        if in_quote {
            if c == quote_char {
                in_quote = false;
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_quote = true;
                    quote_char = c;
                }
                '>' => {
                    return Ok(i + 1);
                }
                _ => {}
            }
        }
    }
    Err(HandlerError::OperationFailed("cannot find tag end".to_string()))
}

fn find_tag_with_attrs(xml: &str, tag: &str) -> Option<usize> {
    let pat = format!("<{} ", tag);
    xml.find(&pat).or_else(|| {
        let pat2 = format!("<{}/", tag);
        xml.find(&pat2)
    })
}

fn tag_name_len_with_attrs(xml_snippet: &str, _tag: &str) -> usize {
    let mut in_quote = false;
    let mut quote_char = '"';
    for (i, c) in xml_snippet.char_indices() {
        if in_quote {
            if c == quote_char {
                in_quote = false;
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_quote = true;
                    quote_char = c;
                }
                '>' => {
                    return i + 1;
                }
                _ => {}
            }
        }
    }
    xml_snippet.len()
}
