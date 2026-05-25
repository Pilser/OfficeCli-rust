use handler_common::HandlerError;
use lopdf::Document as LopdfDocument;
use lopdf::ObjectId;

/// Replace text strings in a PDF page's content stream.
/// Finds all (text) Tj patterns and replaces the string content.
pub fn replace_text_on_page(
    doc: &mut LopdfDocument,
    page_num: usize,
    new_text: &str,
) -> Result<(), HandlerError> {
    let pages = doc.get_pages();
    let page_id = pages.get(&(page_num as u32))
        .ok_or_else(|| HandlerError::PathNotFound(format!("page {}", page_num)))?;

    // Get the page content stream
    let content = doc.get_page_content(*page_id)
        .map_err(|e| HandlerError::OperationFailed(format!("failed to get page content: {}", e)))?;

    let content_str = String::from_utf8_lossy(&content);
    let modified = replace_strings_in_stream(&content_str, new_text);

    // Find and replace the content stream in the document
    let content_ids = doc.get_page_contents(*page_id);
    for content_id in content_ids {
        if let Ok(obj) = doc.get_object_mut(content_id) {
            if let lopdf::Object::Stream(stream) = obj {
                stream.content = modified.as_bytes().to_vec();
                stream.dict.set("Length", lopdf::Object::Integer(modified.len() as i64));
            }
        }
    }

    Ok(())
}

/// Replace all text strings in a PDF content stream with new text.
fn replace_strings_in_stream(stream: &str, new_text: &str) -> String {
    let mut result = String::new();
    let mut in_text_object = false;

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

        // Replace text in simple (text) Tj patterns
        if trimmed.ends_with(" Tj") {
            let string_part = trimmed.trim_end_matches(" Tj").trim();
            if string_part.starts_with('(') && string_part.ends_with(')') {
                let encoded_new = encode_pdf_string(new_text);
                result.push_str(&format!("({}) Tj", encoded_new));
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

    result
}

/// Encode a plain text string for safe use in PDF string literals.
fn encode_pdf_string(text: &str) -> String {
    let mut result = String::new();
    for c in text.chars() {
        match c {
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result
}

/// Replace entire page content with new content bytes.
pub fn replace_page_content(
    doc: &mut LopdfDocument,
    page_id: ObjectId,
    new_content: &[u8],
) -> Result<(), HandlerError> {
    let content_ids = doc.get_page_contents(page_id);
    for content_id in content_ids {
        if let Ok(obj) = doc.get_object_mut(content_id) {
            if let lopdf::Object::Stream(stream) = obj {
                stream.content = new_content.to_vec();
                stream.dict.set("Length", lopdf::Object::Integer(new_content.len() as i64));
            }
        }
    }
    Ok(())
}

/// Delete a page from the PDF document.
/// Uses lopdf's built-in delete_pages which handles page tree updates.
pub fn delete_page(doc: &mut LopdfDocument, page_num: usize) -> Result<(), HandlerError> {
    doc.delete_pages(&[page_num as u32]);
    Ok(())
}