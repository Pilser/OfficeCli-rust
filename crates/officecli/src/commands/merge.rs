use clap::Args;
use handler_common::{HandlerError, OutputFormat};

/// Merge template placeholders with JSON data
#[derive(Args)]
pub struct MergeCommand {
    /// Document file path (template with {{key}} placeholders)
    pub file: String,

    /// JSON data (inline object or path to .json file)
    pub data: String,

    /// Output file path (defaults to overwriting the input)
    #[arg(short, long)]
    pub out: Option<String>,
}

pub fn handle_merge(cmd: MergeCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    let handler = crate::open_handler(&cmd.file, true)?;

    // Parse merge data (file or inline JSON)
    let json_text = if cmd.data.ends_with(".json") && std::path::Path::new(&cmd.data).exists() {
        std::fs::read_to_string(&cmd.data).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to read JSON file '{}': {}", cmd.data, e))
        })?
    } else {
        cmd.data.clone()
    };

    // Parse JSON into flat key-value map
    let data = parse_merge_data(&json_text)?;

    // Perform merge
    let result = handler.merge(&data)?;

    // Save to output or in-place
    if let Some(out) = cmd.out {
        // Save to a different file — we need to save first then copy
        handler.save()?;
        std::fs::copy(&cmd.file, &out).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to copy to '{}': {}", out, e))
        })?;
    } else {
        handler.save()?;
    }

    Ok(format!(
        "Merged: {} replacement(s), {} unresolved placeholder(s)",
        result.replaced_count, result.unresolved_count
    ))
}

/// Parse JSON data (object or file) into a flat HashMap.
/// Supports nested objects (flattened as "a.b") and arrays (as "items[0]").
/// Literal keys take precedence over flattened dot-paths.
fn parse_merge_data(json_text: &str) -> Result<std::collections::HashMap<String, String>, HandlerError> {
    let val: serde_json::Value = serde_json::from_str(json_text).map_err(|e| {
        HandlerError::InvalidArgument(format!("invalid JSON data: {}", e))
    })?;

    let obj = val.as_object().ok_or_else(|| {
        HandlerError::InvalidArgument("JSON data must be an object".to_string())
    })?;

    let mut data = std::collections::HashMap::new();

    // Pass 1: literal top-level keys (take precedence)
    for (key, value) in obj {
        data.insert(key.clone(), json_value_to_string(value));
    }

    // Pass 2: flatten nested objects/arrays (only if key not already present)
    for (key, value) in obj {
        flatten_value(&mut data, key, value, false);
    }

    Ok(data)
}

fn json_value_to_string(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn flatten_value(
    data: &mut std::collections::HashMap<String, String>,
    prefix: &str,
    val: &serde_json::Value,
    skip_existing: bool,
) {
    match val {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let dotted = format!("{}.{}", prefix, key);
                if skip_existing && data.contains_key(&dotted) {
                    continue;
                }
                data.insert(dotted.clone(), json_value_to_string(child));
                flatten_value(data, &dotted, child, skip_existing);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, child) in arr.iter().enumerate() {
                let bracketed = format!("{}[{}]", prefix, i);
                if skip_existing && data.contains_key(&bracketed) {
                    continue;
                }
                data.insert(bracketed.clone(), json_value_to_string(child));
                flatten_value(data, &bracketed, child, skip_existing);
            }
        }
        _ => {}
    }
}
