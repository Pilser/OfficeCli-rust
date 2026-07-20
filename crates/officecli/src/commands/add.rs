use clap::Args;
use handler_common::{HandlerError, InsertPosition, OutputFormat};
use std::collections::HashMap;

/// Insert a new element (paragraph, table, slide, image, bookmark) into the document
#[derive(Args)]
pub struct AddCommand {
    /// Document file path
    pub file: String,

    /// Parent path where to add
    #[arg(long)]
    pub parent: String,

    /// Element type to add
    #[arg(long)]
    pub type_name: String,

    /// Position: index number, "after:/path", or "before:/path"
    #[arg(long)]
    pub position: Option<String>,

    /// Properties (key=value pairs)
    #[arg(long, num_args = 1..)]
    pub properties: Vec<String>,

    /// Wrap an existing element: bookmarkStart goes before, bookmarkEnd goes after the target
    #[arg(long)]
    pub wrap: Option<String>,

    /// Range-paths for bookmark: insert bookmarkStart/End around text at char offsets
    /// Syntax: /path[start..end],/path[start..end] (same as set --range-paths)
    #[arg(long)]
    pub range_paths: Option<String>,

    /// CSS properties (e.g., "font-weight: bold; color: #FF0000; font-size: 18pt")
    #[arg(long)]
    pub css: Option<String>,

    /// Emit the refreshed text+offset map after the edit (JSON output only).
    #[arg(long)]
    pub emit_map: bool,
}

pub fn handle_add(cmd: AddCommand, format: OutputFormat) -> Result<String, HandlerError> {
    let handler = crate::open_handler(&cmd.file, true)?;

    let position = parse_position(cmd.position.as_deref());
    let mut properties = parse_properties(&cmd.properties);

    // Merge CSS properties into the property map
    if let Some(css) = &cmd.css {
        let css_map = handler_common::css::parse_css(css);
        for (k, v) in css_map {
            properties.entry(k).or_insert(v);
        }
    }

    // Merge range_paths into properties (same pattern as set command)
    if let Some(rp) = &cmd.range_paths {
        properties.insert("range_paths".to_string(), rp.clone());
    }

    let new_path = handler.add(
        &cmd.parent,
        &cmd.type_name,
        position,
        &properties,
        cmd.wrap.as_deref(),
    )?;
    handler.save()?;

    let offset_map = if cmd.emit_map {
        super::offset_map_value(handler.as_ref())
    } else {
        None
    };

    match format {
        OutputFormat::Text => Ok(format!("Created: {}", new_path)),
        OutputFormat::Json => {
            let mut out = serde_json::json!({ "path": new_path });
            if let Some(map) = offset_map {
                out["offset_map"] = map;
            }
            Ok(out.to_string())
        }
    }
}

pub fn parse_position(input: Option<&str>) -> InsertPosition {
    match input {
        None => InsertPosition::Append,
        Some(s) => {
            if let Some(idx) = s.parse::<usize>().ok() {
                InsertPosition::AtIndex(idx)
            } else if let Some(rest) = s.strip_prefix("after:") {
                InsertPosition::AfterElement(rest.to_string())
            } else if let Some(rest) = s.strip_prefix("before:") {
                InsertPosition::BeforeElement(rest.to_string())
            } else {
                InsertPosition::Append
            }
        }
    }
}

fn parse_properties(props: &[String]) -> HashMap<String, String> {
    props
        .iter()
        .filter_map(|p| {
            let parts: Vec<&str> = p.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}
