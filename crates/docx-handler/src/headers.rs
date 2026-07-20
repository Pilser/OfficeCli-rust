use handler_common::HandlerError;
use oxml::OxmlPackage;
use std::collections::HashMap;

pub struct HeaderFooterInfo {
    pub part_type: String,
    pub type_val: String,
    pub part_path: String,
    pub content: String,
}

const HEADER_TYPE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";
const FOOTER_TYPE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer";

fn next_header_index(package: &OxmlPackage) -> usize {
    let mut i = 1;
    loop {
        let path = format!("word/header{}.xml", i);
        if package.read_part_xml(&path).is_err() {
            return i;
        }
        i += 1;
    }
}

fn next_footer_index(package: &OxmlPackage) -> usize {
    let mut i = 1;
    loop {
        let path = format!("word/footer{}.xml", i);
        if package.read_part_xml(&path).is_err() {
            return i;
        }
        i += 1;
    }
}

fn next_rel_id(package: &OxmlPackage, rels_path: &str) -> String {
    let xml = package.read_part_xml(rels_path).unwrap_or_default();
    let mut max_id = 0usize;
    for part in xml.split("Id=\"rId") {
        if let Some(end) = part.find('"') {
            if let Ok(id) = part[..end].parse::<usize>() {
                if id > max_id {
                    max_id = id;
                }
            }
        }
    }
    format!("rId{}", max_id + 1)
}

pub fn build_header_footer_xml(body_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{}</w:hdr>"#,
        body_xml
    )
}

pub fn build_footer_xml(body_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{}</w:ftr>"#,
        body_xml
    )
}

pub fn build_paragraph_xml(text: &str, props: &HashMap<String, String>) -> String {
    let mut rpr = String::new();
    if let Some(bold) = props.get("bold").or_else(|| props.get("b")) {
        if bold == "true" || bold == "1" {
            rpr.push_str("<w:b/>");
        }
    }
    if let Some(italic) = props.get("italic").or_else(|| props.get("i")) {
        if italic == "true" || italic == "1" {
            rpr.push_str("<w:i/>");
        }
    }
    if let Some(size) = props.get("size").or_else(|| props.get("fontSize")) {
        if let Ok(pt) = size.parse::<f32>() {
            let hp = (pt * 2.0) as u32;
            rpr.push_str(&format!("<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>", hp, hp));
        }
    }
    if let Some(color) = props.get("color").or_else(|| props.get("fontColor")) {
        let c = color.strip_prefix('#').unwrap_or(color);
        rpr.push_str(&format!("<w:color w:val=\"{}\"/>", c));
    }
    if let Some(font) = props.get("font").or_else(|| props.get("fontFamily")) {
        rpr.push_str(&format!(
            "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\" w:cs=\"{}\"/>",
            font, font, font
        ));
    }
    let mut ppr = String::new();
    if let Some(align) = props.get("alignment").or_else(|| props.get("jc")) {
        ppr.push_str(&format!("<w:jc w:val=\"{}\"/>", align));
    }
    let escaped = text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let text_xml = if text.starts_with(' ') || text.ends_with(' ') {
        format!("<w:t xml:space=\"preserve\">{}</w:t>", escaped)
    } else {
        format!("<w:t>{}</w:t>", escaped)
    };
    if !rpr.is_empty() {
        rpr = format!("<w:rPr>{}</w:rPr>", rpr);
    }
    format!(
        "<w:p>{}{}<w:r>{}{}</w:r></w:p>",
        ppr, rpr, rpr, text_xml
    )
}

pub fn page_number_field() -> String {
    r#"<w:fldSimple w:instr=" PAGE "/><w:t> of </w:t><w:fldSimple w:instr=" NUMPAGES "/>"#.to_string()
}

fn normalize_type_val(type_val: &str) -> &str {
    match type_val {
        "first" | "firstPage" | "first_page" => "first",
        "even" | "evenPage" | "even_page" => "even",
        _ => "default",
    }
}

fn map_type_to_ref_attr(type_val: &str) -> &str {
    match type_val {
        "first" => "first",
        "even" => "even",
        _ => "default",
    }
}

#[allow(dead_code)]
fn find_existing_header_ref(sect_pr_xml: &str, type_val: &str) -> Option<String> {
    let attr = map_type_to_ref_attr(type_val);
    let needle = format!("w:type=\"{}\"", attr);
    if let Some(pos) = sect_pr_xml.find(&needle) {
        let before = &sect_pr_xml[..pos];
        let start = before.rfind("<w:headerReference")?;
        let after = &sect_pr_xml[pos..];
        let end = after.find("/>")? + 2;
        Some(sect_pr_xml[start..pos + end].to_string())
    } else {
        None
    }
}

fn remove_existing_header_ref(sect_pr_xml: &str, type_val: &str) -> String {
    let attr = map_type_to_ref_attr(type_val);
    let needle = format!("w:type=\"{}\"", attr);
    let mut result = sect_pr_xml.to_string();
    while let Some(pos) = result.find(&needle) {
        let before = &result[..pos];
        let start = before.rfind("<w:headerReference");
        let after = &result[pos..];
        let end = after.find("/>");
        if let (Some(s), Some(e)) = (start, end) {
            result.replace_range(s..pos + e + 2, "");
        } else {
            break;
        }
    }
    result
}

fn remove_existing_footer_ref(sect_pr_xml: &str, type_val: &str) -> String {
    let attr = map_type_to_ref_attr(type_val);
    let needle = format!("w:type=\"{}\"", attr);
    let mut result = sect_pr_xml.to_string();
    while let Some(pos) = result.find(&needle) {
        let before = &result[..pos];
        let start = before.rfind("<w:footerReference");
        let after = &result[pos..];
        let end = after.find("/>");
        if let (Some(s), Some(e)) = (start, end) {
            result.replace_range(s..pos + e + 2, "");
        } else {
            break;
        }
    }
    result
}

fn inject_relationship(package: &mut OxmlPackage, rels_path: &str, rel_xml: &str) -> Result<(), HandlerError> {
    let xml = package.read_part_xml(rels_path).unwrap_or_else(|_| {
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>".to_string()
    });
    let new_xml = if let Some(pos) = xml.find("</Relationships>") {
        let mut r = xml;
        r.insert_str(pos, rel_xml);
        r
    } else {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{}</Relationships>",
            rel_xml
        )
    };
    package.write_part_xml(rels_path, &new_xml)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))
}

fn ensure_default_content_type(package: &mut OxmlPackage, extension: &str, content_type: &str) -> Result<(), HandlerError> {
    let xml = package.read_part_xml("[Content_Types].xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let ext_attr = format!("Extension=\"{}\"", extension);
    if xml.contains(&ext_attr) {
        return Ok(());
    }
    let default_xml = format!("<Default Extension=\"{}\" ContentType=\"{}\"/>", extension, content_type);
    let types_start = xml.find("<Types").unwrap_or(0);
    let new_xml = if let Some(close) = xml[types_start..].find('>') {
        let insert_at = types_start + close + 1;
        let mut out = String::with_capacity(xml.len() + default_xml.len());
        out.push_str(&xml[..insert_at]);
        out.push_str(&default_xml);
        out.push_str(&xml[insert_at..]);
        out
    } else {
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">{}</Types>", default_xml)
    };
    package.write_part_xml("[Content_Types].xml", &new_xml)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))
}

fn add_header_footer_reference(
    package: &mut OxmlPackage,
    _doc_rels_path: &str,
    sect_pr_xml: &str,
    is_header: bool,
    type_val: &str,
    rel_id: &str,
    part_path: &str,
) -> Result<String, HandlerError> {
    let ref_attr = map_type_to_ref_attr(type_val);
    let ref_tag = if is_header { "w:headerReference" } else { "w:footerReference" };
    let ref_xml = format!("<{} w:type=\"{}\" r:id=\"{}\"/>", ref_tag, ref_attr, rel_id);

    let existing_removed = if is_header {
        remove_existing_header_ref(sect_pr_xml, type_val)
    } else {
        remove_existing_footer_ref(sect_pr_xml, type_val)
    };

    let new_sect_pr = if let Some(pos) = existing_removed.rfind("</w:sectPr>") {
        let mut r = existing_removed;
        r.insert_str(pos, &ref_xml);
        r
    } else {
        format!("<w:sectPr>{}</w:sectPr>", ref_xml)
    };

    let doc_xml = package.read_part_xml("word/document.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let body_close = "</w:body>";
    let sect_pr_open = if let Some(pos) = doc_xml.rfind("<w:sectPr") {
        let end_tag = doc_xml[pos..].find('>').map(|e| pos + e + 1).unwrap_or(pos);
        Some((pos, end_tag))
    } else {
        None
    };

    let new_doc_xml = if let Some((sect_start, sect_tag_end)) = sect_pr_open {
        let sect_pr_close = doc_xml[sect_tag_end..].find("</w:sectPr>")
            .map(|e| sect_tag_end + e)
            .unwrap_or(doc_xml.len());
        let close_tag = "</w:sectPr>";
        format!("{}{}{}", &doc_xml[..sect_start], new_sect_pr, &doc_xml[(sect_pr_close + close_tag.len())..])
    } else if let Some(body_end) = doc_xml.rfind(body_close) {
        format!("{}{}{}", &doc_xml[..body_end], new_sect_pr, &doc_xml[body_end..])
    } else {
        return Err(HandlerError::OperationFailed("could not locate body or sectPr in document.xml".to_string()));
    };

    package.write_part_xml("word/document.xml", &new_doc_xml)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    Ok(part_path.to_string())
}

pub fn add_header(package: &mut OxmlPackage, type_val: &str, content_xml: &str) -> Result<(), HandlerError> {
    let norm_type = normalize_type_val(type_val);
    let header_idx = next_header_index(package);
    let part_path = format!("word/header{}.xml", header_idx);
    let header_xml = build_header_footer_xml(content_xml);

    package.write_part_xml(&part_path, &header_xml)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let doc_rels_path = "word/_rels/document.xml.rels";
    let rel_id = next_rel_id(package, doc_rels_path);
    let rel_target = format!("header{}.xml", header_idx);
    let rel_xml = format!(
        "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>",
        rel_id, HEADER_TYPE, rel_target
    );
    inject_relationship(package, doc_rels_path, &rel_xml)?;

    ensure_default_content_type(package, "xml", "application/xml")?;
    let override_xml = format!(
        "<Override PartName=\"/word/header{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>",
        header_idx
    );
    let ct_xml = package.read_part_xml("[Content_Types].xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let part_ref = format!("PartName=\"/word/header{}.xml\"", header_idx);
    if !ct_xml.contains(&part_ref) {
        let types_start = ct_xml.find("<Types").unwrap_or(0);
        let new_ct_xml = if let Some(close) = ct_xml[types_start..].find('>') {
            let insert_at = types_start + close + 1;
            let mut r = ct_xml;
            r.insert_str(insert_at, &override_xml);
            r
        } else {
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">{}</Types>", override_xml)
        };
        package.write_part_xml("[Content_Types].xml", &new_ct_xml)
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    }

    let doc_xml = package.read_part_xml("word/document.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let sect_pr = extract_last_sect_pr(&doc_xml);
    add_header_footer_reference(package, doc_rels_path, &sect_pr, true, norm_type, &rel_id, &part_path)?;

    Ok(())
}

pub fn add_footer(package: &mut OxmlPackage, type_val: &str, content_xml: &str) -> Result<(), HandlerError> {
    let norm_type = normalize_type_val(type_val);
    let footer_idx = next_footer_index(package);
    let part_path = format!("word/footer{}.xml", footer_idx);
    let footer_xml = build_footer_xml(content_xml);

    package.write_part_xml(&part_path, &footer_xml)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    let doc_rels_path = "word/_rels/document.xml.rels";
    let rel_id = next_rel_id(package, doc_rels_path);
    let rel_target = format!("footer{}.xml", footer_idx);
    let rel_xml = format!(
        "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>",
        rel_id, FOOTER_TYPE, rel_target
    );
    inject_relationship(package, doc_rels_path, &rel_xml)?;

    ensure_default_content_type(package, "xml", "application/xml")?;
    let override_xml = format!(
        "<Override PartName=\"/word/footer{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>",
        footer_idx
    );
    let ct_xml = package.read_part_xml("[Content_Types].xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let part_ref = format!("PartName=\"/word/footer{}.xml\"", footer_idx);
    if !ct_xml.contains(&part_ref) {
        let types_start = ct_xml.find("<Types").unwrap_or(0);
        let new_ct_xml = if let Some(close) = ct_xml[types_start..].find('>') {
            let insert_at = types_start + close + 1;
            let mut r = ct_xml;
            r.insert_str(insert_at, &override_xml);
            r
        } else {
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">{}</Types>", override_xml)
        };
        package.write_part_xml("[Content_Types].xml", &new_ct_xml)
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    }

    let doc_xml = package.read_part_xml("word/document.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let sect_pr = extract_last_sect_pr(&doc_xml);
    add_header_footer_reference(package, doc_rels_path, &sect_pr, false, norm_type, &rel_id, &part_path)?;

    Ok(())
}

fn extract_last_sect_pr(doc_xml: &str) -> String {
    if let Some(pos) = doc_xml.rfind("<w:sectPr") {
        let tag_end = doc_xml[pos..].find('>').map(|e| pos + e + 1).unwrap_or(pos);
        let close = doc_xml[tag_end..].find("</w:sectPr>")
            .map(|e| tag_end + e + 11)
            .unwrap_or(doc_xml.len());
        doc_xml[pos..close].to_string()
    } else {
        String::new()
    }
}

fn remove_header_footer_ref(
    package: &mut OxmlPackage,
    type_val: &str,
    is_header: bool,
) -> Result<(), HandlerError> {
    let norm_type = normalize_type_val(type_val);
    let doc_rels_path = "word/_rels/document.xml.rels";
    let doc_xml = package.read_part_xml("word/document.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let sect_pr = extract_last_sect_pr(&doc_xml);
    if sect_pr.is_empty() {
        return Err(HandlerError::OperationFailed("no sectPr found in document".to_string()));
    }

    let ref_attr = map_type_to_ref_attr(norm_type);
    let ref_tag = if is_header { "w:headerReference" } else { "w:footerReference" };

    let needle = format!("{} w:type=\"{}\"", ref_tag, ref_attr);
    let rid_needle = "r:id=\"";
    let rid_start = sect_pr.find(&needle).and_then(|pos| {
        let after = &sect_pr[pos..];
        after.find(rid_needle).map(|r| pos + r + rid_needle.len())
    });

    let rel_id = rid_start.and_then(|start| {
        let rest = &sect_pr[start..];
        rest.find('"').map(|end| &sect_pr[start..start + end])
    }).map(|s| s.to_string());

    let new_sect_pr = if is_header {
        remove_existing_header_ref(&sect_pr, norm_type)
    } else {
        remove_existing_footer_ref(&sect_pr, norm_type)
    };

    let new_doc_xml = if let Some(pos) = doc_xml.rfind("<w:sectPr") {
        let tag_end = doc_xml[pos..].find('>').map(|e| pos + e + 1).unwrap_or(pos);
        let sect_pr_close = doc_xml[tag_end..].find("</w:sectPr>")
            .map(|e| tag_end + e)
            .unwrap_or(doc_xml.len());
        let close_tag = "</w:sectPr>";
        format!("{}{}{}", &doc_xml[..pos], new_sect_pr, &doc_xml[(sect_pr_close + close_tag.len())..])
    } else {
        doc_xml.to_string()
    };
    package.write_part_xml("word/document.xml", &new_doc_xml)
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;

    if let Some(rid) = rel_id {
        let rels_xml = package.read_part_xml(doc_rels_path).unwrap_or_default();
        let rel_needle = format!("Id=\"{}\"", rid);
        let rel_start = rels_xml.find(&rel_needle);
        if let Some(start) = rel_start {
            let before = &rels_xml[..start];
            let after = &rels_xml[start..];
            let tag_start = before.rfind('<').unwrap_or(0);
            let tag_end = after.find("/>").map(|e| start + e + 2).unwrap_or(rels_xml.len());
            let mut new_rels = rels_xml;
            new_rels.replace_range(tag_start..tag_end, "");
            package.write_part_xml(doc_rels_path, &new_rels)
                .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
        }
    }

    Ok(())
}

pub fn remove_header(package: &mut OxmlPackage, type_val: &str) -> Result<(), HandlerError> {
    remove_header_footer_ref(package, type_val, true)
}

pub fn remove_footer(package: &mut OxmlPackage, type_val: &str) -> Result<(), HandlerError> {
    remove_header_footer_ref(package, type_val, false)
}

pub fn list_headers(package: &OxmlPackage) -> Result<Vec<HeaderFooterInfo>, HandlerError> {
    let doc_xml = package.read_part_xml("word/document.xml")
        .map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
    let mut results = Vec::new();

    let rels_xml = package.read_part_xml("word/_rels/document.xml.rels")
        .unwrap_or_default();

    let sect_pr = extract_last_sect_pr(&doc_xml);
    if sect_pr.is_empty() {
        return Ok(results);
    }

    let ref_tags = ["w:headerReference", "w:footerReference"];
    for ref_tag in &ref_tags {
        let mut search_from = 0;
        while let Some(start_tag) = sect_pr[search_from..].find(ref_tag) {
            let abs_start = search_from + start_tag;
            let tag_end = sect_pr[abs_start..].find("/>")
                .map(|e| abs_start + e + 2)
                .unwrap_or(sect_pr.len());
            let fragment = &sect_pr[abs_start..tag_end];
            let type_val = extract_attr(fragment, "w:type").unwrap_or_else(|| "default".to_string());
            let rid = extract_attr(fragment, "r:id").unwrap_or_default();

            let part_type = if *ref_tag == "w:headerReference" { "header" } else { "footer" };

            let part_path = if !rid.is_empty() {
                find_part_path_from_rel(&rels_xml, &rid)
            } else {
                None
            };

            let content = part_path.as_ref()
                .and_then(|p| package.read_part_xml(p).ok())
                .unwrap_or_default();

            results.push(HeaderFooterInfo {
                part_type: part_type.to_string(),
                type_val: normalize_type_val(&type_val).to_string(),
                part_path: part_path.unwrap_or_default(),
                content,
            });

            search_from = tag_end;
        }
    }

    Ok(results)
}

fn extract_attr(xml: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = xml.find(&needle)?;
    let val_start = start + needle.len();
    let val_end = xml[val_start..].find('"')?;
    Some(xml[val_start..val_start + val_end].to_string())
}

fn find_part_path_from_rel(rels_xml: &str, rid: &str) -> Option<String> {
    let rid_needle = format!("Id=\"{}\"", rid);
    let pos = rels_xml.find(&rid_needle)?;
    let target_needle = "Target=\"";
    let target_start = rels_xml[pos..].find(target_needle)?;
    let abs_target_start = pos + target_start + target_needle.len();
    let target_end = rels_xml[abs_target_start..].find('"')?;
    let target = &rels_xml[abs_target_start..abs_target_start + target_end];
    Some(if target.starts_with("media/") || target.starts_with("header") || target.starts_with("footer") {
        format!("word/{}", target)
    } else {
        target.to_string()
    })
}
