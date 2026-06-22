use crate::dom_types::{WordDom, WordElementType};
use handler_common::{HandlerError, TextOffsetMap};
use std::collections::HashMap;

/// Build a TextOffsetMap from the Word DOM.
/// Each paragraph contributes its text, and each run gets its own span.
/// Paragraph breaks are represented as separate "paragraph-break" spans.
pub fn extract_text_with_offsets(dom: &WordDom) -> Result<TextOffsetMap, HandlerError> {
    let mut map = TextOffsetMap::empty("docx");

    let body = dom
        .body()
        .ok_or_else(|| HandlerError::OperationFailed("body element not found".to_string()))?;

    let mut para_idx = 0;
    let mut content_idx = 0;
    let mut tbl_idx = 0;
    let mut sdt_idx = 0;

    for child in &body.children {
        match child.element_type {
            WordElementType::Paragraph => {
                para_idx += 1;
                content_idx += 1;
                let para_path = format!("/body/p[{}]", para_idx);
                // Stable anchor: the paragraph's w14:paraId survives run splits and
                // index shifts, so callers can re-locate the paragraph after edits.
                let para_id = child.attributes.get("paraId").cloned();

                if content_idx > 1 {
                    // Add paragraph separator (newline) between paragraphs
                    map.push_span_with_id(
                        "\n",
                        &format!("/body/p[{}]/break", para_idx),
                        "paragraph-break",
                        para_id.clone(),
                    );
                }

                let before_spans = map.spans.len();
                collect_text_spans(child, &para_path, &mut map, para_id.clone());

                // If paragraph has no text, still add an empty span for navigation
                let para_text = child.paragraph_text();
                if para_text.is_empty() && map.spans.len() == before_spans {
                    map.push_span_with_id("", &para_path, "paragraph", para_id.clone());
                }
            }
            WordElementType::Table => {
                tbl_idx += 1;
                content_idx += 1;
                let tbl_path = format!("/body/tbl[{}]", tbl_idx);

                if content_idx > 1 {
                    map.push_span(
                        "\n",
                        &format!("/body/tbl[{}]/break", tbl_idx),
                        "paragraph-break",
                    );
                }

                let mut row_idx = 0;
                for tbl_child in &child.children {
                    if tbl_child.element_type == WordElementType::TableRow {
                        row_idx += 1;
                        let row_path = format!("{}/tr[{}]", tbl_path, row_idx);

                        let mut cell_idx = 0;
                        for tr_child in &tbl_child.children {
                            if tr_child.element_type == WordElementType::TableCell {
                                cell_idx += 1;
                                let cell_path = format!("{}/tc[{}]", row_path, cell_idx);

                                let cell_text = extract_cell_text(tr_child);
                                if !cell_text.is_empty() {
                                    map.push_span(&cell_text, &cell_path, "cell");
                                }

                                // Tab separator between cells in same row
                                if cell_idx < count_cells_in_row(&tbl_child.children) {
                                    map.push_span(
                                        "\t",
                                        &format!("{}/tc[{}]/sep", row_path, cell_idx),
                                        "cell-separator",
                                    );
                                }
                            }
                        }

                        // Newline between rows
                        if row_idx < count_rows_in_table(&child.children) {
                            map.push_span(
                                "\n",
                                &format!("{}/tr[{}]/break", tbl_path, row_idx),
                                "row-break",
                            );
                        }
                    }
                }
            }
            WordElementType::Sdt => {
                sdt_idx += 1;
                content_idx += 1;
                let sdt_path = format!("/body/sdt[{}]", sdt_idx);

                if content_idx > 1 {
                    map.push_span(
                        "\n",
                        &format!("/body/sdt[{}]/break", sdt_idx),
                        "paragraph-break",
                    );
                }

                let before_spans = map.spans.len();
                collect_text_spans(child, &sdt_path, &mut map, None);
                if map.spans.len() == before_spans {
                    map.push_span("", &sdt_path, "sdt");
                }
            }
            _ => {}
        }
    }

    Ok(map)
}

/// Recursively collect run-level text spans in document order.
///
/// PDF-to-DOCX conversion often places visible text in text boxes under
/// w:drawing/w:txbxContent. Those runs are nested several levels below the
/// outer paragraph run, so direct-child scanning misses them.
fn collect_text_spans(
    node: &crate::dom_types::WordNode,
    current_path: &str,
    map: &mut TextOffsetMap,
    owner_id: Option<String>,
) {
    let owner_id = if node.element_type == WordElementType::Paragraph {
        node.attributes.get("paraId").cloned().or(owner_id)
    } else {
        owner_id
    };

    if node.element_type == WordElementType::Run {
        collect_run_text_fragments(node, current_path, map, owner_id);
        return;
    }

    if is_alternate_content(node) {
        collect_preferred_alternate_content(node, current_path, map, owner_id);
        return;
    }

    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for child in &node.children {
        let name = child.element_type.to_path_name().to_string();
        let idx = type_counts.entry(name.clone()).or_insert(0);
        *idx += 1;
        let child_path = format!("{}/{}[{}]", current_path, name, *idx);
        collect_text_spans(child, &child_path, map, owner_id.clone());
    }
}

/// Collect direct text in a run while still descending into nested drawing,
/// field, or text-box content in the correct child order.
fn collect_run_text_fragments(
    run: &crate::dom_types::WordNode,
    run_path: &str,
    map: &mut TextOffsetMap,
    owner_id: Option<String>,
) {
    let mut pending = String::new();
    let mut type_counts: HashMap<String, usize> = HashMap::new();

    for child in &run.children {
        match child.element_type {
            WordElementType::Text => {
                if let Some(t) = &child.text_content {
                    pending.push_str(t);
                }
            }
            WordElementType::Tab => {
                pending.push('\t');
            }
            WordElementType::Break => {
                pending.push('\n');
            }
            WordElementType::RunProperties => {}
            _ => {
                if !pending.is_empty() {
                    map.push_span_with_id(&pending, run_path, "run", owner_id.clone());
                    pending.clear();
                }

                let name = child.element_type.to_path_name().to_string();
                let idx = type_counts.entry(name.clone()).or_insert(0);
                *idx += 1;
                let child_path = format!("{}/{}[{}]", run_path, name, *idx);
                collect_text_spans(child, &child_path, map, owner_id.clone());
            }
        }
    }

    if !pending.is_empty() {
        map.push_span_with_id(&pending, run_path, "run", owner_id);
    }
}

fn is_unknown_named(node: &crate::dom_types::WordNode, name: &str) -> bool {
    matches!(&node.element_type, WordElementType::Unknown(local) if local == name)
}

fn is_alternate_content(node: &crate::dom_types::WordNode) -> bool {
    is_unknown_named(node, "AlternateContent")
}

fn collect_preferred_alternate_content(
    node: &crate::dom_types::WordNode,
    current_path: &str,
    map: &mut TextOffsetMap,
    owner_id: Option<String>,
) {
    let preferred = node
        .children
        .iter()
        .position(|child| is_unknown_named(child, "Choice"))
        .or_else(|| {
            node.children
                .iter()
                .position(|child| is_unknown_named(child, "Fallback"))
        });

    if let Some(preferred_idx) = preferred {
        let child = &node.children[preferred_idx];
        let name = child.element_type.to_path_name();
        let same_type_position = node.children[..=preferred_idx]
            .iter()
            .filter(|sibling| sibling.element_type.to_path_name() == name)
            .count();
        let child_path = format!("{}/{}[{}]", current_path, name, same_type_position);
        collect_text_spans(child, &child_path, map, owner_id);
    }
}

/// Extract text from a table cell (w:tc).
fn extract_cell_text(cell: &crate::dom_types::WordNode) -> String {
    let mut result = String::new();
    let mut para_count = 0;
    for child in &cell.children {
        if child.element_type == WordElementType::Paragraph {
            if para_count > 0 {
                result.push('\n');
            }
            result.push_str(&child.paragraph_text());
            para_count += 1;
        }
    }
    result
}

/// Count cells in a table row.
fn count_cells_in_row(children: &[crate::dom_types::WordNode]) -> usize {
    children
        .iter()
        .filter(|c| c.element_type == WordElementType::TableCell)
        .count()
}

/// Count rows in a table.
fn count_rows_in_table(children: &[crate::dom_types::WordNode]) -> usize {
    children
        .iter()
        .filter(|c| c.element_type == WordElementType::TableRow)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom_types::WordNode;

    fn text_run(text: &str) -> WordNode {
        WordNode::new(WordElementType::Run)
            .with_children(vec![WordNode::new(WordElementType::Text).with_text(text)])
    }

    #[test]
    fn offsets_include_runs_nested_inside_textboxes() {
        let nested_textbox =
            WordNode::new(WordElementType::Run).with_children(vec![WordNode::new(
                WordElementType::Drawing,
            )
            .with_children(vec![WordNode::new(WordElementType::Unknown(
                "txbxContent".to_string(),
            ))
            .with_children(vec![WordNode::new(WordElementType::Paragraph)
                .with_attribute("paraId", "NESTED")
                .with_children(vec![text_run("Inside box")])])])]);

        let dom = WordDom::new(WordNode::new(WordElementType::Document).with_children(vec![
            WordNode::new(WordElementType::Body).with_children(vec![
                WordNode::new(WordElementType::Paragraph).with_children(vec![text_run("Heading")]),
                WordNode::new(WordElementType::Paragraph).with_children(vec![nested_textbox]),
            ]),
        ]));

        let map = extract_text_with_offsets(&dom).unwrap();

        assert_eq!(map.full_text, "Heading\nInside box");
        assert!(map.spans.iter().any(|span| {
            span.text == "Inside box"
                && span.path == "/body/p[2]/r[1]/drawing[1]/txbxContent[1]/p[1]/r[1]"
                && span.id.as_deref() == Some("NESTED")
        }));
    }

    #[test]
    fn offsets_prefer_alternate_content_choice_over_fallback() {
        let alternate_content = WordNode::new(WordElementType::Unknown(
            "AlternateContent".to_string(),
        ))
        .with_children(vec![
            WordNode::new(WordElementType::Unknown("Choice".to_string())).with_children(vec![
                WordNode::new(WordElementType::Drawing)
                    .with_children(vec![WordNode::new(WordElementType::Paragraph)
                        .with_children(vec![text_run("Choice text")])]),
            ]),
            WordNode::new(WordElementType::Unknown("Fallback".to_string()))
                .with_children(vec![WordNode::new(WordElementType::Paragraph)
                    .with_children(vec![text_run("Fallback text")])]),
        ]);

        let dom = WordDom::new(WordNode::new(WordElementType::Document).with_children(vec![
            WordNode::new(WordElementType::Body).with_children(vec![
                WordNode::new(WordElementType::Paragraph).with_children(vec![
                    WordNode::new(WordElementType::Run).with_children(vec![alternate_content]),
                ]),
            ]),
        ]));

        let map = extract_text_with_offsets(&dom).unwrap();

        assert_eq!(map.full_text, "Choice text");
        assert!(map.spans.iter().all(|span| !span.text.contains("Fallback")));
    }
}
