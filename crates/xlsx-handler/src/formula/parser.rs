//! Recursive descent formula parser.
//!
//! Precedence (low→high): comparison → concat → add/sub → mul/div → power → unary → postfix(%) → atom.

use super::tokenizer;
use super::types::*;

/// Parse a formula string and evaluate it against a cell resolver.
pub struct FormulaParser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    resolver: &'a dyn CellResolver,
    _same_sheet_depth: usize,
    parse_depth: usize,
}

/// Trait for resolving cell values during formula evaluation.
/// Implemented by the caller to provide access to the workbook's cell data.
pub trait CellResolver {
    /// Resolve a same-sheet cell reference (e.g. "A1") to a FormulaResult.
    fn resolve_cell(&self, cell_ref: &str) -> FormulaResult;

    /// Resolve a cross-sheet cell reference (e.g. "Sheet1!A1") to a FormulaResult.
    fn resolve_sheet_cell(&self, sheet_cell_ref: &str) -> FormulaResult;

    /// Expand a range reference (e.g. "A1:B3" or "Sheet1!A1:B3") to a Vec of (cell_ref, FormulaResult).
    fn expand_range(&self, range_expr: &str) -> Vec<(String, FormulaResult)>;
}

impl<'a> FormulaParser<'a> {
    pub fn new(formula: &str, resolver: &'a dyn CellResolver) -> Result<Self, String> {
        let tokens = tokenizer::tokenize(formula)?;
        Ok(Self {
            tokens,
            pos: 0,
            resolver,
            _same_sheet_depth: 0,
            parse_depth: 0,
        })
    }

    /// Parse and evaluate the formula.
    pub fn evaluate(&mut self) -> Option<FormulaResult> {
        let result = self.parse_expression();
        if self.pos != self.tokens.len() {
            return None; // unconsumed tokens
        }
        // Top-level Array collapses to scalar (first element)
        match result {
            Some(FormulaResult::Array(ref a)) if !a.is_empty() => Some(FormulaResult::Number(a[0])),
            other => other,
        }
    }

    // ─── Precedence levels ────────────────────────────────────────────

    fn parse_expression(&mut self) -> Option<FormulaResult> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Option<FormulaResult> {
        let left = self.parse_concat()?;
        while self.pos < self.tokens.len() && self.tokens[self.pos].tt == TokenType::Compare {
            let op = self.tokens[self.pos].value.clone();
            self.pos += 1;
            let right = self.parse_concat()?;
            if left.is_error() {
                return Some(left);
            }
            if right.is_error() {
                return Some(right);
            }
            let cmp = compare_values(&left, &right);
            let result = match op.as_str() {
                "=" => cmp == 0,
                "<>" => cmp != 0,
                "<" => cmp < 0,
                ">" => cmp > 0,
                "<=" => cmp <= 0,
                ">=" => cmp >= 0,
                _ => return None,
            };
            return Some(FormulaResult::Bool(result));
        }
        Some(left)
    }

    fn parse_concat(&mut self) -> Option<FormulaResult> {
        self.parse_depth += 1;
        if self.parse_depth > 200 {
            self.parse_depth -= 1;
            return Some(FormulaResult::Error("#NUM!".to_string()));
        }
        let left = self.parse_add_sub()?;
        let result = if self.pos < self.tokens.len()
            && self.tokens[self.pos].tt == TokenType::Op
            && self.tokens[self.pos].value == "&"
        {
            let mut result = left;
            while self.pos < self.tokens.len()
                && self.tokens[self.pos].tt == TokenType::Op
                && self.tokens[self.pos].value == "&"
            {
                self.pos += 1;
                let right = self.parse_add_sub()?;
                if result.is_error() {
                    return Some(result);
                }
                if right.is_error() {
                    return Some(right);
                }
                result = FormulaResult::Str(format!("{}{}", result.as_string(), right.as_string()));
            }
            result
        } else {
            left
        };
        self.parse_depth -= 1;
        Some(result)
    }

    fn parse_add_sub(&mut self) -> Option<FormulaResult> {
        let left = self.parse_mul_div()?;
        let mut result = left;
        while self.pos < self.tokens.len()
            && self.tokens[self.pos].tt == TokenType::Op
            && (self.tokens[self.pos].value == "+" || self.tokens[self.pos].value == "-")
        {
            let op = self.tokens[self.pos].value.clone();
            self.pos += 1;
            let right = self.parse_mul_div()?;
            if result.is_error() {
                return Some(result);
            }
            if right.is_error() {
                return Some(right);
            }
            let lv = result.as_number();
            let rv = right.as_number();
            result = FormulaResult::Number(if op == "+" { lv + rv } else { lv - rv });
        }
        Some(result)
    }

    fn parse_mul_div(&mut self) -> Option<FormulaResult> {
        let left = self.parse_power()?;
        let mut result = left;
        while self.pos < self.tokens.len()
            && self.tokens[self.pos].tt == TokenType::Op
            && (self.tokens[self.pos].value == "*" || self.tokens[self.pos].value == "/")
        {
            let op = self.tokens[self.pos].value.clone();
            self.pos += 1;
            let right = self.parse_power()?;
            if result.is_error() {
                return Some(result);
            }
            if right.is_error() {
                return Some(right);
            }
            let lv = result.as_number();
            let rv = right.as_number();
            if op == "/" {
                if rv == 0.0 {
                    return Some(FormulaResult::Error("#DIV/0!".to_string()));
                }
                result = FormulaResult::Number(lv / rv);
            } else {
                result = FormulaResult::Number(lv * rv);
            }
        }
        Some(result)
    }

    fn parse_power(&mut self) -> Option<FormulaResult> {
        let base = self.parse_unary()?;
        let mut result = base;
        while self.pos < self.tokens.len()
            && self.tokens[self.pos].tt == TokenType::Op
            && self.tokens[self.pos].value == "^"
        {
            self.pos += 1;
            let exp = self.parse_unary()?;
            if result.is_error() {
                return Some(result);
            }
            if exp.is_error() {
                return Some(exp);
            }
            result = FormulaResult::Number(result.as_number().powf(exp.as_number()));
        }
        Some(result)
    }

    fn parse_unary(&mut self) -> Option<FormulaResult> {
        if self.pos < self.tokens.len() && self.tokens[self.pos].tt == TokenType::Op {
            if self.tokens[self.pos].value == "-" {
                self.pos += 1;
                let v = self.parse_unary()?;
                return Some(match v {
                    FormulaResult::Number(n) => FormulaResult::Number(-n),
                    FormulaResult::Error(e) => FormulaResult::Error(e),
                    other => FormulaResult::Number(-other.as_number()),
                });
            }
            if self.tokens[self.pos].value == "+" {
                self.pos += 1;
                return self.parse_unary();
            }
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Option<FormulaResult> {
        let v = self.parse_atom()?;
        let mut result = v;
        while self.pos < self.tokens.len()
            && self.tokens[self.pos].tt == TokenType::Op
            && self.tokens[self.pos].value == "%"
        {
            self.pos += 1;
            result = FormulaResult::Number(result.as_number() / 100.0);
        }
        Some(result)
    }

    fn parse_atom(&mut self) -> Option<FormulaResult> {
        if self.pos >= self.tokens.len() {
            return None;
        }
        let tok = &self.tokens[self.pos].clone();
        match tok.tt {
            TokenType::Number => {
                self.pos += 1;
                let v: f64 = tok.value.parse().ok()?;
                Some(FormulaResult::Number(v))
            }
            TokenType::String => {
                self.pos += 1;
                Some(FormulaResult::Str(tok.value.clone()))
            }
            TokenType::Bool => {
                self.pos += 1;
                Some(FormulaResult::Bool(tok.value == "TRUE"))
            }
            TokenType::CellRef => {
                self.pos += 1;
                Some(self.resolver.resolve_cell(&tok.value))
            }
            TokenType::SheetCellRef => {
                self.pos += 1;
                Some(self.resolver.resolve_sheet_cell(&tok.value))
            }
            TokenType::Range => {
                self.pos += 1;
                let cells = self.resolver.expand_range(&tok.value);
                let values: Vec<f64> = cells.iter().map(|(_, v)| v.as_number()).collect();
                Some(FormulaResult::Array(values))
            }
            TokenType::SheetRange => {
                self.pos += 1;
                let cells = self.resolver.expand_range(&tok.value);
                let values: Vec<f64> = cells.iter().map(|(_, v)| v.as_number()).collect();
                Some(FormulaResult::Array(values))
            }
            TokenType::ArrayLit => {
                self.pos += 1;
                Some(parse_array_constant(&tok.value))
            }
            TokenType::Error => {
                self.pos += 1;
                Some(FormulaResult::Error(tok.value.clone()))
            }
            TokenType::LParen => {
                self.pos += 1;
                let inner = self.parse_expression();
                if self.pos < self.tokens.len() && self.tokens[self.pos].tt == TokenType::RParen {
                    self.pos += 1;
                }
                inner
            }
            TokenType::Func => self.parse_function(),
            _ => None,
        }
    }

    fn parse_function(&mut self) -> Option<FormulaResult> {
        let name = self.tokens[self.pos].value.clone();
        self.pos += 1;
        if self.pos >= self.tokens.len() || self.tokens[self.pos].tt != TokenType::LParen {
            return None;
        }
        self.pos += 1; // skip (

        let mut args: Vec<FormulaResult> = Vec::new();
        if self.pos < self.tokens.len() && self.tokens[self.pos].tt != TokenType::RParen {
            loop {
                // Empty arg = 0
                if self.pos < self.tokens.len()
                    && (self.tokens[self.pos].tt == TokenType::Comma
                        || self.tokens[self.pos].tt == TokenType::RParen)
                {
                    args.push(FormulaResult::Number(0.0));
                } else {
                    let expr = self.parse_expression()?;
                    args.push(expr);
                }
                if self.pos >= self.tokens.len() || self.tokens[self.pos].tt != TokenType::Comma {
                    break;
                }
                self.pos += 1; // skip comma
            }
        }
        if self.pos < self.tokens.len() && self.tokens[self.pos].tt == TokenType::RParen {
            self.pos += 1;
        }

        // Dispatch to function implementation
        super::functions::eval_function(&name, &args, self.resolver)
    }
}

// ─── Array constant parsing ──────────────────────────────────────────────

fn parse_array_constant(body: &str) -> FormulaResult {
    let rows: Vec<&str> = body.split(';').collect();
    let row_cells: Vec<Vec<&str>> = rows.iter().map(|r| r.split(',').collect()).collect();
    let mut values = Vec::new();
    for row in &row_cells {
        for cell in row {
            let s = cell.trim();
            if s.is_empty() {
                values.push(0.0);
            } else if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                // String cell in array — skip for numeric output
                values.push(0.0);
            } else if s.eq_ignore_ascii_case("TRUE") {
                values.push(1.0);
            } else if s.eq_ignore_ascii_case("FALSE") {
                values.push(0.0);
            } else if let Ok(n) = s.parse::<f64>() {
                values.push(n);
            } else {
                values.push(0.0);
            }
        }
    }
    FormulaResult::Array(values)
}

// ─── Comparison helper ──────────────────────────────────────────────────

pub fn compare_values(a: &FormulaResult, b: &FormulaResult) -> i32 {
    // Try numeric comparison first
    let an = a.as_number();
    let bn = b.as_number();
    if an < bn {
        return -1;
    }
    if an > bn {
        return 1;
    }
    // Fall back to string comparison
    let as_str = a.as_string();
    let bs_str = b.as_string();
    as_str.cmp(&bs_str) as i32
}
