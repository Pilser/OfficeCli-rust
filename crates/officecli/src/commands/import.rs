/// `import` command — import CSV/TSV data into an Excel sheet
/// or import Markdown into a Word document.
use clap::Args;
use handler_common::HandlerError;
use oxml::OxmlPackage;
use std::collections::HashMap;

#[derive(Args)]
pub struct ImportCommand {
    /// Target file (.xlsx for CSV/TSV, .docx for Markdown)
    pub file: String,

    /// Parent path (sheet for CSV/TSV, e.g. /Sheet1; unused for Markdown)
    pub parent_path: String,

    /// Source file to import
    #[arg(long, short)]
    pub file_source: Option<String>,

    /// Read data from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Data format: csv or tsv (default: inferred from file extension, or csv)
    #[arg(long)]
    pub format: Option<String>,

    /// First row is header: set AutoFilter and freeze pane (CSV/TSV only)
    #[arg(long)]
    pub header: bool,

    /// Starting cell (CSV/TSV only, default: A1)
    #[arg(long, default_value = "A1")]
    pub start_cell: String,

    /// Import Markdown and convert to docx (requires .docx target file)
    #[arg(long)]
    pub markdown: bool,
}

pub fn handle_import(
    cmd: ImportCommand,
    _format: handler_common::OutputFormat,
) -> Result<String, HandlerError> {
    if cmd.markdown {
        return handle_markdown_import(cmd);
    }

    // ── Existing CSV/TSV import logic ──

    if !cmd.file.to_lowercase().ends_with(".xlsx") {
        return Err(HandlerError::InvalidArgument(
            "Import is only supported for .xlsx files".to_string(),
        ));
    }

    let csv_content = read_input(&cmd)?;

    let delimiter = if let Some(fmt) = &cmd.format {
        match fmt.to_lowercase().as_str() {
            "tsv" => '\t',
            "csv" => ',',
            other => {
                return Err(HandlerError::InvalidArgument(format!(
                    "Unknown format: {}. Use 'csv' or 'tsv'",
                    other
                )))
            }
        }
    } else if let Some(source_path) = &cmd.file_source {
        let ext = source_path.rsplit('.').next().unwrap_or("").to_lowercase();
        if ext == "tsv" || ext == "tab" {
            '\t'
        } else {
            ','
        }
    } else {
        ','
    };

    let mut package =
        OxmlPackage::open(&cmd.file, true).map_err(|e| HandlerError::OpenError(e.to_string()))?;

    let result = xlsx_handler::import::import_csv(
        &mut package,
        &cmd.parent_path,
        &csv_content,
        delimiter,
        cmd.header,
        &cmd.start_cell,
    )
    .map_err(|e| HandlerError::OperationFailed(e))?;

    package
        .save()
        .map_err(|e| HandlerError::SaveError(e.to_string()))?;

    Ok(result)
}

/// Handle `--markdown` import: convert markdown to docx elements.
fn handle_markdown_import(cmd: ImportCommand) -> Result<String, HandlerError> {
    if !cmd.file.to_lowercase().ends_with(".docx") {
        return Err(HandlerError::InvalidArgument(
            "Markdown import requires a .docx target file".to_string(),
        ));
    }

    let markdown = read_input(&cmd)?;

    let elements = handler_common::markdown::markdown_to_docx(&markdown)
        .map_err(|e| HandlerError::OperationFailed(e))?;

    if elements.is_empty() {
        return Err(HandlerError::InvalidArgument(
            "No elements found in markdown input".to_string(),
        ));
    }

    let handler = crate::open_handler(&cmd.file, true)?;

    use handler_common::InsertPosition;

    let mut table_idx = 0usize;
    let mut row_idx = 0usize;
    let mut last_para_path: String = "/body".to_string();

    for el in &elements {
        match el.element_type.as_str() {
            "paragraph" => {
                let path = handler.add(
                    "/body",
                    "paragraph",
                    InsertPosition::Append,
                    &el.properties,
                    None,
                )?;
                last_para_path = path;
            }
            "run" => {
                handler.add(
                    &last_para_path,
                    "run",
                    InsertPosition::Append,
                    &el.properties,
                    None,
                )?;
            }
            "tbl" => {
                table_idx += 1;
                handler.add("/body", "tbl", InsertPosition::Append, &el.properties, None)?;
            }
            "tr" => {
                row_idx += 1;
                let parent = format!("/body/tbl[{}]", table_idx);
                handler.add(&parent, "tr", InsertPosition::Append, &HashMap::new(), None)?;
            }
            "tc" => {
                let parent = format!("/body/tbl[{}]/tr[{}]", table_idx, row_idx);
                let cell_path =
                    handler.add(&parent, "tc", InsertPosition::Append, &el.properties, None)?;
                // Cell with text property gets a paragraph automatically
                // but if there's no text text, ensure at least one paragraph exists
                if !el.properties.contains_key("text")
                    || el.properties.get("text").map(|s| s.is_empty()).unwrap_or(true)
                {
                    handler.add(
                        &cell_path,
                        "paragraph",
                        InsertPosition::Append,
                        &HashMap::new(),
                        None,
                    )?;
                }
            }
            other => {
                return Err(HandlerError::UnsupportedType(format!(
                    "Unknown element type from markdown parser: '{}'",
                    other
                )));
            }
        }
    }

    handler.save()?;

    Ok(format!(
        "Imported {} element(s) from markdown into {}",
        elements.len(),
        cmd.file
    ))
}

/// Read input from --file-source or --stdin.
fn read_input(cmd: &ImportCommand) -> Result<String, HandlerError> {
    if cmd.stdin {
        use std::io::Read;
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|e| HandlerError::OperationFailed(format!("Failed to read stdin: {}", e)))?;
        Ok(content)
    } else if let Some(source_path) = &cmd.file_source {
        std::fs::read_to_string(source_path).map_err(|e| {
            HandlerError::OperationFailed(format!(
                "Failed to read source file '{}': {}",
                source_path, e
            ))
        })
    } else {
        Err(HandlerError::InvalidArgument(
            "Either --file-source or --stdin must be specified".to_string(),
        ))
    }
}
