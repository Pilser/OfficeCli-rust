//! Formula evaluator for xlsx cells.
//! Re-exports from the formula submodule.

pub use crate::formula::{
    evaluate, evaluate_with_resolver, format_number, FormulaResult, WorkbookResolver,
};
