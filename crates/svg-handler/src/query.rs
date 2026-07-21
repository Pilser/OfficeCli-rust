use handler_common::DocumentNode;

use crate::navigation::SvgNavigator;

pub fn query_by_type(xml: &str, element_type: &str) -> Result<Vec<DocumentNode>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;
    let nodes = SvgNavigator::find_all_by_type(&doc, element_type);

    let results = nodes
        .into_iter()
        .map(|resolved| {
            let text = resolved
                .node
                .text()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty());
            let mut node = DocumentNode::new(&resolved.tag_path, &resolved.element_type);

            for attr in ["fill", "stroke", "stroke-width", "opacity", "x", "y", "width", "height",
                          "cx", "cy", "r", "rx", "ry", "font-size", "font-family", "id", "class",
                          "transform", "d"]
            {
                if let Some(val) = resolved.node.attribute(attr) {
                    node = node.with_format(attr, serde_json::json!(val));
                }
            }

            if let Some(t) = text {
                node = node.with_text(t);
            }

            node
        })
        .collect();

    Ok(results)
}

pub fn query_by_id(xml: &str, id: &str) -> Result<Option<DocumentNode>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;

    match SvgNavigator::find_by_id(&doc, id) {
        Some(resolved) => {
            let text = resolved
                .node
                .text()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty());
            let mut node = DocumentNode::new(&resolved.tag_path, &resolved.element_type);

            for attr in ["fill", "stroke", "stroke-width", "opacity", "x", "y", "width", "height",
                          "cx", "cy", "r", "rx", "ry", "font-size", "font-family", "id", "class",
                          "transform", "d"]
            {
                if let Some(val) = resolved.node.attribute(attr) {
                    node = node.with_format(attr, serde_json::json!(val));
                }
            }

            if let Some(t) = text {
                node = node.with_text(t);
            }

            Ok(Some(node))
        }
        None => Ok(None),
    }
}
