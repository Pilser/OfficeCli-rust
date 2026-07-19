/// Text offset mapping for xlsx documents.
/// Maps each cell's display text to a path for AI agent navigation.
use std::collections::HashMap;
use crate::dom_types::*;
use crate::helpers;
use handler_common::BBoxSpan;
use handler_common::HandlerError;
use handler_common::TextOffsetMap;
use oxml::OxmlPackage;

/// Build a TextOffsetMap for the workbook.
/// Each cell gets a span: path = "/SheetName/A1", element_type = "cell".
/// Cells are ordered sheet-by-sheet, row-by-row, col-by-col.
pub fn build_text_offset_map_internal(
    package: &OxmlPackage,
) -> Result<TextOffsetMap, HandlerError> {
    let model = helpers::build_workbook_model(package).map_err(HandlerError::OperationFailed)?;

    let mut map = TextOffsetMap::empty("xlsx");

    for ws in &model.sheets {
        // Sort cells by (row, col) for consistent ordering
        let cell_refs: Vec<&Cell> = ws.cells.values().collect();
        let mut sorted_cells = cell_refs;
        sorted_cells.sort_by(|a, b| (a.row, a.col).cmp(&(b.row, b.col)));

        // Pre-compute column X positions from col_widths
        let mut col_x_positions: HashMap<usize, f64> = HashMap::new();
        let mut x = 0.0_f64;
        let max_col = ws.max_col.max(
            ws.col_widths.keys().copied().max().unwrap_or(0),
        );
        for col in 1..=max_col {
            col_x_positions.insert(col, x);
            x += ws.col_widths.get(&col).copied().unwrap_or(64.0);
        }

        // Sheet header
        let sheet_header = format!("[{}]\n", ws.name);
        map.push_span(&sheet_header, &format!("/{}", ws.name), "sheet-header");

        let mut current_y = 0.0_f64;

        for i in 0..sorted_cells.len() {
            let cell = sorted_cells[i];
            let path = format!("/{}{}", ws.name, cell.ref_str);
            let text = format!("{}: {}\n", cell.ref_str, cell.display_value);

            // Compute BBox for this cell
            let x = col_x_positions.get(&cell.col).copied().unwrap_or(0.0);
            let y = current_y;
            let w = ws.col_widths.get(&cell.col).copied().unwrap_or(64.0);
            let h = ws.row_heights.get(&cell.row).copied().unwrap_or(15.0);

            let bbox = BBoxSpan {
                x: x as f32,
                y: y as f32,
                width: w as f32,
                height: h as f32,
            };

            // Cell content span with bbox
            map.push_span_with_metadata(&text, &path, "cell", Some(bbox), None);

            // Formula span (if present)
            if let Some(formula) = &cell.formula {
                let formula_text = format!("  ={}\n", formula);
                map.push_span(&formula_text, &format!("{}:formula", path), "cell-formula");
            }

            // Advance Y after each row (look ahead to next cell)
            if i + 1 < sorted_cells.len() {
                let next = sorted_cells[i + 1];
                if next.row != cell.row {
                    current_y += ws.row_heights.get(&cell.row).copied().unwrap_or(15.0);
                }
            }
        }

        // Row separator between sheets
        map.push_span("\n", &format!("/{}", ws.name), "sheet-separator");
    }

    Ok(map)
}
