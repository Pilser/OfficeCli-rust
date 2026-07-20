use clap::Args;
use handler_common::{HandlerError, OutputFormat};

#[derive(Args)]
pub struct DiffCommand {
    /// First document file
    pub file1: String,
    /// Second document file
    pub file2: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn handle_diff(cmd: DiffCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    let old_handler = crate::open_handler(&cmd.file1, false)?;
    let new_handler = crate::open_handler(&cmd.file2, false)?;

    let format = if cmd.json {
        document_diff::DiffFormat::Json
    } else {
        document_diff::DiffFormat::Text
    };
    let diffs = document_diff::diff_documents(
        old_handler.as_ref(),
        new_handler.as_ref(),
        format,
    )
    .map_err(|e| HandlerError::OperationFailed(e))?;

    if cmd.json {
        serde_json::to_string_pretty(&diffs)
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))
    } else {
        Ok(document_diff::render_text_diff(&diffs))
    }
}
