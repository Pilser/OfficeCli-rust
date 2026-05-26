use handler_common::{HandlerError, OutputFormat};
use clap::Args;
use std::collections::HashMap;

/// Modify properties of an element at a path (text, style, content)
#[derive(Args)]
pub struct SetCommand {
    /// Document file path
    pub file: String,

    /// Path to the element
    pub path: String,

    /// Properties to set (key=value pairs, e.g. "text=hello" "style=Heading1")
    #[arg(num_args = 1..)]
    pub properties: Vec<String>,
}

pub fn handle_set(cmd: SetCommand, format: OutputFormat) -> Result<String, HandlerError> {
    let handler = crate::open_handler(&cmd.file, true)?;

    let properties: HashMap<String, String> = cmd.properties
        .iter()
        .filter_map(|p| {
            let parts: Vec<&str> = p.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    let unsupported = handler.set(&cmd.path, &properties)?;
    handler.save()?;

    match format {
        OutputFormat::Text => {
            if unsupported.is_empty() {
                Ok("OK".to_string())
            } else {
                Ok(format!("OK (unsupported: {})", unsupported.join(", ")))
            }
        }
        OutputFormat::Json => Ok(serde_json::json!({
            "result": "OK",
            "unsupported": unsupported
        }).to_string()),
    }
}