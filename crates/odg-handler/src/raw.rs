use handler_common::ValidationError;
use lo_zip::archive::ZipArchive;
use lo_zip::ZipEntry;

pub fn get_raw_xml(odg_bytes: &[u8], part_path: &str) -> Result<String, String> {
    let zip = ZipArchive::new(odg_bytes).map_err(|e| format!("ZIP error: {}", e))?;
    let entry_path = match part_path {
        "/" | "" => "content.xml",
        p if p == "content.xml" || p == "styles.xml" || p == "meta.xml" => p,
        p => {
            let clean = p.trim_start_matches('/');
            if clean == "META-INF/manifest.xml" {
                "META-INF/manifest.xml"
            } else {
                clean
            }
        }
    };
    zip.read_string(entry_path)
        .map_err(|e| format!("failed to read '{}' from ODG: {}", entry_path, e))
}

pub fn set_raw_xml(odg_bytes: &mut Vec<u8>, part_path: &str, content: &str) -> Result<(), String> {
    let zip = ZipArchive::new(odg_bytes).map_err(|e| format!("ZIP error: {}", e))?;
    let entry_path = match part_path {
        "/" | "" => "content.xml",
        p if p == "content.xml" || p == "styles.xml" || p == "meta.xml" => p,
        p => {
            let clean = p.trim_start_matches('/');
            if clean == "META-INF/manifest.xml" {
                "META-INF/manifest.xml"
            } else {
                clean
            }
        }
    };

    let original_entries: Vec<ZipEntry> = zip
        .entries()
        .into_iter()
        .map(|name| {
            let data = zip.read(name).unwrap_or_default();
            if name == entry_path {
                ZipEntry::new(name, content.as_bytes().to_vec())
            } else {
                ZipEntry::new(name, data)
            }
        })
        .collect();

    let new_bytes = lo_zip::write_zip_to_vec(&original_entries)
        .map_err(|e| format!("failed to write ZIP: {}", e))?;
    *odg_bytes = new_bytes;
    Ok(())
}

pub fn validate(content_xml: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let doc = match roxmltree::Document::parse(content_xml) {
        Ok(d) => d,
        Err(e) => {
            errors.push(ValidationError {
                error_type: "xml-well-formed".to_string(),
                description: format!("content.xml is not well-formed: {}", e),
                path: None,
                part: Some("content.xml".to_string()),
            });
            return errors;
        }
    };

    let root = doc.root_element();
    let has_office_ns = root
        .namespaces()
        .iter()
        .any(|ns| ns.name() == Some("office") && ns.uri() == "urn:oasis:names:tc:opendocument:xmlns:office:1.0");
    if !has_office_ns {
        errors.push(ValidationError {
            error_type: "missing-office-namespace".to_string(),
            description: "Missing office namespace declaration".to_string(),
            path: Some("/office:document-content".to_string()),
            part: Some("content.xml".to_string()),
        });
    }

    let has_draw_ns = root
        .namespaces()
        .iter()
        .any(|ns| ns.name() == Some("draw") && ns.uri() == "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0");
    if !has_draw_ns {
        errors.push(ValidationError {
            error_type: "missing-draw-namespace".to_string(),
            description: "Missing draw namespace declaration".to_string(),
            path: Some("/office:document-content".to_string()),
            part: Some("content.xml".to_string()),
        });
    }

    let drawing_count = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "page")
        .count();
    if drawing_count == 0 {
        errors.push(ValidationError {
            error_type: "no-pages".to_string(),
            description: "Document contains no draw:page elements".to_string(),
            path: None,
            part: Some("content.xml".to_string()),
        });
    }

    let element_count = doc.descendants().filter(|n| n.is_element()).count();
    if element_count > 50000 {
        errors.push(ValidationError {
            error_type: "oversized".to_string(),
            description: format!("content.xml has {} elements (max recommended: 50000)", element_count),
            path: None,
            part: Some("content.xml".to_string()),
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_content_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
    xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
    office:version="1.2">
  <office:body>
    <office:drawing>
      <draw:page draw:name="page1"/>
    </office:drawing>
  </office:body>
</office:document-content>"#
    }

    #[test]
    fn test_validate_valid() {
        let errors = validate(sample_content_xml());
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_missing_ns() {
        let xml = r#"<?xml version="1.0"?>
<root xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0">
  <draw:page/>
</root>"#;
        let errors = validate(xml);
        assert!(errors.iter().any(|e| e.error_type == "missing-office-namespace"));
    }

    #[test]
    fn test_validate_no_pages() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    office:version="1.2">
  <office:body>
    <office:drawing>
    </office:drawing>
  </office:body>
</office:document-content>"#;
        let errors = validate(xml);
        assert!(errors.iter().any(|e| e.error_type == "no-pages"));
    }

    #[test]
    fn test_validate_malformed_xml() {
        let errors = validate("not xml");
        assert!(errors.iter().any(|e| e.error_type == "xml-well-formed"));
    }
}
