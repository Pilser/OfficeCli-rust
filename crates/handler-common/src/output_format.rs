use serde::{Deserialize, Serialize};

/// Output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Options for view commands (line range, column filter).
#[derive(Debug, Clone, Default)]
pub struct ViewOptions {
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub max_lines: Option<usize>,
    pub cols: Option<Vec<String>>,
    /// Page filter string (e.g. "1", "2-5", "1,3,5"). Parsed by each handler.
    pub page: Option<String>,
    /// Element path or cell range to restrict output (e.g. "/slide[1]/shape[@id=2]" or "Sheet1!A1:C3")
    pub range: Option<String>,
    /// Grid columns for thumbnail contact sheet (0 = off)
    pub grid: u32,
    /// Rendering backend: "auto", "native", "html"
    pub render: String,
}

/// Options for raw commands.
#[derive(Debug, Clone, Default)]
pub struct RawOptions {
    pub start_row: Option<usize>,
    pub end_row: Option<usize>,
    pub cols: Option<Vec<String>>,
}

/// Binary extraction result.
#[derive(Debug, Clone)]
pub struct BinaryInfo {
    pub content_type: String,
    pub byte_count: usize,
}
