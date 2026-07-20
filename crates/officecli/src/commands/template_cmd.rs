use clap::Args;
use handler_common::{HandlerError, OutputFormat};

#[derive(Args)]
pub struct TemplateCommand {
    /// Document file path
    pub file: String,
    /// Inline JSON data
    pub data: Option<String>,
    /// Path to JSON data file
    #[arg(long)]
    pub data_file: Option<String>,
    /// Key=value overrides
    #[arg(long)]
    pub param: Option<Vec<String>>,
}

pub fn handle_template(cmd: TemplateCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    let json_text = if let Some(ref inline) = cmd.data {
        inline.clone()
    } else if let Some(ref path) = cmd.data_file {
        std::fs::read_to_string(path)
            .map_err(|e| HandlerError::OperationFailed(format!("read data file: {}", e)))?
    } else {
        return Err(HandlerError::InvalidArgument("Provide --data or --data-file".into()));
    };

    let mut data: serde_json::Value = serde_json::from_str(&json_text)
        .map_err(|e| HandlerError::InvalidArgument(format!("invalid JSON: {}", e)))?;

    if let Some(ref params) = cmd.param {
        if let Some(ref mut obj) = data.as_object_mut() {
            for p in params {
                if let Some(eq) = p.find('=') {
                    let key = &p[..eq];
                    let val = &p[eq + 1..];
                    obj.insert(key.to_string(), serde_json::Value::String(val.to_string()));
                }
            }
        }
    }

    let handler = crate::open_handler(&cmd.file, true)?;
    let result = template_merger::merge_v2(handler.as_ref(), &data)?;
    handler.save()?;

    Ok(result)
}
