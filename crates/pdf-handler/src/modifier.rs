use crate::content_stream::{
    parse_page_content_stream, pick_fonts_for_text, FontSegment, PdfColor,
};
use handler_common::HandlerError;
use lopdf::Document as LopdfDocument;
use lopdf::ObjectId;

/// Build the replacement token sequence for the Tj line based on font segments.
/// If a single segment with the original font, just returns `[encoded_operand, "Tj"]`.
/// Otherwise emits `/<Font> <size> Tf <hex> Tj` per segment plus a final restore Tf.
fn build_segment_tokens(
    segments: &[FontSegment],
    orig_font: Option<&str>,
    orig_size: f32,
) -> Vec<String> {
    if segments.len() == 1 {
        let only = &segments[0];
        // If the segment already uses the original font, no Tf switching needed.
        if Some(only.font_name.as_str()) == orig_font {
            return vec![only.encoded_operand.clone(), "Tj".to_string()];
        }
    }

    let mut tokens = Vec::with_capacity(segments.len() * 5 + 3);
    for seg in segments {
        tokens.push(format!("/{}", seg.font_name));
        tokens.push(format_size(orig_size));
        tokens.push("Tf".to_string());
        tokens.push(seg.encoded_operand.clone());
        tokens.push("Tj".to_string());
    }

    if let Some(name) = orig_font {
        // Restore the original font so subsequent blocks in the same BT are unaffected.
        tokens.push(format!("/{}", name));
        tokens.push(format_size(orig_size));
        tokens.push("Tf".to_string());
    }

    tokens
}

fn format_size(size: f32) -> String {
    if size.fract().abs() < 1e-3 {
        format!("{}", size as i32)
    } else {
        format!("{}", size)
    }
}

/// Replace text at a specific path like /page[1]/text[3].
/// Only modifies the Tj/TJ line for that specific text block.
/// If the new text contains characters not in the target block's font,
/// it splits into multi-font segments using other fonts on the page.
pub fn replace_text_at_path(
    doc: &mut LopdfDocument,
    page_num: usize,
    text_index: usize, // 1-based
    new_text: &str,
    preferred_font: Option<&str>,
) -> Result<(), HandlerError> {
    let pages = doc.get_pages();
    let page_id = *pages
        .get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc
        .get_page_content(page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))?;

    let parsed = parse_page_content_stream(&content, page_id, doc).map_err(|e| {
        HandlerError::OperationFailed(format!("failed to parse content stream: {}", e))
    })?;

    let block_idx = text_index - 1;
    if block_idx >= parsed.text_blocks.len() {
        return Err(HandlerError::PathNotFound(format!(
            "text[{}] not found (page {} has {} text blocks)",
            text_index,
            page_num,
            parsed.text_blocks.len()
        )));
    }

    let target_block = &parsed.text_blocks[block_idx];
    let orig_font_owned = target_block.style.font_name.clone();
    let orig_font = orig_font_owned.as_deref();
    // Use the RAW Tf operand (without Tm scaling). The active Tm matrix from
    // the original content will still scale our re-emitted Tf; writing the
    // effective (already-scaled) size here would compound Tm twice and blow
    // up the rendered font size.
    let orig_size = target_block
        .style
        .raw_font_size
        .or(target_block.style.font_size)
        .unwrap_or(1.0);

    // Pick fonts: preferred_font wins; otherwise default to target block's font first.
    let pref = preferred_font.or(orig_font);
    let mut missing: Vec<char> = Vec::new();
    let segments = pick_fonts_for_text(doc, page_id, pref, new_text, &mut missing)?;

    if !missing.is_empty() {
        return Err(HandlerError::OperationFailed(format!(
            "characters not encodable in any page font: {}. Provide --prop fontFile=<path> or --prop font=<name> to override.",
            missing.iter().collect::<String>()
        )));
    }

    let mut modified_lines = parsed.lines.clone();
    let line = &modified_lines[target_block.text_line_index];
    let mut line_tokens = crate::content_stream::tokenize_pdf_line(line);

    let new_tokens = build_segment_tokens(&segments, orig_font, orig_size);

    if target_block.line_token_index < line_tokens.len() {
        // Replace the operand + operator (Tj/TJ) with our token sequence
        let op_idx = target_block.line_token_index;
        let consume_extra = matches!(
            line_tokens.get(op_idx + 1).map(|s| s.as_str()),
            Some("Tj") | Some("TJ")
        );
        let end = if consume_extra {
            op_idx + 2
        } else {
            op_idx + 1
        };
        line_tokens.splice(op_idx..end, new_tokens);
        modified_lines[target_block.text_line_index] = line_tokens.join(" ");
    } else {
        modified_lines[target_block.text_line_index] = new_tokens.join(" ");
    }

    let modified_content = modified_lines.join("\n");
    write_content_to_page(doc, page_id, modified_content.as_bytes())?;
    Ok(())
}

/// Replace text at a specific path with style modifications.
/// After changing the target block's style, restores the original style for subsequent blocks
/// in the same BT section so they don't inherit the changed style.
/// Also supports cross-font fallback via `preferred_font`.
#[allow(clippy::too_many_arguments)]
pub fn replace_text_with_style(
    doc: &mut LopdfDocument,
    page_num: usize,
    text_index: usize,
    new_text: Option<&str>,
    font_name: Option<&str>,
    font_size: Option<f32>,
    fill_color: Option<&PdfColor>,
    char_spacing: Option<f32>,
    word_spacing: Option<f32>,
    bg_color: Option<&PdfColor>,
) -> Result<(), HandlerError> {
    let pages = doc.get_pages();
    let page_id = *pages
        .get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc
        .get_page_content(page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))?;

    let parsed = parse_page_content_stream(&content, page_id, doc).map_err(|e| {
        HandlerError::OperationFailed(format!("failed to parse content stream: {}", e))
    })?;

    let block_idx = text_index - 1;
    if block_idx >= parsed.text_blocks.len() {
        return Err(HandlerError::PathNotFound(format!(
            "text[{}] not found",
            text_index
        )));
    }

    let target_block = parsed.text_blocks[block_idx].clone();
    let mut modified_lines = parsed.lines.clone();

    // Build style insertion lines (font/size/color/spacing changes)
    let mut style_lines = Vec::new();
    let effective_font = font_name
        .or(target_block.style.font_name.as_deref())
        .unwrap_or("F1")
        .to_string();
    // For Tf operands we want the RAW size, not the Tm-multiplied effective size.
    // User-supplied --prop size=X keeps the historical "raw operand" semantics.
    let effective_size = font_size
        .or(target_block.style.raw_font_size)
        .or(target_block.style.font_size)
        .unwrap_or(12.0);

    if font_name.is_some() || font_size.is_some() {
        style_lines.push(format!(
            "/{} {} Tf",
            effective_font,
            format_size(effective_size)
        ));
    }

    if let Some(color) = fill_color {
        match color {
            PdfColor::Gray(g) => style_lines.push(format!("{} g {} G", g, g)),
            PdfColor::Rgb(r, g, b) => {
                style_lines.push(format!("{} {} {} rg {} {} {} RG", r, g, b, r, g, b))
            }
            PdfColor::Cmyk(c, m, y, k) => style_lines.push(format!(
                "{} {} {} {} k {} {} {} {} K",
                c, m, y, k, c, m, y, k
            )),
        }
    }

    if let Some(cs) = char_spacing {
        style_lines.push(format!("{} Tc", cs));
    }
    if let Some(ws) = word_spacing {
        style_lines.push(format!("{} Tw", ws));
    }

    // Build restore lines to reset the original style for subsequent blocks
    let mut restore_lines = Vec::new();
    let has_subsequent = parsed.text_blocks[block_idx + 1..].iter().any(|b| {
        b.bt_start_line == target_block.bt_start_line && b.bt_end_line == target_block.bt_end_line
    });

    if has_subsequent {
        if font_name.is_some() || font_size.is_some() {
            let orig_font = target_block.style.font_name.as_deref().unwrap_or("F1");
            let orig_size = target_block
                .style
                .raw_font_size
                .or(target_block.style.font_size)
                .unwrap_or(12.0);
            restore_lines.push(format!("/{} {} Tf", orig_font, format_size(orig_size)));
        }
        if let Some(_color) = fill_color {
            if let Some(ref orig_color) = target_block.style.fill_color {
                match orig_color {
                    PdfColor::Gray(g) => restore_lines.push(format!("{} g {} G", g, g)),
                    PdfColor::Rgb(r, g, b) => {
                        restore_lines.push(format!("{} {} {} rg {} {} {} RG", r, g, b, r, g, b))
                    }
                    PdfColor::Cmyk(c, m, y, k) => restore_lines.push(format!(
                        "{} {} {} {} k {} {} {} {} K",
                        c, m, y, k, c, m, y, k
                    )),
                }
            }
        }
        if char_spacing.is_some() {
            restore_lines.push(format!("{} Tc", target_block.style.char_spacing));
        }
        if word_spacing.is_some() {
            restore_lines.push(format!("{} Tw", target_block.style.word_spacing));
        }
    }

    // Build the text Tj line — supports multi-font segments
    let effective_text = new_text
        .map(|s| s.to_string())
        .unwrap_or_else(|| target_block.text.clone());

    let mut missing: Vec<char> = Vec::new();
    let segments = pick_fonts_for_text(
        doc,
        page_id,
        Some(&effective_font),
        &effective_text,
        &mut missing,
    )?;
    if !missing.is_empty() {
        return Err(HandlerError::OperationFailed(format!(
            "characters not encodable in any page font: {}. Provide --prop fontFile=<path> or --prop font=<name> to override.",
            missing.iter().collect::<String>()
        )));
    }

    let new_tokens = build_segment_tokens(&segments, Some(&effective_font), effective_size);

    let mut final_tokens = Vec::new();
    final_tokens.extend(style_lines);
    final_tokens.extend(new_tokens);
    final_tokens.extend(restore_lines);

    let line = &modified_lines[target_block.text_line_index];
    let mut line_tokens = crate::content_stream::tokenize_pdf_line(line);

    if target_block.line_token_index < line_tokens.len() {
        let op_idx = target_block.line_token_index;
        let consume_extra = matches!(
            line_tokens.get(op_idx + 1).map(|s| s.as_str()),
            Some("Tj") | Some("TJ")
        );
        let end = if consume_extra {
            op_idx + 2
        } else {
            op_idx + 1
        };
        line_tokens.splice(op_idx..end, final_tokens);
        modified_lines[target_block.text_line_index] = line_tokens.join(" ");
    } else {
        modified_lines[target_block.text_line_index] = final_tokens.join(" ");
    }

    // Insert background-color rectangle BEFORE the BT block (outside text object)
    if let Some(bg) = bg_color {
        let bb = &target_block.user_bbox;
        let (r, g, b_val) = match bg {
            PdfColor::Gray(g) => (*g, *g, *g),
            PdfColor::Rgb(r, g, b) => (*r, *g, *b),
            PdfColor::Cmyk(c, m, y, k) => {
                // Approximate CMYK->RGB for bg rendering
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };
        let bg_lines = vec![
            "q".to_string(),
            format!("{} {} {} rg", r, g, b_val),
            format!("{} {} {} {} re", bb.x, bb.y, bb.width, bb.height),
            "f".to_string(),
            "Q".to_string(),
        ];

        let insert_pos = target_block.bt_start_line;
        let mut new_lines = modified_lines[..insert_pos].to_vec();
        for line in &bg_lines {
            new_lines.push(line.clone());
        }
        new_lines.extend_from_slice(&modified_lines[insert_pos..]);
        modified_lines = new_lines;
    }

    let modified_content = modified_lines.join("\n");
    write_content_to_page(doc, page_id, modified_content.as_bytes())?;
    Ok(())
}

fn write_content_to_page(
    doc: &mut LopdfDocument,
    page_id: ObjectId,
    content: &[u8],
) -> Result<(), HandlerError> {
    let content_ids = doc.get_page_contents(page_id);
    if content_ids.is_empty() {
        return Err(HandlerError::OperationFailed(
            "page has no content streams".to_string(),
        ));
    }

    // Write modified content to the first stream
    let first_id = content_ids[0];
    if let Ok(lopdf::Object::Stream(stream)) = doc.get_object_mut(first_id) {
        // Remove any existing compression filter first — the content bytes
        // we receive are already decompressed (lopdf transparently inflates
        // FlateDecode streams in get_page_content()). Setting raw bytes
        // while /Filter /FlateDecode remains in the dict causes blank pages
        // on the next load because lopdf tries to deflate raw data.
        stream.dict.remove(b"Filter");
        stream.content = content.to_vec();
        // Re-compress with FlateDecode so the saved PDF stays compact
        // and the /Filter + /Length are consistent.
        let _ = stream.compress();
        // lopdf's compress() may leave a stale /Length when content shrank,
        // which corrupts subsequent loads (the parser reads past the real
        // end of the stream). Always rewrite Length to match actual bytes.
        let current_len = stream.content.len();
        stream
            .dict
            .set("Length", lopdf::Object::Integer(current_len as i64));
    }

    // Clear subsequent streams to prevent duplicate content rendering and viewer corruption
    for &other_id in &content_ids[1..] {
        if let Ok(lopdf::Object::Stream(stream)) = doc.get_object_mut(other_id) {
            stream.dict.remove(b"Filter");
            stream.content = Vec::new();
            stream.dict.set("Length", lopdf::Object::Integer(0));
        }
    }

    Ok(())
}

/// Legacy: replace all Tj strings on a page with the same text.
pub fn replace_text_on_page(
    doc: &mut LopdfDocument,
    page_num: usize,
    new_text: &str,
) -> Result<(), HandlerError> {
    let pages = doc.get_pages();
    let page_id = pages
        .get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc
        .get_page_content(*page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))?;

    let content_str = String::from_utf8_lossy(&content);
    let modified = blanket_replace_strings(doc, *page_id, &content_str, new_text)?;

    write_content_to_page(doc, *page_id, modified.as_bytes())?;
    Ok(())
}

fn blanket_replace_strings(
    doc: &LopdfDocument,
    page_id: ObjectId,
    stream: &str,
    new_text: &str,
) -> Result<String, HandlerError> {
    let mut result = String::new();
    let mut in_text_object = false;
    let mut active_font: Option<String> = None;
    let mut active_size: f32 = 1.0;

    for line in stream.lines() {
        let trimmed = line.trim();
        if trimmed == "BT" {
            in_text_object = true;
            result.push_str(line);
            result.push('\n');
            continue;
        }
        if trimmed == "ET" {
            in_text_object = false;
            result.push_str(line);
            result.push('\n');
            continue;
        }
        if !in_text_object {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if trimmed.ends_with(" Tf") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let font_name = parts[parts.len() - 3].trim_start_matches('/');
                active_font = Some(font_name.to_string());
                if let Ok(sz) = parts[parts.len() - 2].parse::<f32>() {
                    active_size = sz;
                }
            }
        }

        if trimmed.ends_with(" Tj") {
            let string_part = trimmed.trim_end_matches(" Tj").trim();
            if (string_part.starts_with('(') && string_part.ends_with(')'))
                || (string_part.starts_with('<') && string_part.ends_with('>'))
            {
                let mut missing = Vec::new();
                let segments = pick_fonts_for_text(
                    doc,
                    page_id,
                    active_font.as_deref(),
                    new_text,
                    &mut missing,
                )?;
                if !missing.is_empty() {
                    return Err(HandlerError::OperationFailed(format!(
                        "characters not encodable in any page font: {}",
                        missing.iter().collect::<String>()
                    )));
                }
                let tokens = build_segment_tokens(&segments, active_font.as_deref(), active_size);
                result.push_str(&tokens.join(" "));
                result.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
}

/// Replace entire page content with new content bytes.
pub fn replace_page_content(
    doc: &mut LopdfDocument,
    page_id: ObjectId,
    new_content: &[u8],
) -> Result<(), HandlerError> {
    write_content_to_page(doc, page_id, new_content)?;
    Ok(())
}

/// Apply find/replace across all text in a page content stream.
///
/// Walks the content stream line by line. For each `... Tj` line, decodes the
/// text operand (literal `(...)` or hex `<...>`), applies the find/replace
/// against the decoded string, and re-encodes using the original string form.
/// TJ arrays are handled element-by-element in the same way.
///
/// Returns the total number of replacements applied. Pages without matching
/// text return zero.
pub fn apply_find_replace_on_page(
    doc: &mut LopdfDocument,
    page_num: usize,
    find: &str,
    replace: &str,
    opts: &handler_common::FindReplaceOptions,
) -> Result<usize, HandlerError> {
    let pages = doc.get_pages();
    let page_id = pages
        .get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc
        .get_page_content(*page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("page content read: {}", e)))?;
    let content_str = String::from_utf8_lossy(&content);

    let mut total = 0usize;
    let mut out = String::with_capacity(content_str.len());
    for line in content_str.lines() {
        let trimmed = line.trim_end();
        if trimmed.ends_with(" Tj") {
            let (rewritten, count) = rewrite_tj_line(trimmed, find, replace, opts);
            out.push_str(&rewritten);
            out.push('\n');
            total += count;
        } else if trimmed.ends_with(" TJ") {
            let (rewritten, count) = rewrite_tj_array_line(trimmed, find, replace, opts);
            out.push_str(&rewritten);
            out.push('\n');
            total += count;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    if total > 0 {
        write_content_to_page(doc, *page_id, out.as_bytes())?;
    }
    Ok(total)
}

/// Apply find/replace to all pages in the document. Returns the total count.
pub fn apply_find_replace_all_pages(
    doc: &mut LopdfDocument,
    find: &str,
    replace: &str,
    opts: &handler_common::FindReplaceOptions,
) -> Result<usize, HandlerError> {
    let page_count = doc
        .get_pages()
        .keys()
        .map(|n| *n as usize)
        .max()
        .unwrap_or(0);
    let mut total = 0usize;
    for page in 1..=page_count {
        total += apply_find_replace_on_page(doc, page, find, replace, opts).unwrap_or(0);
    }
    Ok(total)
}

/// Rewrite a single `operand Tj` line. Returns (new line, replacement count).
fn rewrite_tj_line(
    line: &str,
    find: &str,
    replace: &str,
    opts: &handler_common::FindReplaceOptions,
) -> (String, usize) {
    use handler_common::find_replace::replace_in_string;

    // Strip trailing " Tj"
    let body = &line[..line.len() - 3].trim_end();
    let leading_ws_len = line.len() - line.trim_start().len();
    let leading_ws = &line[..leading_ws_len];

    let operand = body.trim();
    if let Some(decoded) = decode_pdf_string_operand(operand) {
        let (new_text, count) = replace_in_string(&decoded, find, replace, opts);
        if count > 0 {
            let new_operand = encode_pdf_string_operand_preserve_form(operand, &new_text);
            return (format!("{}{} Tj", leading_ws, new_operand), count);
        }
    }
    (line.to_string(), 0)
}

/// Rewrite a `[(...) ... (-N) ...] TJ` line element by element.
fn rewrite_tj_array_line(
    line: &str,
    find: &str,
    replace: &str,
    opts: &handler_common::FindReplaceOptions,
) -> (String, usize) {
    use handler_common::find_replace::replace_in_string;

    let leading_ws_len = line.len() - line.trim_start().len();
    let leading_ws = &line[..leading_ws_len];
    let trimmed = line.trim();

    // Must end with " TJ" and start with '['
    if !trimmed.ends_with(" TJ") || !trimmed.starts_with('[') {
        return (line.to_string(), 0);
    }
    let array_body = &trimmed[1..trimmed.len() - 3].trim_end();

    let mut total = 0usize;
    let mut rebuilt = String::with_capacity(array_body.len());
    rebuilt.push('[');

    let bytes = array_body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        // Skip whitespace
        if c.is_whitespace() {
            rebuilt.push(c);
            i += 1;
            continue;
        }
        // Numeric element (kerning): copy verbatim
        if c == '-' || c.is_ascii_digit() || c == '+' {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            rebuilt.push_str(&array_body[start..i]);
            continue;
        }
        // String element: literal (...) or hex <...>
        if c == '(' || c == '<' {
            let start = i;
            let (end_idx, element) = match c {
                '(' => {
                    let mut depth = 1;
                    let mut j = i + 1;
                    while j < bytes.len() && depth > 0 {
                        let bc = bytes[j] as char;
                        if bc == '(' && (j == 0 || bytes[j - 1] as char != '\\') {
                            depth += 1;
                        } else if bc == ')' && (j == 0 || bytes[j - 1] as char != '\\') {
                            depth -= 1;
                        }
                        j += 1;
                    }
                    (j, &array_body[start..j])
                }
                '<' => {
                    let mut j = i + 1;
                    while j < bytes.len() && bytes[j] as char != '>' {
                        j += 1;
                    }
                    (j + 1, &array_body[start..j + 1])
                }
                _ => unreachable!(),
            };
            if let Some(decoded) = decode_pdf_string_operand(element) {
                let (new_text, count) = replace_in_string(&decoded, find, replace, opts);
                total += count;
                if count > 0 {
                    rebuilt.push_str(&encode_pdf_string_operand_preserve_form(element, &new_text));
                } else {
                    rebuilt.push_str(element);
                }
            } else {
                rebuilt.push_str(element);
            }
            i = end_idx;
            continue;
        }
        // Anything else: copy verbatim
        rebuilt.push(c);
        i += 1;
    }
    rebuilt.push(']');

    if total > 0 {
        (format!("{}{} TJ", leading_ws, rebuilt), total)
    } else {
        (line.to_string(), 0)
    }
}

/// Decode a single Tj operand string: `(...)` or `<...>` form.
/// Returns None if the operand form is unrecognized.
fn decode_pdf_string_operand(operand: &str) -> Option<String> {
    let s = operand.trim();
    if s.starts_with('(') && s.ends_with(')') {
        Some(decode_literal_pdf_string(&s[1..s.len() - 1]))
    } else if s.starts_with('<') && s.ends_with('>') {
        let hex = &s[1..s.len() - 1];
        Some(decode_hex_pdf_string(hex))
    } else {
        None
    }
}

/// Re-encode text using the same form as `original_operand`.
fn encode_pdf_string_operand_preserve_form(original_operand: &str, text: &str) -> String {
    let s = original_operand.trim();
    if s.starts_with('<') {
        encode_hex_pdf_string(text)
    } else {
        crate::content_stream::encode_pdf_string(text)
    }
}

fn decode_literal_pdf_string(body: &str) -> String {
    let bytes = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1] as char;
            match next {
                '(' => {
                    out.push('(');
                    i += 2;
                }
                ')' => {
                    out.push(')');
                    i += 2;
                }
                '\\' => {
                    out.push('\\');
                    i += 2;
                }
                'n' => {
                    out.push('\n');
                    i += 2;
                }
                'r' => {
                    out.push('\r');
                    i += 2;
                }
                't' => {
                    out.push('\t');
                    i += 2;
                }
                d if d.is_ascii_digit() => {
                    // Up to 3 octal digits
                    let mut j = i + 1;
                    let mut val = 0u32;
                    while j < bytes.len() && (bytes[j] as char).is_ascii_digit() && j - i < 4 {
                        val = val * 8 + (bytes[j] - b'0') as u32;
                        j += 1;
                    }
                    if let Some(ch) = std::char::from_u32(val & 0xFF) {
                        out.push(ch);
                    }
                    i = j;
                }
                _ => {
                    out.push(next);
                    i += 2;
                }
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn decode_hex_pdf_string(hex: &str) -> String {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    let mut bytes = Vec::with_capacity(cleaned.len() / 2 + 1);
    let mut chars = cleaned.chars();
    while let Some(h) = chars.next() {
        if let Some(l) = chars.next() {
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h, l), 16) {
                bytes.push(byte);
            }
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn encode_hex_pdf_string(text: &str) -> String {
    let mut hex = String::with_capacity(text.len() * 2 + 2);
    hex.push('<');
    for byte in text.bytes() {
        hex.push_str(&format!("{:02X}", byte));
    }
    hex.push('>');
    hex
}

/// Delete a page from the PDF document.
pub fn delete_page(doc: &mut LopdfDocument, page_num: usize) -> Result<(), HandlerError> {
    doc.delete_pages(&[page_num as u32]);
    Ok(())
}

/// Append a blank page to the document. The new page inherits the page size
/// of the last existing page (or letter 612×792 if the document is empty).
/// Returns the 1-based number of the new page.
pub fn add_blank_page(doc: &mut LopdfDocument) -> Result<usize, HandlerError> {
    let (w, h) = last_page_size(doc).unwrap_or((612.0, 792.0));
    add_page_with_size(doc, w, h)
}

/// Add a page with explicit dimensions (in points). Returns the new page number.
pub fn add_page_with_size(
    doc: &mut LopdfDocument,
    width: f32,
    height: f32,
) -> Result<usize, HandlerError> {
    use lopdf::{Dictionary, Object};

    // Build an empty content stream so the page renders cleanly.
    let content_stream = lopdf::Stream::new(Dictionary::new(), Vec::new());
    let content_id = doc.add_object(content_stream);

    // Build the page dictionary: empty Resources, MediaBox, reference to content.
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name(b"Page".to_vec()));
    page_dict.set(
        "MediaBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(width as i64),
            Object::Integer(height as i64),
        ]),
    );
    page_dict.set("Contents", Object::Reference(content_id));

    // Clone Resources from the last page if present so fonts/procsets carry over.
    if let Some(last_res) = last_page_resources(doc) {
        page_dict.set("Resources", last_res);
    } else {
        let mut res = Dictionary::new();
        res.set("ProcSet", Object::Array(vec![]));
        page_dict.set("Resources", Object::Dictionary(res));
    }

    let page_id = doc.add_object(Object::Dictionary(page_dict));

    // Hook the new page into /Kids of the Pages tree.
    if let Ok(pages_id) = doc
        .catalog()
        .and_then(|d| d.get(b"Pages"))
        .and_then(Object::as_reference)
    {
        if let Ok(pages_obj) = doc.get_object_mut(pages_id) {
            if let Ok(pages_dict) = pages_obj.as_dict_mut() {
                if let Ok(Object::Array(kids)) = pages_dict.get_mut(b"Kids") {
                    let new_count = kids.len() as i64 + 1;
                    kids.push(Object::Reference(page_id));
                    pages_dict.set("Count", Object::Integer(new_count));
                }
            }
        }
    }

    Ok(doc.get_pages().len())
}

/// Add text to a page's content stream as a single BT/ET block at `(x, y)`
/// using font `font_name` (PDF font resource name like `/F1`) and the given
/// point size. If `font_name` is missing, the first page font is used.
pub fn add_text_block(
    doc: &mut LopdfDocument,
    page_num: usize,
    text: &str,
    x: f32,
    y: f32,
    font_name: Option<&str>,
    size: f32,
) -> Result<(), HandlerError> {
    let pages = doc.get_pages();
    let page_id = pages
        .get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc
        .get_page_content(*page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("page content read: {}", e)))?;
    let content_str = String::from_utf8_lossy(&content);

    // Resolve a font name: caller > first page font > fallback /F1.
    let font = font_name
        .map(|s| s.trim_start_matches('/').to_string())
        .or_else(|| first_page_font_name(doc, *page_id))
        .unwrap_or_else(|| "F1".to_string());

    // Build the new BT/ET block. We escape literal-string special chars here so
    // consumers can use parentheses and backslashes safely.
    let escaped = escape_pdf_literal(text);
    let block = format!(
        "\nBT\n/{font} {size} Tf\n{x:.2} {y:.2} Td\n({escaped}) Tj\nET\n",
        font = font,
        size = size,
        x = x,
        y = y,
        escaped = escaped
    );

    let mut new_content = String::with_capacity(content_str.len() + block.len());
    new_content.push_str(&content_str);
    new_content.push_str(&block);

    write_content_to_page(doc, *page_id, new_content.as_bytes())?;
    Ok(())
}

/// Reorder pages: move the page at `from` to position `to`. 1-based indices,
/// `to` may be in [1, page_count + 1]. After the move, all pages are
/// re-numbered to reflect the new order.
pub fn move_page(doc: &mut LopdfDocument, from: usize, to: usize) -> Result<usize, HandlerError> {
    use lopdf::Object;
    let total = doc.get_pages().len();
    if from == 0 || from > total {
        return Err(HandlerError::InvalidPath(format!(
            "page {} out of range (1..={})",
            from, total
        )));
    }
    if to == 0 || to > total + 1 {
        return Err(HandlerError::InvalidArgument(format!(
            "target position {} out of range (1..={})",
            to,
            total + 1
        )));
    }
    if from == to || from + 1 == to {
        return Ok(to.min(total));
    }

    // Operate on the /Kids array of the catalog's Pages node.
    let pages_id = doc
        .catalog()
        .and_then(|d| d.get(b"Pages"))
        .and_then(Object::as_reference)
        .or(Err(HandlerError::OperationFailed(
            "could not locate catalog Pages".to_string(),
        )))?;

    if let Ok(pages_obj) = doc.get_object_mut(pages_id) {
        if let Ok(pages_dict) = pages_obj.as_dict_mut() {
            if let Ok(Object::Array(kids)) = pages_dict.get_mut(b"Kids") {
                let item_idx = from - 1;
                let removed = kids.remove(item_idx);
                // If to > from, removal shifted indices down by one.
                let insert_at = if to > from { to - 2 } else { to - 1 };
                let insert_at = insert_at.min(kids.len());
                kids.insert(insert_at, removed);
                return Ok(to.min(total));
            }
        }
    }
    Err(HandlerError::OperationFailed(
        "could not reorder /Kids array".to_string(),
    ))
}

/// Copy a page from `source_doc` into `target_doc`, appending at the end.
/// Returns the page number of the new page in the target.
pub fn copy_page_from(
    target_doc: &mut LopdfDocument,
    source_doc: &LopdfDocument,
    source_page_num: usize,
) -> Result<usize, HandlerError> {
    use lopdf::{Dictionary, Object};

    let src_pages = source_doc.get_pages();
    let src_page_id = src_pages
        .get(&(source_page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("source page {}", source_page_num)))?;

    let content = source_doc
        .get_page_content(*src_page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("source page content: {}", e)))?;

    let (w, h) = page_size(source_doc, *src_page_id).unwrap_or((612.0, 792.0));

    // Build a new content stream object in the target.
    let content_stream = lopdf::Stream::new(Dictionary::new(), content);
    let content_id = target_doc.add_object(content_stream);

    // Page dictionary — copy MediaBox and Resources from source.
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name(b"Page".to_vec()));
    page_dict.set(
        "MediaBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(w as i64),
            Object::Integer(h as i64),
        ]),
    );
    page_dict.set("Contents", Object::Reference(content_id));

    // Clone Resources dictionary from the source page if available.
    if let Ok(res_dict) = source_doc
        .get_page_resources(*src_page_id)
        .map(|(dict, _)| dict.cloned().unwrap_or_default())
    {
        page_dict.set("Resources", Object::Dictionary(res_dict));
    } else {
        let mut res = Dictionary::new();
        res.set("ProcSet", Object::Array(vec![]));
        page_dict.set("Resources", Object::Dictionary(res));
    }

    let page_id = target_doc.add_object(Object::Dictionary(page_dict));

    // Append to /Kids of the target's Pages tree.
    if let Ok(pages_id) = target_doc
        .catalog()
        .and_then(|d| d.get(b"Pages"))
        .and_then(Object::as_reference)
    {
        if let Ok(pages_obj) = target_doc.get_object_mut(pages_id) {
            if let Ok(pages_dict) = pages_obj.as_dict_mut() {
                if let Ok(Object::Array(kids)) = pages_dict.get_mut(b"Kids") {
                    let new_count = kids.len() as i64 + 1;
                    kids.push(Object::Reference(page_id));
                    pages_dict.set("Count", Object::Integer(new_count));
                }
            }
        }
    }

    Ok(target_doc.get_pages().len())
}

/// Return the size of the last page in the document, or None if empty.
fn last_page_size(doc: &LopdfDocument) -> Option<(f32, f32)> {
    let pages = doc.get_pages();
    let max_n = pages.keys().copied().max()?;
    let id = pages.get(&max_n)?;
    page_size(doc, *id)
}

/// Return the Resources dictionary of the last page, if any.
fn last_page_resources(doc: &LopdfDocument) -> Option<lopdf::Dictionary> {
    let pages = doc.get_pages();
    let max_n = pages.keys().copied().max()?;
    let id = pages.get(&max_n)?;
    doc.get_page_resources(*id)
        .ok()
        .and_then(|(d, _)| d.cloned())
}

/// Read a page's MediaBox to extract (width, height). Falls back to None on
/// parse failure.
fn page_size(doc: &LopdfDocument, page_id: ObjectId) -> Option<(f32, f32)> {
    let page = doc.get_object(page_id).ok()?.as_dict().ok()?;
    let mbox = page.get(b"MediaBox").ok()?;
    let mbox_obj = if let Ok(r) = mbox.as_reference() {
        doc.get_object(r).ok()?
    } else {
        mbox
    };
    let arr = mbox_obj.as_array().ok()?;
    if arr.len() < 4 {
        return None;
    }
    let w = arr.get(2).and_then(|o| {
        o.as_float()
            .ok()
            .or_else(|| o.as_i64().ok().map(|i| i as f32))
    })?;
    let h = arr.get(3).and_then(|o| {
        o.as_float()
            .ok()
            .or_else(|| o.as_i64().ok().map(|i| i as f32))
    })?;
    Some((w, h))
}

/// Find the first font resource name on a page (e.g. "F1").
/// Returns None if no fonts are defined for the page.
fn first_page_font_name(doc: &LopdfDocument, page_id: ObjectId) -> Option<String> {
    let fonts = doc.get_page_fonts(page_id).ok()?;
    fonts
        .keys()
        .next()
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
}

/// Escape a literal PDF string body — only the three chars that terminate
/// or escape inside `(...)` operands: ( ) and \.
fn escape_pdf_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    for c in text.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

/// Parse a text block path like /page[N]/text[M] into (page_num, text_index).
fn parse_text_block_path(path: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        return None;
    }

    let page_part = parts[0];
    if !page_part.starts_with("page") {
        return None;
    }
    let page_num = page_part
        .strip_prefix("page[")
        .and_then(|s| s.strip_suffix("]"))
        .and_then(|s| s.parse::<usize>().ok())?;

    let text_part = parts[1];
    if !text_part.starts_with("text") {
        return None;
    }
    let text_index = text_part
        .strip_prefix("text[")
        .and_then(|s| s.strip_suffix("]"))
        .and_then(|s| s.parse::<usize>().ok())?;

    Some((page_num, text_index))
}

/// Apply foreground text colors to a specific character range of text blocks.
pub fn apply_range_text_colors(
    doc: &mut LopdfDocument,
    color: &PdfColor,
    segments: &[handler_common::PathRangeSegment],
) -> Result<(), HandlerError> {
    use std::collections::HashMap;

    // Helper to format color operators — sets BOTH fill (rg/g/k) and stroke (RG/G/K)
    // so that Tr=2 (fill+stroke) text also gets the target color.
    let format_color_op = |col: &PdfColor| -> String {
        match col {
            PdfColor::Gray(g) => format!("{} g {} G", g, g),
            PdfColor::Rgb(r, g, b) => format!("{} {} {} rg {} {} {} RG", r, g, b, r, g, b),
            PdfColor::Cmyk(c, m, y, k) => {
                format!("{} {} {} {} k {} {} {} {} K", c, m, y, k, c, m, y, k)
            }
        }
    };

    // Group segments by page
    let mut page_groups: HashMap<usize, Vec<handler_common::PathRangeSegment>> = HashMap::new();
    for seg in segments {
        if let Some((page_num, _)) = parse_text_block_path(&seg.path) {
            page_groups.entry(page_num).or_default().push(seg.clone());
        }
    }

    for (page_num, page_segs) in page_groups {
        let pages = doc.get_pages();
        let page_id = *pages
            .get(&(page_num as u32))
            .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

        let content = doc.get_page_content(page_id).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to get page content: {}", e))
        })?;

        let parsed = parse_page_content_stream(&content, page_id, doc).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to parse content stream: {}", e))
        })?;

        let mut modified_lines = parsed.lines.clone();

        for seg in page_segs {
            if let Some((_, text_index)) = parse_text_block_path(&seg.path) {
                let block_idx = text_index - 1;
                if block_idx >= parsed.text_blocks.len() {
                    return Err(HandlerError::PathNotFound(format!(
                        "text block {} not found on page {}",
                        text_index, page_num
                    )));
                }
                let block = &parsed.text_blocks[block_idx];

                let start = seg.start.unwrap_or(0);
                let char_count = block.text.chars().count();
                let end = seg.end.unwrap_or(char_count).min(char_count).max(start);

                let prefix_chars: String = block.text.chars().take(start).collect();
                let selected_chars: String =
                    block.text.chars().skip(start).take(end - start).collect();
                let suffix_chars: String = block.text.chars().skip(end).collect();

                let font_name = block.style.font_name.as_deref().unwrap_or("F1");

                let mut ops = Vec::new();

                if !prefix_chars.is_empty() {
                    let enc = crate::content_stream::encode_chunk_with_font(
                        doc,
                        page_id,
                        font_name,
                        &prefix_chars,
                    )?;
                    ops.push(format!("{} Tj", enc));
                }

                // Set new color
                ops.push(format_color_op(color));

                if !selected_chars.is_empty() {
                    let enc = crate::content_stream::encode_chunk_with_font(
                        doc,
                        page_id,
                        font_name,
                        &selected_chars,
                    )?;
                    ops.push(format!("{} Tj", enc));
                }

                // Restore original color
                let orig_color = block
                    .style
                    .fill_color
                    .clone()
                    .unwrap_or(PdfColor::Gray(0.0));
                ops.push(format_color_op(&orig_color));

                if !suffix_chars.is_empty() {
                    let enc = crate::content_stream::encode_chunk_with_font(
                        doc,
                        page_id,
                        font_name,
                        &suffix_chars,
                    )?;
                    ops.push(format!("{} Tj", enc));
                }

                // Splice ops into content stream
                let line = &modified_lines[block.text_line_index];
                let mut line_tokens = crate::content_stream::tokenize_pdf_line(line);

                if block.line_token_index < line_tokens.len() {
                    let op_idx = block.line_token_index;
                    let consume_extra = matches!(
                        line_tokens.get(op_idx + 1).map(|s| s.as_str()),
                        Some("Tj") | Some("TJ")
                    );
                    let end_token = if consume_extra {
                        op_idx + 2
                    } else {
                        op_idx + 1
                    };

                    let replacement = ops.join(" ");
                    line_tokens.splice(op_idx..end_token, vec![replacement]);
                    modified_lines[block.text_line_index] = line_tokens.join(" ");
                }
            }
        }

        // Save page content
        let new_content = modified_lines.join("\n").into_bytes();
        doc.change_page_content(page_id, new_content).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to save page content: {}", e))
        })?;
    }

    Ok(())
}

/// Apply native Highlight annotation for a cross-node text block range.
pub fn apply_range_highlights(
    doc: &mut LopdfDocument,
    color: &PdfColor,
    segments: &[handler_common::PathRangeSegment],
) -> Result<(), HandlerError> {
    use std::collections::HashMap;

    // Group segments by page
    let mut page_groups: HashMap<usize, Vec<handler_common::PathRangeSegment>> = HashMap::new();
    for seg in segments {
        if let Some((page_num, _)) = parse_text_block_path(&seg.path) {
            page_groups.entry(page_num).or_default().push(seg.clone());
        }
    }

    for (page_num, page_segs) in page_groups {
        let pages = doc.get_pages();
        let page_id = *pages
            .get(&(page_num as u32))
            .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

        let content = doc.get_page_content(page_id).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to get page content: {}", e))
        })?;

        let parsed = parse_page_content_stream(&content, page_id, doc).map_err(|e| {
            HandlerError::OperationFailed(format!("failed to parse content stream: {}", e))
        })?;

        let mut rects = Vec::new();

        for seg in page_segs {
            if let Some((_, text_index)) = parse_text_block_path(&seg.path) {
                let block_idx = text_index - 1;
                if block_idx >= parsed.text_blocks.len() {
                    return Err(HandlerError::PathNotFound(format!(
                        "text block {} not found on page {}",
                        text_index, page_num
                    )));
                }
                let block = &parsed.text_blocks[block_idx];

                // Calculate sub-bounding boxes
                let start = seg.start.unwrap_or(0);
                let end = seg.end.unwrap_or(block.text.chars().count());

                // Safety checks for indices
                let char_count = block.text.chars().count();
                let start = start.min(char_count);
                let end = end.min(char_count).max(start);

                let font_name = block.style.font_name.as_deref().unwrap_or("F1");
                let font_info = parsed.font_map.get(font_name);

                let (sub_bbox_x, sub_bbox_width) = if start == 0 && end == char_count {
                    // Full highlight
                    (block.bbox.x, block.bbox.width)
                } else if let Some(fi) = font_info {
                    let font_size = block.style.font_size.unwrap_or(12.0);
                    let char_spacing = block.style.char_spacing;
                    let word_spacing = block.style.word_spacing;

                    // Prefix width
                    let prefix_chars: String = block.text.chars().take(start).collect();
                    let prefix_width = crate::content_stream::estimate_text_width(
                        &prefix_chars,
                        fi,
                        font_size,
                        char_spacing,
                        word_spacing,
                    );

                    // Selected width
                    let selected_chars: String =
                        block.text.chars().skip(start).take(end - start).collect();
                    let selected_width = crate::content_stream::estimate_text_width(
                        &selected_chars,
                        fi,
                        font_size,
                        char_spacing,
                        word_spacing,
                    );

                    (block.bbox.x + prefix_width, selected_width)
                } else {
                    // Fallback to proportional split
                    let ratio_start = start as f32 / char_count as f32;
                    let ratio_end = end as f32 / char_count as f32;
                    let prefix_width = block.bbox.width * ratio_start;
                    let selected_width = block.bbox.width * (ratio_end - ratio_start);
                    (block.bbox.x + prefix_width, selected_width)
                };

                eprintln!(
                    "[DEBUG highlight] block.bbox=({},{},{},{}), sub_bbox_x={}, sub_bbox_width={}",
                    block.bbox.x,
                    block.bbox.y,
                    block.bbox.width,
                    block.bbox.height,
                    sub_bbox_x,
                    sub_bbox_width
                );
                rects.push(crate::content_stream::BBox {
                    x: sub_bbox_x,
                    y: block.bbox.y,
                    width: sub_bbox_width,
                    height: block.bbox.height,
                });
            }
        }

        if rects.is_empty() {
            continue;
        }

        // Add Native Highlight Annotation to PDF page dictionary
        let mut annot_dict = lopdf::Dictionary::new();
        annot_dict.set("Type", lopdf::Object::Name(b"Annot".to_vec()));
        annot_dict.set("Subtype", lopdf::Object::Name(b"Highlight".to_vec()));

        let mut x_min = f32::MAX;
        let mut y_min = f32::MAX;
        let mut x_max = f32::MIN;
        let mut y_max = f32::MIN;

        let mut quad_points = Vec::new();
        for rect in &rects {
            x_min = x_min.min(rect.x);
            y_min = y_min.min(rect.y);
            x_max = x_max.max(rect.x + rect.width);
            y_max = y_max.max(rect.y + rect.height);

            // QuadPoints: top-left, top-right, bottom-left, bottom-right
            let x_tl = rect.x;
            let y_tl = rect.y + rect.height;
            let x_tr = rect.x + rect.width;
            let y_tr = rect.y + rect.height;
            let x_bl = rect.x;
            let y_bl = rect.y;
            let x_br = rect.x + rect.width;
            let y_br = rect.y;

            // Standard PDF Spec QuadPoints order: top-left, top-right, bottom-left, bottom-right
            quad_points.push(lopdf::Object::Real(x_tl));
            quad_points.push(lopdf::Object::Real(y_tl));
            quad_points.push(lopdf::Object::Real(x_tr));
            quad_points.push(lopdf::Object::Real(y_tr));
            quad_points.push(lopdf::Object::Real(x_bl));
            quad_points.push(lopdf::Object::Real(y_bl));
            quad_points.push(lopdf::Object::Real(x_br));
            quad_points.push(lopdf::Object::Real(y_br));
        }

        annot_dict.set(
            "Rect",
            lopdf::Object::Array(vec![
                lopdf::Object::Real(x_min),
                lopdf::Object::Real(y_min),
                lopdf::Object::Real(x_max),
                lopdf::Object::Real(y_max),
            ]),
        );
        annot_dict.set("QuadPoints", lopdf::Object::Array(quad_points));

        let (r, g, b) = match color {
            PdfColor::Gray(gray) => (*gray, *gray, *gray),
            PdfColor::Rgb(r, g, b) => (*r, *g, *b),
            PdfColor::Cmyk(c, m, y, k) => {
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };
        annot_dict.set(
            "C",
            lopdf::Object::Array(vec![
                lopdf::Object::Real(r),
                lopdf::Object::Real(g),
                lopdf::Object::Real(b),
            ]),
        );

        // 1. Check if "Annots" exists on the page (immutable borrow of doc)
        let mut has_annots = false;
        let mut is_reference = None;
        let mut inline_array = None;

        if let Ok(page_dict) = doc.get_dictionary(page_id) {
            if let Ok(obj) = page_dict.get(b"Annots") {
                has_annots = true;
                match obj {
                    lopdf::Object::Reference(ref_id) => {
                        is_reference = Some(*ref_id);
                    }
                    lopdf::Object::Array(arr) => {
                        inline_array = Some(arr.clone());
                    }
                    _ => {}
                }
            }
        }

        // 2. Add the annotation object (mutable borrow of doc)
        let annot_id = doc.add_object(lopdf::Object::Dictionary(annot_dict));

        // 3. Insert annotation ID into Annots array
        if has_annots {
            if let Some(ref_id) = is_reference {
                if let Ok(lopdf::Object::Array(ref mut arr)) = doc.get_object_mut(ref_id) {
                    arr.push(lopdf::Object::Reference(annot_id));
                }
            } else if let Some(mut arr) = inline_array {
                arr.push(lopdf::Object::Reference(annot_id));
                if let Ok(page_dict) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
                    page_dict.set("Annots", lopdf::Object::Array(arr));
                }
            }
        } else {
            let arr = vec![lopdf::Object::Reference(annot_id)];
            let arr_id = doc.add_object(lopdf::Object::Array(arr));
            if let Ok(page_dict) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
                page_dict.set("Annots", lopdf::Object::Reference(arr_id));
            }
        }
    }

    Ok(())
}
