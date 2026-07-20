use handler_common::HandlerError;
use oxml::OxmlPackage;

fn extract_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = xml.find(&pattern)?;
    let val_start = start + pattern.len();
    let end = xml[val_start..].find('"')?;
    Some(xml[val_start..val_start + end].to_string())
}

fn find_element_end(xml: &str, start: usize, tag: &str) -> usize {
    let first_gt = xml[start..]
        .find('>')
        .map(|pos| start + pos)
        .unwrap_or(xml.len());
    if first_gt > 0 && xml.as_bytes().get(first_gt - 1) == Some(&b'/') {
        first_gt + 1
    } else {
        let close_tag = format!("</{}>", tag);
        xml[first_gt..]
            .find(&close_tag)
            .map(|pos| first_gt + pos + close_tag.len())
            .unwrap_or(xml.len())
    }
}

pub fn rename_sheet(workbook_xml: &str, old_name: &str, new_name: &str) -> Result<String, String> {
    let pattern = format!("name=\"{}\"", old_name);
    if let Some(pos) = workbook_xml.find(&pattern) {
        let val_start = pos + pattern.len();
        Ok(format!(
            "{}name=\"{}\"{}",
            &workbook_xml[..pos],
            new_name,
            &workbook_xml[val_start..]
        ))
    } else {
        Err(format!("sheet '{}' not found in workbook.xml", old_name))
    }
}

pub fn reorder_sheets(workbook_xml: &str, order: &[String]) -> Result<String, String> {
    let sheets_open = "<sheets>";
    let sheets_close = "</sheets>";

    let sheets_start = workbook_xml.find(sheets_open)
        .ok_or_else(|| "no <sheets> found in workbook.xml".to_string())? + sheets_open.len();
    let sheets_end = workbook_xml.find(sheets_close)
        .ok_or_else(|| "no </sheets> found in workbook.xml".to_string())?;

    let sheets_section = &workbook_xml[sheets_start..sheets_end];
    let mut sheet_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut cursor = 0;

    while let Some(start) = sheets_section[cursor..].find("<sheet") {
        let abs_start = cursor + start;
        let end = find_element_end(sheets_section, abs_start, "sheet");
        let entry = &sheets_section[abs_start..end];
        if let Some(name) = extract_attr(entry, "name") {
            sheet_map.insert(name, entry.to_string());
        }
        cursor = end;
    }

    let mut new_sheets = String::new();
    for name in order {
        match sheet_map.remove(name.as_str()) {
            Some(xml) => new_sheets.push_str(&xml),
            None => return Err(format!("sheet '{}' not found in workbook.xml", name)),
        }
    }
    // Add any remaining sheets not in order at the end
    for (_name, xml) in &sheet_map {
        new_sheets.push_str(xml);
    }

    Ok(format!(
        "{}{}{}",
        &workbook_xml[..sheets_start],
        new_sheets,
        &workbook_xml[sheets_end..]
    ))
}

pub fn copy_sheet(package: &mut OxmlPackage, sheet_name: &str) -> Result<String, HandlerError> {
    use crate::helpers;

    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    let ws = model.sheets.iter().find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;

    let new_sheet_index = model.sheets.len() + 1;
    let new_part_path = format!("xl/worksheets/sheet{}.xml", new_sheet_index);
    let new_sheet_name = format!("{} (2)", sheet_name);

    // Copy the sheet XML
    let sheet_xml = package.read_part_xml(&ws.part_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    package.write_part_xml(&new_part_path, &sheet_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    // Copy relationships if they exist
    let rels_path = format!("xl/_rels/{}", ws.part_path.strip_prefix("xl/").unwrap_or(&ws.part_path).replace(".xml", ".xml.rels"));
    let new_rels_path = format!("xl/_rels/{}", new_part_path.strip_prefix("xl/").unwrap_or(&new_part_path).replace(".xml", ".xml.rels"));
    if package.has_part(&rels_path) {
        let rels_xml = package.read_part_xml(&rels_path)
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
        package.write_part_xml(&new_rels_path, &rels_xml)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    // Update workbook.xml
    let wb_xml = package.read_part_xml("xl/workbook.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let new_sheet_entry = format!(
        "<sheet name=\"{}\" sheetId=\"{}\" r:id=\"rId{}\"/>",
        new_sheet_name, new_sheet_index, new_sheet_index
    );
    let modified_wb = if let Some(sheets_end) = wb_xml.find("</sheets>") {
        format!("{}{}{}", &wb_xml[..sheets_end], new_sheet_entry, &wb_xml[sheets_end..])
    } else {
        return Err(HandlerError::OperationFailed("no </sheets> in workbook.xml".to_string()));
    };
    package.write_part_xml("xl/workbook.xml", &modified_wb)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    // Update workbook relationships
    let rels_xml = package.read_part_xml("xl/_rels/workbook.xml.rels")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let new_rel = format!(
        "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{}.xml\"/>",
        new_sheet_index, new_sheet_index
    );
    let modified_rels = if let Some(rels_end) = rels_xml.find("</Relationships>") {
        format!("{}{}{}", &rels_xml[..rels_end], new_rel, &rels_xml[rels_end..])
    } else {
        return Err(HandlerError::OperationFailed("no </Relationships> in workbook rels".to_string()));
    };
    package.write_part_xml("xl/_rels/workbook.xml.rels", &modified_rels)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    Ok(new_sheet_name)
}

pub fn set_sheet_visibility(workbook_xml: &str, sheet_name: &str, hidden: bool) -> Result<String, String> {
    let name_pattern = format!("name=\"{}\"", sheet_name);
    let sheet_start = workbook_xml.find("<sheet").ok_or_else(|| "no <sheet> elements found".to_string())?;
    let search_from = workbook_xml[sheet_start..].find(&name_pattern)
        .ok_or_else(|| format!("sheet '{}' not found", sheet_name))?;
    let abs_sheet_start = sheet_start + search_from;

    let sheet_open_start = workbook_xml[..abs_sheet_start].rfind("<sheet")
        .ok_or_else(|| "malformed workbook.xml: <sheet> not found".to_string())?;

    let sheet_element_end = find_element_end(workbook_xml, sheet_open_start, "sheet");
    let element = &workbook_xml[sheet_open_start..sheet_element_end];

    let mut new_element = String::from("<sheet");
    let mut saw_name = false;
    let mut saw_state = false;

    let mut i = 5; // skip "<sheet"
    while i < element.len() {
        while i < element.len() && element.as_bytes()[i] == b' ' { i += 1; }
        if i >= element.len() { break; }
        let eq = element[i..].find('=').map(|p| i + p).unwrap_or(element.len());
        let key = element[i..eq].trim();
        let val_start = eq + 1;
        if val_start < element.len() && element.as_bytes()[val_start] == b'"' {
            let val_end = element[val_start + 1..].find('"').map(|p| val_start + 1 + p).unwrap_or(element.len());
            let val = &element[val_start + 1..val_end];
            match key {
                "name" => {
                    new_element.push_str(&format!(" name=\"{}\"", val));
                    saw_name = true;
                }
                "state" => { saw_state = true; }
                _ => {
                    new_element.push_str(&format!(" {}=\"{}\"", key, val));
                }
            }
            i = val_end + 1;
        } else {
            i = eq + 1;
        }
    }

    if !saw_name {
        new_element.push_str(&format!(" name=\"{}\"", sheet_name));
    }
    if hidden {
        new_element.push_str(" state=\"hidden\"");
    } else if !saw_state {
        // visible is default, no state attribute needed
    }
    new_element.push_str("/>");

    Ok(format!("{}{}{}",
        &workbook_xml[..sheet_open_start],
        new_element,
        &workbook_xml[sheet_element_end..]
    ))
}

pub fn set_tab_color(workbook_xml: &str, sheet_name: &str, color: &str) -> Result<String, String> {
    let name_pattern = format!("name=\"{}\"", sheet_name);
    let sheet_start = workbook_xml.find("<sheet").ok_or_else(|| "no <sheet> elements found".to_string())?;
    let search_from = workbook_xml[sheet_start..].find(&name_pattern)
        .ok_or_else(|| format!("sheet '{}' not found", sheet_name))?;
    let abs_sheet_start = sheet_start + search_from;

    let sheet_open_start = workbook_xml[..abs_sheet_start].rfind("<sheet")
        .ok_or_else(|| "malformed workbook.xml".to_string())?;
    let sheet_element_end = find_element_end(workbook_xml, sheet_open_start, "sheet");
    let element = &workbook_xml[sheet_open_start..sheet_element_end];

    let hex_color = color.trim_start_matches('#');
    let hex_upper = format!("FF{}", hex_color.to_uppercase());
    let tab_color_xml = format!("<sheetPr><tabColor rgb=\"{}\"/></sheetPr>", hex_upper);

    let mut result = workbook_xml[..sheet_open_start].to_string();
    result.push_str("<sheet");
    let mut i = 6;
    while i < element.len() {
        while i < element.len() && element.as_bytes()[i] == b' ' { i += 1; }
        if i >= element.len() { break; }
        let eq = element[i..].find('=').map(|p| i + p).unwrap_or(element.len());
        let key = element[i..eq].trim();
        let val_start = eq + 1;
        if val_start < element.len() && element.as_bytes()[val_start] == b'"' {
            let val_end = element[val_start + 1..].find('"').map(|p| val_start + 1 + p).unwrap_or(element.len());
            let val = &element[val_start + 1..val_end];
            result.push_str(&format!(" {}=\"{}\"", key, val));
            i = val_end + 1;
        } else {
            i = eq + 1;
        }
    }

    // Check for existing <sheetPr> to replace
    if element.contains("<sheetPr>") {
        if let Some(sp_start) = element.find("<sheetPr>") {
            let sp_end = element[sp_start..].find("</sheetPr>")
                .map(|p| sp_start + p + "</sheetPr>".len())
                .unwrap_or(element.len());
            result.push_str(&format!(">{}</sheet>", tab_color_xml));
            let inner_before = &element[element.find('>').unwrap_or(0) + 1..sp_start];
            let inner_after = &element[sp_end..element.len() - 6]; // strip </sheet
            result.push_str(inner_before);
            result.push_str(inner_after);
            result.push_str(&workbook_xml[sheet_element_end..]);
            return Ok(result);
        }
    }

    // Self-closing: <sheet .../>
    let first_gt = element.find('>').unwrap_or(element.len());
    if element.as_bytes().get(first_gt - 1) == Some(&b'/') {
        result.push_str(&format!(">{}", tab_color_xml));
        result.push_str("</sheet>");
    } else {
        // Non-self-closing: insert before </sheet>
        let sheet_close = element.rfind("</sheet>").unwrap_or(element.len());
        let inner_end = sheet_close;
        result.push('>');
        result.push_str(&tab_color_xml);
        result.push_str(&element[first_gt + 1..inner_end]);
        result.push_str("</sheet>");
    }
    result.push_str(&workbook_xml[sheet_element_end..]);
    Ok(result)
}

#[allow(dead_code)]
fn parse_cell_ref(s: &str) -> (u32, u32) {
    let bytes = s.as_bytes();
    let mut col = 0u32;
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        col = col * 26 + (bytes[i].to_ascii_uppercase() as u32 - b'A' as u32 + 1);
        i += 1;
    }
    let row: u32 = s[i..].parse().unwrap_or(0);
    (col, row)
}

fn col_letter_to_num(s: &str) -> u32 {
    let mut num = 0u32;
    for ch in s.chars() {
        num = num * 26 + (ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1);
    }
    num
}

fn col_num_to_letters(num: u32) -> String {
    let mut letters = String::new();
    let mut n = num;
    while n > 0 {
        n -= 1;
        letters.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    letters.chars().rev().collect()
}

pub fn insert_row(sheet_xml: &str, row_index: u32) -> Result<String, String> {
    let _row_pat = format!("r=\"{}\"", row_index);
    let _inserted = format!("r=\"{}\"", row_index + 1);

    // First shift all row numbers >= row_index up by 1
    let mut result = String::new();
    let mut cursor = 0;

    while let Some(pos) = sheet_xml[cursor..].find("r=\"") {
        let abs_pos = cursor + pos;
        result.push_str(&sheet_xml[cursor..abs_pos + 3]);
        let val_start = abs_pos + 3;
        let val_end = sheet_xml[val_start..].find('"').map(|i| val_start + i).unwrap_or(sheet_xml.len());
        let val = &sheet_xml[val_start..val_end];
        if let Ok(n) = val.parse::<u32>() {
            if n >= row_index {
                result.push_str(&(n + 1).to_string());
            } else {
                result.push_str(val);
            }
        } else {
            result.push_str(val);
        }
        cursor = val_end;
    }
    result.push_str(&sheet_xml[cursor..]);

    // Now shift all cell references that are on or below row_index
    let mut result2 = String::new();
    cursor = 0;
    while let Some(pos) = result[cursor..].find("r=\"") {
        let abs_pos = cursor + pos;
        result2.push_str(&result[cursor..abs_pos + 3]);
        let val_start = abs_pos + 3;
        let val_end = result[val_start..].find('"').map(|i| val_start + i).unwrap_or(result.len());
        let val = &result[val_start..val_end];
        // Check if this is a cell ref like "A1" or a row ref like "1"
        if val.contains(|c: char| c.is_ascii_alphabetic()) {
            // Cell ref: parse and shift row
            let (col_part, row_part) = split_cell_ref(val);
            if let Ok(r) = row_part.parse::<u32>() {
                if r >= row_index {
                    result2.push_str(&format!("{}{}", col_part, r + 1));
                } else {
                    result2.push_str(val);
                }
            } else {
                result2.push_str(val);
            }
        } else {
            // Already handled above for row r="N"
            result2.push_str(val);
        }
        cursor = val_end;
    }
    result2.push_str(&result[cursor..]);

    Ok(result2)
}

pub fn delete_row(sheet_xml: &str, row_index: u32) -> Result<String, String> {
    let p = super::layout::detect_prefix(sheet_xml);
    let row_open = format!("<{}row r=\"{}\"", p, row_index);

    let mut result = String::new();
    let mut cursor;

    // Remove the row element
    if let Some(start) = sheet_xml.find(&row_open) {
        let end = find_element_end(sheet_xml, start, &format!("{}row", p));
        result.push_str(&sheet_xml[..start]);
        cursor = end;
    } else {
        return Ok(sheet_xml.to_string());
    }
    result.push_str(&sheet_xml[cursor..]);

    // Shift remaining rows up
    let mut result2 = String::new();
    cursor = 0;
    while let Some(pos) = result[cursor..].find("r=\"") {
        let abs_pos = cursor + pos;
        result2.push_str(&result[cursor..abs_pos + 3]);
        let val_start = abs_pos + 3;
        let val_end = result[val_start..].find('"').map(|i| val_start + i).unwrap_or(result.len());
        let val = &result[val_start..val_end];
        if val.contains(|c: char| c.is_ascii_alphabetic()) {
            let (col_part, row_part) = split_cell_ref(val);
            if let Ok(r) = row_part.parse::<u32>() {
                if r > row_index {
                    result2.push_str(&format!("{}{}", col_part, r - 1));
                } else {
                    result2.push_str(val);
                }
            } else {
                result2.push_str(val);
            }
        } else if let Ok(n) = val.parse::<u32>() {
            if n > row_index {
                result2.push_str(&(n - 1).to_string());
            } else {
                result2.push_str(val);
            }
        } else {
            result2.push_str(val);
        }
        cursor = val_end;
    }
    result2.push_str(&result[cursor..]);

    Ok(result2)
}

fn split_cell_ref(s: &str) -> (String, String) {
    let letters: String = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    (letters, digits)
}

pub fn insert_column(sheet_xml: &str, col_index: u32) -> Result<String, String> {
    let mut result = String::new();
    let mut cursor = 0;

    // Shift cell references
    while let Some(pos) = sheet_xml[cursor..].find("r=\"") {
        let abs_pos = cursor + pos;
        result.push_str(&sheet_xml[cursor..abs_pos + 3]);
        let val_start = abs_pos + 3;
        let val_end = sheet_xml[val_start..].find('"').map(|i| val_start + i).unwrap_or(sheet_xml.len());
        let val = &sheet_xml[val_start..val_end];
        if val.contains(|c: char| c.is_ascii_alphabetic()) {
            let (col_part, row_part) = split_cell_ref(val);
            let cn = col_letter_to_num(&col_part);
            if cn >= col_index {
                let new_col = col_num_to_letters(cn + 1);
                result.push_str(&format!("{}{}", new_col, row_part));
            } else {
                result.push_str(val);
            }
        } else {
            result.push_str(val);
        }
        cursor = val_end;
    }
    result.push_str(&sheet_xml[cursor..]);

    // Shift col elements
    let p = super::layout::detect_prefix(&result);
    let cols_open = format!("<{}cols", p);
    if let Some(cols_start) = result.find(&cols_open) {
        let cols_close = format!("</{}cols>", p);
        let cols_end = result[cols_start..].find(&cols_close)
            .map(|i| cols_start + i + cols_close.len())
            .unwrap_or(result.len());
        let cols_block = &result[cols_start..cols_end].to_string();
        let mut new_block = format!("<{}cols>", p);
        let col_tag = format!("<{}col", p);
        let mut cc = 0;
        while let Some(cs) = cols_block[cc..].find(&col_tag) {
            let abs_cs = cc + cs;
            let ce = cols_block[abs_cs..].find("/>").map(|i| abs_cs + i + 2).unwrap_or(cols_block.len());
            let entry = &cols_block[abs_cs..ce];
            let min = extract_attr(entry, "min").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            let max = extract_attr(entry, "max").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            if min >= col_index {
                new_block.push_str(&format!("<{}col min=\"{}\" max=\"{}\"", p, min + 1, max + 1));
                // Copy remaining attrs
                let mut attrs = String::new();
                let mut ei = entry.find(' ').unwrap_or(entry.len());
                while ei < entry.len() {
                    while ei < entry.len() && entry.as_bytes()[ei] == b' ' { ei += 1; }
                    if ei >= entry.len() { break; }
                    let eq = entry[ei..].find('=').map(|p| ei + p).unwrap_or(entry.len());
                    let key = entry[ei..eq].trim();
                    let vs = eq + 1;
                    if vs < entry.len() && entry.as_bytes()[vs] == b'"' {
                        let ve = entry[vs + 1..].find('"').map(|p| vs + 1 + p).unwrap_or(entry.len());
                        let v = &entry[vs + 1..ve];
                        match key {
                            "min" | "max" => {}
                            _ => { attrs.push_str(&format!(" {}=\"{}\"", key, v)); }
                        }
                        ei = ve + 1;
                    } else { ei = eq + 1; }
                }
                new_block.push_str(&attrs);
                new_block.push_str("/>");
            } else {
                new_block.push_str(entry);
            }
            cc = ce;
        }
        new_block.push_str(&format!("</{}cols>", p));
        result = format!("{}{}{}", &result[..cols_start], new_block, &result[cols_end..]);
    }

    Ok(result)
}

pub fn delete_column(sheet_xml: &str, col_index: u32) -> Result<String, String> {
    let mut result = String::new();
    let mut cursor = 0;

    while let Some(pos) = sheet_xml[cursor..].find("r=\"") {
        let abs_pos = cursor + pos;
        result.push_str(&sheet_xml[cursor..abs_pos + 3]);
        let val_start = abs_pos + 3;
        let val_end = sheet_xml[val_start..].find('"').map(|i| val_start + i).unwrap_or(sheet_xml.len());
        let val = &sheet_xml[val_start..val_end];
        if val.contains(|c: char| c.is_ascii_alphabetic()) {
            let (col_part, row_part) = split_cell_ref(val);
            let cn = col_letter_to_num(&col_part);
            if cn > col_index {
                let new_col = col_num_to_letters(cn - 1);
                result.push_str(&format!("{}{}", new_col, row_part));
            } else if cn == col_index {
                // Skip this cell reference — the column is being removed
                // But we need to keep the ref to maintain XML structure;
                // we just keep it as-is (it'll be orphaned but not broken)
                result.push_str(val);
            } else {
                result.push_str(val);
            }
        } else {
            result.push_str(val);
        }
        cursor = val_end;
    }
    result.push_str(&sheet_xml[cursor..]);

    // Update col elements
    let p = super::layout::detect_prefix(&result);
    let cols_open = format!("<{}cols", p);
    if let Some(cols_start) = result.find(&cols_open) {
        let cols_close = format!("</{}cols>", p);
        let cols_end = result[cols_start..].find(&cols_close)
            .map(|i| cols_start + i + cols_close.len())
            .unwrap_or(result.len());
        let cols_block = &result[cols_start..cols_end].to_string();
        let mut new_block = format!("<{}cols>", p);
        let col_tag = format!("<{}col", p);
        let mut cc = 0;
        while let Some(cs) = cols_block[cc..].find(&col_tag) {
            let abs_cs = cc + cs;
            let ce = cols_block[abs_cs..].find("/>").map(|i| abs_cs + i + 2).unwrap_or(cols_block.len());
            let entry = &cols_block[abs_cs..ce];
            let min = extract_attr(entry, "min").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            let max = extract_attr(entry, "max").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            if max < col_index {
                new_block.push_str(entry);
            } else if min > col_index {
                let new_min = min - 1;
                let new_max = max - 1;
                let mut attrs = String::new();
                let mut ei = entry.find(' ').unwrap_or(entry.len());
                while ei < entry.len() {
                    while ei < entry.len() && entry.as_bytes()[ei] == b' ' { ei += 1; }
                    if ei >= entry.len() { break; }
                    let eq = entry[ei..].find('=').map(|p| ei + p).unwrap_or(entry.len());
                    let key = entry[ei..eq].trim();
                    let vs = eq + 1;
                    if vs < entry.len() && entry.as_bytes()[vs] == b'"' {
                        let ve = entry[vs + 1..].find('"').map(|p| vs + 1 + p).unwrap_or(entry.len());
                        let v = &entry[vs + 1..ve];
                        match key {
                            "min" | "max" => {}
                            _ => { attrs.push_str(&format!(" {}=\"{}\"", key, v)); }
                        }
                        ei = ve + 1;
                    } else { ei = eq + 1; }
                }
                if new_min <= new_max {
                    new_block.push_str(&format!("<{}col min=\"{}\" max=\"{}\"{}/>", p, new_min, new_max, attrs));
                }
            } else {
                // min <= col_index <= max: split
                if min < col_index {
                    let mut attrs = String::new();
                    let mut ei = entry.find(' ').unwrap_or(entry.len());
                    while ei < entry.len() {
                        while ei < entry.len() && entry.as_bytes()[ei] == b' ' { ei += 1; }
                        if ei >= entry.len() { break; }
                        let eq = entry[ei..].find('=').map(|p| ei + p).unwrap_or(entry.len());
                        let key = entry[ei..eq].trim();
                        let vs = eq + 1;
                        if vs < entry.len() && entry.as_bytes()[vs] == b'"' {
                            let ve = entry[vs + 1..].find('"').map(|p| vs + 1 + p).unwrap_or(entry.len());
                            let v = &entry[vs + 1..ve];
                            match key {
                                "min" | "max" => {}
                                _ => { attrs.push_str(&format!(" {}=\"{}\"", key, v)); }
                            }
                            ei = ve + 1;
                        } else { ei = eq + 1; }
                    }
                    new_block.push_str(&format!("<{}col min=\"{}\" max=\"{}\"{}/>", p, min, col_index - 1, attrs));
                }
                if max > col_index {
                    let mut attrs = String::new();
                    let mut ei = entry.find(' ').unwrap_or(entry.len());
                    while ei < entry.len() {
                        while ei < entry.len() && entry.as_bytes()[ei] == b' ' { ei += 1; }
                        if ei >= entry.len() { break; }
                        let eq = entry[ei..].find('=').map(|p| ei + p).unwrap_or(entry.len());
                        let key = entry[ei..eq].trim();
                        let vs = eq + 1;
                        if vs < entry.len() && entry.as_bytes()[vs] == b'"' {
                            let ve = entry[vs + 1..].find('"').map(|p| vs + 1 + p).unwrap_or(entry.len());
                            let v = &entry[vs + 1..ve];
                            match key {
                                "min" | "max" => {}
                                _ => { attrs.push_str(&format!(" {}=\"{}\"", key, v)); }
                            }
                            ei = ve + 1;
                        } else { ei = eq + 1; }
                    }
                    new_block.push_str(&format!("<{}col min=\"{}\" max=\"{}\"{}/>", p, col_index, max - 1, attrs));
                }
            }
            cc = ce;
        }
        new_block.push_str(&format!("</{}cols>", p));
        result = format!("{}{}{}", &result[..cols_start], new_block, &result[cols_end..]);
    }

    Ok(result)
}

/// Delete a sheet from the workbook. Used by remove_element for sheet-level paths.
pub fn delete_sheet(package: &mut OxmlPackage, sheet_name: &str) -> Result<(), HandlerError> {
    use crate::helpers;
    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    let ws = model.sheets.iter().find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;

    if package.has_part(&ws.part_path) {
        package.write_part(&ws.part_path, Vec::<u8>::new())
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    }

    let wb_xml = package.read_part_xml("xl/workbook.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let name_pattern = format!("name=\"{}\"", sheet_name);
    if let Some(name_pos) = wb_xml.find(&name_pattern) {
        let element_start = wb_xml[..name_pos].rfind("<sheet").unwrap_or(0);
        let element_end = find_element_end(&wb_xml, element_start, "sheet");
        let result = format!("{}{}", &wb_xml[..element_start], &wb_xml[element_end..]);
        package.write_part_xml("xl/workbook.xml", &result)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    Ok(())
}
