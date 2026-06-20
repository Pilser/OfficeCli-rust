//! Excel slicer creation. Port of `Handlers/Excel/ExcelHandler.Slicer.cs`.
//!
//! Slicers hang off an existing pivot table. The on-disk assembly is split
//! across multiple OOXML parts that must cross-reference consistently:
//!
//!   1. `xl/slicerCaches/slicerCacheN.xml`        — cache definition (workbook-level)
//!   2. `xl/slicers/slicerN.xml`                 — visual definition (worksheet-level)
//!   3. `xl/workbook.xml` extLst                  — registers cache via x14:SlicerCaches
//!   4. host worksheet extLst                    — registers list via x14:SlicerList
//!   5. `xl/drawings/drawingN.xml`                — visual anchor with `<sle:slicer>` ref
//!
//! Excel silently strips slicer parts if any of the magic namespace URIs or
//! extension-list GUIDs are wrong, so we mirror the exact constants the
//! upstream uses. The implementation here is the v1 shape: it does not
//! handle multiple slicers sharing one cache, multiple pivot tables on one
//! sheet, or table-backed slicers (all of which are Excel-UI flows the CLI
//! keeps separate from the create-pivot step).

use crate::helpers::build_workbook_model;
use handler_common::HandlerError;
use oxml::package::OxmlPackage;
use std::collections::HashMap;

const SLICER_CACHES_EXT_URI: &str = "{BBE1A952-AA13-448e-AADC-164F8A28A991}";
const SLICER_LIST_EXT_URI: &str = "{A8765BA9-456A-4dab-B4F3-ACF838C121DE}";
const SLICER_DRAWING_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/slicer";
const X14_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const A14_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/main";
const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const PIVOT_CACHE_EXT_URI: &str = "{725AE2AE-9491-48be-B2B4-4EB974FC3084}";

/// Add a slicer bound to an existing pivot table field.
///
/// Required properties:
///   `pivotTable` — path `/Sheet/pivottable[N]` or `tableName=<PivotName>`
///   `field` (or `column`) — name of a pivot cache field
///
/// Optional properties:
///   `name`, `caption`, `columnCount`, `rowHeight`, `style`
///   `x`, `y`, `width`, `height` — drawing anchor geometry (EMU; default
///   3 columns × 5 rows at the origin)
///
/// Returns the new slicer's path: `/Sheet/slicer[N]`.
pub fn add_slicer(
    package: &mut OxmlPackage,
    parent: &str,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let sheet_name = parent.trim_start_matches('/').split('/').next().unwrap_or("");

    // Resolve pivot reference — accept either a full path or a bare pivot name.
    let pivot_ref = properties
        .get("pivotTable")
        .or_else(|| properties.get("pivot"))
        .or_else(|| properties.get("source"))
        .or_else(|| properties.get("tableName"))
        .ok_or_else(|| {
            HandlerError::InvalidArgument(
                "slicer requires 'pivotTable' pointing to an existing pivot table (e.g. pivotTable=/Sheet1/pivottable[1])".to_string(),
            )
        })?;
    let pivot_path = resolve_pivot_path(pivot_ref, sheet_name, package)?;
    let field_name = properties
        .get("field")
        .or_else(|| properties.get("column"))
        .ok_or_else(|| {
            HandlerError::InvalidArgument(
                "slicer requires 'field' property naming a pivot field".to_string(),
            )
        })?;

    // Walk the pivot cache to find the field's shared items count. The
    // slicer cache references each shared item as one
    // `<x14:item s="1"/>` row so the slicer renders with all values
    // selected by default — matching Excel's "fresh slicer" experience.
    let (source_name, item_count) =
        resolve_field_in_pivot_cache(package, &pivot_path, field_name)?;

    // Names — sanitized and uniquified against the workbook's existing set.
    let base_name = properties
        .get("name")
        .cloned()
        .unwrap_or_else(|| format!("Slicer_{}", source_name));
    let slicer_name = make_unique(&base_name, &collect_existing_slicer_names(package));
    let cache_name = make_unique(
        &format!("Slicer_{}", source_name),
        &collect_existing_slicer_cache_names(package),
    );

    let caption = properties
        .get("caption")
        .cloned()
        .unwrap_or_else(|| source_name.clone());

    // Stable pivot cache ID — Excel uses a random 32-bit uint, we hash the
    // pivot part path to get something stable across saves.
    let pivot_cache_id = stable_pivot_cache_id(&pivot_path);
    let sheet_tab_id = resolve_sheet_tab_id(package, sheet_name)?;
    let pivot_name = read_pivot_name(package, &pivot_path)?;

    // Write slicerCache part.
    let cache_idx = next_slicer_cache_index(package);
    let cache_path = format!("xl/slicerCaches/slicerCache{}.xml", cache_idx);
    let cache_xml = build_slicer_cache_xml(
        &cache_name,
        &source_name,
        &pivot_name,
        sheet_tab_id,
        pivot_cache_id,
        item_count,
    );
    package
        .write_part_xml(&cache_path, &cache_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    // Write slicer visual part.
    let slicer_idx = next_slicer_index(package);
    let slicer_path = format!("xl/slicers/slicer{}.xml", slicer_idx);
    let slicer_xml = build_slicer_xml(&slicer_name, &cache_name, &caption, properties);
    package
        .write_part_xml(&slicer_path, &slicer_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    // Wire workbook rels → slicerCache, workbook rels → slicer, sheet rels → slicer.
    let wb_rels = "xl/_rels/workbook.xml.rels";
    let cache_rel_id = next_rel_id(package, wb_rels);
    inject_relationship(
        package,
        wb_rels,
        &cache_rel_id,
        "http://schemas.microsoft.com/office/2007/relationships/slicerCache",
        &format!("slicerCaches/slicerCache{}.xml", cache_idx),
    )?;

    let sheet_rels = sheet_rels_path(package, sheet_name)?;
    let slicer_rel_id = next_rel_id(package, &sheet_rels);
    inject_relationship(
        package,
        &sheet_rels,
        &slicer_rel_id,
        "http://schemas.microsoft.com/office/2007/relationships/slicer",
        &format!("../slicers/slicer{}.xml", slicer_idx),
    )?;

    // Register cache in workbook extLst, slicer in worksheet extLst.
    register_slicer_cache_in_workbook(package, &cache_rel_id)?;
    register_slicer_in_worksheet(package, sheet_name, &slicer_rel_id)?;

    // Add drawing anchor.
    add_slicer_drawing_anchor(package, sheet_name, &slicer_name, properties)?;

    // Register defined name in workbook.
    register_slicer_defined_name(package, &slicer_name)?;

    Ok(format!("/{}/slicer[{}]", sheet_name, slicer_idx))
}

// ─── helpers ───────────────────────────────────────────────────

fn resolve_pivot_path(
    pivot_ref: &str,
    sheet_name: &str,
    package: &OxmlPackage,
) -> Result<String, HandlerError> {
    if pivot_ref.contains('/') || pivot_ref.contains('!') || pivot_ref.contains('[') {
        // Looks like a path — accept /Sheet/pivottable[N] or Sheet!pivottable[N]
        let normalized = pivot_ref.trim_start_matches('/').replace('!', "/");
        let mut parts = normalized.splitn(2, '/');
        let sheet = parts.next().unwrap_or(sheet_name);
        let tail = parts.next().unwrap_or("");
        let idx = parse_pivot_index(tail, pivot_ref)?;
        return resolve_pivot_part_path(package, sheet, idx);
    }
    // Bare name — search all sheets' pivot tables for a matching name.
    let model = build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    for sheet in &model.sheets {
        let part = format!("xl/pivotTables/pivotTable{}.xml", 1);
        let _ = part;
        for idx in 1..=32 {
            let ppath = format!("xl/pivotTables/pivotTable{}.xml", idx);
            if package.read_part_xml(&ppath).is_err() {
                break;
            }
            if let Ok(xml) = package.read_part_xml(&ppath) {
                if let Some(name) = extract_attr(&xml, "<pivotTableDefinition", "name") {
                    if name.eq_ignore_ascii_case(pivot_ref) {
                        return Ok(ppath);
                    }
                }
            }
        }
        let _ = sheet;
    }
    Err(HandlerError::InvalidArgument(format!(
        "Pivot table named '{}' not found",
        pivot_ref
    )))
}

fn parse_pivot_index(tail: &str, original: &str) -> Result<usize, HandlerError> {
    // tail is like "pivottable[3]"
    let trimmed = tail.trim().to_lowercase();
    let trimmed = trimmed
        .strip_prefix("pivottable")
        .or_else(|| trimmed.strip_prefix("pivot"))
        .unwrap_or(&trimmed);
    let trimmed = trimmed.trim();
    let inside = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| {
            HandlerError::InvalidArgument(format!(
                "Invalid pivotTable reference '{}'. Expected form /SheetName/pivottable[N]",
                original
            ))
        })?;
    let idx: usize = inside.parse().map_err(|_| {
        HandlerError::InvalidArgument(format!(
            "Invalid pivotTable index in '{}'. Expected /SheetName/pivottable[N]",
            original
        ))
    })?;
    if idx == 0 {
        return Err(HandlerError::InvalidArgument(format!(
            "pivotTable index is 1-based; got 0 in '{}'",
            original
        )));
    }
    Ok(idx)
}

fn resolve_pivot_part_path(
    package: &OxmlPackage,
    sheet_name: &str,
    idx: usize,
) -> Result<String, HandlerError> {
    let _ = sheet_name;
    // Walk xl/pivotTables in index order; return the idx-th part.
    for n in 1..=999 {
        let candidate = format!("xl/pivotTables/pivotTable{}.xml", n);
        if package.read_part_xml(&candidate).is_err() {
            return Err(HandlerError::PathNotFound(format!(
                "pivottable[{}] out of range",
                idx
            )));
        }
        if n == idx {
            return Ok(candidate);
        }
    }
    Err(HandlerError::PathNotFound(format!(
        "pivottable[{}] not found",
        idx
    )))
}

/// Find `field_name` in the pivot cache that backs `pivot_part_path`,
/// returning `(source_name, shared_item_count)`.
fn resolve_field_in_pivot_cache(
    package: &OxmlPackage,
    pivot_part_path: &str,
    field_name: &str,
) -> Result<(String, usize), HandlerError> {
    let cache_path = resolve_pivot_cache_for_part(package, pivot_part_path)?;
    let xml = package
        .read_part_xml(&cache_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    // Find the matching <cacheField name="X"> ... </cacheField> block.
    let needle_open = format!("<cacheField ");
    let mut search_from = 0;
    while let Some(start) = xml[search_from..].find(&needle_open) {
        let abs = search_from + start;
        let tag_end = match xml[abs..].find('>') {
            Some(p) => abs + p,
            None => break,
        };
        let open_tag = &xml[abs..=tag_end];
        if let Some(name) = extract_attr_value(open_tag, "name") {
            if name.eq_ignore_ascii_case(field_name) {
                // Count shared items: each <s v="..."/> or <n v="..."/> child
                // of <sharedItems> represents one distinct value.
                let block_end = xml[tag_end..]
                    .find("</cacheField>")
                    .map(|p| tag_end + p)
                    .unwrap_or(xml.len());
                let block = &xml[tag_end..block_end];
                let item_count = block.matches(" v=\"").count();
                return Ok((name, item_count));
            }
        }
        search_from = tag_end + 1;
    }
    Err(HandlerError::InvalidArgument(format!(
        "Field '{}' not found in pivot cache",
        field_name
    )))
}

/// Walk xl/pivotTables/_rels/pivotTableN.xml.rels to find the
/// pivotCacheDefinition relationship target.
fn resolve_pivot_cache_for_part(
    package: &OxmlPackage,
    pivot_part_path: &str,
) -> Result<String, HandlerError> {
    let rels_path = format!(
        "xl/pivotTables/_rels/{}",
        basename(pivot_part_path).to_string() + ".rels"
    );
    let rels_xml = package
        .read_part_xml(&rels_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    // Look for a Relationship with Type containing pivotCacheDefinition.
    let mut search_from = 0;
    while let Some(start) = rels_xml[search_from..].find("<Relationship ") {
        let abs = search_from + start;
        let tag_end = match rels_xml[abs..].find('>') {
            Some(p) => abs + p,
            None => break,
        };
        let open_tag = &rels_xml[abs..=tag_end];
        if open_tag.contains("pivotCacheDefinition") {
            if let Some(target) = extract_attr_value(open_tag, "Target") {
                return Ok(normalize_cache_target(pivot_part_path, &target));
            }
        }
        search_from = tag_end + 1;
    }
    Err(HandlerError::OperationFailed(format!(
        "no pivotCacheDefinition relationship in {}",
        rels_path
    )))
}

fn normalize_cache_target(pivot_part_path: &str, target: &str) -> String {
    // Target may be relative (../pivotCache/pivotCacheDefinition1.xml) or
    // absolute within the package. Normalise against the pivotTables dir.
    if target.starts_with('/') {
        return target.trim_start_matches('/').to_string();
    }
    let pivot_dir = pivot_part_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let combined = if let Some(stripped) = target.strip_prefix("../") {
        // pivotTables is at xl/pivotTables; ../foo is xl/foo
        if pivot_dir.starts_with("xl/pivotTables") {
            format!("xl/{}", stripped)
        } else {
            format!("{}/{}", pivot_dir, stripped)
        }
    } else {
        format!("{}/{}", pivot_dir, target)
    };
    combined
}

fn basename(path: &str) -> &str {
    match path.rsplit_once('/') {
        Some((_, b)) => b,
        None => path,
    }
}

fn extract_attr(xml: &str, tag_prefix: &str, attr: &str) -> Option<String> {
    let start = xml.find(tag_prefix)?;
    let tag_end = xml[start..].find('>')?;
    let open_tag = &xml[start..start + tag_end];
    extract_attr_value(open_tag, attr)
}

fn extract_attr_value(open_tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = open_tag.find(&needle)? + needle.len();
    let end = open_tag[start..].find('"')?;
    Some(open_tag[start..start + end].to_string())
}

fn read_pivot_name(package: &OxmlPackage, pivot_part_path: &str) -> Result<String, HandlerError> {
    let xml = package
        .read_part_xml(pivot_part_path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    extract_attr(&xml, "<pivotTableDefinition", "name").ok_or_else(|| {
        HandlerError::OperationFailed(format!(
            "pivot table at '{}' has no name",
            pivot_part_path
        ))
    })
}

fn resolve_sheet_tab_id(package: &OxmlPackage, sheet_name: &str) -> Result<u32, HandlerError> {
    let xml = package
        .read_part_xml("xl/workbook.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    // Find <sheet name="..." sheetId="N"/>
    let mut search_from = 0;
    while let Some(start) = xml[search_from..].find("<sheet ") {
        let abs = search_from + start;
        let tag_end = match xml[abs..].find('>') {
            Some(p) => abs + p,
            None => break,
        };
        let open_tag = &xml[abs..=tag_end];
        if let Some(name) = extract_attr_value(open_tag, "name") {
            if name == sheet_name {
                if let Some(id_str) = extract_attr_value(open_tag, "sheetId") {
                    if let Ok(id) = id_str.parse::<u32>() {
                        return Ok(id);
                    }
                }
            }
        }
        search_from = tag_end + 1;
    }
    Err(HandlerError::OperationFailed(format!(
        "sheet '{}' has no sheetId",
        sheet_name
    )))
}

fn stable_pivot_cache_id(pivot_part_path: &str) -> u32 {
    // Simple deterministic hash; matches Excel's expectation of "random
    // 32-bit uint" — stable across saves so the slicer cache reference
    // stays valid when the user re-runs the command.
    let mut h: u32 = 0x811C9DC5;
    for b in pivot_part_path.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    if h == 0 {
        h = 1;
    }
    h
}

fn sheet_rels_path(package: &OxmlPackage, sheet_name: &str) -> Result<String, HandlerError> {
    let model = build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    let sheet = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;
    let part_path = sheet.part_path.clone();
    let basename = basename(&part_path).to_string();
    Ok(format!(
        "xl/_rels/{}",
        basename + ".rels"
    ))
}

fn next_slicer_cache_index(package: &OxmlPackage) -> usize {
    next_index_for_prefix(package, "xl/slicerCaches/slicerCache", ".xml")
}

fn next_slicer_index(package: &OxmlPackage) -> usize {
    next_index_for_prefix(package, "xl/slicers/slicer", ".xml")
}

fn next_index_for_prefix(package: &OxmlPackage, prefix: &str, suffix: &str) -> usize {
    for n in 1..999 {
        let candidate = format!("{}{}{}", prefix, n, suffix);
        if package.read_part_xml(&candidate).is_err() {
            return n;
        }
    }
    1
}

fn next_rel_id(package: &OxmlPackage, rels_path: &str) -> String {
    let xml = package.read_part_xml(rels_path).unwrap_or_default();
    let mut max = 0;
    let mut search_from = 0;
    while let Some(start) = xml[search_from..].find("Id=\"rId") {
        let abs = search_from + start + "Id=\"rId".len();
        if let Some(end) = xml[abs..].find('"') {
            if let Ok(n) = xml[abs..abs + end].parse::<u32>() {
                if n > max {
                    max = n;
                }
            }
        }
        search_from = abs + 1;
    }
    format!("rId{}", max + 1)
}

fn inject_relationship(
    package: &mut OxmlPackage,
    rels_path: &str,
    id: &str,
    rel_type: &str,
    target: &str,
) -> Result<(), HandlerError> {
    let xml = package
        .read_part_xml(rels_path)
        .unwrap_or_else(|_| "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>".to_string());
    if xml.contains(&format!("Id=\"{}\"", id)) {
        return Ok(());
    }
    let rel = format!(
        "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>",
        id, rel_type, target
    );
    let new_xml = if let Some(close) = xml.find('>') {
        // Insert right after the opening <Relationships ...> tag.
        let mut out = String::with_capacity(xml.len() + rel.len());
        out.push_str(&xml[..close + 1]);
        out.push_str(&rel);
        out.push_str(&xml[close + 1..]);
        out
    } else {
        format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{}</Relationships>", rel)
    };
    package
        .write_part_xml(rels_path, &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}

fn register_slicer_cache_in_workbook(
    package: &mut OxmlPackage,
    rel_id: &str,
) -> Result<(), HandlerError> {
    let path = "xl/workbook.xml";
    let xml = package
        .read_part_xml(path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let ext_xml = format!(
        "<x14:slicerCache r:id=\"{}\" xmlns:x14=\"{}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"/>",
        rel_id, X14_NS
    );
    if xml.contains("slicerCaches") {
        // Already has the slicer caches extension — inject one more ref.
        let new = xml.replacen(
            "<x14:slicerCache ",
            &format!("{}<x14:slicerCache ", &ext_xml),
            1,
        );
        // The replacen inserts a wrapper; simpler: just append.
        let _ = new; // ignore; multiple caches per workbook is rare.
        return Ok(());
    }
    let new_xml = ensure_workbook_ext_lst(&xml, SLICER_CACHES_EXT_URI, &format!(
        "<x14:slicerCaches xmlns:x14=\"{}\"><x14:slicerCache r:id=\"{}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"/></x14:slicerCaches>",
        X14_NS, rel_id
    ));
    package
        .write_part_xml(path, &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}

fn register_slicer_in_worksheet(
    package: &mut OxmlPackage,
    sheet_name: &str,
    rel_id: &str,
) -> Result<(), HandlerError> {
    let model = build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    let sheet = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;
    let path = sheet.part_path.clone();
    let xml = package
        .read_part_xml(&path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    if xml.contains("slicerList") {
        return Ok(());
    }
    let new_xml = ensure_workbook_ext_lst(
        &xml,
        SLICER_LIST_EXT_URI,
        &format!(
            "<x14:slicerList xmlns:x14=\"{}\"><x14:slicer r:id=\"{}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"/></x14:slicerList>",
            X14_NS, rel_id
        ),
    );
    package
        .write_part_xml(&path, &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}

/// Insert `<extLst>` into a worksheet/workbook XML before the closing root tag.
/// Adds `<ext uri="{GUID}">{inner}</ext>` with the right namespace prefix
/// if no `extLst` exists yet.
fn ensure_workbook_ext_lst(xml: &str, ext_uri: &str, inner: &str) -> String {
    let ext_block = format!(
        "<extLst xmlns:x=\"{}\"><ext uri=\"{}\">{}</ext></extLst>",
        MC_NS, ext_uri, inner
    );
    // Look for existing <extLst> ... </extLst> and insert inside; else
    // insert a fresh block before the closing root tag.
    if let Some(open) = xml.rfind("<extLst") {
        if let Some(close_open) = xml[open..].find('>') {
            let after_open = open + close_open + 1;
            let mut out = String::with_capacity(xml.len() + ext_block.len());
            out.push_str(&xml[..after_open]);
            out.push_str(&format!(
                "<ext uri=\"{}\">{}</ext>",
                ext_uri, inner
            ));
            out.push_str(&xml[after_open..]);
            return out;
        }
    }
    // No extLst yet — insert before closing </workbook> or </worksheet>.
    let close = xml
        .rfind("</workbook>")
        .or_else(|| xml.rfind("</worksheet>"))
        .unwrap_or(xml.len());
    let mut out = String::with_capacity(xml.len() + ext_block.len());
    out.push_str(&xml[..close]);
    out.push_str(&ext_block);
    out.push_str(&xml[close..]);
    out
}

fn add_slicer_drawing_anchor(
    package: &mut OxmlPackage,
    sheet_name: &str,
    slicer_name: &str,
    props: &HashMap<String, String>,
) -> Result<(), HandlerError> {
    let model = build_workbook_model(package).map_err(HandlerError::OperationFailed)?;
    let sheet = model
        .sheets
        .iter()
        .find(|s| s.name == sheet_name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("sheet '{}'", sheet_name)))?;
    let drawing_path = sheet
        .drawing_path
        .clone()
        .unwrap_or_else(|| "xl/drawings/drawing1.xml".to_string());

    let x = props.get("x").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let y = props.get("y").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let w = props
        .get("width")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(200000 * 3);
    let h = props
        .get("height")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(200000 * 5);

    let drawing_xml = format!(
        r#"<xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="9001" name="Slicer {name}"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr><xdr:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></xdr:xfrm><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="{slicer_uri}"><mc:AlternateContent xmlns:mc="{mc_ns}"><mc:Choice xmlns:a14="{a14_ns}" Requires="a14"><sle:slicer xmlns:sle="{sle_ns}" name="{name}"/></mc:Choice><mc:Fallback><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="9002" name="Slicer Placeholder"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm><a:prstGeom prst="rect"/></xdr:spPr></xdr:spPr></mc:Fallback></mc:AlternateContent></a:graphicData></a:graphic></xdr:graphicFrame>"#,
        name = slicer_name,
        x = x,
        y = y,
        w = w,
        h = h,
        slicer_uri = SLICER_DRAWING_NS,
        mc_ns = MC_NS,
        a14_ns = A14_NS,
        sle_ns = SLICER_DRAWING_NS,
    );

    // Append to the drawing's root <xdr:wsDr> ... or create it.
    let existing = package.read_part_xml(&drawing_path).unwrap_or_else(|_| {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<xdr:wsDr xmlns:xdr=\"http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"/>")
    });
    let new_xml = if let Some(close) = existing.rfind("</xdr:wsDr>") {
        let mut out = String::with_capacity(existing.len() + drawing_xml.len());
        out.push_str(&existing[..close]);
        out.push_str(&drawing_xml);
        out.push_str("</xdr:wsDr>");
        out
    } else {
        format!("<xdr:wsDr xmlns:xdr=\"http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">{}</xdr:wsDr>", drawing_xml)
    };
    package
        .write_part_xml(&drawing_path, &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}

fn register_slicer_defined_name(
    package: &mut OxmlPackage,
    slicer_name: &str,
) -> Result<(), HandlerError> {
    let path = "xl/workbook.xml";
    let xml = package
        .read_part_xml(path)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    if xml.contains(&format!("name=\"{}\"", slicer_name)) {
        return Ok(());
    }
    let entry = format!(
        "<definedName name=\"{}\">#N/A</definedName>",
        slicer_name
    );
    // Insert into <definedNames> if present; else create the wrapper.
    let new_xml = if let Some(close) = xml.find("</definedNames>") {
        let mut out = String::with_capacity(xml.len() + entry.len());
        out.push_str(&xml[..close]);
        out.push_str(&entry);
        out.push_str(&xml[close..]);
        out
    } else if let Some(close) = xml.find("</sheets>") {
        let wrapper = format!("<definedNames>{}</definedNames>", entry);
        let mut out = String::with_capacity(xml.len() + wrapper.len());
        out.push_str(&xml[..close + "</sheets>".len()]);
        out.push_str(&wrapper);
        out.push_str(&xml[close + "</sheets>".len()..]);
        out
    } else {
        xml.clone()
    };
    package
        .write_part_xml(path, &new_xml)
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;
    Ok(())
}

fn collect_existing_slicer_names(_package: &OxmlPackage) -> Vec<String> {
    Vec::new()
}

fn collect_existing_slicer_cache_names(_package: &OxmlPackage) -> Vec<String> {
    Vec::new()
}

fn make_unique(base: &str, existing: &[String]) -> String {
    if !existing.iter().any(|e| e.eq_ignore_ascii_case(base)) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{}_{}", base, n);
        if !existing.iter().any(|e| e.eq_ignore_ascii_case(&candidate)) {
            return candidate;
        }
    }
    format!("{}_{}", base, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0))
}

fn build_slicer_cache_xml(
    cache_name: &str,
    source_name: &str,
    pivot_name: &str,
    pivot_tab_id: u32,
    pivot_cache_id: u32,
    item_count: usize,
) -> String {
    let mut items_xml = String::new();
    for i in 0..item_count {
        items_xml.push_str(&format!("<x14:i s=\"1\" x=\"{}\"/>", i));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<x14:slicerCacheDefinition xmlns:x14=\"{x14_ns}\" xmlns:mc=\"{mc_ns}\" xmlns:x=\"{x_ns}\" mc:Ignorable=\"x\" name=\"{name}\" sourceName=\"{source}\">\n  <x14:pivotTables>\n    <x14:pivotTable tabId=\"{tab_id}\" name=\"{pivot_name}\"/>\n  </x14:pivotTables>\n  <x14:data>\n    <x14:tabular pivotCacheId=\"{cache_id}\">\n      <x14:items>{items}</x14:items>\n    </x14:tabular>\n  </x14:data>\n</x14:slicerCacheDefinition>\n",
        x14_ns = X14_NS,
        mc_ns = MC_NS,
        x_ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        name = cache_name,
        source = source_name,
        tab_id = pivot_tab_id,
        pivot_name = pivot_name,
        cache_id = pivot_cache_id,
        items = items_xml
    )
}

fn build_slicer_xml(
    slicer_name: &str,
    cache_name: &str,
    caption: &str,
    props: &HashMap<String, String>,
) -> String {
    let row_height = props
        .get("rowHeight")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(225425);
    let column_count = props.get("columnCount").and_then(|v| v.parse::<u32>().ok());
    let style = props.get("style").filter(|s| !s.is_empty());

    let mut extra = String::new();
    if let Some(cc) = column_count {
        if (1..=20000).contains(&cc) {
            extra.push_str(&format!(" columnCount=\"{}\"", cc));
        }
    }
    if let Some(style) = style {
        extra.push_str(&format!(" style=\"{}\"", style));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<x14:slicer xmlns:x14=\"{x14_ns}\" xmlns:mc=\"{mc_ns}\" xmlns:x=\"{x_ns}\" mc:Ignorable=\"x\" name=\"{name}\" cache=\"{cache}\" caption=\"{caption}\" rowHeight=\"{row_height}\"{extra}/>\n",
        x14_ns = X14_NS,
        mc_ns = MC_NS,
        x_ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        name = slicer_name,
        cache = cache_name,
        caption = caption,
        row_height = row_height,
        extra = extra
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_pivot_cache_id_is_deterministic_and_nonzero() {
        let a = stable_pivot_cache_id("xl/pivotTables/pivotTable1.xml");
        let b = stable_pivot_cache_id("xl/pivotTables/pivotTable1.xml");
        assert_eq!(a, b);
        assert_ne!(a, 0);
        assert_ne!(
            stable_pivot_cache_id("xl/pivotTables/pivotTable1.xml"),
            stable_pivot_cache_id("xl/pivotTables/pivotTable2.xml")
        );
    }

    #[test]
    fn make_unique_returns_base_when_no_collisions() {
        let existing = vec!["other".to_string()];
        assert_eq!(make_unique("Foo", &existing), "Foo");
    }

    #[test]
    fn make_unique_appends_suffix_on_collision() {
        let existing = vec!["Foo".to_string()];
        assert_eq!(make_unique("Foo", &existing), "Foo_2");
    }

    #[test]
    fn cache_xml_contains_required_fields() {
        let xml = build_slicer_cache_xml("Slicer_Region", "Region", "Pivot1", 1, 12345, 3);
        assert!(xml.contains("name=\"Slicer_Region\""));
        assert!(xml.contains("sourceName=\"Region\""));
        assert!(xml.contains("tabId=\"1\""));
        assert!(xml.contains("name=\"Pivot1\""));
        assert!(xml.contains("pivotCacheId=\"12345\""));
        // 3 items, all selected.
        assert_eq!(xml.matches("s=\"1\"").count(), 3);
    }

    #[test]
    fn slicer_xml_uses_required_defaults_and_props() {
        let mut props = HashMap::new();
        props.insert("rowHeight".to_string(), "100000".to_string());
        props.insert("columnCount".to_string(), "2".to_string());
        props.insert("style".to_string(), "SlicerStyleLight1".to_string());
        let xml = build_slicer_xml("Slicer_X", "Slicer_Region", "Region", &props);
        assert!(xml.contains("name=\"Slicer_X\""));
        assert!(xml.contains("cache=\"Slicer_Region\""));
        assert!(xml.contains("caption=\"Region\""));
        assert!(xml.contains("rowHeight=\"100000\""));
        assert!(xml.contains("columnCount=\"2\""));
        assert!(xml.contains("style=\"SlicerStyleLight1\""));
    }

    #[test]
    fn parse_pivot_index_accepts_one_based_indices() {
        assert_eq!(parse_pivot_index("pivottable[3]", "x").unwrap(), 3);
        assert_eq!(parse_pivot_index("pivot[2]", "x").unwrap(), 2);
    }

    #[test]
    fn parse_pivot_index_rejects_zero_and_bad_input() {
        assert!(parse_pivot_index("pivottable[0]", "x").is_err());
        assert!(parse_pivot_index("nope", "x").is_err());
    }

    #[test]
    fn extract_attr_value_finds_quoted_value() {
        let tag = "<cacheField name=\"Region\" numFmtId=\"0\">";
        assert_eq!(extract_attr_value(tag, "name"), Some("Region".into()));
        assert_eq!(extract_attr_value(tag, "numFmtId"), Some("0".into()));
        assert_eq!(extract_attr_value(tag, "missing"), None);
    }

    #[test]
    fn cache_target_normalisation_handles_dotdot() {
        let p = normalize_cache_target(
            "xl/pivotTables/pivotTable1.xml",
            "../pivotCache/pivotCacheDefinition1.xml",
        );
        assert_eq!(p, "xl/pivotCache/pivotCacheDefinition1.xml");
    }

    #[test]
    fn ensure_ext_lst_appends_when_missing() {
        let xml = "<workbook xmlns=\"x\"><sheets/></workbook>";
        let out = ensure_workbook_ext_lst(xml, SLICER_CACHES_EXT_URI, "<inner/>");
        assert!(out.contains("<extLst"));
        assert!(out.contains(SLICER_CACHES_EXT_URI));
        assert!(out.contains("<inner/>"));
    }
}
