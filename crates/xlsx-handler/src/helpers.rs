/// Parsing helpers for xlsx OOXML parts.
use crate::dom_types::*;
use oxml::OxmlPackage;
use quick_xml::events::Event;
use quick_xml::Reader;

/// Parse the workbook.xml to extract the sheet list.
/// Returns (sheet_name, part_path, rel_id) for each sheet.
pub fn parse_workbook(package: &OxmlPackage) -> Result<Vec<(String, String, String)>, String> {
    let xml = package
        .read_part_xml("xl/workbook.xml")
        .map_err(|e| format!("failed to read xl/workbook.xml: {}", e))?;

    // Parse relationships for workbook to resolve sheet paths
    let rels = package
        .part_rels("xl/workbook.xml")
        .map_err(|e| format!("failed to read workbook rels: {}", e))?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut sheets = Vec::new();
    let mut in_sheets = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                let local_name_ref: &[u8] = local_name.as_ref();
                match local_name_ref {
                    b"sheets" => in_sheets = true,
                    b"sheet" if in_sheets => {
                        let name = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"name")
                            .map(|a| String::from_utf8_lossy(a.value.as_ref()).to_string())
                            .unwrap_or_default();

                        let rel_id = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| {
                                let key = a.key.as_ref();
                                key == b"r:id" || key.ends_with(b":id") || key == b"id"
                            })
                            .map(|a| String::from_utf8_lossy(a.value.as_ref()).to_string())
                            .unwrap_or_default();

                        // Resolve the relationship target to get the part path
                        let target = rels
                            .get(&rel_id)
                            .map(|r| r.target.clone())
                            .unwrap_or_default();
                        let part_path = package.resolve_rel_target("xl/workbook.xml", &target);

                        sheets.push((name, part_path, rel_id));
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"sheets" {
                    in_sheets = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }

    Ok(sheets)
}

/// Parse the shared strings table from xl/sharedStrings.xml.
/// Returns a vector where index i corresponds to the shared string at that index.
pub fn parse_shared_strings(package: &OxmlPackage) -> Vec<String> {
    if !package.has_part("xl/sharedStrings.xml") {
        return Vec::new();
    }

    let xml = package
        .read_part_xml("xl/sharedStrings.xml")
        .unwrap_or_default();
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut strings = Vec::new();
    let mut current_text = String::new();
    let mut in_si = false;
    let mut in_t = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"si" {
                    in_si = true;
                    current_text.clear();
                }
                if e.local_name().as_ref() == b"t" && in_si {
                    in_t = true;
                }
            }
            Ok(Event::Text(e)) => {
                if in_t {
                    current_text.push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"t" {
                    in_t = false;
                }
                if e.local_name().as_ref() == b"si" {
                    in_si = false;
                    strings.push(current_text.clone());
                }
            }
            Ok(Event::Empty(e)) => {
                // Handle <t/> empty text elements
                if e.local_name().as_ref() == b"t" && in_si {
                    // Empty text — nothing to add
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
    }

    strings
}

/// Parse a single worksheet XML and extract cells.
pub fn parse_sheet(
    package: &OxmlPackage,
    part_path: &str,
    shared_strings: &[String],
) -> Result<Worksheet, String> {
    let xml = package
        .read_part_xml(part_path)
        .map_err(|e| format!("failed to read {}: {}", part_path, e))?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut cells: std::collections::HashMap<(usize, usize), Cell> =
        std::collections::HashMap::new();
    let mut max_col: usize = 0;
    let mut max_row: usize = 0;

    let mut in_cell = false;
    let mut cell_ref_str = String::new();
    let mut cell_value_type = CellValueType::Number;
    let mut cell_style_index: Option<usize> = None;
    let mut cell_value: Option<String> = None;
    let mut cell_formula: Option<String> = None;
    let mut in_v = false;
    let mut in_f = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"c" => {
                    in_cell = true;
                    cell_ref_str.clear();
                    cell_value = None;
                    cell_formula = None;
                    cell_value_type = CellValueType::Number;
                    cell_style_index = None;

                    for attr in e.attributes().filter_map(|a| a.ok()) {
                        let key = attr.key.as_ref();
                        if key == b"r" {
                            cell_ref_str = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                        } else if key == b"t" {
                            cell_value_type = CellValueType::from_attr(Some(
                                &String::from_utf8_lossy(attr.value.as_ref()),
                            ));
                        } else if key == b"s" {
                            let s_val = String::from_utf8_lossy(attr.value.as_ref());
                            cell_style_index = s_val.parse::<usize>().ok();
                        }
                    }
                }
                b"v" if in_cell => {
                    in_v = true;
                }
                b"f" if in_cell => {
                    in_f = true;
                }
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if in_v {
                    cell_value = Some(e.unescape().unwrap_or_default().to_string());
                }
                if in_f {
                    cell_formula = Some(e.unescape().unwrap_or_default().to_string());
                }
            }
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"v" => in_v = false,
                b"f" => in_f = false,
                b"c" => {
                    in_cell = false;

                    let cref = CellRef::parse(&cell_ref_str);
                    if let Some(cr) = cref {
                        let display_value =
                            resolve_display_value(&cell_value_type, &cell_value, shared_strings);

                        if cr.col > max_col {
                            max_col = cr.col;
                        }
                        if cr.row > max_row {
                            max_row = cr.row;
                        }

                        cells.insert(
                            (cr.row, cr.col),
                            Cell {
                                ref_str: cell_ref_str.clone(),
                                col: cr.col,
                                row: cr.row,
                                value_type: cell_value_type.clone(),
                                raw_value: cell_value.clone(),
                                formula: cell_formula.clone(),
                                display_value,
                                style_index: cell_style_index,
                            },
                        );
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => {
                // Handle <c r="A1"/> cells without v or f children
                if e.local_name().as_ref() == b"c" {
                    cell_ref_str.clear();
                    cell_value_type = CellValueType::Number;
                    cell_style_index = None;

                    for attr in e.attributes().filter_map(|a| a.ok()) {
                        let key = attr.key.as_ref();
                        if key == b"r" {
                            cell_ref_str = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                        } else if key == b"t" {
                            cell_value_type = CellValueType::from_attr(Some(
                                &String::from_utf8_lossy(attr.value.as_ref()),
                            ));
                        } else if key == b"s" {
                            let s_val = String::from_utf8_lossy(attr.value.as_ref());
                            cell_style_index = s_val.parse::<usize>().ok();
                        }
                    }

                    let cref = CellRef::parse(&cell_ref_str);
                    if let Some(cr) = cref {
                        if cr.col > max_col {
                            max_col = cr.col;
                        }
                        if cr.row > max_row {
                            max_row = cr.row;
                        }

                        cells.insert(
                            (cr.row, cr.col),
                            Cell {
                                ref_str: cell_ref_str.clone(),
                                col: cr.col,
                                row: cr.row,
                                value_type: cell_value_type.clone(),
                                raw_value: None,
                                formula: None,
                                display_value: String::new(),
                                style_index: cell_style_index,
                            },
                        );
                    }
                }
                // Handle <v/> or <f/> empty elements
                if e.local_name().as_ref() == b"v" && in_cell {
                    cell_value = Some(String::new());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error in {}: {}", part_path, e)),
            _ => {}
        }
    }

    Ok(Worksheet {
        name: String::new(),
        index: 0,
        part_path: part_path.to_string(),
        rel_id: String::new(),
        cells,
        max_col,
        max_row,
    })
}

/// Resolve the display value for a cell, considering its type and shared strings.
fn resolve_display_value(
    value_type: &CellValueType,
    raw_value: &Option<String>,
    shared_strings: &[String],
) -> String {
    match value_type {
        CellValueType::SharedString => {
            if let Some(val) = raw_value {
                let idx = val.parse::<usize>().unwrap_or(0);
                if idx < shared_strings.len() {
                    shared_strings[idx].clone()
                } else {
                    format!("[ss:{}]", idx)
                }
            } else {
                String::new()
            }
        }
        CellValueType::Boolean => {
            if let Some(val) = raw_value {
                if val == "1" {
                    "TRUE".to_string()
                } else if val == "0" {
                    "FALSE".to_string()
                } else {
                    val.clone()
                }
            } else {
                "".to_string()
            }
        }
        CellValueType::Error => raw_value.clone().unwrap_or_default(),
        CellValueType::InlineString | CellValueType::Number => {
            raw_value.clone().unwrap_or_default()
        }
    }
}

/// Build the full workbook model from the package, including formula evaluation.
pub fn build_workbook_model(package: &OxmlPackage) -> Result<WorkbookModel, String> {
    let shared_strings = parse_shared_strings(package);
    let sheet_info = parse_workbook(package)?;

    let mut sheets = Vec::new();
    for (idx, (name, part_path, rel_id)) in sheet_info.iter().enumerate() {
        let ws = parse_sheet(package, part_path, &shared_strings)?;
        sheets.push(Worksheet {
            name: name.clone(),
            index: idx + 1,
            part_path: part_path.clone(),
            rel_id: rel_id.clone(),
            ..ws
        });
    }

    let mut model = WorkbookModel {
        sheets,
        shared_strings,
        pivot_tables: Vec::new(),
    };

    // Evaluate formulas and populate display_value for formula cells
    evaluate_formulas(&mut model);

    // Discover pivot table definitions
    model.pivot_tables = parse_pivot_tables(package);

    Ok(model)
}

/// Evaluate all formula cells in the workbook and update their display_value.
fn evaluate_formulas(model: &mut WorkbookModel) {
    use crate::formula;

    // Collect all formula cells (sheet_idx, (row, col), formula_text)
    let formula_cells: Vec<(usize, (usize, usize), String)> = model
        .sheets
        .iter()
        .flat_map(|ws| {
            ws.cells
                .iter()
                .filter(|(_, c)| c.formula.is_some())
                .map(|(key, c)| (ws.index - 1, *key, c.formula.clone().unwrap_or_default()))
        })
        .collect();

    for (sheet_idx, key, formula_text) in formula_cells {
        if sheet_idx >= model.sheets.len() {
            continue;
        }
        // Strip the leading '=' if present
        let expr = formula_text.strip_prefix('=').unwrap_or(&formula_text);
        if let Some(result) = formula::evaluate(expr, model) {
            if let Some(cell) = model.sheets[sheet_idx].cells.get_mut(&key) {
                cell.display_value = result.as_string();
            }
        }
    }
}

/// Parse all pivot table definitions from xl/pivotTables/*.xml parts.
fn parse_pivot_tables(package: &OxmlPackage) -> Vec<PivotTableDef> {
    let mut pivot_tables = Vec::new();

    // Find all pivot table parts
    let all_parts = package.list_parts();
    let pivot_parts: Vec<String> = all_parts
        .iter()
        .filter(|p| p.starts_with("xl/pivotTables/") && p.ends_with(".xml"))
        .map(|s| (*s).clone())
        .collect();

    for part_path in &pivot_parts {
        let xml = match package.read_part_xml(part_path) {
            Ok(x) => x,
            Err(_) => continue,
        };

        let mut name = String::new();
        let mut cache_id: Option<String> = None;
        let mut field_count: usize = 0;

        // Parse the pivotTable definition XML
        let mut reader = Reader::from_str(&xml);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let local_name = e.local_name();
                    let local_name_ref: &[u8] = local_name.as_ref();
                    match local_name_ref {
                        b"pivotTableDefinition" => {
                            for attr in e.attributes().filter_map(|a| a.ok()) {
                                let key = attr.key.as_ref();
                                if key == b"name" {
                                    name = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                } else if key == b"cacheId" {
                                    cache_id = Some(
                                        String::from_utf8_lossy(attr.value.as_ref()).to_string(),
                                    );
                                }
                            }
                        }
                        b"pivotFields" => {
                            // Count pivotField children
                            for attr in e.attributes().filter_map(|a| a.ok()) {
                                if attr.key.as_ref() == b"count" {
                                    field_count =
                                        String::from_utf8_lossy(attr.value.as_ref())
                                            .parse()
                                            .unwrap_or(0);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Try to resolve the source range from the pivot cache definition
        let source_range = cache_id.as_ref().and_then(|cid| {
            resolve_pivot_cache_range(package, cid)
        });

        pivot_tables.push(PivotTableDef {
            name,
            cache_id,
            source_range,
            field_count,
            part_path: part_path.clone(),
        });
    }

    pivot_tables
}

/// Resolve the source range for a pivot table from its cache definition.
/// Looks for the cache definition in xl/pivotCache/pivotCacheDefinition*.xml
/// that matches the given cache ID.
fn resolve_pivot_cache_range(package: &OxmlPackage, cache_id: &str) -> Option<String> {
    // The cache ID is referenced from the pivot table definition.
    // We need to find the pivotCacheDefinition that corresponds to this cache ID.
    // The mapping is through workbook.xml relationships.

    // Try to find the cache definition by looking at pivotCache parts
    let all_parts = package.list_parts();
    let cache_parts: Vec<String> = all_parts
        .iter()
        .filter(|p| {
            p.starts_with("xl/pivotCache/") && p.contains("pivotCacheDefinition")
        })
        .map(|s| (*s).clone())
        .collect();

    for cache_part in &cache_parts {
        let xml = package.read_part_xml(cache_part).ok()?;

        // Check if this cache definition references the same cache ID
        if !xml.contains(&format!("cacheId=\"{}\"", cache_id))
            && !xml.contains(&format!("cacheId=\"{}\" ", cache_id))
            && !xml.contains(&format!("cacheId=\"{}\">", cache_id))
        {
            // If we can't match by cacheId, try the first cache definition
            // (many workbooks have only one)
            if cache_parts.len() > 1 {
                continue;
            }
        }

        // Parse the cache definition to extract the source range
        let mut reader = Reader::from_str(&xml);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let local_name = e.local_name();
                    let local_name_ref: &[u8] = local_name.as_ref();
                    if local_name_ref == b"cacheSource" {
                        for attr in e.attributes().filter_map(|a| a.ok()) {
                            if attr.key.as_ref() == b"type" {
                                let val = String::from_utf8_lossy(attr.value.as_ref());
                                if val != "worksheet" {
                                    // Only handle worksheet data sources
                                    return None;
                                }
                            }
                        }
                    }
                    if local_name_ref == b"worksheetSource" {
                        // worksheetSource has ref="Sheet1!A1:E100" or similar
                        for attr in e.attributes().filter_map(|a| a.ok()) {
                            let key = attr.key.as_ref();
                            if key == b"ref" {
                                return Some(
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string(),
                                );
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
    }

    None
}
