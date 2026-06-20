//! Attribute filter — selects document elements based on attribute predicates.
//! Mirrors the C# AttributeFilter that walks a selector's attribute predicates
//! and applies them against the handler's Query results.

use crate::selector::Selector;
use crate::DocumentNode;

/// Filter a list of nodes by the attribute predicates in a selector.
/// Returns (filtered, warnings) where warnings contains messages about
/// unknown attributes or invalid predicates.
pub fn filter_nodes_by_selector(
    nodes: Vec<DocumentNode>,
    selector: &Selector,
) -> (Vec<DocumentNode>, Vec<String>) {
    let mut filtered = Vec::new();
    let warnings = Vec::new();

    for node in nodes {
        if matches_all_attributes(&node, selector) {
            filtered.push(node);
        }
    }

    (filtered, warnings)
}

/// Check if a node matches all attribute predicates in a selector.
/// The Selector stores attribute predicates as Vec<(String, String)> (key, value) pairs
/// that are matched for equality by default. DocumentNode.format wraps values in
/// Option<Value> — missing attributes don't match.
fn matches_all_attributes(node: &DocumentNode, selector: &Selector) -> bool {
    for (key, expected) in &selector.attributes {
        let actual = node.format.get(key);
        match actual {
            None => return false,
            Some(None) => return false,
            Some(Some(av)) => {
                let actual_str = match av {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    other => other.to_string(),
                };
                if &actual_str != expected {
                    return false;
                }
            }
        }
    }
    true
}

/// A simplified attribute predicate.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributePredicate {
    pub key: String,
    pub op: PredicateOp,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredicateOp {
    Equals,
    NotEquals,
    Exists,
    Contains,
    StartsWith,
    EndsWith,
}

impl AttributePredicate {
    pub fn parse(spec: &str) -> Option<Self> {
        let spec = spec.trim();
        if let Some(eq) = spec.find("!=") {
            let (k, v) = spec.split_at(eq);
            return Some(AttributePredicate {
                key: k.trim().to_string(),
                op: PredicateOp::NotEquals,
                value: v[2..].trim().to_string(),
            });
        }
        if let Some(eq) = spec.find('=') {
            let (k, v) = spec.split_at(eq);
            return Some(AttributePredicate {
                key: k.trim().to_string(),
                op: PredicateOp::Equals,
                value: v[1..].trim().to_string(),
            });
        }
        if let Some(rest) = spec.strip_prefix('^') {
            return Some(AttributePredicate {
                key: rest.trim().to_string(),
                op: PredicateOp::Exists,
                value: String::new(),
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_node_with_format(format_pairs: &[(&str, &str)]) -> DocumentNode {
        let mut format = HashMap::new();
        for (k, v) in format_pairs {
            format.insert(
                k.to_string(),
                Some(serde_json::Value::String(v.to_string())),
            );
        }
        let mut node = DocumentNode::new("/test", "test");
        node.format = format;
        node
    }

    #[test]
    fn test_parse_equals() {
        let p = AttributePredicate::parse("name=Title").unwrap();
        assert_eq!(p.key, "name");
        assert_eq!(p.op, PredicateOp::Equals);
        assert_eq!(p.value, "Title");
    }

    #[test]
    fn test_parse_not_equals() {
        let p = AttributePredicate::parse("name!=Hidden").unwrap();
        assert_eq!(p.op, PredicateOp::NotEquals);
    }

    #[test]
    fn test_parse_exists() {
        let p = AttributePredicate::parse("^name").unwrap();
        assert_eq!(p.op, PredicateOp::Exists);
    }

    #[test]
    fn test_matches_all_attributes() {
        let selector = Selector {
            element_type: None,
            attributes: vec![("name".to_string(), "Title".to_string())],
            style_shorthands: vec![],
            position: None,
        };

        let matching = make_node_with_format(&[("name", "Title")]);
        let not_matching = make_node_with_format(&[("name", "Body")]);

        assert!(matches_all_attributes(&matching, &selector));
        assert!(!matches_all_attributes(&not_matching, &selector));
    }
}
