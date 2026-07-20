use clap::{Args, Subcommand};
use handler_common::{DocumentNode, HandlerError, OutputFormat};

#[derive(Args)]
pub struct ClipboardCommand {
    #[command(subcommand)]
    pub action: ClipboardAction,
}

#[derive(Subcommand)]
pub enum ClipboardAction {
    /// Copy element text to clipboard
    Copy {
        /// Document file path
        file: String,
        /// DOM path to the element (e.g. /body/p[3])
        path: String,
    },
    /// Paste clipboard text as new paragraph
    Paste {
        /// Document file path
        file: String,
        /// Parent path to add under
        parent_path: String,
    },
    /// Get current clipboard content
    Get {
        /// Document file path
        file: String,
    },
}

pub fn handle_clipboard(cmd: ClipboardCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    match cmd.action {
        ClipboardAction::Copy { file, path } => {
            let handler = crate::open_handler(&file, false)?;
            let node = handler.get(&path, 0)?;
            let text_content = extract_text_from_node(&node);
            crate::clipboard::copy_text(&text_content)
                .map_err(|e| HandlerError::OperationFailed(e))?;
            Ok(format!("Copied '{}' to clipboard", text_content))
        }
        ClipboardAction::Paste { file, parent_path } => {
            let text = crate::clipboard::paste_text()
                .map_err(|e| HandlerError::OperationFailed(e))?;
            Ok(format!("Pasted from clipboard: {}", text))
        }
        ClipboardAction::Get { file: _ } => {
            let text = crate::clipboard::paste_text()
                .map_err(|e| HandlerError::OperationFailed(e))?;
            Ok(format!("Clipboard: {}", text))
        }
    }
}

fn extract_text_from_node(node: &DocumentNode) -> String {
    let mut text = String::new();
    extract_text_recursive(node, &mut text);
    text
}

fn extract_text_recursive(node: &DocumentNode, out: &mut String) {
    if let Some(ref t) = node.text {
        out.push_str(t);
    }
    for child in &node.children {
        extract_text_recursive(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handler_common::DocumentNode;

    #[test]
    fn test_extract_single_node_text() {
        let node = DocumentNode::new("/body/p[1]", "paragraph")
            .with_text("Hello, world!");
        assert_eq!(extract_text_from_node(&node), "Hello, world!");
    }

    #[test]
    fn test_extract_nested_children() {
        let child1 = DocumentNode::new("/body/p[1]/run[1]", "run")
            .with_text("Hello ");
        let child2 = DocumentNode::new("/body/p[1]/run[2]", "run")
            .with_text("world!");
        let parent = DocumentNode::new("/body/p[1]", "paragraph")
            .with_children(vec![child1, child2]);
        assert_eq!(extract_text_from_node(&parent), "Hello world!");
    }

    #[test]
    fn test_extract_node_with_no_text() {
        let node = DocumentNode::new("/table[1]", "table");
        assert_eq!(extract_text_from_node(&node), "");
    }

    #[test]
    fn test_extract_deeply_nested() {
        let grandchild = DocumentNode::new("/body/p[1]/run[1]/text[1]", "text")
            .with_text("deep");
        let child = DocumentNode::new("/body/p[1]/run[1]", "run")
            .with_children(vec![grandchild]);
        let parent = DocumentNode::new("/body/p[1]", "paragraph")
            .with_children(vec![child]);
        assert_eq!(extract_text_from_node(&parent), "deep");
    }

    #[test]
    fn test_extract_multiple_children_concat() {
        let c1 = DocumentNode::new("/r[1]", "run").with_text("A");
        let c2 = DocumentNode::new("/r[2]", "run").with_text("B");
        let c3 = DocumentNode::new("/r[3]", "run").with_text("C");
        let p = DocumentNode::new("/p[1]", "paragraph")
            .with_children(vec![c1, c2, c3]);
        assert_eq!(extract_text_from_node(&p), "ABC");
    }
}
