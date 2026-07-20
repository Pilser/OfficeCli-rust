use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashMap;

/// An element operation to be applied via `DocumentHandler::add`.
#[derive(Debug, Clone)]
pub struct MdElement {
    pub element_type: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug)]
enum Ctx {
    Paragraph,
    Heading(HeadingLevel),
    ListItem { ordered: bool, number: u64 },
    Blockquote,
    CodeBlock,
    Table,
    TableRow,
    TableCell,
}

#[derive(Debug, Clone)]
struct InlineSpan {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
    strike: bool,
    link_url: Option<String>,
}

/// Parse markdown into a sequence of element operations.
///
/// Block elements emit `"paragraph"` with properties:
///   `text`, `heading`, `bullet`, `numbering`, `indent`, `rule`, `font`
///
/// When a paragraph contains inline formatting (bold, italic, code, link),
/// the first inline span's text is placed in the paragraph's `text` property,
/// and subsequent spans are emitted as `"run"` elements with `text` plus
/// formatting properties (`bold`, `italic`, `font`, `strike`, `link`).
///
/// Tables are emitted as `"tbl"`, `"tr"`, `"tc"` elements in sequence.
/// Each `"tc"` element has a `text` property for the cell content.
pub fn markdown_to_docx(markdown: &str) -> Result<Vec<MdElement>, String> {
    let parser = Parser::new(markdown);
    let mut out: Vec<MdElement> = Vec::new();

    let mut bold = false;
    let mut italic = false;
    let mut strike = false;
    let mut link_url = String::new();

    let mut spans: Vec<InlineSpan> = Vec::new();
    let mut text_buf = String::new();
    let mut block_text = String::new();

    // Table assembly
    let mut _in_table_head = false;
    let mut table_cols: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut cell_text = String::new();
    let mut has_table = false;

    let mut stack: Vec<Ctx> = Vec::new();
    let mut list_number = 0u64;
    let mut list_ordered = false;

    for event in parser {
        match event {
            // ── Start tags ──────────────────────────────────────────
            Event::Start(tag) => match tag {
                Tag::Paragraph => stack.push(Ctx::Paragraph),
                Tag::Heading { level, .. } => stack.push(Ctx::Heading(level)),
                Tag::List(ordered) => {
                    list_ordered = ordered.is_some();
                    list_number = 0;
                }
                Tag::Item => {
                    list_number += 1;
                    stack.push(Ctx::ListItem {
                        ordered: list_ordered,
                        number: list_number,
                    });
                }
                Tag::BlockQuote(_) => stack.push(Ctx::Blockquote),
                Tag::CodeBlock(_) => stack.push(Ctx::CodeBlock),
                Tag::Table(_) => {
                    flush_para(&mut out, &stack, &mut spans, &mut text_buf, &mut block_text);
                    stack.push(Ctx::Table);
                    table_cols.clear();
                    table_rows.clear();
                    has_table = true;
                    _in_table_head = true;
                }
                Tag::TableHead => {}
                Tag::TableRow => stack.push(Ctx::TableRow),
                Tag::TableCell => stack.push(Ctx::TableCell),
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Strikethrough => strike = true,
                Tag::Link { dest_url, .. } => link_url = dest_url.to_string(),
                _ => {}
            },

            // ── End tags ────────────────────────────────────────────
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    if !has_table {
                        flush_para(&mut out, &stack, &mut spans, &mut text_buf, &mut block_text);
                    }
                    stack.pop();
                }
                TagEnd::Heading(_) => {
                    flush_para(&mut out, &stack, &mut spans, &mut text_buf, &mut block_text);
                    stack.pop();
                }
                TagEnd::Item => {
                    flush_para(&mut out, &stack, &mut spans, &mut text_buf, &mut block_text);
                    stack.pop();
                }
                TagEnd::BlockQuote(_) => {
                    stack.pop();
                }
                TagEnd::CodeBlock => {
                    flush_para(&mut out, &stack, &mut spans, &mut text_buf, &mut block_text);
                    stack.pop();
                }
                TagEnd::Table => {
                    emit_table(&mut out, &table_rows);
                    stack.pop();
                    has_table = false;
                }
                TagEnd::TableHead => {
                    _in_table_head = false;
                }
                TagEnd::TableRow => {
                    let row_cells: Vec<String> = table_cols.drain(..).collect();
                    table_rows.push(row_cells);
                    stack.pop();
                }
                TagEnd::TableCell => {
                    let content = cell_text.clone();
                    table_cols.push(content);
                    cell_text.clear();
                    stack.pop();
                }
                TagEnd::Emphasis => {
                    flush_span(&mut spans, &mut text_buf, bold, italic, strike, &link_url);
                    italic = false;
                }
                TagEnd::Strong => {
                    flush_span(&mut spans, &mut text_buf, bold, italic, strike, &link_url);
                    bold = false;
                }
                TagEnd::Strikethrough => {
                    flush_span(&mut spans, &mut text_buf, bold, italic, strike, &link_url);
                    strike = false;
                }
                TagEnd::Link => {
                    flush_span(&mut spans, &mut text_buf, bold, italic, strike, &link_url);
                    link_url.clear();
                }
                _ => {}
            },

            // ── Text / Code / Events ────────────────────────────────
            Event::Text(t) => {
                let s = t.to_string();
                if in_code_block(&stack) {
                    block_text.push_str(&s);
                } else if in_table_cell(&stack) {
                    cell_text.push_str(&s);
                } else {
                    flush_span(&mut spans, &mut text_buf, bold, italic, strike, &link_url);
                    text_buf = s;
                }
            }

            Event::Code(t) => {
                let code_text = t.to_string();
                if in_table_cell(&stack) {
                    cell_text.push_str(&code_text);
                } else {
                    flush_span(&mut spans, &mut text_buf, bold, italic, strike, &link_url);
                    spans.push(InlineSpan {
                        text: code_text,
                        bold: false,
                        italic: false,
                        code: true,
                        strike: false,
                        link_url: None,
                    });
                }
            }

            Event::Rule => {
                let mut p = HashMap::new();
                p.insert("rule".to_string(), "true".to_string());
                out.push(MdElement { element_type: "paragraph".to_string(), properties: p });
            }

            Event::SoftBreak | Event::HardBreak => {
                if in_code_block(&stack) {
                    block_text.push('\n');
                } else if in_table_cell(&stack) {
                    cell_text.push(' ');
                } else {
                    text_buf.push(' ');
                }
            }

            Event::Html(t) | Event::InlineHtml(t) => {
                if in_code_block(&stack) {
                    block_text.push_str(&t);
                }
            }

            Event::FootnoteReference(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::TaskListMarker(_) => {}
        }
    }

    // Flush remaining text
    if !text_buf.is_empty() || !spans.is_empty() || !block_text.is_empty() {
        flush_para(&mut out, &stack, &mut spans, &mut text_buf, &mut block_text);
    }

    Ok(out)
}

// ─── Helper functions ───────────────────────────────────────────────

fn in_code_block(stack: &[Ctx]) -> bool {
    stack.iter().any(|c| matches!(c, Ctx::CodeBlock))
}

fn in_table_cell(stack: &[Ctx]) -> bool {
    stack.iter().any(|c| matches!(c, Ctx::TableCell))
}

fn flush_span(
    spans: &mut Vec<InlineSpan>,
    buf: &mut String,
    bold: bool,
    italic: bool,
    strike: bool,
    link: &str,
) {
    if !buf.is_empty() {
        spans.push(InlineSpan {
            text: buf.clone(),
            bold,
            italic,
            code: false,
            strike,
            link_url: if link.is_empty() {
                None
            } else {
                Some(link.to_string())
            },
        });
        buf.clear();
    }
}

fn flush_para(
    out: &mut Vec<MdElement>,
    stack: &[Ctx],
    spans: &mut Vec<InlineSpan>,
    text_buf: &mut String,
    block_text: &mut String,
) {
    if !text_buf.is_empty() {
        spans.push(InlineSpan {
            text: text_buf.clone(),
            bold: false,
            italic: false,
            code: false,
            strike: false,
            link_url: None,
        });
        text_buf.clear();
    }

    let mut heading: Option<HeadingLevel> = None;
    let mut bullet = false;
    let mut numbering: Option<String> = None;
    let mut indent = false;
    let mut is_code = false;

    for ctx in stack.iter().rev() {
        match ctx {
            Ctx::Heading(lv) => heading = Some(*lv),
            Ctx::ListItem { ordered, number } => {
                if *ordered {
                    numbering = Some(number.to_string());
                } else {
                    bullet = true;
                }
            }
            Ctx::Blockquote => indent = true,
            Ctx::CodeBlock => is_code = true,
            _ => {}
        }
    }

    let text_content: String = if !spans.is_empty() {
        spans.iter().map(|s| s.text.as_str()).collect()
    } else {
        block_text.clone()
    };

    if text_content.is_empty()
        && heading.is_none()
        && !bullet
        && numbering.is_none()
        && !indent
        && !is_code
    {
        return;
    }

    if is_code {
        let mut p = HashMap::new();
        p.insert("text".to_string(), text_content);
        p.insert("font".to_string(), "monospace".to_string());
        out.push(MdElement { element_type: "paragraph".to_string(), properties: p });
        spans.clear();
        block_text.clear();
        return;
    }

    if spans.len() <= 1 {
        let text = if !spans.is_empty() {
            spans[0].text.clone()
        } else {
            text_content.clone()
        };
        let mut p = HashMap::new();
        if !text.is_empty() {
            p.insert("text".to_string(), text);
        }
        if let Some(lv) = heading {
            p.insert("heading".to_string(), format!("{}", lv as usize));
        }
        if bullet {
            p.insert("bullet".to_string(), "true".to_string());
        }
        if let Some(num) = numbering {
            p.insert("numbering".to_string(), num);
        }
        if indent {
            p.insert("indent".to_string(), "true".to_string());
        }
        if !spans.is_empty() {
            let s = &spans[0];
            if s.bold {
                p.insert("bold".to_string(), "true".to_string());
            }
            if s.italic {
                p.insert("italic".to_string(), "true".to_string());
            }
            if s.code {
                p.insert("font".to_string(), "monospace".to_string());
            }
            if s.strike {
                p.insert("strike".to_string(), "true".to_string());
            }
            if let Some(url) = &s.link_url {
                p.insert("link".to_string(), url.clone());
            }
        }
        out.push(MdElement { element_type: "paragraph".to_string(), properties: p });
    } else {
        let mut par = HashMap::new();
        if let Some(lv) = heading {
            par.insert("heading".to_string(), format!("{}", lv as usize));
        }
        if bullet {
            par.insert("bullet".to_string(), "true".to_string());
        }
        if let Some(num) = numbering {
            par.insert("numbering".to_string(), num);
        }
        if indent {
            par.insert("indent".to_string(), "true".to_string());
        }

        let first = &spans[0];
        par.insert("text".to_string(), first.text.clone());
        if first.bold {
            par.insert("bold".to_string(), "true".to_string());
        }
        if first.italic {
            par.insert("italic".to_string(), "true".to_string());
        }
        if first.code {
            par.insert("font".to_string(), "monospace".to_string());
        }
        if first.strike {
            par.insert("strike".to_string(), "true".to_string());
        }
        if let Some(url) = &first.link_url {
            par.insert("link".to_string(), url.clone());
        }
        out.push(MdElement { element_type: "paragraph".to_string(), properties: par });

        for s in &spans[1..] {
            let mut rp = HashMap::new();
            rp.insert("text".to_string(), s.text.clone());
            if s.bold {
                rp.insert("bold".to_string(), "true".to_string());
            }
            if s.italic {
                rp.insert("italic".to_string(), "true".to_string());
            }
            if s.code {
                rp.insert("font".to_string(), "monospace".to_string());
            }
            if s.strike {
                rp.insert("strike".to_string(), "true".to_string());
            }
            if let Some(url) = &s.link_url {
                rp.insert("link".to_string(), url.clone());
            }
            out.push(MdElement { element_type: "run".to_string(), properties: rp });
        }
    }

    spans.clear();
    block_text.clear();
}

fn emit_table(out: &mut Vec<MdElement>, rows: &[Vec<String>]) {
    if rows.is_empty() {
        return;
    }

    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);

    let mut tbl = HashMap::new();
    tbl.insert("cols".to_string(), max_cols.to_string());
    tbl.insert("rows".to_string(), rows.len().to_string());
    out.push(MdElement { element_type: "tbl".to_string(), properties: tbl });

    for (ri, row) in rows.iter().enumerate() {
        out.push(MdElement { element_type: "tr".to_string(), properties: HashMap::new() });
        for cell in row {
            let mut cp = HashMap::new();
            cp.insert("text".to_string(), cell.clone());
            if ri == 0 {
                cp.insert("bold".to_string(), "true".to_string());
            }
            out.push(MdElement { element_type: "tc".to_string(), properties: cp });
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(props: &HashMap<String, String>) -> &str {
        props.get("text").map(|s| s.as_str()).unwrap_or("")
    }

    #[test]
    fn test_plain_text() {
        let out = markdown_to_docx("Hello world").unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].element_type, "paragraph");
        assert_eq!(text_of(&out[0].properties), "Hello world");
    }

    #[test]
    fn test_heading() {
        let out = markdown_to_docx("# Heading 1").unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].properties.get("heading").unwrap(), "1");
        assert_eq!(text_of(&out[0].properties), "Heading 1");
    }

    #[test]
    fn test_heading_levels() {
        let out = markdown_to_docx("## H2\n\n### H3").unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].properties.get("heading").unwrap(), "2");
        assert_eq!(out[1].properties.get("heading").unwrap(), "3");
    }

    #[test]
    fn test_bold() {
        let out = markdown_to_docx("**bold**").unwrap();
        assert_eq!(out[0].properties.get("bold").unwrap(), "true");
        assert_eq!(text_of(&out[0].properties), "bold");
    }

    #[test]
    fn test_italic() {
        let out = markdown_to_docx("*italic*").unwrap();
        assert_eq!(out[0].properties.get("italic").unwrap(), "true");
        assert_eq!(text_of(&out[0].properties), "italic");
    }

    #[test]
    fn test_code_inline() {
        let out = markdown_to_docx("`code`").unwrap();
        assert_eq!(out[0].properties.get("font").unwrap(), "monospace");
        assert_eq!(text_of(&out[0].properties), "code");
    }

    #[test]
    fn test_bullet_list() {
        let out = markdown_to_docx("- item").unwrap();
        assert_eq!(out[0].properties.get("bullet").unwrap(), "true");
        assert_eq!(text_of(&out[0].properties), "item");
    }

    #[test]
    fn test_ordered_list() {
        let out = markdown_to_docx("1. item").unwrap();
        assert_eq!(out[0].properties.get("numbering").unwrap(), "1");
        assert_eq!(text_of(&out[0].properties), "item");
    }

    #[test]
    fn test_hyperlink() {
        let out = markdown_to_docx("[text](http://example.com)").unwrap();
        assert_eq!(
            out[0].properties.get("link").unwrap(),
            "http://example.com"
        );
        assert_eq!(text_of(&out[0].properties), "text");
    }

    #[test]
    fn test_blockquote() {
        let out = markdown_to_docx("> quote").unwrap();
        assert_eq!(out[0].properties.get("indent").unwrap(), "true");
        assert_eq!(text_of(&out[0].properties), "quote");
    }

    #[test]
    fn test_thematic_break() {
        let out = markdown_to_docx("---").unwrap();
        assert_eq!(out[0].properties.get("rule").unwrap(), "true");
    }

    #[test]
    fn test_code_block() {
        let out = markdown_to_docx("```\ncode block\n```").unwrap();
        assert_eq!(out[0].properties.get("font").unwrap(), "monospace");
        assert!(text_of(&out[0].properties).contains("code block"));
    }

    #[test]
    fn test_mixed_inline() {
        let out = markdown_to_docx("**bold** and *italic*").unwrap();
        assert!(out.len() >= 2);
        assert_eq!(out[0].element_type, "paragraph");
        assert_eq!(out[1].element_type, "run");
    }

    #[test]
    fn test_multiple_paragraphs() {
        let out = markdown_to_docx("Para one\n\nPara two").unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(text_of(&out[0].properties), "Para one");
        assert_eq!(text_of(&out[1].properties), "Para two");
    }

    #[test]
    fn test_table() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |";
        let out = markdown_to_docx(md).unwrap();
        let tbl: Vec<&MdElement> = out.iter().filter(|e| e.element_type == "tbl").collect();
        let trs: Vec<&MdElement> = out.iter().filter(|e| e.element_type == "tr").collect();
        let tcs: Vec<&MdElement> = out.iter().filter(|e| e.element_type == "tc").collect();
        assert!(!tbl.is_empty(), "expected at least one tbl element");
        assert!(!trs.is_empty(), "expected at least one tr element");
        assert!(!tcs.is_empty(), "expected at least one tc element");
    }

    #[test]
    fn test_empty_input() {
        let out = markdown_to_docx("").unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn test_bold_italic_mixed() {
        let out = markdown_to_docx("plain **bold** after").unwrap();
        assert!(out.len() >= 2);
        let p = &out[0];
        assert_eq!(p.element_type, "paragraph");
        // First span's text goes into paragraph properties
        let pt = text_of(&p.properties);
        assert!(pt.contains("plain"), "expected 'plain' in paragraph text, got '{pt}'");
        let r = &out[1];
        assert_eq!(r.element_type, "run");
        assert_eq!(r.properties.get("bold").unwrap(), "true");
        assert_eq!(text_of(&r.properties), "bold");
    }
}
