/// Mutation operations for xlsx documents: set cell values, formulas, remove, move, copy.
use crate::dom_types::*;
use crate::helpers;
use crate::layout;
use crate::navigation;
use crate::sheets;
use crate::styles;
use handler_common::{
    self, extract_find_replace_props, replace_in_string, FindReplaceOptions, HandlerError,
};
use oxml::OxmlPackage;
use std::collections::HashMap;

/// Remove an element from the workbook.
/// Supported paths:
///   /SheetName/A1 — remove a cell (clear its content from the worksheet XML)
///   /SheetName     — remove a sheet (remove part + update workbook.xml)
pub fn remove_element(
    package: &mut OxmlPackage,
    path: &str,
) -> Result<Option<String>, HandlerError> {
    let pc = navigation::parse_path(path)?;

    match (pc.sheet_name, pc.cell_ref) {
        (Some(sheet_name), Some(cell_ref)) => {
            remove_cell(package, &sheet_name, &cell_ref)?;
            Ok(Some(format!(
                "removed cell {}{}",
                sheet_name,
                cell_ref.to_string_ref()
            )))
        }
        (Some(sheet_name), None) => {
            sheets::delete_sheet(package, &sheet_name)?;
            Ok(Some(format!("removed sheet {}", sheet_name)))
        }
        (None, None) => Err(HandlerError::InvalidPath(
            "remove requires a sheet or cell path".to_string(),
        )),
        (None, Some(_)) => Err(HandlerError::InvalidPath(
            "cell path requires a sheet name".to_string(),
        )),
    }
}

/// Remove a cell from a worksheet by finding and deleting its <c> element.
fn remove_cell(
    package: &mut OxmlPackage,
    sheet_name: &str,
    cell_ref: &CellRef,
) -> Result<(), HandlerError> {
    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;

    let ws = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;

    let part_path = ws.part_path.clone();
    let ref_str = cell_ref.to_string_ref();

    let xml = package
        .read_part_xml(&part_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let p = detect_namespace_prefix(&xml);

    // Find the cell element
    let cell_pattern = format!("<{}c r=\"{}\"", p, ref_str);
    if let Some(cell_start) = xml.find(&cell_pattern) {
        let cell_end = find_cell_element_end(&xml, cell_start, &p)?;
        let mut result = xml[..cell_start].to_string();
        result.push_str(&xml[cell_end..]);
        package
            .write_part_xml(&part_path, &result)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    Ok(())
}

/// Move a cell's content from source to target.
/// Source: /SheetName/A1, Target: /SheetName/B1 (or different sheet)
pub fn move_cell(
    package: &mut OxmlPackage,
    source: &str,
    target_parent: Option<&str>,
) -> Result<String, HandlerError> {
    let source_pc = navigation::parse_path(source)?;

    let sheet_name = source_pc.sheet_name.ok_or_else(|| {
        HandlerError::InvalidPath("move source requires a sheet name".to_string())
    })?;
    let source_ref = source_pc.cell_ref.ok_or_else(|| {
        HandlerError::InvalidPath("move source requires a cell reference".to_string())
    })?;

    // Determine target
    let target_path = target_parent.unwrap_or("/");
    let target_pc = navigation::parse_path(target_path)?;

    let target_sheet = target_pc.sheet_name.unwrap_or(sheet_name.clone());
    let target_ref = target_pc.cell_ref.ok_or_else(|| {
        HandlerError::InvalidPath("move target requires a cell reference".to_string())
    })?;

    // 1. Copy cell content to target
    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;

    let src_ws = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;

    let src_cell = src_ws
        .cells
        .get(&(source_ref.row, source_ref.col))
        .ok_or_else(|| {
            HandlerError::PathNotFound(format!("cell {}{}", sheet_name, source_ref.to_string_ref()))
        })?;

    // Build properties from source cell
    let mut props = HashMap::new();
    if let Some(v) = &src_cell.raw_value {
        props.insert("value".to_string(), v.clone());
    }
    if let Some(f) = &src_cell.formula {
        props.insert("formula".to_string(), f.clone());
    }
    if let Some(si) = src_cell.style_index {
        props.insert("style".to_string(), si.to_string());
    }

    // Set the target cell
    let target_path_str = format!("/{}/{}", target_sheet, target_ref.to_string_ref());
    set_cell_properties(package, &target_path_str, &props)?;

    // 2. Remove the source cell
    remove_cell(package, &sheet_name, &source_ref)?;

    Ok(target_path_str)
}

/// Copy a cell's content from source to target (keeping source intact).
pub fn copy_cell(
    package: &mut OxmlPackage,
    source: &str,
    target_parent: &str,
) -> Result<String, HandlerError> {
    let source_pc = navigation::parse_path(source)?;

    let sheet_name = source_pc.sheet_name.ok_or_else(|| {
        HandlerError::InvalidPath("copy source requires a sheet name".to_string())
    })?;
    let source_ref = source_pc.cell_ref.ok_or_else(|| {
        HandlerError::InvalidPath("copy source requires a cell reference".to_string())
    })?;

    let target_pc = navigation::parse_path(target_parent)?;

    let target_sheet = target_pc.sheet_name.unwrap_or(sheet_name.clone());
    let target_ref = target_pc.cell_ref.ok_or_else(|| {
        HandlerError::InvalidPath("copy target requires a cell reference".to_string())
    })?;

    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;

    let src_ws = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;

    let src_cell = src_ws
        .cells
        .get(&(source_ref.row, source_ref.col))
        .ok_or_else(|| {
            HandlerError::PathNotFound(format!("cell {}{}", sheet_name, source_ref.to_string_ref()))
        })?;

    let mut props = HashMap::new();
    if let Some(v) = &src_cell.raw_value {
        props.insert("value".to_string(), v.clone());
    }
    if let Some(f) = &src_cell.formula {
        props.insert("formula".to_string(), f.clone());
    }
    if let Some(si) = src_cell.style_index {
        props.insert("style".to_string(), si.to_string());
    }

    let target_path_str = format!("/{}/{}", target_sheet, target_ref.to_string_ref());
    set_cell_properties(package, &target_path_str, &props)?;

    Ok(target_path_str)
}

/// Swap two cells' content (values, formulas, styles).
pub fn swap_cells(
    package: &mut OxmlPackage,
    path1: &str,
    path2: &str,
) -> Result<(String, String), HandlerError> {
    let pc1 = navigation::parse_path(path1)?;
    let pc2 = navigation::parse_path(path2)?;

    let sheet1 = pc1
        .sheet_name
        .ok_or_else(|| HandlerError::InvalidPath("swap path1 requires a sheet name".to_string()))?;
    let ref1 = pc1.cell_ref.ok_or_else(|| {
        HandlerError::InvalidPath("swap path1 requires a cell reference".to_string())
    })?;
    let sheet2 = pc2
        .sheet_name
        .ok_or_else(|| HandlerError::InvalidPath("swap path2 requires a sheet name".to_string()))?;
    let ref2 = pc2.cell_ref.ok_or_else(|| {
        HandlerError::InvalidPath("swap path2 requires a cell reference".to_string())
    })?;

    if sheet1 == sheet2 && ref1.row == ref2.row && ref1.col == ref2.col {
        return Err(HandlerError::InvalidArgument(
            "swap requires two different cells".to_string(),
        ));
    }

    // Read both cells' content
    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;

    let get_cell_props =
        |sheet_name: &str, cell_ref: &CellRef| -> Result<HashMap<String, String>, HandlerError> {
            let ws = model
                .sheets
                .iter()
                .find(|s| s.name == sheet_name)
                .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;
            let cell = ws.cells.get(&(cell_ref.row, cell_ref.col));
            let mut props = HashMap::new();
            if let Some(c) = cell {
                if let Some(v) = &c.raw_value {
                    props.insert("value".to_string(), v.clone());
                }
                if let Some(f) = &c.formula {
                    props.insert("formula".to_string(), f.clone());
                }
                if let Some(si) = c.style_index {
                    props.insert("style".to_string(), si.to_string());
                }
            }
            Ok(props)
        };

    let props1 = get_cell_props(&sheet1, &ref1)?;
    let props2 = get_cell_props(&sheet2, &ref2)?;

    // Apply cell2's content to cell1 and vice versa
    let path1_str = format!("/{}/{}", sheet1, ref1.to_string_ref());
    let path2_str = format!("/{}/{}", sheet2, ref2.to_string_ref());

    set_cell_properties(package, &path1_str, &props2)?;
    set_cell_properties(package, &path2_str, &props1)?;

    Ok((path1_str, path2_str))
}

/// Find the end position of an XML element (handles both self-closing and regular closing tags).
#[allow(dead_code)]
fn find_element_end(xml: &str, start: usize, tag: &str) -> usize {
    // Check if self-closing: look for /> before >
    let first_gt = xml[start..]
        .find('>')
        .map(|pos| start + pos)
        .unwrap_or(xml.len());

    if first_gt > 0 && xml.as_bytes().get(first_gt - 1) == Some(&b'/') {
        // Self-closing element: <tag .../>
        first_gt + 1
    } else {
        // Regular element: find </tag>
        let close_tag = format!("</{}>", tag);
        xml[first_gt..]
            .find(&close_tag)
            .map(|pos| first_gt + pos + close_tag.len())
            .unwrap_or(xml.len())
    }
}

/// Set properties on a cell identified by path like /Sheet1/A1.
pub fn set_cell_properties(
    package: &mut OxmlPackage,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    // Find/replace short-circuit: when `find` is present, scan shared strings and
    // inline worksheet strings across the whole workbook (or a single sheet when
    // path is "/SheetName").
    if properties.contains_key("find") {
        return apply_xlsx_find_replace(package, path, properties);
    }

    let pc = navigation::parse_path(path)?;

    // Need both sheet name and cell reference for set operations
    let sheet_name = pc.sheet_name.ok_or_else(|| {
        HandlerError::InvalidPath("set requires a sheet name in the path".to_string())
    })?;
    let cell_ref = pc.cell_ref.ok_or_else(|| {
        HandlerError::InvalidPath("set requires a cell reference (e.g. /Sheet1/A1)".to_string())
    })?;

    // Parse the model to find the sheet part path
    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;

    let ws = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;

    let part_path = ws.part_path.clone();
    let cell_ref_str = cell_ref.to_string_ref();

    // Read the current worksheet XML
    let xml = package
        .read_part_xml(&part_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let p = detect_namespace_prefix(&xml);

    let mut modified_xml = xml.clone();
    let mut unsupported = Vec::new();

    // Collect style-related properties for styles::register_style
    let mut style_props: HashMap<String, String> = HashMap::new();
    let mut has_style_props = false;

    for (key, value) in properties {
        match key.as_str() {
            "value" | "text" => {
                modified_xml = set_cell_value(
                    &modified_xml,
                    &cell_ref_str,
                    value,
                    &model.shared_strings,
                    &p,
                )?;
            }
            "formula" => {
                modified_xml = set_cell_formula(&modified_xml, &cell_ref_str, value, &p)?;
            }
            "style" => {
                style_props.insert(key.clone(), value.clone());
                has_style_props = true;
            }
            "numberformat" | "numberFormat" | "numFmt"
            | "font" | "fontName" | "font.name"
            | "fontSize" | "size" | "font.size"
            | "color" | "fontColor" | "font.color"
            | "bold" | "b" | "font.bold"
            | "italic" | "i" | "font.italic"
            | "underline" | "u" | "font.underline"
            | "fill" | "bgColor" | "bg" | "backgroundColor"
            | "fontColor2" | "color2"
            | "border" | "borderColor"
            | "alignment" | "align"
            | "valign" | "verticalAlignment"
            | "wrap" | "wrapText"
            | "indent"
            | "rotation" | "textRotation" => {
                style_props.insert(key.clone(), value.clone());
                has_style_props = true;
            }
            "type" => {
                style_props.insert(key.clone(), value.clone());
                has_style_props = true;
            }
            _ => {
                unsupported.push(key.clone());
            }
        }
    }

    // Register style and update cell if style properties were provided
    if has_style_props {
        let mut styles_model = styles::ensure_styles_part(package)?;
        let xf_id = styles::register_style(&mut styles_model, &style_props);
        let styles_xml = styles::serialize_styles_xml(&styles_model);
        package
            .write_part_xml("xl/styles.xml", &styles_xml)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
        modified_xml = set_cell_style(&modified_xml, &cell_ref_str, &xf_id.to_string(), &p)?;
    }

    // Write back the modified XML
    package
        .write_part_xml(&part_path, &modified_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    Ok(unsupported)
}

/// Set sheet-level properties (column widths, row heights, freeze panes, etc.).
pub fn set_sheet_properties(
    package: &mut OxmlPackage,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let pc = navigation::parse_path(path)?;
    let sheet_name = pc.sheet_name.ok_or_else(|| {
        HandlerError::InvalidPath("set requires a sheet name in the path".to_string())
    })?;

    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    let ws = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;
    let part_path = ws.part_path.clone();

    let sheet_xml = package
        .read_part_xml(&part_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let mut modified_sheet = sheet_xml.clone();
    let mut unsupported = Vec::new();

    for (key, value) in properties {
        match key.as_str() {
            "columnWidth" | "colWidth" => {
                if let Ok(w) = value.parse::<f64>() {
                    let col = pc.cell_ref.as_ref()
                        .and_then(|c| Some(c.col as u32))
                        .unwrap_or(1);
                    modified_sheet = layout::set_column_width(&modified_sheet, col, w);
                }
            }
            "rowHeight" => {
                if let Ok(h) = value.parse::<f64>() {
                    let row = pc.cell_ref.as_ref()
                        .and_then(|c| Some(c.row as u32))
                        .unwrap_or(1);
                    modified_sheet = layout::set_row_height(&modified_sheet, row, h);
                }
            }
            "freeze" | "freezePanes" => {
                modified_sheet = layout::freeze_panes(&modified_sheet, value);
            }
            "autoFilter" => {
                modified_sheet = layout::set_auto_filter(&modified_sheet, value);
            }
            "rename" => {
                let wb_xml = package.read_part_xml("xl/workbook.xml")
                    .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
                let new_wb = sheets::rename_sheet(&wb_xml, &sheet_name, value)
                    .map_err(HandlerError::OperationFailed)?;
                package.write_part_xml("xl/workbook.xml", &new_wb)
                    .map_err(|e| HandlerError::SaveError(e.to_string()))?;
            }
            "hideSheet" => {
                let wb_xml = package.read_part_xml("xl/workbook.xml")
                    .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
                let hidden = value == "true" || value == "1";
                let new_wb = sheets::set_sheet_visibility(&wb_xml, &sheet_name, hidden)
                    .map_err(HandlerError::OperationFailed)?;
                package.write_part_xml("xl/workbook.xml", &new_wb)
                    .map_err(|e| HandlerError::SaveError(e.to_string()))?;
            }
            "tabColor" => {
                let wb_xml = package.read_part_xml("xl/workbook.xml")
                    .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
                let new_wb = sheets::set_tab_color(&wb_xml, &sheet_name, value)
                    .map_err(HandlerError::OperationFailed)?;
                package.write_part_xml("xl/workbook.xml", &new_wb)
                    .map_err(|e| HandlerError::SaveError(e.to_string()))?;
            }
            "insertRow" => {
                if let Ok(r) = value.parse::<u32>() {
                    modified_sheet = sheets::insert_row(&modified_sheet, r)
                        .map_err(HandlerError::OperationFailed)?;
                }
            }
            "deleteRow" => {
                if let Ok(r) = value.parse::<u32>() {
                    modified_sheet = sheets::delete_row(&modified_sheet, r)
                        .map_err(HandlerError::OperationFailed)?;
                }
            }
            "insertColumn" => {
                if let Ok(c) = value.parse::<u32>() {
                    modified_sheet = sheets::insert_column(&modified_sheet, c)
                        .map_err(HandlerError::OperationFailed)?;
                }
            }
            "deleteColumn" => {
                if let Ok(c) = value.parse::<u32>() {
                    modified_sheet = sheets::delete_column(&modified_sheet, c)
                        .map_err(HandlerError::OperationFailed)?;
                }
            }
            _ => {
                unsupported.push(key.clone());
            }
        }
    }

    if modified_sheet != sheet_xml {
        package.write_part_xml(&part_path, &modified_sheet)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    Ok(unsupported)
}

/// Helper to detect namespacing prefix (e.g., "x:") used in worksheet XML.
fn detect_namespace_prefix(xml: &str) -> String {
    if let Some(pos) = xml.find("worksheet") {
        if let Some(lt_pos) = xml[..pos].rfind('<') {
            let prefix = &xml[lt_pos + 1..pos];
            if !prefix.is_empty() && prefix.ends_with(':') {
                return prefix.to_string();
            }
        }
    }
    "".to_string()
}

/// Set the value of a cell in the worksheet XML.
/// If the cell exists, update its <v> element. If not, insert a new <c> element.
fn set_cell_value(
    xml: &str,
    cell_ref: &str,
    value: &str,
    shared_strings: &[String],
    p: &str,
) -> Result<String, HandlerError> {
    // Check if the value matches an existing shared string
    let ss_idx = shared_strings.iter().position(|s| s == value);

    let (t_attr, v_content) = if let Some(idx) = ss_idx {
        // Use shared string reference
        ("t=\"s\"".to_string(), idx.to_string())
    } else if value == "TRUE" || value == "FALSE" {
        // Boolean
        (
            "t=\"b\"".to_string(),
            if value == "TRUE" {
                "1".to_string()
            } else {
                "0".to_string()
            },
        )
    } else if value.parse::<f64>().is_ok() {
        // Numeric
        ("".to_string(), value.to_string())
    } else {
        // Inline string
        ("t=\"str\"".to_string(), value.to_string())
    };

    // Try to find and update existing cell
    let cell_pattern = format!("<{}c r=\"{}\"", p, cell_ref);
    if let Some(cell_start) = xml.find(&cell_pattern) {
        // Find the end of this <c> element
        let cell_end = find_cell_element_end(xml, cell_start, p)?;

        let cell_xml = &xml[cell_start..cell_end];

        // Build new cell XML
        let new_cell = build_cell_xml(
            cell_ref,
            &t_attr,
            &v_content,
            None,
            &extract_existing_style(cell_xml),
            p,
        );

        let mut result = xml[..cell_start].to_string();
        result.push_str(&new_cell);
        result.push_str(&xml[cell_end..]);
        Ok(result)
    } else {
        // Insert new cell — find the <sheetData> element and insert
        insert_new_cell(xml, cell_ref, &t_attr, &v_content, None, "", p)
    }
}

/// Set the formula of a cell.
fn set_cell_formula(
    xml: &str,
    cell_ref: &str,
    formula: &str,
    p: &str,
) -> Result<String, HandlerError> {
    let cell_pattern = format!("<{}c r=\"{}\"", p, cell_ref);
    if let Some(cell_start) = xml.find(&cell_pattern) {
        let cell_end = find_cell_element_end(xml, cell_start, p)?;

        let cell_xml = &xml[cell_start..cell_end];
        let existing_style = extract_existing_style(cell_xml);
        let existing_type = extract_existing_type(cell_xml);
        let existing_value = extract_existing_value(cell_xml, p);

        // Formula cells: type should be empty (calculated) or "str" if result is inline string
        let new_cell = build_cell_xml(
            cell_ref,
            &existing_type,
            &existing_value,
            Some(formula),
            &existing_style,
            p,
        );

        let mut result = xml[..cell_start].to_string();
        result.push_str(&new_cell);
        result.push_str(&xml[cell_end..]);
        Ok(result)
    } else {
        // Insert new cell with formula (type defaults to calculated)
        insert_new_cell(xml, cell_ref, "", "", Some(formula), "", p)
    }
}

/// Set the style index of a cell.
fn set_cell_style(
    xml: &str,
    cell_ref: &str,
    style_index: &str,
    p: &str,
) -> Result<String, HandlerError> {
    let cell_pattern = format!("<{}c r=\"{}\"", p, cell_ref);
    if let Some(cell_start) = xml.find(&cell_pattern) {
        let cell_end = find_cell_element_end(xml, cell_start, p)?;
        let cell_xml = &xml[cell_start..cell_end];

        // Modify the s= attribute in the cell opening tag
        let new_cell_xml = modify_style_in_cell(cell_xml, style_index);

        let mut result = xml[..cell_start].to_string();
        result.push_str(&new_cell_xml);
        result.push_str(&xml[cell_end..]);
        Ok(result)
    } else {
        Err(HandlerError::PathNotFound(format!("cell {}", cell_ref)))
    }
}

/// Build a complete <c> element XML string.
fn build_cell_xml(
    ref_str: &str,
    t_attr: &str,
    v_content: &str,
    formula: Option<&str>,
    style_attr: &str,
    p: &str,
) -> String {
    let mut attrs = format!("r=\"{}\"", ref_str);
    if !t_attr.is_empty() {
        attrs.push_str(&format!(" {}", t_attr));
    }
    if !style_attr.is_empty() {
        attrs.push_str(&format!(" {}", style_attr));
    }

    if formula.is_none() && v_content.is_empty() {
        // Empty cell — self-closing
        return format!("<{}c {}/>", p, attrs);
    }

    let mut cell = format!("<{}c {}>", p, attrs);

    if let Some(f) = formula {
        cell.push_str(&format!("<{}f>{}</{}f>", p, f, p));
    }

    if !v_content.is_empty() {
        cell.push_str(&format!("<{}v>{}</{}v>", p, v_content, p));
    }

    cell.push_str(&format!("</{}c>", p));
    cell
}

/// Find the end position of a <c> element in XML text.
/// Handles both self-closing <c .../> and regular <c ...>...</c>.
fn find_cell_element_end(xml: &str, start: usize, p: &str) -> Result<usize, HandlerError> {
    // Check for regular closing tag — need to find matching </c>
    // Look for the next '>' after start to see if self-closing or not
    let first_gt = xml[start..]
        .find('>')
        .map(|pos| start + pos)
        .ok_or_else(|| {
            HandlerError::OperationFailed("malformed XML: no '>' in cell tag".to_string())
        })?;

    // Check if the character before '>' is '/' (self-closing)
    if xml.as_bytes().get(first_gt - 1) == Some(&b'/') {
        // Self-closing: end is at first_gt + 1
        Ok(first_gt + 1)
    } else {
        // Regular element: find </c>
        let close_tag = format!("</{}c>", p);
        let close_tag_pos = xml[first_gt..]
            .find(&close_tag)
            .map(|pos| first_gt + pos + close_tag.len())
            .ok_or_else(|| {
                HandlerError::OperationFailed(format!(
                    "malformed XML: no '{}' closing tag",
                    close_tag
                ))
            })?;
        Ok(close_tag_pos)
    }
}

/// Extract the s= attribute from an existing cell XML element.
fn extract_existing_style(cell_xml: &str) -> String {
    // Look for s="N" in the opening tag
    let s_pattern = "s=\"";
    if let Some(s_start) = cell_xml.find(s_pattern) {
        let val_start = s_start + s_pattern.len();
        if let Some(val_end) = cell_xml[val_start..].find('"') {
            return format!("s=\"{}\"", &cell_xml[val_start..val_start + val_end]);
        }
    }
    "".to_string()
}

/// Extract the t= attribute from an existing cell XML element.
fn extract_existing_type(cell_xml: &str) -> String {
    let t_pattern = "t=\"";
    if let Some(t_start) = cell_xml.find(t_pattern) {
        let val_start = t_start + t_pattern.len();
        if let Some(val_end) = cell_xml[val_start..].find('"') {
            return format!("t=\"{}\"", &cell_xml[val_start..val_start + val_end]);
        }
    }
    "".to_string()
}

/// Extract the value from the <v> element in an existing cell.
fn extract_existing_value(cell_xml: &str, p: &str) -> String {
    let v_start_pattern = format!("<{}v>", p);
    if let Some(v_start) = cell_xml.find(&v_start_pattern) {
        let content_start = v_start + v_start_pattern.len();
        let v_end_pattern = format!("</{}v>", p);
        if let Some(v_end) = cell_xml.find(&v_end_pattern) {
            if v_end > content_start {
                return cell_xml[content_start..v_end].to_string();
            }
        }
    }
    "".to_string()
}

/// Modify the s= attribute in a cell element's XML.
fn modify_style_in_cell(cell_xml: &str, new_style: &str) -> String {
    let s_pattern = "s=\"";
    if let Some(s_start) = cell_xml.find(s_pattern) {
        let val_start = s_start + s_pattern.len();
        if let Some(val_end) = cell_xml[val_start..].find('"') {
            let full_val_end = val_start + val_end;
            let mut result = cell_xml[..s_start].to_string();
            result.push_str(&format!("s=\"{}\"", new_style));
            result.push_str(&cell_xml[full_val_end + 1..]);
            return result;
        }
    }
    // No existing style — insert s= attribute into the opening tag
    let insert_pos = cell_xml
        .find("/>")
        .or_else(|| cell_xml.find('>'))
        .unwrap_or(cell_xml.len());
    let mut result = cell_xml[..insert_pos].to_string();
    result.push_str(&format!(" s=\"{}\"", new_style));
    result.push_str(&cell_xml[insert_pos..]);
    result
}

/// Insert a new <c> element into the <sheetData> section.
fn insert_new_cell(
    xml: &str,
    ref_str: &str,
    t_attr: &str,
    v_content: &str,
    formula: Option<&str>,
    style_attr: &str,
    p: &str,
) -> Result<String, HandlerError> {
    let new_cell = build_cell_xml(ref_str, t_attr, v_content, formula, style_attr, p);

    // Find <sheetData> opening tag
    let sd_start = xml.find(&format!("<{}sheetData", p)).ok_or_else(|| {
        HandlerError::OperationFailed(format!("no <{}sheetData> element found", p))
    })?;

    // Find the first <row> inside sheetData, or the closing </sheetData>
    let after_sd = &xml[sd_start..];
    let sd_gt = after_sd
        .find('>')
        .map(|pos| sd_start + pos + 1)
        .ok_or_else(|| HandlerError::OperationFailed(format!("malformed <{}sheetData>", p)))?;

    // Determine the row number from the cell reference
    let row_num = CellRef::parse(ref_str)
        .ok_or_else(|| HandlerError::InvalidPath(format!("invalid cell ref '{}'", ref_str)))?
        .row;

    // Try to find the matching <row r="N"> element
    let row_pattern = format!("<{}row r=\"{}\"", p, row_num);
    if let Some(row_start) = xml[sd_gt..].find(&row_pattern) {
        let abs_row_start = sd_gt + row_start;

        // Find end of row element
        let row_gt = xml[abs_row_start..]
            .find('>')
            .map(|pos| abs_row_start + pos + 1)
            .ok_or_else(|| HandlerError::OperationFailed(format!("malformed <{}row>", p)))?;

        // Insert cell after the row opening tag
        let mut result = xml[..row_gt].to_string();
        result.push_str(&new_cell);
        result.push_str(&xml[row_gt..]);
        Ok(result)
    } else {
        // No existing row — insert a new <row> with the cell
        let new_row = format!("<{}row r=\"{}\">{}  </{}row>", p, row_num, new_cell, p);

        // Insert before </sheetData>
        let sd_end_pattern = format!("</{}sheetData>", p);
        let sd_end = xml.find(&sd_end_pattern).ok_or_else(|| {
            HandlerError::OperationFailed(format!("no {} closing tag", sd_end_pattern))
        })?;

        let mut result = xml[..sd_end].to_string();
        result.push_str(&new_row);
        result.push('\n');
        result.push_str(&xml[sd_end..]);
        Ok(result)
    }
}

/// Apply font color and background highlights on cells.
pub fn apply_xlsx_range_highlights(
    package: &mut OxmlPackage,
    properties: &HashMap<String, String>,
    segments: &[handler_common::PathRangeSegment],
) -> Result<Vec<String>, HandlerError> {
    let color = properties
        .get("color")
        .or_else(|| properties.get("fontColor"));
    let bg_color = properties
        .get("bgColor")
        .or_else(|| properties.get("highlight"))
        .or_else(|| properties.get("bg"));

    if color.is_none() && bg_color.is_none() {
        return Ok(Vec::new());
    }

    // 1. Read and parse styles.xml
    let mut styles_xml = package.read_part_xml("xl/styles.xml").map_err(|e| {
        HandlerError::OperationFailed(format!("failed to read xl/styles.xml: {}", e))
    })?;

    let p = detect_stylesheet_namespace_prefix(&styles_xml);

    // 2. Build the new font XML if color specified
    let mut final_font_id = 0;
    if let Some(color_val) = color {
        let hex = format_excel_color(color_val);
        let mut new_font_xml = format!("<{}font>", p);

        let doc = roxmltree::Document::parse(&styles_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("failed to parse styles: {}", e)))?;
        let fonts_node = doc.descendants().find(|n| n.has_tag_name("fonts"));
        let mut font_copied = false;
        if let Some(fn_node) = fonts_node {
            if let Some(first_font) = fn_node.children().find(|n| n.has_tag_name("font")) {
                for child in first_font.children().filter(|n| n.is_element()) {
                    if child.tag_name().name() != "color" {
                        let child_slice = &styles_xml[child.range()];
                        new_font_xml.push_str(child_slice);
                    }
                }
                font_copied = true;
            }
        }
        if !font_copied {
            new_font_xml.push_str(&format!(
                "<{}sz val=\"11\"/><{}name val=\"Calibri\"/>",
                p, p
            ));
        }
        new_font_xml.push_str(&format!("<{}color rgb=\"{}\"/>", p, hex));
        new_font_xml.push_str(&format!("</{}font>", p));

        final_font_id = append_element_to_tag(&mut styles_xml, "fonts", &new_font_xml)?;
    }

    // 3. Build the new fill XML if bg_color specified
    let mut final_fill_id = 0;
    if let Some(bg_val) = bg_color {
        let hex = format_excel_color(bg_val);
        let new_fill_xml = format!(
            "<{}fill><{}patternFill patternType=\"solid\"><{}fgColor rgb=\"{}\"/></{}patternFill></{}fill>",
            p, p, p, hex, p, p
        );
        final_fill_id = append_element_to_tag(&mut styles_xml, "fills", &new_fill_xml)?;
    }

    // 4. Parse original cellXfs list from original styles_xml to allow style inheritance without borrowing styles_xml
    struct XfInfo {
        font_id: usize,
        fill_id: usize,
        xml: String,
    }

    let xf_infos: Vec<XfInfo> = {
        let doc = roxmltree::Document::parse(&styles_xml)
            .map_err(|e| HandlerError::OperationFailed(format!("failed to parse styles: {}", e)))?;
        let cell_xfs_node = doc
            .descendants()
            .find(|n| n.has_tag_name("cellXfs"))
            .ok_or_else(|| HandlerError::OperationFailed("cellXfs not found".to_string()))?;
        cell_xfs_node
            .children()
            .filter(|n| n.has_tag_name("xf"))
            .map(|n| {
                let font_id = n
                    .attribute("fontId")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                let fill_id = n
                    .attribute("fillId")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                let xml_slice = &styles_xml[n.range()];
                XfInfo {
                    font_id,
                    fill_id,
                    xml: xml_slice.to_string(),
                }
            })
            .collect()
    };

    // 5. Group target cells by worksheet
    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    let mut sheets_cells: HashMap<String, Vec<CellRef>> = HashMap::new();

    for seg in segments {
        let pc = navigation::parse_path(&seg.path)?;
        if let Some(sheet_name) = pc.sheet_name {
            if let Some(cell_ref) = pc.cell_ref {
                sheets_cells.entry(sheet_name).or_default().push(cell_ref);
            }
        }
    }

    // Cache to reuse new style indices: (orig_xf_id, font_id, fill_id) -> new_xf_id
    let mut style_cache: HashMap<(usize, usize, usize), usize> = HashMap::new();

    // 6. Process each worksheet
    for (sheet_name, cell_refs) in sheets_cells {
        let ws = model
            .sheets
            .iter()
            .find(|s| s.name == sheet_name)
            .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;

        let part_path = ws.part_path.clone();
        let mut sheet_xml = package.read_part_xml(&part_path).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to read sheet XML: {}", e))
        })?;

        let sp = detect_namespace_prefix(&sheet_xml);

        for cell_ref in cell_refs {
            let cell_ref_str = cell_ref.to_string_ref();
            let cell_pattern = format!("<{}c r=\"{}\"", sp, cell_ref_str);

            if let Some(cell_start) = sheet_xml.find(&cell_pattern) {
                let cell_end = find_cell_element_end(&sheet_xml, cell_start, &sp)?;
                let cell_xml = &sheet_xml[cell_start..cell_end];

                // Extract original style index
                let orig_style_index = if let Some(s_pos) = cell_xml.find("s=\"") {
                    let val_start = s_pos + "s=\"".len();
                    if let Some(val_len) = cell_xml[val_start..].find('"') {
                        cell_xml[val_start..val_start + val_len]
                            .parse::<usize>()
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                };

                let (orig_font_id, orig_fill_id) = if orig_style_index < xf_infos.len() {
                    let xf = &xf_infos[orig_style_index];
                    (xf.font_id, xf.fill_id)
                } else {
                    (0, 0)
                };

                let target_font_id = if color.is_some() {
                    final_font_id
                } else {
                    orig_font_id
                };
                let target_fill_id = if bg_color.is_some() {
                    final_fill_id
                } else {
                    orig_fill_id
                };

                let cache_key = (orig_style_index, target_font_id, target_fill_id);
                let new_style_index = if let Some(&xf_id) = style_cache.get(&cache_key) {
                    xf_id
                } else {
                    let xf_xml = if orig_style_index < xf_infos.len() {
                        &xf_infos[orig_style_index].xml
                    } else {
                        &xf_infos[0].xml
                    };
                    let new_xf_xml = clone_xf_with_changes(
                        xf_xml,
                        target_font_id,
                        target_fill_id,
                        color.is_some(),
                        bg_color.is_some(),
                        &p,
                    )?;
                    let xf_id = append_element_to_tag(&mut styles_xml, "cellXfs", &new_xf_xml)?;
                    style_cache.insert(cache_key, xf_id);
                    xf_id
                };

                let updated_cell_xml = modify_style_in_cell(cell_xml, &new_style_index.to_string());
                sheet_xml = format!(
                    "{}{}{}",
                    &sheet_xml[..cell_start],
                    updated_cell_xml,
                    &sheet_xml[cell_end..]
                );
            } else {
                // Cell doesn't exist. We use default style 0 as original style
                let target_font_id = if color.is_some() { final_font_id } else { 0 };
                let target_fill_id = if bg_color.is_some() { final_fill_id } else { 0 };

                let cache_key = (0, target_font_id, target_fill_id);
                let new_style_index = if let Some(&xf_id) = style_cache.get(&cache_key) {
                    xf_id
                } else {
                    let xf_xml = &xf_infos[0].xml;
                    let new_xf_xml = clone_xf_with_changes(
                        xf_xml,
                        target_font_id,
                        target_fill_id,
                        color.is_some(),
                        bg_color.is_some(),
                        &p,
                    )?;
                    let xf_id = append_element_to_tag(&mut styles_xml, "cellXfs", &new_xf_xml)?;
                    style_cache.insert(cache_key, xf_id);
                    xf_id
                };

                sheet_xml = insert_new_cell(
                    &sheet_xml,
                    &cell_ref_str,
                    "",
                    "",
                    None,
                    &format!("s=\"{}\"", new_style_index),
                    &sp,
                )?;
            }
        }

        package
            .write_part_xml(&part_path, &sheet_xml)
            .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    }

    // Write styles.xml back
    package
        .write_part_xml("xl/styles.xml", &styles_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    Ok(Vec::new())
}

fn detect_stylesheet_namespace_prefix(xml: &str) -> String {
    if let Some(pos) = xml.find("styleSheet") {
        if let Some(lt_pos) = xml[..pos].rfind('<') {
            let prefix = &xml[lt_pos + 1..pos];
            if !prefix.is_empty() && prefix.ends_with(':') {
                return prefix.to_string();
            }
        }
    }
    "".to_string()
}

fn format_excel_color(color_str: &str) -> String {
    let clean = color_str.trim_start_matches('#');
    let hex_lower = clean.to_lowercase();
    let resolved_hex = match hex_lower.as_str() {
        "yellow" => "FFFF00",
        "green" => "00FF00",
        "blue" => "0000FF",
        "magenta" => "FF00FF",
        "cyan" => "00FFFF",
        "red" => "FF0000",
        "white" => "FFFFFF",
        "black" => "000000",
        other => other,
    };
    if resolved_hex.len() == 6 {
        format!("FF{}", resolved_hex.to_uppercase())
    } else if resolved_hex.len() == 8 {
        resolved_hex.to_uppercase()
    } else {
        "FF000000".to_string()
    }
}

fn clone_xf_with_changes(
    xf_xml: &str,
    font_id: usize,
    fill_id: usize,
    apply_font: bool,
    apply_fill: bool,
    p: &str,
) -> Result<String, HandlerError> {
    let wrapped = format!(
        "<x:dummy xmlns:x=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">{}</x:dummy>",
        xf_xml
    );
    let doc = roxmltree::Document::parse(&wrapped)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to parse single xf: {}", e)))?;
    let xf_node = doc.root_element().first_element_child().ok_or_else(|| {
        HandlerError::OperationFailed("xf element not found inside dummy root".to_string())
    })?;

    let mut attrs = Vec::new();
    for attr in xf_node.attributes() {
        let name = attr.name();
        if name != "fontId" && name != "fillId" && name != "applyFont" && name != "applyFill" {
            attrs.push(format!("{}=\"{}\"", attr.name(), attr.value()));
        }
    }

    attrs.push(format!("fontId=\"{}\"", font_id));
    attrs.push(format!("fillId=\"{}\"", fill_id));
    if apply_font {
        attrs.push("applyFont=\"1\"".to_string());
    } else if let Some(val) = xf_node.attribute("applyFont") {
        attrs.push(format!("applyFont=\"{}\"", val));
    }
    if apply_fill {
        attrs.push("applyFill=\"1\"".to_string());
    } else if let Some(val) = xf_node.attribute("applyFill") {
        attrs.push(format!("applyFill=\"{}\"", val));
    }

    let mut children_xml = String::new();
    for child in xf_node.children().filter(|n| n.is_element()) {
        children_xml.push_str(&wrapped[child.range()]);
    }

    if children_xml.is_empty() {
        Ok(format!("<{}xf {}/>", p, attrs.join(" ")))
    } else {
        Ok(format!(
            "<{}xf {}>{}</{}xf>",
            p,
            attrs.join(" "),
            children_xml,
            p
        ))
    }
}

fn append_element_to_tag(
    xml: &mut String,
    tag_name: &str,
    new_element_xml: &str,
) -> Result<usize, HandlerError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to parse XML: {}", e)))?;
    let node = doc
        .descendants()
        .find(|n| n.has_tag_name(tag_name))
        .ok_or_else(|| HandlerError::OperationFailed(format!("tag <{}> not found", tag_name)))?;

    let node_start = node.range().start;

    let open_tag_end = xml[node_start..]
        .find('>')
        .map(|pos| node_start + pos)
        .ok_or_else(|| HandlerError::OperationFailed("malformed tag: no '>'".to_string()))?;

    let open_tag_text = &xml[node_start..=open_tag_end];

    let mut current_count = 0;
    let mut count_attr_range = None;

    if let Some(pos) = open_tag_text.find("count=\"") {
        let val_start = node_start + pos + "count=\"".len();
        if let Some(val_len) = xml[val_start..].find('"') {
            let val_end = val_start + val_len;
            if let Ok(c) = xml[val_start..val_end].parse::<usize>() {
                current_count = c;
                count_attr_range = Some(val_start..val_end);
            }
        }
    }

    let last_child = node.children().filter(|n| n.is_element()).next_back();

    let new_count = current_count + 1;

    let mut result = String::new();

    let full_name = xml[node_start + 1..]
        .split([' ', '>', '/', '\n', '\r', '\t'])
        .next()
        .unwrap_or("");
    let prefix = if let Some(colon_pos) = full_name.find(':') {
        full_name[..colon_pos + 1].to_string()
    } else {
        "".to_string()
    };

    if let Some(r) = count_attr_range {
        result.push_str(&xml[..r.start]);
        result.push_str(&new_count.to_string());

        if let Some(lc) = last_child {
            let lc_end = lc.range().end;
            result.push_str(&xml[r.end..lc_end]);
            result.push_str(new_element_xml);
            result.push_str(&xml[lc_end..]);
        } else if open_tag_text.trim_end().ends_with("/>") {
            let tag_open_without_slash = open_tag_text.replace("/>", ">");
            result.push_str(&xml[r.end..node_start]);
            result.push_str(&tag_open_without_slash);
            result.push_str(new_element_xml);
            result.push_str(&format!("</{}{}>", prefix, node.tag_name().name()));
            result.push_str(&xml[open_tag_end + 1..]);
        } else {
            result.push_str(&xml[r.end..open_tag_end + 1]);
            result.push_str(new_element_xml);
            result.push_str(&xml[open_tag_end + 1..]);
        }
    } else if let Some(lc) = last_child {
        let lc_end = lc.range().end;
        result.push_str(&xml[..lc_end]);
        result.push_str(new_element_xml);
        result.push_str(&xml[lc_end..]);
    } else if open_tag_text.trim_end().ends_with("/>") {
        let tag_open_without_slash = open_tag_text.replace("/>", ">");
        result.push_str(&xml[..node_start]);
        result.push_str(&tag_open_without_slash);
        result.push_str(new_element_xml);
        result.push_str(&format!("</{}{}>", prefix, node.tag_name().name()));
        result.push_str(&xml[open_tag_end + 1..]);
    } else {
        result.push_str(&xml[..open_tag_end + 1]);
        result.push_str(new_element_xml);
        result.push_str(&xml[open_tag_end + 1..]);
    }

    *xml = result;
    Ok(current_count)
}

// ─── Find & Replace ──────────────────────────────────────────────────

/// Apply find/replace to xlsx shared strings and worksheet inline strings.
///
/// Scope rules:
///   - Path "/" or "" → all shared strings + all worksheets
///   - Path "/SheetName" → only that sheet's inline strings
///   - Path "/SheetName/A1" → just that cell's <is><t>...</t></is> inline string
pub fn apply_xlsx_find_replace(
    package: &mut OxmlPackage,
    path: &str,
    properties: &HashMap<String, String>,
) -> Result<Vec<String>, HandlerError> {
    let (find, replace, opts) = extract_find_replace_props(properties).ok_or_else(|| {
        HandlerError::InvalidArgument(
            "find/replace requires at least a 'find=<text>' property".to_string(),
        )
    })?;

    let mut total = 0usize;

    // 1. Shared strings table — always scan unless scoped to a specific cell.
    let pc = navigation::parse_path(path).ok();
    let scoped_to_cell = pc.as_ref().and_then(|p| p.cell_ref.clone()).is_some();

    if !scoped_to_cell {
        if let Some(ss_xml) = read_part_xml_optional(package, "xl/sharedStrings.xml")? {
            let xml = ss_xml;
            let (new_xml, n) = replace_in_xml_text_nodes(&xml, &find, &replace, &opts, "t", "</t>");
            if n > 0 {
                total += n;
                package
                    .write_part_xml("xl/sharedStrings.xml", &new_xml)
                    .map_err(|e| HandlerError::SaveError(e.to_string()))?;
            }
        }
    }

    // 2. Worksheet inline (only if path is root, /SheetName, or /SheetName/cell)
    let sheet_filter = pc.as_ref().and_then(|p| p.sheet_name.as_deref());

    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    for ws in &model.sheets {
        if let Some(s) = sheet_filter {
            if ws.name != s {
                continue;
            }
        }
        let part = match ws.part_path.strip_prefix('/') {
            Some(p) => p.to_string(),
            None => ws.part_path.clone(),
        };
        let Some(xml) = read_part_xml_optional(package, &part)? else {
            continue;
        };
        // Worksheet text lives in two shapes: shared-string references become
        // `<c t="s"><v>idx</v></c>` (numeric index, not actual text — skip), while
        // inline/typed strings become `<c t="str"><v>text</v></c>` or
        // `<c t="inlineStr"><is><t>text</t></is></c>`. We scan both `<v>` and `<t>`
        // here; for shared-string-index `<v>` nodes the value is a number and
        // won't match user-supplied find text, so it's safe.
        let mut xml = xml;
        let (next, n_t) = replace_in_xml_text_nodes(&xml, &find, &replace, &opts, "t", "</t>");
        xml = next;
        let (newer, n_v) = replace_in_xml_text_nodes(&xml, &find, &replace, &opts, "v", "</v>");
        let n = n_t + n_v;
        if n > 0 {
            xml = newer;
            total += n;
            package
                .write_part_xml(&part, &xml)
                .map_err(|e| HandlerError::SaveError(e.to_string()))?;
        }
    }

    Ok(vec![format!("replaced={}", total)])
}

/// Read a part as XML if it exists, returning None if not present.
fn read_part_xml_optional(
    package: &OxmlPackage,
    part_path: &str,
) -> Result<Option<String>, HandlerError> {
    match package.read_part_xml(part_path) {
        Ok(xml) => Ok(Some(xml)),
        Err(_) => Ok(None),
    }
}

/// Find every `<open_prefix...>...</close_tag>` block in `xml` and run
/// replace_in_string on its inner text. Returns (new_xml, count).
/// `open_prefix` is the leading tag name without `<`, e.g. "t" matches `<t>`,
/// `<t ...>`, `<t/>`. `close_tag` includes the angle brackets, e.g. "</t>".
fn replace_in_xml_text_nodes(
    xml: &str,
    find: &str,
    replace: &str,
    opts: &FindReplaceOptions,
    open_prefix: &str,
    close_tag: &str,
) -> (String, usize) {
    let needle = format!("<{}", open_prefix);
    let mut out = String::with_capacity(xml.len());
    let mut cursor = 0;
    let mut total = 0usize;

    while let Some(close_start) = xml[cursor..].find(close_tag) {
        let close_abs = cursor + close_start;
        // Walk forward through every `<tag` occurrence in the prefix and keep
        // the last one (the innermost open for this close tag).
        let prefix = &xml[..close_abs];
        let mut open_idx = None;
        let mut search_from = 0;
        while let Some(o) = prefix[search_from..].find(&needle) {
            let abs = search_from + o;
            // Must be followed by `>`, ` `, `/`, or attribute whitespace
            let after = &prefix[abs + needle.len()..];
            let c = after.as_bytes().first().copied();
            match c {
                Some(b'>') | Some(b' ') | Some(b'/') | Some(b'\t') | Some(b'\n') => {
                    open_idx = Some(abs);
                }
                _ => {}
            }
            search_from = abs + needle.len();
        }

        let Some(open_abs) = open_idx else {
            out.push_str(&xml[cursor..close_abs + close_tag.len()]);
            cursor = close_abs + close_tag.len();
            continue;
        };

        // Find the close of the opening tag (`>`)
        let Some(gt_rel) = xml[open_abs..close_abs].find('>') else {
            out.push_str(&xml[cursor..close_abs + close_tag.len()]);
            cursor = close_abs + close_tag.len();
            continue;
        };
        let open_close = open_abs + gt_rel + 1;
        let inner = &xml[open_close..close_abs];
        let (new_inner, n) = replace_in_string(inner, find, replace, opts);
        total += n;

        out.push_str(&xml[cursor..open_close]);
        out.push_str(&new_inner);
        cursor = close_abs;
        out.push_str(&xml[cursor..cursor + close_tag.len()]);
        cursor += close_tag.len();
    }
    out.push_str(&xml[cursor..]);
    (out, total)
}

// Re-export the find/replace property key list so the handler surface
// matches the C# command registration.
pub use handler_common::find_replace_property_keys;
