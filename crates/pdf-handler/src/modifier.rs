use handler_common::HandlerError;
use lopdf::Document as LopdfDocument;
use lopdf::ObjectId;
use crate::content_stream::{
    parse_page_content_stream, pick_fonts_for_text, FontSegment, PdfColor,
};

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
    let page_id = *pages.get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc.get_page_content(page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))?;

    let parsed = parse_page_content_stream(&content, page_id, doc)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to parse content stream: {}", e)))?;

    let block_idx = text_index - 1;
    if block_idx >= parsed.text_blocks.len() {
        return Err(HandlerError::PathNotFound(format!(
            "text[{}] not found (page {} has {} text blocks)",
            text_index, page_num, parsed.text_blocks.len()
        )));
    }

    let target_block = &parsed.text_blocks[block_idx];
    let orig_font_owned = target_block.style.font_name.clone();
    let orig_font = orig_font_owned.as_deref();
    // Use the RAW Tf operand (without Tm scaling). The active Tm matrix from
    // the original content will still scale our re-emitted Tf; writing the
    // effective (already-scaled) size here would compound Tm twice and blow
    // up the rendered font size.
    let orig_size = target_block.style.raw_font_size
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
        let end = if consume_extra { op_idx + 2 } else { op_idx + 1 };
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
    let page_id = *pages.get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc.get_page_content(page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))?;

    let parsed = parse_page_content_stream(&content, page_id, doc)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to parse content stream: {}", e)))?;

    let block_idx = text_index - 1;
    if block_idx >= parsed.text_blocks.len() {
        return Err(HandlerError::PathNotFound(format!("text[{}] not found", text_index)));
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
        style_lines.push(format!("/{} {} Tf", effective_font, format_size(effective_size)));
    }

    if let Some(color) = fill_color {
        match color {
            PdfColor::Gray(g) => style_lines.push(format!("{} g", g)),
            PdfColor::Rgb(r, g, b) => style_lines.push(format!("{} {} {} rg", r, g, b)),
            PdfColor::Cmyk(c, m, y, k) => style_lines.push(format!("{} {} {} {} k", c, m, y, k)),
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
    let has_subsequent = parsed.text_blocks[block_idx + 1..]
        .iter()
        .any(|b| b.bt_start_line == target_block.bt_start_line && b.bt_end_line == target_block.bt_end_line);

    if has_subsequent {
        if font_name.is_some() || font_size.is_some() {
            let orig_font = target_block.style.font_name.as_deref().unwrap_or("F1");
            let orig_size = target_block.style.raw_font_size
                .or(target_block.style.font_size)
                .unwrap_or(12.0);
            restore_lines.push(format!("/{} {} Tf", orig_font, format_size(orig_size)));
        }
        if let Some(_color) = fill_color {
            if let Some(ref orig_color) = target_block.style.fill_color {
                match orig_color {
                    PdfColor::Gray(g) => restore_lines.push(format!("{} g", g)),
                    PdfColor::Rgb(r, g, b) => restore_lines.push(format!("{} {} {} rg", r, g, b)),
                    PdfColor::Cmyk(c, m, y, k) => restore_lines.push(format!("{} {} {} {} k", c, m, y, k)),
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
    let segments = pick_fonts_for_text(doc, page_id, Some(&effective_font), &effective_text, &mut missing)?;
    if !missing.is_empty() {
        return Err(HandlerError::OperationFailed(format!(
            "characters not encodable in any page font: {}. Provide --prop fontFile=<path> or --prop font=<name> to override.",
            missing.iter().collect::<String>()
        )));
    }

    let new_tokens = build_segment_tokens(&segments, Some(&effective_font), effective_size);

    let line = &modified_lines[target_block.text_line_index];
    let mut line_tokens = crate::content_stream::tokenize_pdf_line(line);

    if target_block.line_token_index < line_tokens.len() {
        let op_idx = target_block.line_token_index;
        let consume_extra = matches!(
            line_tokens.get(op_idx + 1).map(|s| s.as_str()),
            Some("Tj") | Some("TJ")
        );
        let end = if consume_extra { op_idx + 2 } else { op_idx + 1 };
        line_tokens.splice(op_idx..end, new_tokens);
        modified_lines[target_block.text_line_index] = line_tokens.join(" ");
    } else {
        modified_lines[target_block.text_line_index] = new_tokens.join(" ");
    }

    // Insert style lines before text line, restore lines after text line
    if !style_lines.is_empty() || !restore_lines.is_empty() {
        let insert_pos = target_block.text_line_index;
        let mut new_lines = modified_lines[..insert_pos].to_vec();
        for line in &style_lines {
            new_lines.push(line.clone());
        }
        new_lines.push(modified_lines[insert_pos].clone());
        for line in &restore_lines {
            new_lines.push(line.clone());
        }
        new_lines.extend_from_slice(&modified_lines[insert_pos + 1..]);
        modified_lines = new_lines;
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

fn write_content_to_page(doc: &mut LopdfDocument, page_id: ObjectId, content: &[u8]) -> Result<(), HandlerError> {
    let content_ids = doc.get_page_contents(page_id);
    if content_ids.is_empty() {
        return Err(HandlerError::OperationFailed("page has no content streams".to_string()));
    }

    // Write modified content to the first stream
    let first_id = content_ids[0];
    if let Ok(obj) = doc.get_object_mut(first_id) {
        if let lopdf::Object::Stream(stream) = obj {
            // Remove any existing compression filter first — the content bytes
            // we receive are already decompressed (lopdf transparently inflates
            // FlateDecode streams in get_page_content()). Setting raw bytes
            // while /Filter /FlateDecode remains in the dict causes blank pages
            // on the next load because lopdf tries to deflate raw data.
            stream.dict.remove(b"Filter");
            stream.content = content.to_vec();
            // Re-compress with FlateDecode so the saved PDF stays compact
            // and the /Filter + /Length are consistent.
            if stream.compress().is_err() {
                // Fallback: if compression fails, keep uncompressed but
                // update Length to match the raw content.
                stream.dict.set("Length", lopdf::Object::Integer(content.len() as i64));
            }
        }
    }

    // Clear subsequent streams to prevent duplicate content rendering and viewer corruption
    for &other_id in &content_ids[1..] {
        if let Ok(obj) = doc.get_object_mut(other_id) {
            if let lopdf::Object::Stream(stream) = obj {
                stream.dict.remove(b"Filter");
                stream.content = Vec::new();
                stream.dict.set("Length", lopdf::Object::Integer(0));
            }
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
    let page_id = pages.get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    let content = doc.get_page_content(*page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))?;

    let content_str = String::from_utf8_lossy(&content);
    let modified = blanket_replace_strings(doc, *page_id, &content_str, new_text)?;

    write_content_to_page(doc, *page_id, modified.as_bytes())?;
    Ok(())
}

fn blanket_replace_strings(doc: &LopdfDocument, page_id: ObjectId, stream: &str, new_text: &str) -> Result<String, HandlerError> {
    let mut result = String::new();
    let mut in_text_object = false;
    let mut active_font: Option<String> = None;
    let mut active_size: f32 = 1.0;

    for line in stream.lines() {
        let trimmed = line.trim();
        if trimmed == "BT" { in_text_object = true; result.push_str(line); result.push('\n'); continue; }
        if trimmed == "ET" { in_text_object = false; result.push_str(line); result.push('\n'); continue; }
        if !in_text_object { result.push_str(line); result.push('\n'); continue; }

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
                let segments = pick_fonts_for_text(doc, page_id, active_font.as_deref(), new_text, &mut missing)?;
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
                result.push_str(line); result.push('\n');
            }
        } else {
            result.push_str(line); result.push('\n');
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

/// Delete a page from the PDF document.
pub fn delete_page(doc: &mut LopdfDocument, page_num: usize) -> Result<(), HandlerError> {
    doc.delete_pages(&[page_num as u32]);
    Ok(())
}