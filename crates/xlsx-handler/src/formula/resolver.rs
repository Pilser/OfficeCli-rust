//! Cell resolver — resolves cell references to values during formula evaluation.

use super::types::*;
use crate::dom_types::{WorkbookModel, Worksheet};
use std::collections::HashSet;

/// Resolver that looks up cell values from a workbook model.
pub struct WorkbookResolver<'a> {
    model: &'a WorkbookModel,
    visiting: HashSet<String>,
    depth: usize,
}

impl<'a> WorkbookResolver<'a> {
    pub fn new(model: &'a WorkbookModel) -> Self {
        Self {
            model,
            visiting: HashSet::new(),
            depth: 0,
        }
    }

    /// Find a worksheet by name (case-insensitive).
    pub fn find_sheet(&self, name: &str) -> Option<&Worksheet> {
        self.model
            .sheets
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }
}

impl<'a> crate::formula::parser::CellResolver for WorkbookResolver<'a> {
    fn resolve_cell(&self, cell_ref: &str) -> FormulaResult {
        let cell_ref = cell_ref.to_uppercase();
        // Look up in the first sheet (same-sheet context)
        if let Some(ws) = self.model.sheets.first() {
            if let Some(cell) = crate::dom_types::CellRef::parse(&cell_ref) {
                if let Some(c) = ws.cells.get(&(cell.row, cell.col)) {
                    if let Some(ref formula) = c.formula {
                        // Avoid infinite recursion
                        let key = format!("{}!{}", ws.name, cell_ref);
                        if self.visiting.contains(&key) || self.depth > 20 {
                            return FormulaResult::Number(0.0); // circular ref guard
                        }
                        // Evaluate the formula
                        let mut inner_visiting = self.visiting.clone();
                        inner_visiting.insert(key);
                        let inner = WorkbookResolver {
                            model: self.model,
                            visiting: inner_visiting,
                            depth: self.depth + 1,
                        };
                        return crate::formula::evaluate_with_resolver(formula, &inner);
                    }
                    // Return the cell's display value
                    return cell_value_to_formula_result(c);
                }
            }
        }
        FormulaResult::Blank
    }

    fn resolve_sheet_cell(&self, sheet_cell_ref: &str) -> FormulaResult {
        let bang = sheet_cell_ref.find('!').unwrap_or(0);
        if bang == 0 {
            return FormulaResult::Number(0.0);
        }
        let sheet_name = &sheet_cell_ref[..bang];
        let cell_ref = &sheet_cell_ref[bang + 1..];

        if self.depth > 20 {
            return FormulaResult::Error("#NUM!".to_string());
        }

        if let Some(ws) = self.find_sheet(sheet_name) {
            if let Some(cr) = crate::dom_types::CellRef::parse(cell_ref) {
                if let Some(c) = ws.cells.get(&(cr.row, cr.col)) {
                    if let Some(ref formula) = c.formula {
                        let key = format!("{}!{}", sheet_name, cell_ref.to_uppercase());
                        if self.visiting.contains(&key) {
                            return FormulaResult::Number(0.0);
                        }
                        let mut inner_visiting = self.visiting.clone();
                        inner_visiting.insert(key);
                        let inner = WorkbookResolver {
                            model: self.model,
                            visiting: inner_visiting,
                            depth: self.depth + 1,
                        };
                        return crate::formula::evaluate_with_resolver(formula, &inner);
                    }
                    return cell_value_to_formula_result(c);
                }
            }
            // Sheet exists but cell is empty
            return FormulaResult::Blank;
        }

        // Sheet not found
        if !sheet_name.is_empty() {
            return FormulaResult::Error("#REF!".to_string());
        }
        FormulaResult::Number(0.0)
    }

    fn expand_range(&self, range_expr: &str) -> Vec<(String, FormulaResult)> {
        // Parse Sheet1!A1:B3 or A1:B3
        let (sheet_name, range_part) = if let Some(bang) = range_expr.find('!') {
            (Some(&range_expr[..bang]), &range_expr[bang + 1..])
        } else {
            (None, range_expr)
        };

        let ws = if let Some(name) = sheet_name {
            self.find_sheet(name)
        } else {
            self.model.sheets.first()
        };

        let Some(ws) = ws else {
            return Vec::new();
        };

        let parts: Vec<&str> = range_part.split(':').collect();
        if parts.len() != 2 {
            return Vec::new();
        }

        let left = strip_dollar(parts[0]);
        let right = strip_dollar(parts[1]);

        // Entire-column range like A:A
        let left_col_only = left.chars().all(|c| c.is_ascii_alphabetic());
        let right_col_only = right.chars().all(|c| c.is_ascii_alphabetic());
        if left_col_only && right_col_only {
            let c1 = col_to_index(&left);
            let c2 = col_to_index(&right);
            let min_col = c1.min(c2);
            let max_col = c1.max(c2);
            let mut result = Vec::new();
            for row in 1..=ws.max_row {
                for col in min_col..=max_col {
                    let ref_str = format!("{}{}", index_to_col(col), row);
                    let val = ws.cells.get(&(row, col))
                        .map(|c| cell_value_to_formula_result(c))
                        .unwrap_or(FormulaResult::Blank);
                    result.push((ref_str, val));
                }
            }
            return result;
        }

        // Normal range A1:B3
        let Some((col1, row1)) = parse_ref(&left) else {
            return Vec::new();
        };
        let Some((col2, row2)) = parse_ref(&right) else {
            return Vec::new();
        };

        let c1 = col_to_index(&col1);
        let c2 = col_to_index(&col2);
        let min_row = row1.min(row2);
        let max_row = row1.max(row2);
        let min_col = c1.min(c2);
        let max_col = c1.max(c2);

        let mut result = Vec::new();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let ref_str = format!("{}{}", index_to_col(col), row);
                let val = ws.cells.get(&(row, col))
                    .map(|c| cell_value_to_formula_result(c))
                    .unwrap_or(FormulaResult::Blank);
                result.push((ref_str, val));
            }
        }
        result
    }
}

/// Convert a Cell's display value to a FormulaResult.
fn cell_value_to_formula_result(cell: &crate::dom_types::Cell) -> FormulaResult {
    match cell.value_type {
        crate::dom_types::CellValueType::Number => {
            if let Some(ref v) = cell.raw_value {
                if let Ok(n) = v.parse::<f64>() {
                    return FormulaResult::Number(n);
                }
            }
            if let Ok(n) = cell.display_value.parse::<f64>() {
                FormulaResult::Number(n)
            } else {
                FormulaResult::Str(cell.display_value.clone())
            }
        }
        crate::dom_types::CellValueType::SharedString
        | crate::dom_types::CellValueType::InlineString => {
            FormulaResult::Str(cell.display_value.clone())
        }
        crate::dom_types::CellValueType::Boolean => {
            FormulaResult::Bool(cell.display_value == "1" || cell.display_value.eq_ignore_ascii_case("true"))
        }
        crate::dom_types::CellValueType::Error => {
            FormulaResult::Error(cell.display_value.clone())
        }
    }
}
