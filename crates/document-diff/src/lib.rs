use handler_common::{DocumentHandler, HandlerError, TextOffsetMap};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    pub path: String,
    pub kind: DiffKind,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
}

pub enum DiffFormat {
    Text,
    Json,
}

/// Compare two documents and return list of differences.
pub fn diff_documents(
    old_handler: &dyn DocumentHandler,
    new_handler: &dyn DocumentHandler,
    _format: DiffFormat,
) -> Result<Vec<DiffEntry>, String> {
    let old_map = old_handler
        .extract_text_with_offsets()
        .map_err(|e| format!("failed to extract from old: {}", e))?;
    let new_map = new_handler
        .extract_text_with_offsets()
        .map_err(|e| format!("failed to extract from new: {}", e))?;
    diff_text_offset_maps(&old_map, &new_map)
}

pub fn diff_text_offset_maps(old_map: &TextOffsetMap, new_map: &TextOffsetMap) -> Result<Vec<DiffEntry>, String> {
    let mut diffs = Vec::new();

    let old_paths: HashMap<&str, &str> = old_map
        .spans
        .iter()
        .map(|s| (s.path.as_str(), s.text.as_str()))
        .collect();
    let new_paths: HashMap<&str, &str> = new_map
        .spans
        .iter()
        .map(|s| (s.path.as_str(), s.text.as_str()))
        .collect();

    for (path, old_text) in &old_paths {
        match new_paths.get(path) {
            None => diffs.push(DiffEntry {
                path: path.to_string(),
                kind: DiffKind::Removed,
                old_value: Some(old_text.to_string()),
                new_value: None,
            }),
            Some(new_text) if *new_text != *old_text => diffs.push(DiffEntry {
                path: path.to_string(),
                kind: DiffKind::Modified,
                old_value: Some(old_text.to_string()),
                new_value: Some(new_text.to_string()),
            }),
            _ => {}
        }
    }

    for (path, new_text) in &new_paths {
        if !old_paths.contains_key(path) {
            diffs.push(DiffEntry {
                path: path.to_string(),
                kind: DiffKind::Added,
                old_value: None,
                new_value: Some(new_text.to_string()),
            });
        }
    }

    diffs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(diffs)
}

/// Render diff entries as colorized text (using ANSI codes).
pub fn render_text_diff(diff: &[DiffEntry]) -> String {
    let mut out = String::new();
    for entry in diff {
        let symbol = match entry.kind {
            DiffKind::Added => "\x1b[32m+",
            DiffKind::Removed => "\x1b[31m-",
            DiffKind::Modified => "\x1b[33m~",
        };
        out.push_str(&format!("{} {}\n", symbol, entry.path));
        match entry.kind {
            DiffKind::Added => {
                if let Some(ref v) = entry.new_value {
                    for line in v.lines() {
                        out.push_str(&format!("  \x1b[32m+ {}\n", line));
                    }
                }
            }
            DiffKind::Removed => {
                if let Some(ref v) = entry.old_value {
                    for line in v.lines() {
                        out.push_str(&format!("  \x1b[31m- {}\n", line));
                    }
                }
            }
            DiffKind::Modified => {
                if let Some(ref v) = entry.old_value {
                    for line in v.lines() {
                        out.push_str(&format!("  \x1b[31m- {}\n", line));
                    }
                }
                if let Some(ref v) = entry.new_value {
                    for line in v.lines() {
                        out.push_str(&format!("  \x1b[32m+ {}\n", line));
                    }
                }
            }
        }
        out.push_str("\x1b[0m");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use handler_common::{OffsetSpan, TextMapMeta, TextOffsetMap};

    fn make_map(spans: Vec<(&str, &str, &str)>) -> TextOffsetMap {
        let mut map = TextOffsetMap {
            full_text: String::new(),
            spans: Vec::new(),
            meta: TextMapMeta {
                format: "test".to_string(),
                total_chars: 0,
                total_spans: 0,
            },
        };
        for (path, text, elem_type) in spans {
            let start = map.full_text.chars().count();
            map.full_text.push_str(text);
            let end = map.full_text.chars().count();
            map.spans.push(OffsetSpan {
                start,
                end,
                path: path.to_string(),
                text: text.to_string(),
                element_type: elem_type.to_string(),
                id: None,
                bbox: None,
                style: None,
            });
        }
        map.meta.total_chars = map.full_text.chars().count();
        map.meta.total_spans = map.spans.len();
        map
    }

    #[test]
    fn test_identical_documents() {
        let old = make_map(vec![
            ("/body/p[1]", "Hello", "paragraph"),
            ("/body/p[2]", "World", "paragraph"),
        ]);
        let new = make_map(vec![
            ("/body/p[1]", "Hello", "paragraph"),
            ("/body/p[2]", "World", "paragraph"),
        ]);
        let diffs = diff_text_offset_maps(&old, &new).unwrap();
        assert!(diffs.is_empty(), "identical docs should have no diffs");
    }

    #[test]
    fn test_added_content() {
        let old = make_map(vec![("/body/p[1]", "Hello", "paragraph")]);
        let new = make_map(vec![
            ("/body/p[1]", "Hello", "paragraph"),
            ("/body/p[2]", "New text", "paragraph"),
        ]);
        let diffs = diff_text_offset_maps(&old, &new).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, DiffKind::Added);
        assert_eq!(diffs[0].path, "/body/p[2]");
        assert_eq!(diffs[0].new_value.as_deref(), Some("New text"));
        assert!(diffs[0].old_value.is_none());
    }

    #[test]
    fn test_removed_content() {
        let old = make_map(vec![
            ("/body/p[1]", "Hello", "paragraph"),
            ("/body/p[2]", "Removed", "paragraph"),
        ]);
        let new = make_map(vec![("/body/p[1]", "Hello", "paragraph")]);
        let diffs = diff_text_offset_maps(&old, &new).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, DiffKind::Removed);
        assert_eq!(diffs[0].path, "/body/p[2]");
        assert_eq!(diffs[0].old_value.as_deref(), Some("Removed"));
        assert!(diffs[0].new_value.is_none());
    }

    #[test]
    fn test_modified_content() {
        let old = make_map(vec![("/body/p[1]", "Hello", "paragraph")]);
        let new = make_map(vec![("/body/p[1]", "Changed", "paragraph")]);
        let diffs = diff_text_offset_maps(&old, &new).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, DiffKind::Modified);
        assert_eq!(diffs[0].path, "/body/p[1]");
        assert_eq!(diffs[0].old_value.as_deref(), Some("Hello"));
        assert_eq!(diffs[0].new_value.as_deref(), Some("Changed"));
    }

    #[test]
    fn test_mixed_changes() {
        let old = make_map(vec![
            ("/body/p[1]", "A", "paragraph"),
            ("/body/p[2]", "B", "paragraph"),
            ("/body/p[3]", "C", "paragraph"),
        ]);
        let new = make_map(vec![
            ("/body/p[1]", "A", "paragraph"),
            ("/body/p[3]", "Modified", "paragraph"),
            ("/body/p[4]", "New", "paragraph"),
        ]);
        let diffs = diff_text_offset_maps(&old, &new).unwrap();
        assert_eq!(diffs.len(), 3);

        // Sorted by path: /body/p[2] (removed), /body/p[3] (modified), /body/p[4] (added)
        assert_eq!(diffs[0].path, "/body/p[2]");
        assert_eq!(diffs[0].kind, DiffKind::Removed);

        assert_eq!(diffs[1].path, "/body/p[3]");
        assert_eq!(diffs[1].kind, DiffKind::Modified);
        assert_eq!(diffs[1].old_value.as_deref(), Some("C"));
        assert_eq!(diffs[1].new_value.as_deref(), Some("Modified"));

        assert_eq!(diffs[2].path, "/body/p[4]");
        assert_eq!(diffs[2].kind, DiffKind::Added);
        assert_eq!(diffs[2].new_value.as_deref(), Some("New"));
    }

    #[test]
    fn test_empty_documents() {
        let old = make_map(vec![]);
        let new = make_map(vec![]);
        let diffs = diff_text_offset_maps(&old, &new).unwrap();
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_identical_empty_vs_nonempty() {
        let old = make_map(vec![]);
        let new = make_map(vec![("/body/p[1]", "X", "paragraph")]);
        let diffs = diff_text_offset_maps(&old, &new).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, DiffKind::Added);
    }

    #[test]
    fn test_render_text_diff_added() {
        let diffs = vec![DiffEntry {
            path: "/body/p[1]".to_string(),
            kind: DiffKind::Added,
            old_value: None,
            new_value: Some("Hello".to_string()),
        }];
        let rendered = render_text_diff(&diffs);
        assert!(rendered.contains("+ /body/p[1]"));
        assert!(rendered.contains("+ Hello"));
    }

    #[test]
    fn test_render_text_diff_removed() {
        let diffs = vec![DiffEntry {
            path: "/body/p[1]".to_string(),
            kind: DiffKind::Removed,
            old_value: Some("Goodbye".to_string()),
            new_value: None,
        }];
        let rendered = render_text_diff(&diffs);
        assert!(rendered.contains("- /body/p[1]"));
        assert!(rendered.contains("- Goodbye"));
    }

    #[test]
    fn test_render_text_diff_modified() {
        let diffs = vec![DiffEntry {
            path: "/body/p[1]".to_string(),
            kind: DiffKind::Modified,
            old_value: Some("Old".to_string()),
            new_value: Some("New".to_string()),
        }];
        let rendered = render_text_diff(&diffs);
        assert!(rendered.contains("~ /body/p[1]"));
        assert!(rendered.contains("- Old"));
        assert!(rendered.contains("+ New"));
    }
}
