use crate::dom_types::{WordDom, WordElementType, WordNode};
use crate::helpers::{generate_bookmark_id, quote_attr_value_if_needed, validate_bookmark_name};
use crate::navigation::{navigate_to_element, navigate_to_element_mut, parse_path};
use handler_common::{HandlerError, InsertPosition, PathRangeSegment};
use std::collections::HashMap;

/// Add a new element at the given parent path.
/// Expanded element vocabulary matching C# WordHandler.Add:
/// paragraph/p, run/r, table/tbl, row/tr, cell/tc, bookmark,
/// hyperlink, image/drawing, field/fldSimple, break/br, tab, sectionBreak,
/// footnote, endnote, sdt/contentControl
pub fn add_element(
    dom: &mut WordDom,
    parent: &str,
    element_type: &str,
    position: InsertPosition,
    properties: &HashMap<String, String>,
    wrap: Option<&str>,
) -> Result<String, HandlerError> {
    let resolved_type = resolve_add_type(element_type)?;

    match resolved_type {
        AddType::Paragraph => add_paragraph(dom, parent, position, properties),
        AddType::Run => add_run(dom, parent, position, properties),
        AddType::Table => add_table(dom, parent, position, properties),
        AddType::TableRow => add_table_row(dom, parent, position, properties),
        AddType::TableCell => add_table_cell(dom, parent, position, properties),
        AddType::Bookmark => add_bookmark(dom, parent, position, properties, wrap),
        AddType::Hyperlink => add_hyperlink(dom, parent, position, properties),
        AddType::Image => add_image(dom, parent, position, properties),
        AddType::Field => add_field(dom, parent, position, properties),
        AddType::Break => add_break(dom, parent, position, properties),
        AddType::Tab => add_tab(dom, parent, position, properties),
        AddType::SectionBreak => add_section_break(dom, parent, position, properties),
        AddType::FootnoteRef => add_footnote_reference(dom, parent, position, properties),
        AddType::EndnoteRef => add_endnote_reference(dom, parent, position, properties),
        AddType::Sdt => add_sdt_block(dom, parent, position, properties),
        AddType::SdtRun => add_sdt_run(dom, parent, position, properties),
    }
}

#[derive(Debug, Clone)]
enum AddType {
    Paragraph,
    Run,
    Table,
    TableRow,
    TableCell,
    Bookmark,
    Hyperlink,
    Image,
    Field,
    Break,
    Tab,
    SectionBreak,
    FootnoteRef,
    EndnoteRef,
    Sdt,
    SdtRun,
}

#[derive(Debug, Clone)]
struct ParagraphRange {
    path: String,
    start: usize,
    end: usize,
}

fn resolve_add_type(name: &str) -> Result<AddType, HandlerError> {
    match name {
        "p" | "paragraph" => Ok(AddType::Paragraph),
        "r" | "run" => Ok(AddType::Run),
        "tbl" | "table" => Ok(AddType::Table),
        "tr" | "row" => Ok(AddType::TableRow),
        "tc" | "cell" => Ok(AddType::TableCell),
        "bookmark" | "bookmarkStart" | "bookmarkstart" => Ok(AddType::Bookmark),
        "hyperlink" | "link" => Ok(AddType::Hyperlink),
        "image" | "drawing" | "picture" => Ok(AddType::Image),
        "field" | "fldSimple" | "fldsimple" => Ok(AddType::Field),
        "break" | "br" => Ok(AddType::Break),
        "tab" => Ok(AddType::Tab),
        "sectionBreak" | "sectionbreak" => Ok(AddType::SectionBreak),
        "footnote" | "footnoteReference" | "footnoteRef" => Ok(AddType::FootnoteRef),
        "endnote" | "endnoteReference" | "endnoteRef" => Ok(AddType::EndnoteRef),
        "sdt" | "contentControl" | "sdtBlock" => Ok(AddType::Sdt),
        "sdtRun" | "inlineSdt" => Ok(AddType::SdtRun),
        other => Err(HandlerError::UnsupportedType(format!(
            "cannot add element type: '{}' (supported: paragraph/p, run/r, table/tbl, row/tr, cell/tc, bookmark, hyperlink, image, field, break, tab, sectionBreak, footnote, endnote, sdt)",
            other
        ))),
    }
}

// ─── Bookmark Add ──────────────────────────────────────────────

fn add_bookmark(
    dom: &mut WordDom,
    parent: &str,
    position: InsertPosition,
    properties: &HashMap<String, String>,
    wrap: Option<&str>,
) -> Result<String, HandlerError> {
    // 1. Handle --range-paths mode (atomic: split runs + insert bookmark pair)
    if let Some(range_paths_str) = properties.get("range_paths") {
        let segments = handler_common::parse_range_paths(range_paths_str)
            .map_err(|e| HandlerError::InvalidArgument(format!("invalid range paths: {}", e)))?;
        return add_bookmark_by_range(dom, parent, properties, &segments);
    }

    // 2. Handle --wrap mode
    if let Some(wrap_path) = wrap {
        return add_bookmark_wrap(dom, parent, properties, wrap_path);
    }

    // 3. Standard positional mode (migrated from C# AddBookmark)
    add_bookmark_positional(dom, parent, position, properties)
}

/// Resolve a range-paths segment to paragraph-level path and offsets.
/// Supports paragraph-level paths (/body/p[3][5..20]), run-level paths
/// (/body/p[3]/r[2][10..15]), and hyperlink paths
/// (/body/p[3]/hyperlink[1][0..5] or /body/p[3]/hyperlink[1]/r[1][0..5]).
fn resolve_range_to_paragraph(
    dom: &WordDom,
    seg_path: &str,
    seg_start: Option<usize>,
    seg_end: Option<usize>,
) -> Result<(String, usize, usize), HandlerError> {
    // Try navigating to the path directly (read-only)
    let node = navigate_to_element(dom, seg_path)?;

    if node.element_type == WordElementType::Paragraph {
        // Already a paragraph path — use offsets directly
        let total = node.paragraph_text().chars().count();
        let start = seg_start.unwrap_or(0);
        let end = seg_end.unwrap_or(total);
        return Ok((seg_path.to_string(), start, end));
    }

    if node.element_type == WordElementType::Run || node.element_type == WordElementType::Hyperlink
    {
        // Run/hyperlink-level path — convert to paragraph-level offsets.
        let para_path = extract_paragraph_path(seg_path)?;

        let para_node = navigate_to_element(dom, &para_path)?;
        if para_node.element_type != WordElementType::Paragraph {
            return Err(HandlerError::InvalidArgument(format!(
                "cannot find paragraph for range path '{}'",
                seg_path
            )));
        }

        let (node_start, node_end) =
            compute_text_range_in_paragraph(para_node, &para_path, seg_path)?;
        let node_text_len = node_end.saturating_sub(node_start);

        let start = node_start + seg_start.unwrap_or(0);
        let end = node_start + seg_end.unwrap_or(node_text_len);

        Ok((para_path, start, end))
    } else {
        Err(HandlerError::InvalidArgument(format!(
            "range-paths must point to a Paragraph, Run, Hyperlink, or TableCell, found: {:?}",
            node.element_type
        )))
    }
}

fn resolve_segments_to_paragraph_ranges(
    dom: &WordDom,
    segments: &[PathRangeSegment],
) -> Result<Vec<ParagraphRange>, HandlerError> {
    let mut ranges = Vec::new();

    for seg in segments {
        ranges.extend(resolve_segment_to_paragraph_ranges(dom, seg)?);
    }

    if ranges.is_empty() {
        return Err(HandlerError::InvalidArgument(
            "range-paths did not resolve to any paragraph text".to_string(),
        ));
    }

    Ok(ranges)
}

fn resolve_segment_to_paragraph_ranges(
    dom: &WordDom,
    seg: &PathRangeSegment,
) -> Result<Vec<ParagraphRange>, HandlerError> {
    if is_virtual_text_offset_path(&seg.path) {
        return Ok(Vec::new());
    }

    let node = navigate_to_element(dom, &seg.path)?;

    if node.element_type == WordElementType::TableCell {
        return resolve_cell_range_to_paragraph_ranges(node, &seg.path, seg.start, seg.end);
    }

    if node.element_type == WordElementType::Paragraph
        || node.element_type == WordElementType::Run
        || node.element_type == WordElementType::Hyperlink
    {
        let (path, start, end) = resolve_range_to_paragraph(dom, &seg.path, seg.start, seg.end)?;
        return Ok(vec![ParagraphRange { path, start, end }]);
    }

    Err(HandlerError::InvalidArgument(format!(
        "range-paths must point to a Paragraph, Run, Hyperlink, or TableCell, found: {:?}",
        node.element_type
    )))
}

fn is_virtual_text_offset_path(path: &str) -> bool {
    path.ends_with("/sep") || path.ends_with("/break")
}

fn resolve_cell_range_to_paragraph_ranges(
    cell: &WordNode,
    cell_path: &str,
    seg_start: Option<usize>,
    seg_end: Option<usize>,
) -> Result<Vec<ParagraphRange>, HandlerError> {
    let para_count = cell
        .children
        .iter()
        .filter(|child| child.element_type == WordElementType::Paragraph)
        .count();

    if para_count == 0 {
        return Err(HandlerError::InvalidArgument(format!(
            "range-paths target table cell '{}' has no paragraphs",
            cell_path
        )));
    }

    let total_len = cell_text_len(cell);
    let target_start = seg_start.unwrap_or(0);
    let target_end = seg_end.unwrap_or(total_len);

    if target_start > target_end {
        return Err(HandlerError::InvalidArgument(format!(
            "range {}[{}..{}] is reversed",
            cell_path, target_start, target_end
        )));
    }

    let mut ranges = Vec::new();
    let mut para_idx = 0;
    let mut cursor = 0;

    for child in &cell.children {
        if child.element_type != WordElementType::Paragraph {
            continue;
        }

        para_idx += 1;
        let text_len = child.paragraph_text().chars().count();
        let para_start = cursor;
        let para_end = para_start + text_len;

        let overlap_start = target_start.max(para_start);
        let overlap_end = target_end.min(para_end);
        if overlap_start < overlap_end || (text_len == 0 && target_start == para_start) {
            ranges.push(ParagraphRange {
                path: format!("{}/p[{}]", cell_path, para_idx),
                start: overlap_start.saturating_sub(para_start),
                end: overlap_end.saturating_sub(para_start),
            });
        }

        cursor = para_end;
        if para_idx < para_count {
            cursor += 1; // extract-text joins paragraphs inside a cell with '\n'.
        }
    }

    if ranges.is_empty() {
        return Err(HandlerError::InvalidArgument(format!(
            "range {}[{}..{}] did not overlap table cell text length {}",
            cell_path, target_start, target_end, total_len
        )));
    }

    Ok(ranges)
}

fn cell_text_len(cell: &WordNode) -> usize {
    let para_count = cell
        .children
        .iter()
        .filter(|child| child.element_type == WordElementType::Paragraph)
        .count();
    let text_len: usize = cell
        .children
        .iter()
        .filter(|child| child.element_type == WordElementType::Paragraph)
        .map(|child| child.paragraph_text().chars().count())
        .sum();

    text_len + para_count.saturating_sub(1)
}

/// Extract the enclosing paragraph path from a paragraph/run/hyperlink path.
fn extract_paragraph_path(path: &str) -> Result<String, HandlerError> {
    let Some(pos) = path.rfind("/p[") else {
        return Err(HandlerError::InvalidArgument(format!(
            "cannot extract paragraph path from '{}'",
            path
        )));
    };
    let rest = &path[pos..];
    let Some(end) = rest.find(']') else {
        return Err(HandlerError::InvalidArgument(format!(
            "malformed paragraph path '{}'",
            path
        )));
    };
    Ok(path[..pos + end + 1].to_string())
}

/// Compute the cumulative character range of a run or hyperlink within its paragraph.
fn compute_text_range_in_paragraph(
    para_node: &crate::dom_types::WordNode,
    para_path: &str,
    target_path: &str,
) -> Result<(usize, usize), HandlerError> {
    let mut offset = 0;
    if let Some(range) = find_text_range_by_path(para_node, para_path, target_path, &mut offset) {
        return Ok(range);
    }
    Err(HandlerError::PathNotFound(format!(
        "range path '{}' was not found in paragraph '{}'",
        target_path, para_path
    )))
}

fn find_text_range_by_path(
    node: &WordNode,
    current_path: &str,
    target_path: &str,
    offset: &mut usize,
) -> Option<(usize, usize)> {
    if current_path == target_path {
        let start = *offset;
        let len = node.paragraph_text().chars().count();
        *offset += len;
        return Some((start, start + len));
    }

    if node.element_type == WordElementType::Run {
        *offset += node.paragraph_text().chars().count();
        return None;
    }

    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for child in &node.children {
        let name = child.element_type.to_path_name().to_string();
        let idx = type_counts.entry(name.clone()).or_insert(0);
        *idx += 1;
        let child_path = format!("{}/{}[{}]", current_path, name, *idx);
        if let Some(range) = find_text_range_by_path(child, &child_path, target_path, offset) {
            return Some(range);
        }
    }
    None
}

/// Add bookmark using --range-paths: atomically split runs at char offsets
/// and insert bookmarkStart/bookmarkEnd around the highlighted region.
fn add_bookmark_by_range(
    dom: &mut WordDom,
    _parent: &str,
    properties: &HashMap<String, String>,
    segments: &[PathRangeSegment],
) -> Result<String, HandlerError> {
    // Validate name
    let bk_name = properties.get("name").cloned().unwrap_or_default();
    validate_bookmark_name(&bk_name)?;

    // Reject duplicate names
    if !skip_bookmark_duplicate_check(properties) {
        reject_duplicate_bookmark_name(dom, &bk_name)?;
    }

    // Resolve bookmark ID
    let bk_id = resolve_bookmark_id(dom, properties)?;

    let bookmark_start = WordNode::new(WordElementType::BookmarkStart)
        .with_attribute("id", &bk_id)
        .with_attribute("name", &bk_name);
    let bookmark_end = WordNode::new(WordElementType::BookmarkEnd).with_attribute("id", &bk_id);

    // Format properties for the bookmarked content
    let mut format_props = properties.clone();
    format_props.remove("name");
    format_props.remove("id");
    remove_bookmark_batch_hints(&mut format_props);
    format_props.remove("range_paths");
    format_props.remove("endPara");
    format_props.remove("endpara");

    let paragraph_ranges = resolve_segments_to_paragraph_ranges(dom, segments)?;

    // For range-paths with a single segment, wrap around that range
    if paragraph_ranges.len() == 1 {
        let range = &paragraph_ranges[0];
        let para_path = range.path.clone();
        let target_start = range.start;
        let target_end = range.end;

        let para_node = navigate_to_element_mut(dom, &para_path)?;
        if para_node.element_type != WordElementType::Paragraph {
            return Err(HandlerError::InvalidArgument(format!(
                "range-paths for bookmark must point to a Paragraph, found: {:?}",
                para_node.element_type
            )));
        }

        // Collect runs with their text offsets
        let mut collected_runs = Vec::new();
        let mut path_tracker = Vec::new();
        collect_run_locations(para_node, &mut path_tracker, &mut collected_runs);

        let mut global_start = 0;
        let mut runs_with_spans: Vec<(Vec<usize>, usize, usize)> = Vec::new();
        for (path, text) in &collected_runs {
            let len = text.chars().count();
            let global_end = global_start + len;
            runs_with_spans.push((path.clone(), global_start, global_end));
            global_start = global_end;
        }

        let _total_text_len = global_start;
        // target_start/target_end already resolved to paragraph-level offsets by resolve_range_to_paragraph

        // Phase 1: Split runs at char boundaries (reverse order for stable indices)
        // Track which split fragments fall inside the bookmark range.
        // For each overlapping run, we produce [left, mid, right] where:
        //   left  = chars before the range  (outside bookmark)
        //   mid   = chars inside the range  (inside bookmark — gets format props)
        //   right = chars after the range   (outside bookmark)
        let mut inside_fragment_indices: Vec<(Vec<usize>, usize)> = Vec::new();

        for (path, r_start, r_end) in runs_with_spans.iter().rev() {
            let overlap_start = (*r_start).max(target_start);
            let overlap_end = (*r_end).min(target_end);

            if overlap_start >= overlap_end {
                continue;
            }

            let local_start = overlap_start - *r_start;
            let local_end = overlap_end - *r_start;

            let parent_path = &path[..path.len() - 1];
            let last_idx = path[path.len() - 1];

            let para_node = navigate_to_element_mut(dom, &para_path)?;
            let run_parent = get_node_mut_by_path(para_node, parent_path);
            let run = run_parent.children[last_idx].clone();
            let text = run.paragraph_text();

            let byte_start = text
                .char_indices()
                .nth(local_start)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            let byte_end = text
                .char_indices()
                .nth(local_end)
                .map(|(i, _)| i)
                .unwrap_or(text.len());

            let (left, rest) = crate::helpers::split_run_at_offset(&run, byte_start);
            let mut mid = None;
            let mut right = None;
            if let Some(r) = rest {
                let (m, rg) = crate::helpers::split_run_at_offset(&r, byte_end - byte_start);
                mid = m;
                right = rg;
            }

            // Track fragment presence before moving values
            let left_present = left.is_some();
            let mid_present = mid.is_some();

            let mut inserted = Vec::new();
            if let Some(l) = left {
                inserted.push(l);
            }
            if let Some(mut m) = mid {
                if !format_props.is_empty() {
                    merge_run_properties(&mut m, &format_props);
                }
                inserted.push(m);
            }
            if let Some(rg) = right {
                inserted.push(rg);
            }

            // Record which inserted fragment is the "mid" (inside-range) one
            // left occupies index 0 if present, mid occupies index left_count
            let left_count = if left_present { 1 } else { 0 };
            if mid_present {
                inside_fragment_indices.push((parent_path.to_vec(), last_idx + left_count));
            }

            let para_node = navigate_to_element_mut(dom, &para_path)?;
            let run_parent = get_node_mut_by_path(para_node, parent_path);
            run_parent.children.splice(last_idx..=last_idx, inserted);
        }

        // Phase 2: Insert bookmarkStart/bookmarkEnd around the inside-range fragments.
        // Recursively locate text boundaries so ranges inside hyperlinks place the
        // bookmark markers inside the hyperlink rather than around the whole wrapper.
        let para_node = navigate_to_element_mut(dom, &para_path)?;
        let start_point = find_insertion_point_for_text_offset(para_node, target_start, false);
        let end_point = find_insertion_point_for_text_offset(para_node, target_end, true);

        // Insert bookmarkEnd first (at the later text boundary) so bookmarkStart
        // indices stay valid when both markers share a parent.
        insert_at_text_point(para_node, end_point, bookmark_end.clone(), true);

        let para_node = navigate_to_element_mut(dom, &para_path)?;
        insert_at_text_point(para_node, start_point, bookmark_start.clone(), false);

        let quoted = quote_attr_value_if_needed(&bk_name)?;
        return Ok(format!("{}/bookmarkStart[@name={}]", para_path, quoted));
    }

    // Multi-segment range: bookmarkStart at first segment start, bookmarkEnd at last segment end
    // Process each segment to split runs at its boundaries.
    let first_range = &paragraph_ranges[0];
    let last_range = paragraph_ranges.last().unwrap();
    let first_para_path = first_range.path.clone();
    let first_target_start = first_range.start;
    let last_para_path = last_range.path.clone();
    let last_target_end = last_range.end;

    // Process each segment: split runs at boundaries
    for range in &paragraph_ranges {
        let para_path = &range.path;
        let seg_target_start = range.start;
        let seg_target_end = range.end;
        let para_node = navigate_to_element_mut(dom, para_path)?;
        if para_node.element_type != WordElementType::Paragraph {
            return Err(HandlerError::InvalidArgument(
                "range-paths for bookmark must point to Paragraphs".to_string(),
            ));
        }

        let mut collected_runs = Vec::new();
        let mut path_tracker = Vec::new();
        collect_run_locations(para_node, &mut path_tracker, &mut collected_runs);

        let mut global_start = 0;
        let mut runs_with_spans: Vec<(Vec<usize>, usize, usize)> = Vec::new();
        for (path, text) in &collected_runs {
            let len = text.chars().count();
            let global_end = global_start + len;
            runs_with_spans.push((path.clone(), global_start, global_end));
            global_start = global_end;
        }

        let target_start = seg_target_start;
        let target_end = seg_target_end;

        for (path, r_start, r_end) in runs_with_spans.iter().rev() {
            let overlap_start = (*r_start).max(target_start);
            let overlap_end = (*r_end).min(target_end);

            if overlap_start >= overlap_end {
                continue;
            }

            let local_start = overlap_start - *r_start;
            let local_end = overlap_end - *r_start;

            let parent_path = &path[..path.len() - 1];
            let last_idx = path[path.len() - 1];

            let para_node = navigate_to_element_mut(dom, para_path)?;
            let run_parent = get_node_mut_by_path(para_node, parent_path);
            let run = run_parent.children[last_idx].clone();
            let text = run.paragraph_text();

            let byte_start = text
                .char_indices()
                .nth(local_start)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            let byte_end = text
                .char_indices()
                .nth(local_end)
                .map(|(i, _)| i)
                .unwrap_or(text.len());

            let (left, rest) = crate::helpers::split_run_at_offset(&run, byte_start);
            let mut mid = None;
            let mut right = None;
            if let Some(r) = rest {
                let (m, rg) = crate::helpers::split_run_at_offset(&r, byte_end - byte_start);
                mid = m;
                right = rg;
            }

            let mut inserted = Vec::new();
            if let Some(l) = left {
                inserted.push(l);
            }
            if let Some(mut m) = mid {
                if !format_props.is_empty() {
                    merge_run_properties(&mut m, &format_props);
                }
                inserted.push(m);
            }
            if let Some(rg) = right {
                inserted.push(rg);
            }

            let para_node = navigate_to_element_mut(dom, para_path)?;
            let run_parent = get_node_mut_by_path(para_node, parent_path);
            run_parent.children.splice(last_idx..=last_idx, inserted);
        }
    }

    // Insert bookmarkStart at the correct position in the first segment's paragraph
    {
        let target_start = first_target_start;
        let para_node = navigate_to_element_mut(dom, &first_para_path)?;

        let mut cumulative = 0;
        let mut bk_start_idx: Option<usize> = None;
        for (i, child) in para_node.children.iter().enumerate() {
            if child.element_type == WordElementType::ParagraphProperties
                || child.element_type == WordElementType::BookmarkStart
                || child.element_type == WordElementType::BookmarkEnd
            {
                continue;
            }
            let text_len = child.paragraph_text().chars().count();
            if text_len == 0 {
                continue;
            }
            if bk_start_idx.is_none() && cumulative + text_len > target_start {
                bk_start_idx = Some(i);
            }
            cumulative += text_len;
        }

        if let Some(start_idx) = bk_start_idx {
            para_node.children.insert(start_idx, bookmark_start.clone());
        } else {
            let past_ppr = if para_node.children.first().map(|c| c.element_type.clone())
                == Some(WordElementType::ParagraphProperties)
            {
                1
            } else {
                0
            };
            para_node.children.insert(past_ppr, bookmark_start.clone());
        }
    }

    // Insert bookmarkEnd at the correct position in the last segment's paragraph
    {
        let target_end = last_target_end;
        let para_node = navigate_to_element_mut(dom, &last_para_path)?;

        let mut cumulative = 0;
        let mut bk_end_idx: Option<usize> = None;
        for (i, child) in para_node.children.iter().enumerate() {
            if child.element_type == WordElementType::ParagraphProperties
                || child.element_type == WordElementType::BookmarkStart
                || child.element_type == WordElementType::BookmarkEnd
            {
                continue;
            }
            let text_len = child.paragraph_text().chars().count();
            if text_len == 0 {
                continue;
            }
            cumulative += text_len;
            if bk_end_idx.is_none() && cumulative >= target_end {
                bk_end_idx = Some(i + 1);
            }
        }

        if let Some(end_idx) = bk_end_idx {
            para_node.children.insert(end_idx, bookmark_end.clone());
        } else {
            para_node.children.push(bookmark_end.clone());
        }
    }

    let quoted = quote_attr_value_if_needed(&bk_name)?;
    Ok(format!(
        "{}/bookmarkStart[@name={}]",
        first_para_path, quoted
    ))
}

/// Add bookmark using --wrap mode: insert bookmarkStart before the target element
/// and bookmarkEnd after it.
#[allow(clippy::only_used_in_recursion)]
fn add_bookmark_wrap(
    dom: &mut WordDom,
    parent: &str,
    properties: &HashMap<String, String>,
    wrap_path: &str,
) -> Result<String, HandlerError> {
    // Validate name
    let bk_name = properties.get("name").cloned().unwrap_or_default();
    validate_bookmark_name(&bk_name)?;

    // Reject duplicate names
    if !skip_bookmark_duplicate_check(properties) {
        reject_duplicate_bookmark_name(dom, &bk_name)?;
    }

    // Resolve bookmark ID
    let bk_id = resolve_bookmark_id(dom, properties)?;

    let bookmark_start = WordNode::new(WordElementType::BookmarkStart)
        .with_attribute("id", &bk_id)
        .with_attribute("name", &bk_name);
    let bookmark_end = WordNode::new(WordElementType::BookmarkEnd).with_attribute("id", &bk_id);

    // Navigate to the wrap target
    let target = navigate_to_element(dom, wrap_path)?;

    // Determine wrapping strategy based on target element type
    if target.element_type == WordElementType::Run {
        // Run-level wrap: both bookmark nodes go inside the same paragraph as siblings of the run
        // Need to find the target's parent and its child index
        let wrap_segments = parse_path(wrap_path)?;
        if wrap_segments.len() < 2 {
            return Err(HandlerError::InvalidPath(format!(
                "wrap path must have a parent: {}",
                wrap_path
            )));
        }

        // Find the parent path
        let parent_path_str = crate::navigation::parent_path(wrap_path)
            .ok_or_else(|| HandlerError::InvalidPath("wrap path has no parent".to_string()))?;

        // Find the target's 0-based index among its siblings of the same type
        let target_type = target.element_type.clone();
        let target_idx_in_parent = {
            let parent_node = navigate_to_element(dom, &parent_path_str)?;
            let mut count = 0;
            let mut result_idx: Option<usize> = None;
            for (i, child) in parent_node.children.iter().enumerate() {
                if child.element_type == target_type {
                    count += 1;
                    let last_seg = wrap_segments.last().unwrap();
                    if last_seg.index == Some(count) {
                        result_idx = Some(i);
                        break;
                    }
                }
            }
            result_idx.ok_or_else(|| {
                HandlerError::PathNotFound(format!("target not found in parent: {}", wrap_path))
            })?
        };

        let parent_node = navigate_to_element_mut(dom, &parent_path_str)?;

        // Insert bookmarkStart before the target run
        parent_node
            .children
            .insert(target_idx_in_parent, bookmark_start.clone());

        // Insert bookmarkEnd after the target run (target_idx + 2 because bookmarkStart was just inserted)
        parent_node
            .children
            .insert(target_idx_in_parent + 2, bookmark_end.clone());

        // Optionally apply highlight/color to the wrapped run
        let mut format_props = properties.clone();
        format_props.remove("name");
        format_props.remove("id");
        remove_bookmark_batch_hints(&mut format_props);
        format_props.remove("endPara");
        format_props.remove("endpara");
        if !format_props.is_empty() {
            let parent_node = navigate_to_element_mut(dom, &parent_path_str)?;
            let wrapped_run = &mut parent_node.children[target_idx_in_parent + 1];
            if wrapped_run.element_type == WordElementType::Run {
                merge_run_properties(wrapped_run, &format_props);
            }
        }

        let quoted = quote_attr_value_if_needed(&bk_name)?;
        Ok(format!(
            "{}/bookmarkStart[@name={}]",
            parent_path_str, quoted
        ))
    } else if target.element_type == WordElementType::Paragraph {
        // Paragraph-level wrap: bookmarkStart/bookmarkEnd go inside the paragraph
        // bookmarkStart after pPr, bookmarkEnd at end of paragraph content
        let para = navigate_to_element_mut(dom, wrap_path)?;

        // Insert bookmarkStart after pPr (skip pPr if it exists)
        let insert_start_idx = if para.children.first().map(|c| c.element_type.clone())
            == Some(WordElementType::ParagraphProperties)
        {
            1
        } else {
            0
        };
        para.children
            .insert(insert_start_idx, bookmark_start.clone());

        // Insert bookmarkEnd at the end of paragraph content
        let para = navigate_to_element_mut(dom, wrap_path)?;
        // Find last content position (before any trailing sectPr-like elements)
        let insert_end_idx = para
            .children
            .iter()
            .rposition(|c| {
                c.element_type == WordElementType::Run
                    || c.element_type == WordElementType::BookmarkEnd
                    || c.element_type == WordElementType::Text
            })
            .map(|i| i + 1)
            .unwrap_or(para.children.len());
        para.children.insert(insert_end_idx, bookmark_end.clone());

        // Handle endPara: relocate bookmarkEnd to a downstream paragraph
        let cross_para_end_offset = parse_end_para(properties);
        if cross_para_end_offset > 0 {
            relocate_bookmark_end_cross_para(dom, wrap_path, cross_para_end_offset)?;
        }

        // Optionally apply highlight/color to paragraph runs
        let mut format_props = properties.clone();
        format_props.remove("name");
        format_props.remove("id");
        remove_bookmark_batch_hints(&mut format_props);
        format_props.remove("endPara");
        format_props.remove("endpara");
        if !format_props.is_empty() {
            let para = navigate_to_element_mut(dom, wrap_path)?;
            for child in &mut para.children {
                if child.element_type == WordElementType::Run {
                    merge_run_properties(child, &format_props);
                }
            }
        }

        let quoted = quote_attr_value_if_needed(&bk_name)?;
        Ok(format!("{}/bookmarkStart[@name={}]", wrap_path, quoted))
    } else if target.element_type == WordElementType::TableCell {
        // TableCell redirect: redirect to the cell's first paragraph
        let cell = navigate_to_element_mut(dom, wrap_path)?;

        // Find or create first paragraph in the cell
        let first_para_idx = cell
            .children
            .iter()
            .position(|c| c.element_type == WordElementType::Paragraph);

        if first_para_idx.is_none() {
            let para_id = crate::helpers::generate_para_id();
            let empty_para =
                WordNode::new(WordElementType::Paragraph).with_attribute("paraId", &para_id);
            cell.children.push(empty_para);
        }

        // Count paragraphs to build the path
        let cell = navigate_to_element(dom, wrap_path)?;
        let para_count = cell
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Paragraph)
            .count();

        let para_path = format!("{}/p[{}]", wrap_path, para_count);

        // Recursively call wrap on the paragraph inside the cell
        add_bookmark_wrap(dom, parent, properties, &para_path)
    } else {
        Err(HandlerError::InvalidArgument(format!(
            "bookmark --wrap only supports Run, Paragraph, or TableCell targets, found: {:?}",
            target.element_type
        )))
    }
}

/// Standard positional bookmark insertion (migrated from C# AddBookmark).
fn add_bookmark_positional(
    dom: &mut WordDom,
    parent: &str,
    position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    // Handle TableCell redirect
    let (actual_parent, actual_position) = handle_tablecell_redirect(dom, parent, position)?;

    // Validate name
    let bk_name = properties.get("name").cloned().unwrap_or_default();
    validate_bookmark_name(&bk_name)?;

    // Reject duplicate names
    if !skip_bookmark_duplicate_check(properties) {
        reject_duplicate_bookmark_name(dom, &bk_name)?;
    }

    // Resolve bookmark ID
    let bk_id = resolve_bookmark_id(dom, properties)?;

    let bookmark_start = WordNode::new(WordElementType::BookmarkStart)
        .with_attribute("id", &bk_id)
        .with_attribute("name", &bk_name);
    let bookmark_end = WordNode::new(WordElementType::BookmarkEnd).with_attribute("id", &bk_id);

    // Parse endPara
    let cross_para_end_offset = parse_end_para(properties);

    // Determine parent type and insertion strategy
    let parent_node = navigate_to_element(dom, &actual_parent)?;
    let parent_is_body = parent_node.element_type == WordElementType::Body;
    let parent_is_para = parent_node.element_type == WordElementType::Paragraph;

    let has_text = properties.contains_key("text");

    // Resolve insertion index
    let resolved_idx = resolve_insert_index(dom, &actual_parent, &actual_position)?;

    let mut wrapping_para = false;

    if has_text {
        let bk_text = properties.get("text").unwrap();

        if resolved_idx.is_some() && parent_is_para {
            // Has text + anchor in paragraph: insert [bookmarkStart, run, bookmarkEnd]
            let run = make_run_with_text(bk_text, properties);
            let para = navigate_to_element_mut(dom, &actual_parent)?;

            // Clamp index past pPr
            let insert_idx = clamp_past_ppr(&para.children, resolved_idx.unwrap_or(0));

            para.children.insert(insert_idx, bookmark_start.clone());
            para.children.insert(insert_idx + 1, run);
            para.children.insert(insert_idx + 2, bookmark_end.clone());
        } else if parent_is_body {
            // Has text + body parent: wrap in a new paragraph
            let run = make_run_with_text(bk_text, properties);
            let para_id = crate::helpers::generate_para_id();
            let wrap_para = WordNode::new(WordElementType::Paragraph)
                .with_attribute("paraId", &para_id)
                .with_children(vec![bookmark_start.clone(), run, bookmark_end.clone()]);

            let body = navigate_to_element_mut(dom, &actual_parent)?;
            // Respect sectPr invariant: insert before trailing sectPr
            let insert_idx = resolved_idx.unwrap_or_else(|| {
                // Append before sectPr
                body.children
                    .iter()
                    .rposition(|c| c.element_type == WordElementType::SectionProperties)
                    .unwrap_or(body.children.len())
            });
            body.children.insert(insert_idx, wrap_para);
            wrapping_para = true;
        } else if parent_is_para {
            // Has text + paragraph, no anchor: try wrapping existing runs
            let wrapped = try_wrap_existing_runs_with_bookmark(
                dom,
                &actual_parent,
                bk_text,
                &bookmark_start,
                &bookmark_end,
            )?;

            if !wrapped {
                // Fall back: positional insert of bookmarkStart + run + bookmarkEnd
                let run = make_run_with_text(bk_text, properties);
                let para = navigate_to_element_mut(dom, &actual_parent)?;
                let insert_idx =
                    clamp_past_ppr(&para.children, resolved_idx.unwrap_or(para.children.len()));
                para.children.insert(insert_idx, bookmark_start.clone());
                para.children.insert(insert_idx + 1, run);
                para.children.insert(insert_idx + 2, bookmark_end.clone());
            }
        } else {
            // Other parent types: positional insert
            let run = make_run_with_text(bk_text, properties);
            let container = navigate_to_element_mut(dom, &actual_parent)?;
            let insert_idx = resolved_idx.unwrap_or(container.children.len());
            container
                .children
                .insert(insert_idx, bookmark_start.clone());
            container.children.insert(insert_idx + 1, run);
            container
                .children
                .insert(insert_idx + 2, bookmark_end.clone());
        }
    } else if resolved_idx.is_some() && parent_is_para {
        // No text + anchor in paragraph: insert [bookmarkStart, bookmarkEnd]
        let para = navigate_to_element_mut(dom, &actual_parent)?;
        let insert_idx = clamp_past_ppr(&para.children, resolved_idx.unwrap());
        para.children.insert(insert_idx, bookmark_start.clone());
        para.children.insert(insert_idx + 1, bookmark_end.clone());
    } else {
        // No text + body/other: positional insert of bookmarkStart, then bookmarkEnd
        let container = navigate_to_element_mut(dom, &actual_parent)?;

        let insert_idx = if parent_is_body {
            // Respect sectPr invariant
            resolved_idx.unwrap_or_else(|| {
                container
                    .children
                    .iter()
                    .rposition(|c| c.element_type == WordElementType::SectionProperties)
                    .unwrap_or(container.children.len())
            })
        } else {
            resolved_idx.unwrap_or(container.children.len())
        };

        container
            .children
            .insert(insert_idx, bookmark_start.clone());
        let end_idx = if resolved_idx.is_some() {
            insert_idx + 1
        } else {
            container.children.len()
        };
        container.children.insert(end_idx, bookmark_end.clone());
    }

    // Handle endPara relocation
    if cross_para_end_offset > 0 && !wrapping_para {
        relocate_bookmark_end_cross_para(dom, &actual_parent, cross_para_end_offset)?;
    }

    // Build return path
    let quoted = quote_attr_value_if_needed(&bk_name)?;
    if wrapping_para {
        // Find the wrapping paragraph's index
        let body = navigate_to_element(dom, &actual_parent)?;
        let para_idx = body
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Paragraph)
            .count();
        Ok(format!(
            "{}/p[{}]/bookmarkStart[@name={}]",
            actual_parent, para_idx, quoted
        ))
    } else {
        Ok(format!("{}/bookmarkStart[@name={}]", actual_parent, quoted))
    }
}

// ─── Bookmark Helper Functions ──────────────────────────────────

/// Reject duplicate bookmark names across the entire document body.
fn reject_duplicate_bookmark_name(dom: &WordDom, name: &str) -> Result<(), HandlerError> {
    let body = dom
        .root
        .children
        .iter()
        .find(|c| c.element_type == WordElementType::Body);
    if let Some(body) = body {
        if find_bookmark_by_name(body, name) {
            return Err(HandlerError::InvalidArgument(format!(
                "bookmark name '{}' already exists; pick a unique name.",
                name
            )));
        }
    }
    Ok(())
}

/// Recursively search for a BookmarkStart with the given name.
fn find_bookmark_by_name(node: &WordNode, name: &str) -> bool {
    if node.element_type == WordElementType::BookmarkStart
        && node.attributes.get("name").map(|s| s.as_str()) == Some(name)
    {
        return true;
    }
    node.children.iter().any(|c| find_bookmark_by_name(c, name))
}

fn skip_bookmark_duplicate_check(properties: &HashMap<String, String>) -> bool {
    properties
        .get("__officecliBatchSkipDuplicateCheck")
        .is_some_and(|v| v.eq_ignore_ascii_case("true") || v == "1")
}

fn remove_bookmark_batch_hints(properties: &mut HashMap<String, String>) {
    properties.remove("__officecliBatchSkipDuplicateCheck");
}

/// Resolve bookmark ID: custom id property or auto-generated.
fn resolve_bookmark_id(
    dom: &WordDom,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    if let Some(custom_id) = properties.get("id") {
        let id_val: i32 = custom_id.parse().map_err(|_| {
            HandlerError::InvalidArgument(format!(
                "bookmark id must be a non-negative integer, got: {}",
                custom_id
            ))
        })?;
        if id_val < 0 {
            return Err(HandlerError::InvalidArgument(
                "bookmark id must be non-negative".to_string(),
            ));
        }
        Ok(custom_id.clone())
    } else {
        Ok(generate_bookmark_id(dom))
    }
}

/// Parse endPara property (cross-paragraph BookmarkEnd offset).
fn parse_end_para(properties: &HashMap<String, String>) -> usize {
    properties
        .get("endPara")
        .or_else(|| properties.get("endpara"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
}

/// Handle TableCell redirect: bookmarks under a cell redirect to the cell's first paragraph.
fn handle_tablecell_redirect(
    dom: &mut WordDom,
    parent: &str,
    position: InsertPosition,
) -> Result<(String, InsertPosition), HandlerError> {
    let parent_node = navigate_to_element(dom, parent)?;

    if parent_node.element_type == WordElementType::TableCell {
        // Find or create first paragraph in the cell
        let has_para = parent_node
            .children
            .iter()
            .any(|c| c.element_type == WordElementType::Paragraph);

        if !has_para {
            let para_id = crate::helpers::generate_para_id();
            let empty_para =
                WordNode::new(WordElementType::Paragraph).with_attribute("paraId", &para_id);
            let cell = navigate_to_element_mut(dom, parent)?;
            cell.children.push(empty_para);
        }

        // Count paragraphs to build the path (the newly created one is the last)
        let cell = navigate_to_element(dom, parent)?;
        let para_count = cell
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Paragraph)
            .count();

        let new_parent = format!("{}/p[{}]", parent, para_count);
        // Reset position to Append — original was relative to the cell
        Ok((new_parent, InsertPosition::Append))
    } else {
        Ok((parent.to_string(), position))
    }
}

/// Relocate BookmarkEnd to a downstream sibling paragraph when endPara > 0.
fn relocate_bookmark_end_cross_para(
    dom: &mut WordDom,
    start_path: &str,
    cross_para_offset: usize,
) -> Result<(), HandlerError> {
    // Find the enclosing paragraph of bookmarkStart
    let enclosing_para_path = if start_path.contains("/p[") {
        let segments = parse_path(start_path)?;
        if segments.len() >= 2 {
            let mut para_path = String::new();
            for seg in &segments {
                para_path.push('/');
                para_path.push_str(&seg.to_path_fragment());
                if seg.name == "p" {
                    break;
                }
            }
            para_path
        } else {
            start_path.to_string()
        }
    } else {
        start_path.to_string()
    };

    // Find the enclosing paragraph's parent (Body, TableCell, etc.)
    let para_parent_path = crate::navigation::parent_path(&enclosing_para_path)
        .ok_or_else(|| HandlerError::InvalidPath("paragraph has no parent".to_string()))?;

    // Get the bookmark ID from the BookmarkStart
    let enclosing_para = navigate_to_element(dom, &enclosing_para_path)?;
    let bk_id = enclosing_para
        .children
        .iter()
        .find(|c| c.element_type == WordElementType::BookmarkStart)
        .and_then(|bs| bs.attributes.get("id").cloned())
        .unwrap_or_default();

    // Find the paragraph's 1-based index among sibling paragraphs
    let para_parent = navigate_to_element(dom, &para_parent_path)?;
    let siblings: Vec<usize> = para_parent
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type == WordElementType::Paragraph)
        .map(|(i, _)| i)
        .collect();

    // Find the enclosing paragraph's real index in parent children
    // Match by content/attributes rather than pointer identity
    let start_real_idx = siblings
        .iter()
        .find(|&idx| {
            let para = &para_parent.children[*idx];
            // Match by paraId or by finding bookmarkStart with our bk_id inside
            para.children.iter().any(|c| {
                c.element_type == WordElementType::BookmarkStart
                    && c.attributes.get("id").map(|s| s.as_str()) == Some(&bk_id)
            })
        })
        .copied()
        .unwrap_or(0);

    let start_sibling_idx = siblings
        .iter()
        .position(|i| *i == start_real_idx)
        .unwrap_or(0);

    let target_sibling_idx = start_sibling_idx + cross_para_offset;
    if target_sibling_idx >= siblings.len() {
        return Ok(()); // silently ignore invalid offset, same as C#
    }

    let target_real_idx = siblings[target_sibling_idx];

    // Remove BookmarkEnd from its current location
    // Search the entire body for the bookmarkEnd with matching id
    let bookmark_end_node = remove_bookmark_end_by_id(dom, &bk_id)?;

    // Append BookmarkEnd to the target paragraph
    let para_parent = navigate_to_element_mut(dom, &para_parent_path)?;
    para_parent.children[target_real_idx]
        .children
        .push(bookmark_end_node);

    Ok(())
}

/// Find and remove a BookmarkEnd with the given ID from anywhere in the document body.
fn remove_bookmark_end_by_id(dom: &mut WordDom, bk_id: &str) -> Result<WordNode, HandlerError> {
    let body_idx = dom
        .root
        .children
        .iter()
        .position(|c| c.element_type == WordElementType::Body)
        .ok_or_else(|| HandlerError::OperationFailed("body element not found".to_string()))?;

    remove_bookmark_end_from_node(&mut dom.root.children[body_idx], bk_id)
}

fn remove_bookmark_end_from_node(
    node: &mut WordNode,
    bk_id: &str,
) -> Result<WordNode, HandlerError> {
    // Check direct children first
    let end_idx = node.children.iter().position(|c| {
        c.element_type == WordElementType::BookmarkEnd
            && c.attributes.get("id").map(|s| s.as_str()) == Some(bk_id)
    });

    if let Some(idx) = end_idx {
        return Ok(node.children.remove(idx));
    }

    // Recurse into children
    for child in &mut node.children {
        if let Ok(removed) = remove_bookmark_end_from_node(child, bk_id) {
            return Ok(removed);
        }
    }

    Err(HandlerError::PathNotFound(format!(
        "bookmarkEnd with id '{}' not found",
        bk_id
    )))
}

/// Create a Run node containing the given text, with optional run properties.
pub fn make_run_with_text(text: &str, properties: &HashMap<String, String>) -> WordNode {
    let run_props: HashMap<String, String> = properties
        .iter()
        .filter(|(k, _)| {
            !matches!(
                k.as_str(),
                "name" | "id" | "text" | "endPara" | "endpara" | "range_paths"
            )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let mut run = WordNode::new(WordElementType::Run);
    if let Some(rpr) = crate::helpers::build_run_properties(&run_props) {
        run.children.push(rpr);
    }

    let mut text_node = WordNode::new(WordElementType::Text).with_text(text);
    if text.starts_with(' ') || text.ends_with(' ') {
        text_node
            .attributes
            .insert("xml:space".to_string(), "preserve".to_string());
        text_node.preserve_space = true;
    }
    run.children.push(text_node);
    run
}

/// Clamp insertion index past pPr (ParagraphProperties) if present.
fn clamp_past_ppr(children: &[WordNode], idx: usize) -> usize {
    if children.first().map(|c| c.element_type.clone())
        == Some(WordElementType::ParagraphProperties)
    {
        if idx == 0 {
            1 // skip past pPr
        } else {
            idx
        }
    } else {
        idx
    }
}

/// Try to wrap existing runs matching the target text with bookmarkStart/bookmarkEnd.
/// Returns Ok(true) if wrapping succeeded, Ok(false) if no match found.
fn try_wrap_existing_runs_with_bookmark(
    dom: &mut WordDom,
    parent_path: &str,
    target_text: &str,
    bookmark_start: &WordNode,
    bookmark_end: &WordNode,
) -> Result<bool, HandlerError> {
    let para = navigate_to_element(dom, parent_path)?;
    if para.element_type != WordElementType::Paragraph {
        return Ok(false);
    }

    // Collect runs with their text and cumulative offsets
    let runs: Vec<&WordNode> = para
        .children
        .iter()
        .filter(|c| c.element_type == WordElementType::Run)
        .collect();

    if runs.is_empty() {
        return Ok(false);
    }

    // Concatenate all run texts
    let full_text: String = runs.iter().map(|r| r.paragraph_text()).collect();

    // Find target_text in full_text
    let match_byte_start = full_text.find(target_text);
    if match_byte_start.is_none() {
        return Ok(false);
    }
    let match_byte_start = match_byte_start.unwrap();

    // Convert byte offset to char offset for the match
    let match_char_start = full_text[..match_byte_start].chars().count();
    let match_char_end = match_char_start + target_text.chars().count();

    // Map char offsets to byte offsets within full_text
    let match_byte_end = full_text
        .char_indices()
        .nth(match_char_end)
        .map(|(i, _)| i)
        .unwrap_or(full_text.len());

    // Find which runs overlap the match range
    let mut cumulative_byte = 0;
    let mut first_run_idx = None;
    let mut last_run_idx = None;

    for (i, run) in runs.iter().enumerate() {
        let run_text = run.paragraph_text();
        let run_byte_start = cumulative_byte;
        let run_byte_end = cumulative_byte + run_text.len();

        // Check overlap
        let overlap_start = run_byte_start.max(match_byte_start);
        let overlap_end = run_byte_end.min(match_byte_end);

        if overlap_start < overlap_end {
            if first_run_idx.is_none() {
                first_run_idx = Some(i);
            }
            last_run_idx = Some(i);
        }

        cumulative_byte = run_byte_end;
    }

    if first_run_idx.is_none() {
        return Ok(false);
    }

    let first_idx = first_run_idx.unwrap();
    let last_idx = last_run_idx.unwrap();

    // Get the real child indices of these runs in the paragraph
    let para = navigate_to_element(dom, parent_path)?;
    let run_real_indices: Vec<usize> = para
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type == WordElementType::Run)
        .map(|(i, _)| i)
        .collect();

    let first_real = run_real_indices[first_idx];
    let last_real = run_real_indices[last_idx];

    // Insert bookmark markers around the matched run sequence
    let para = navigate_to_element_mut(dom, parent_path)?;

    // Insert bookmarkEnd after the last matching run
    para.children.insert(last_real + 1, bookmark_end.clone());

    // Insert bookmarkStart before the first matching run
    // (last_real index shifted by +1 because bookmarkEnd was just inserted)
    let adjusted_first = if first_real > last_real {
        first_real + 1
    } else {
        first_real
    };
    para.children.insert(adjusted_first, bookmark_start.clone());

    Ok(true)
}

/// Resolve insertion index from InsertPosition, properly handling AfterElement/BeforeElement.
fn resolve_insert_index(
    dom: &WordDom,
    parent_path: &str,
    position: &InsertPosition,
) -> Result<Option<usize>, HandlerError> {
    match position {
        InsertPosition::AtIndex(idx) => Ok(Some(*idx)),
        InsertPosition::Append => Ok(None),
        InsertPosition::AfterElement(anchor) | InsertPosition::BeforeElement(anchor) => {
            // Find anchor element's position in parent's children
            let parent_node = navigate_to_element(dom, parent_path)?;
            let anchor_node = navigate_to_element(dom, anchor)?;

            // Find the anchor's real child index
            let anchor_real_idx = parent_node
                .children
                .iter()
                .position(|c| {
                    c.element_type == anchor_node.element_type
                        && c.attributes == anchor_node.attributes
                        && c.text_content == anchor_node.text_content
                })
                .unwrap_or(parent_node.children.len() - 1);

            match position {
                InsertPosition::AfterElement(_) => Ok(Some(anchor_real_idx + 1)),
                InsertPosition::BeforeElement(_) => Ok(Some(anchor_real_idx)),
                _ => unreachable!(),
            }
        }
    }
}

// ─── Helper functions shared with range highlights ──────────────

fn collect_run_locations(
    node: &WordNode,
    current_path: &mut Vec<usize>,
    runs: &mut Vec<(Vec<usize>, String)>,
) {
    if node.element_type == WordElementType::Run {
        runs.push((current_path.clone(), node.paragraph_text()));
        return;
    }
    for (i, child) in node.children.iter().enumerate() {
        current_path.push(i);
        collect_run_locations(child, current_path, runs);
        current_path.pop();
    }
}

fn get_node_mut_by_path<'a>(mut node: &'a mut WordNode, path: &[usize]) -> &'a mut WordNode {
    for &idx in path {
        node = &mut node.children[idx];
    }
    node
}

#[derive(Debug, Clone)]
struct InsertionPoint {
    parent_path: Vec<usize>,
    index: usize,
}

fn find_insertion_point_for_text_offset(
    para_node: &WordNode,
    target: usize,
    end_boundary: bool,
) -> Option<InsertionPoint> {
    let mut cumulative = 0;
    let mut parent_path = Vec::new();
    find_insertion_point_recursive(
        para_node,
        &mut parent_path,
        target,
        end_boundary,
        &mut cumulative,
    )
}

fn find_insertion_point_recursive(
    node: &WordNode,
    parent_path: &mut Vec<usize>,
    target: usize,
    end_boundary: bool,
    cumulative: &mut usize,
) -> Option<InsertionPoint> {
    for (idx, child) in node.children.iter().enumerate() {
        if child.element_type == WordElementType::ParagraphProperties
            || child.element_type == WordElementType::RunProperties
            || child.element_type == WordElementType::BookmarkStart
            || child.element_type == WordElementType::BookmarkEnd
        {
            continue;
        }

        if child.element_type == WordElementType::Run {
            let len = child.paragraph_text().chars().count();
            if len == 0 {
                continue;
            }
            let start = *cumulative;
            let end = start + len;
            if end_boundary {
                if target > start && target <= end {
                    return Some(InsertionPoint {
                        parent_path: parent_path.clone(),
                        index: idx + 1,
                    });
                }
            } else if target < end {
                return Some(InsertionPoint {
                    parent_path: parent_path.clone(),
                    index: idx,
                });
            }
            *cumulative = end;
            continue;
        }

        parent_path.push(idx);
        if let Some(point) =
            find_insertion_point_recursive(child, parent_path, target, end_boundary, cumulative)
        {
            parent_path.pop();
            return Some(point);
        }
        parent_path.pop();
    }
    None
}

fn insert_at_text_point(
    para_node: &mut WordNode,
    point: Option<InsertionPoint>,
    marker: WordNode,
    append_when_missing: bool,
) {
    if let Some(point) = point {
        let parent = get_node_mut_by_path(para_node, &point.parent_path);
        let idx = point.index.min(parent.children.len());
        parent.children.insert(idx, marker);
        return;
    }

    if append_when_missing {
        para_node.children.push(marker);
        return;
    }

    let past_ppr = if para_node.children.first().map(|c| c.element_type.clone())
        == Some(WordElementType::ParagraphProperties)
    {
        1
    } else {
        0
    };
    para_node.children.insert(past_ppr, marker);
}

fn merge_run_properties(run: &mut WordNode, format_props: &HashMap<String, String>) {
    if let Some(new_rpr) = crate::helpers::build_run_properties(format_props) {
        if let Some(existing_rpr_idx) = run
            .children
            .iter()
            .position(|c| c.element_type == WordElementType::RunProperties)
        {
            let mut existing_rpr = run.children.remove(existing_rpr_idx);
            for new_child in new_rpr.children {
                existing_rpr
                    .children
                    .retain(|c| c.element_type != new_child.element_type);
                existing_rpr.children.push(new_child);
            }
            run.children.insert(existing_rpr_idx, existing_rpr);
        } else {
            run.children.insert(0, new_rpr);
        }
    }
}

// ─── Existing Add Functions (unchanged logic, updated signatures) ──

/// Add a paragraph to the body or after a specific paragraph.
fn add_paragraph(
    dom: &mut WordDom,
    parent: &str,
    position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let segments = parse_path(parent)?;
    let first_seg = segments.first().ok_or_else(|| {
        HandlerError::InvalidPath("parent path must start with /body".to_string())
    })?;

    if first_seg.name != "body" {
        return Err(HandlerError::InvalidPath(format!(
            "paragraphs can only be added under /body, got: {}",
            parent
        )));
    }

    let para_id = crate::helpers::generate_para_id();
    let mut para = WordNode::new(WordElementType::Paragraph).with_attribute("paraId", &para_id);

    // Add paragraph properties if provided
    if let Some(ppr) = crate::helpers::build_paragraph_properties(properties) {
        para.children.push(ppr);
    }

    // If "text" property is provided, add a run with that text
    if let Some(text) = properties.get("text") {
        let mut run = WordNode::new(WordElementType::Run);
        let run_props: HashMap<String, String> = properties
            .iter()
            .filter(|(k, _)| {
                k.as_str() != "text"
                    && k.as_str() != "style"
                    && k.as_str() != "alignment"
                    && !k.starts_with("indent")
                    && !k.starts_with("spacing")
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if let Some(rpr) = crate::helpers::build_run_properties(&run_props) {
            run.children.push(rpr);
        }
        let mut text_node = WordNode::new(WordElementType::Text).with_text(text);
        if text.starts_with(' ') || text.ends_with(' ') {
            text_node
                .attributes
                .insert("xml:space".to_string(), "preserve".to_string());
            text_node.preserve_space = true;
        }
        run.children.push(text_node);
        para.children.push(run);
    }

    // Get body and determine insertion index
    let body_idx = dom
        .root
        .children
        .iter()
        .position(|c| c.element_type == WordElementType::Body)
        .ok_or_else(|| HandlerError::OperationFailed("body element not found".to_string()))?;

    let content_items: Vec<usize> = dom.root.children[body_idx]
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type.is_body_child())
        .map(|(i, _)| i)
        .collect();

    let insert_idx = resolve_insert_index_simple(&position, content_items.len());

    match insert_idx {
        Some(idx) => {
            let real_idx = if idx < content_items.len() {
                content_items[idx]
            } else {
                dom.root.children[body_idx].children.len()
            };
            dom.root.children[body_idx].children.insert(real_idx, para);
        }
        None => {
            dom.root.children[body_idx].children.push(para);
        }
    }

    // Calculate the path of the new paragraph
    let mut new_para_idx = 0;
    for child in &dom.root.children[body_idx].children {
        if child.element_type == WordElementType::Paragraph {
            new_para_idx += 1;
        }
    }

    Ok(format!("/body/p[{}]", new_para_idx))
}

/// Add a run to a paragraph.
fn add_run(
    dom: &mut WordDom,
    parent: &str,
    position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    // First, check if path exists (immutable borrow to verify)
    let existing_run_count = {
        let para = navigate_to_element(dom, parent)?;
        para.runs().len()
    };

    // Build the run node
    let mut run = WordNode::new(WordElementType::Run);

    let run_props: HashMap<String, String> = properties
        .iter()
        .filter(|(k, _)| k.as_str() != "text")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if let Some(rpr) = crate::helpers::build_run_properties(&run_props) {
        run.children.push(rpr);
    }

    if let Some(text) = properties.get("text") {
        let mut text_node = WordNode::new(WordElementType::Text).with_text(text);
        if text.starts_with(' ') || text.ends_with(' ') {
            text_node
                .attributes
                .insert("xml:space".to_string(), "preserve".to_string());
            text_node.preserve_space = true;
        }
        run.children.push(text_node);
    }

    // Now get mutable access
    let para = navigate_to_element_mut(dom, parent)?;

    let existing_runs: Vec<usize> = para
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type == WordElementType::Run)
        .map(|(i, _)| i)
        .collect();

    let insert_idx = resolve_insert_index_simple(&position, existing_runs.len());

    match insert_idx {
        Some(idx) => {
            let real_idx = if idx < existing_runs.len() {
                existing_runs[idx]
            } else {
                para.children.len()
            };
            para.children.insert(real_idx, run);
        }
        None => {
            para.children.push(run);
        }
    }

    Ok(format!("{}/r[{}]", parent, existing_run_count + 1))
}

/// Add an empty table to the body.
fn add_table(
    dom: &mut WordDom,
    parent: &str,
    position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let segments = parse_path(parent)?;
    let first_seg = segments.first().ok_or_else(|| {
        HandlerError::InvalidPath("parent path must start with /body".to_string())
    })?;

    if first_seg.name != "body" {
        return Err(HandlerError::InvalidPath(
            "tables can only be added under /body".to_string(),
        ));
    }

    // Parse cols/rows properties (default 1 col x 1 row if not specified)
    let cols: usize = properties
        .get("cols")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let rows: usize = properties
        .get("rows")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    // Build table properties (supports style, width, border, etc.)
    let tbl_pr = if !properties.is_empty() {
        let mut pr = WordNode::new(WordElementType::TableProperties);
        let mut children = Vec::new();

        if let Some(style) = properties
            .get("style")
            .or_else(|| properties.get("tblStyle"))
        {
            children.push(
                WordNode::new(WordElementType::Unknown("tblStyle".to_string()))
                    .with_attribute("val", style.as_str()),
            );
        }
        if let Some(caption) = properties.get("title").or_else(|| properties.get("caption")) {
            children.push(
                WordNode::new(WordElementType::Unknown("tblCaption".to_string()))
                    .with_attribute("val", caption.as_str()),
            );
        }
        if let Some(width) = properties.get("width") {
            children.push(
                WordNode::new(WordElementType::Unknown("tblW".to_string()))
                    .with_attribute("w", width.as_str())
                    .with_attribute("type", "dxa"),
            );
        }
        if let Some(border) = properties.get("border") {
            children.push(crate::mutations::build_table_borders(border));
        }
        if let Some(shading) = properties.get("shading").or_else(|| properties.get("shd")) {
            children.push(crate::mutations::build_shd_node(shading));
        }
        if let Some(alignment) = properties.get("alignment").or_else(|| properties.get("jc")) {
            children.push(
                WordNode::new(WordElementType::Unknown("jc".to_string()))
                    .with_attribute("val", alignment.as_str()),
            );
        }

        pr.children = children;
        pr
    } else {
        WordNode::new(WordElementType::TableProperties)
    };

    // Build table grid
    let mut rows_nodes = Vec::new();
    for row in 0..rows {
        let mut cells = Vec::new();
        for col_idx in 0..cols {
            let text = properties.get(&format!("r{}c{}", row + 1, col_idx + 1)).cloned();
            let mut cell = WordNode::new(WordElementType::TableCell);
            if let Some(text) = text {
                let para = WordNode::new(WordElementType::Paragraph)
                    .with_children(vec![WordNode::new(WordElementType::Run).with_children(
                        vec![WordNode::new(WordElementType::Text).with_text(text)],
                    )]);
                cell.children.push(para);
            } else {
                cell.children
                    .push(WordNode::new(WordElementType::Paragraph));
            }
            cells.push(cell);
        }
        rows_nodes.push(WordNode::new(WordElementType::TableRow).with_children(cells));
    }

    let mut table = WordNode::new(WordElementType::Table);
    table.children.push(tbl_pr);
    // Add tblGrid if multiple columns
    if cols > 1 {
        let mut grid = WordNode::new(WordElementType::Unknown("tblGrid".to_string()));
        let widths: Vec<&str> = properties
            .get("colWidths")
            .or_else(|| properties.get("colWidth"))
            .map(|s| s.split(',').collect())
            .unwrap_or_default();
        for col_idx in 0..cols {
            let mut gc = WordNode::new(WordElementType::Unknown("gridCol".to_string()));
            if let Some(w) = widths.get(col_idx) {
                if let Ok(pt) = w.trim().parse::<f64>() {
                    let twips = (pt * 20.0) as i64;
                    gc = gc.with_attribute("w", &twips.to_string());
                }
            }
            grid.children.push(gc);
        }
        table.children.push(grid);
    }
    for row in rows_nodes {
        table.children.push(row);
    }

    let body_idx = dom
        .root
        .children
        .iter()
        .position(|c| c.element_type == WordElementType::Body)
        .ok_or_else(|| HandlerError::OperationFailed("body element not found".to_string()))?;

    let content_items: Vec<usize> = dom.root.children[body_idx]
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type.is_body_child())
        .map(|(i, _)| i)
        .collect();

    let insert_idx = resolve_insert_index_simple(&position, content_items.len());

    match insert_idx {
        Some(idx) => {
            let real_idx = if idx < content_items.len() {
                content_items[idx]
            } else {
                dom.root.children[body_idx].children.len()
            };
            dom.root.children[body_idx].children.insert(real_idx, table);
        }
        None => {
            dom.root.children[body_idx].children.push(table);
        }
    }

    let mut tbl_idx = 0;
    for child in &dom.root.children[body_idx].children {
        if child.element_type == WordElementType::Table {
            tbl_idx += 1;
        }
    }
    Ok(format!("/body/tbl[{}]", tbl_idx))
}

/// Add a row to a table.
fn add_table_row(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    _properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    // First check table structure (immutable)
    let col_count = {
        let table = navigate_to_element(dom, parent)?;
        table
            .children
            .iter()
            .find(|c| c.element_type == WordElementType::TableRow)
            .map(|row| {
                row.children
                    .iter()
                    .filter(|c| c.element_type == WordElementType::TableCell)
                    .count()
            })
            .unwrap_or(1)
    };

    let mut cells = Vec::new();
    for _ in 0..col_count {
        cells.push(
            WordNode::new(WordElementType::TableCell)
                .with_children(vec![WordNode::new(WordElementType::Paragraph)]),
        );
    }
    let row = WordNode::new(WordElementType::TableRow).with_children(cells);

    // Now get mutable access
    let table = navigate_to_element_mut(dom, parent)?;

    let existing_rows: Vec<usize> = table
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type == WordElementType::TableRow)
        .map(|(i, _)| i)
        .collect();

    let insert_idx = resolve_insert_index_simple(&InsertPosition::Append, existing_rows.len());

    match insert_idx {
        Some(idx) => {
            let real_idx = if idx < existing_rows.len() {
                existing_rows[idx]
            } else {
                table.children.len()
            };
            table.children.insert(real_idx, row);
        }
        None => {
            table.children.push(row);
        }
    }

    let row_count = table
        .children
        .iter()
        .filter(|c| c.element_type == WordElementType::TableRow)
        .count();
    Ok(format!("{}/tr[{}]", parent, row_count))
}

/// Add a cell to a table row.
fn add_table_cell(
    dom: &mut WordDom,
    parent: &str,
    position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let mut para = WordNode::new(WordElementType::Paragraph);
    if let Some(text) = properties.get("text") {
        let run = WordNode::new(WordElementType::Run)
            .with_children(vec![WordNode::new(WordElementType::Text).with_text(text)]);
        para.children.push(run);
    }

    let cell = WordNode::new(WordElementType::TableCell).with_children(vec![para]);

    let row = navigate_to_element_mut(dom, parent)?;

    let existing_cells: Vec<usize> = row
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.element_type == WordElementType::TableCell)
        .map(|(i, _)| i)
        .collect();

    let insert_idx = resolve_insert_index_simple(&position, existing_cells.len());

    match insert_idx {
        Some(idx) => {
            let real_idx = if idx < existing_cells.len() {
                existing_cells[idx]
            } else {
                row.children.len()
            };
            row.children.insert(real_idx, cell);
        }
        None => {
            row.children.push(cell);
        }
    }

    let cell_count = row
        .children
        .iter()
        .filter(|c| c.element_type == WordElementType::TableCell)
        .count();
    Ok(format!("{}/tc[{}]", parent, cell_count))
}

/// Simple insertion index resolution (for existing add functions).
fn resolve_insert_index_simple(position: &InsertPosition, _child_count: usize) -> Option<usize> {
    match position {
        InsertPosition::AtIndex(idx) => Some(*idx),
        InsertPosition::Append => None,
        InsertPosition::AfterElement(_) | InsertPosition::BeforeElement(_) => {
            // For existing functions, just append. Bookmark uses the proper resolver.
            None
        }
    }
}

// ─── New Element Types ─────────────────────────────────────────────────

/// Add a hyperlink to a paragraph or body. Properties: text, url/target, tooltip
fn add_hyperlink(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    // Hyperlinks must be added to a paragraph (run-level)
    let text = properties.get("text").cloned().unwrap_or_default();
    let url = properties
        .get("url")
        .or_else(|| properties.get("target"))
        .or_else(|| properties.get("link"))
        .ok_or_else(|| {
            HandlerError::InvalidArgument(
                "hyperlink requires 'url' or 'target' property".to_string(),
            )
        })?;
    let tooltip = properties.get("tooltip");

    let mut link = WordNode::new(WordElementType::Hyperlink);
    link.attributes.insert("r:id".to_string(), url.clone());
    if let Some(tt) = tooltip {
        link.attributes.insert("tooltip".to_string(), tt.clone());
    }

    // Build the run with text
    let run = make_run_with_text(
        &text,
        &properties
            .iter()
            .filter(|(k, _)| {
                matches!(
                    k.as_str(),
                    "bold" | "italic" | "underline" | "color" | "font" | "size"
                )
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    link.children.push(run);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(link);

    let link_count = parent_node
        .children
        .iter()
        .filter(|c| c.element_type == WordElementType::Hyperlink)
        .count();

    Ok(format!("{}/hyperlink[{}]", parent, link_count))
}

/// Add an image (drawing element) to a paragraph.
/// Properties: src/path, width, height, alt
fn add_image(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let _src = properties
        .get("src")
        .or_else(|| properties.get("path"))
        .or_else(|| properties.get("file"))
        .ok_or_else(|| {
            HandlerError::InvalidArgument(
                "image requires 'src', 'path', or 'file' property pointing to image file"
                    .to_string(),
            )
        })?;

    // NOTE: Full image embedding requires modifying the OOXML package parts,
    // which our DOM model doesn't fully support yet. We create a placeholder
    // drawing element that can be customized via raw-set.
    let width = properties
        .get("width")
        .cloned()
        .unwrap_or_else(|| "500000".to_string()); // EMU default
    let height = properties
        .get("height")
        .cloned()
        .unwrap_or_else(|| "500000".to_string());

    let mut drawing = WordNode::new(WordElementType::Drawing);
    let mut inline = WordNode::new(WordElementType::InlineImage);
    inline.attributes.insert("cx".to_string(), width);
    inline.attributes.insert("cy".to_string(), height);
    drawing.children.push(inline);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(drawing);

    Ok(format!(
        "{}/drawing[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Drawing)
            .count()
    ))
}

/// Add a field (fldSimple). Properties: type/fieldType, instruction, name, value, format
fn add_field(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let instruction = properties
        .get("instruction")
        .or_else(|| properties.get("instr"))
        .or_else(|| properties.get("type"))
        .cloned()
        .unwrap_or_else(|| "PAGE".to_string());

    let value = properties
        .get("value")
        .or_else(|| properties.get("result"))
        .cloned()
        .unwrap_or_default();

    let mut field = WordNode::new(WordElementType::FieldSimple);
    field
        .attributes
        .insert("instr".to_string(), format!(" {} ", instruction));
    field.text_content = Some(value);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(field);

    Ok(format!(
        "{}/fldSimple[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::FieldSimple)
            .count()
    ))
}

/// Add a break element (line or page break). Properties: type (line/page)
fn add_break(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let break_type = properties
        .get("type")
        .cloned()
        .unwrap_or_else(|| "line".to_string());

    let mut br = WordNode::new(WordElementType::Break);
    if break_type == "page" {
        br.attributes.insert("type".to_string(), "page".to_string());
    } else if break_type == "column" {
        br.attributes
            .insert("type".to_string(), "column".to_string());
    }

    // Breaks go inside a run
    let mut run = WordNode::new(WordElementType::Run);
    run.children.push(br);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(run);

    Ok(format!(
        "{}/r[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Run)
            .count()
    ))
}

/// Add a tab element (inside a run).
fn add_tab(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    _properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let tab = WordNode::new(WordElementType::Tab);
    let mut run = WordNode::new(WordElementType::Run);
    run.children.push(tab);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(run);

    Ok(format!(
        "{}/r[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Run)
            .count()
    ))
}

/// Add a section break. Properties: orientation, pageWidth, pageHeight, margins
fn add_section_break(
    dom: &mut WordDom,
    _parent: &str,
    _position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let mut sect_pr = WordNode::new(WordElementType::SectionProperties);

    // Page size (pgSz)
    let page_width = properties
        .get("pageWidth")
        .cloned()
        .unwrap_or_else(|| "12240".to_string());
    let page_height = properties
        .get("pageHeight")
        .cloned()
        .unwrap_or_else(|| "15840".to_string());

    let mut pg_sz = WordNode::new(WordElementType::Unknown("pgSz".to_string()))
        .with_attribute("w", page_width.as_str())
        .with_attribute("h", page_height.as_str());

    if let Some(orient) = properties.get("orientation") {
        pg_sz
            .attributes
            .insert("orient".to_string(), orient.clone());
    }
    sect_pr.children.push(pg_sz);

    // Page margins (pgMar)
    let m_left = properties
        .get("marginLeft")
        .cloned()
        .unwrap_or_else(|| "1440".to_string());
    let m_right = properties
        .get("marginRight")
        .cloned()
        .unwrap_or_else(|| "1440".to_string());
    let m_top = properties
        .get("marginTop")
        .cloned()
        .unwrap_or_else(|| "1440".to_string());
    let m_bottom = properties
        .get("marginBottom")
        .cloned()
        .unwrap_or_else(|| "1440".to_string());
    let pg_mar = WordNode::new(WordElementType::Unknown("pgMar".to_string()))
        .with_attribute("left", m_left.as_str())
        .with_attribute("right", m_right.as_str())
        .with_attribute("top", m_top.as_str())
        .with_attribute("bottom", m_bottom.as_str());
    sect_pr.children.push(pg_mar);

    // Add sectPr to body
    let body_idx = dom
        .root
        .children
        .iter()
        .position(|c| c.element_type == WordElementType::Body)
        .ok_or_else(|| HandlerError::OperationFailed("body not found".to_string()))?;
    dom.root.children[body_idx].children.push(sect_pr);

    Ok("/body/sectPr".to_string())
}

/// Add a footnote reference (inside a run).
fn add_footnote_reference(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    _properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let fn_ref = WordNode::new(WordElementType::FootnoteReference);
    let mut run = WordNode::new(WordElementType::Run);
    run.children.push(fn_ref);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(run);

    Ok(format!(
        "{}/r[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Run)
            .count()
    ))
}

/// Add an endnote reference (inside a run).
fn add_endnote_reference(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    _properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let en_ref = WordNode::new(WordElementType::EndnoteReference);
    let mut run = WordNode::new(WordElementType::Run);
    run.children.push(en_ref);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(run);

    Ok(format!(
        "{}/r[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Run)
            .count()
    ))
}

/// Add a block-level SDT (Structured Document Tag / Content Control).
/// Properties: alias/name, tag, lock, text, type
fn add_sdt_block(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let mut sdt = WordNode::new(WordElementType::Sdt);

    // Build sdtPr (properties)
    let mut sdt_pr = WordNode::new(WordElementType::SdtPr);
    if let Some(alias) = properties.get("alias").or_else(|| properties.get("name")) {
        sdt_pr.children.push(
            WordNode::new(WordElementType::Unknown("alias".to_string()))
                .with_attribute("val", alias.as_str()),
        );
    }
    if let Some(tag) = properties.get("tag") {
        sdt_pr.children.push(
            WordNode::new(WordElementType::Unknown("tag".to_string()))
                .with_attribute("val", tag.as_str()),
        );
    }
    if let Some(lock) = properties.get("lock") {
        sdt_pr.children.push(
            WordNode::new(WordElementType::Unknown("lock".to_string()))
                .with_attribute("val", lock.as_str()),
        );
    }
    // Default to plain text type
    if let Some(sdt_type) = properties.get("type") {
        sdt_pr
            .children
            .push(WordNode::new(WordElementType::Unknown(sdt_type.clone())));
    }
    sdt.children.push(sdt_pr);

    // Build sdtContent
    let mut content = WordNode::new(WordElementType::SdtContent);
    if let Some(text) = properties.get("text") {
        let para = WordNode::new(WordElementType::Paragraph)
            .with_children(vec![WordNode::new(WordElementType::Run).with_children(
                vec![WordNode::new(WordElementType::Text).with_text(text)],
            )]);
        content.children.push(para);
    } else {
        content
            .children
            .push(WordNode::new(WordElementType::Paragraph));
    }
    sdt.children.push(content);

    // SDTs can be added to body
    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(sdt);

    Ok(format!(
        "{}/sdt[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Sdt)
            .count()
    ))
}

/// Add an inline SDT (run-level content control).
fn add_sdt_run(
    dom: &mut WordDom,
    parent: &str,
    _position: InsertPosition,
    properties: &HashMap<String, String>,
) -> Result<String, HandlerError> {
    let mut sdt = WordNode::new(WordElementType::Sdt);

    let mut sdt_pr = WordNode::new(WordElementType::SdtPr);
    sdt_pr
        .children
        .push(WordNode::new(WordElementType::Unknown("text".to_string())));
    if let Some(tag) = properties.get("tag") {
        sdt_pr.children.push(
            WordNode::new(WordElementType::Unknown("tag".to_string()))
                .with_attribute("val", tag.as_str()),
        );
    }
    sdt.children.push(sdt_pr);

    let mut content = WordNode::new(WordElementType::SdtContent);
    if let Some(text) = properties.get("text") {
        let run = WordNode::new(WordElementType::Run)
            .with_children(vec![WordNode::new(WordElementType::Text).with_text(text)]);
        content.children.push(run);
    }
    sdt.children.push(content);

    let parent_node = navigate_to_element_mut(dom, parent)?;
    parent_node.children.push(sdt);

    Ok(format!(
        "{}/sdt[{}]",
        parent,
        parent_node
            .children
            .iter()
            .filter(|c| c.element_type == WordElementType::Sdt)
            .count()
    ))
}
