use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FontInfo {
    pub name: String,
    pub family: String,
    pub style: String,
    pub weight: u16,
    pub file_path: String,
}

pub fn init_font_db() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    db
}

pub fn find_font(
    db: &fontdb::Database,
    family: &str,
    style: Option<&str>,
    weight: Option<u16>,
) -> Option<FontInfo> {
    let families = vec![fontdb::Family::Name(family)];
    let style_val = match style {
        Some("italic") => fontdb::Style::Italic,
        Some("oblique") => fontdb::Style::Oblique,
        _ => fontdb::Style::Normal,
    };
    let weight_val = fontdb::Weight(weight.unwrap_or(400));

    let query = fontdb::Query {
        families: &families,
        weight: weight_val,
        stretch: fontdb::Stretch::Normal,
        style: style_val,
    };

    let face_id = db.query(&query)?;
    let face = db.face(face_id)?;

    let file_path = match &face.source {
        fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => {
            path.to_string_lossy().to_string()
        }
        fontdb::Source::Binary(_) => String::new(),
    };

    Some(FontInfo {
        name: face.post_script_name.clone(),
        family: face
            .families
            .first()
            .map(|(n, _)| n.clone())
            .unwrap_or_default(),
        style: match face.style {
            fontdb::Style::Normal => "normal",
            fontdb::Style::Italic => "italic",
            fontdb::Style::Oblique => "oblique",
        }
        .to_string(),
        weight: face.weight.0,
        file_path,
    })
}

pub fn list_font_families(db: &fontdb::Database) -> Vec<String> {
    let mut families: Vec<String> = db
        .faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    families.sort();
    families.dedup();
    families
}

pub fn list_family_variants(db: &fontdb::Database, family: &str) -> Vec<FontInfo> {
    db.faces()
        .filter(|face| face.families.iter().any(|(name, _)| name == family))
        .map(|face| {
            let file_path = match &face.source {
                fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => {
                    path.to_string_lossy().to_string()
                }
                fontdb::Source::Binary(_) => String::new(),
            };
            FontInfo {
                name: face.post_script_name.clone(),
                family: face
                    .families
                    .first()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_default(),
                style: match face.style {
                    fontdb::Style::Normal => "normal",
                    fontdb::Style::Italic => "italic",
                    fontdb::Style::Oblique => "oblique",
                }
                .to_string(),
                weight: face.weight.0,
                file_path,
            }
        })
        .collect()
}

pub fn read_font_bytes(info: &FontInfo) -> Result<Vec<u8>, String> {
    std::fs::read(&info.file_path).map_err(|e| format!("Failed to read font file: {}", e))
}

pub fn get_font_metrics(font_bytes: &[u8]) -> Result<HashMap<String, f64>, String> {
    let face = ttf_parser::Face::parse(font_bytes, 0)
        .map_err(|e| format!("Failed to parse font: {:?}", e))?;
    let metrics = HashMap::from([
        ("units_per_em".to_string(), face.units_per_em() as f64),
        ("ascent".to_string(), face.ascender() as f64),
        ("descent".to_string(), face.descender() as f64),
        ("glyph_count".to_string(), face.number_of_glyphs() as f64),
    ]);
    Ok(metrics)
}

pub fn font_supports_char(font_bytes: &[u8], ch: char) -> bool {
    ttf_parser::Face::parse(font_bytes, 0)
        .ok()
        .and_then(|face| face.glyph_index(ch))
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_metrics_invalid_bytes() {
        let result = get_font_metrics(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_font_supports_char_empty_bytes() {
        assert!(!font_supports_char(&[], 'A'));
    }

    #[test]
    fn test_ttf_parser_parse_fails_on_empty() {
        let result = ttf_parser::Face::parse(&[], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_font_info_defaults() {
        let info = FontInfo {
            name: "Test".to_string(),
            family: "Test Family".to_string(),
            style: "normal".to_string(),
            weight: 400,
            file_path: "/nonexistent/font.ttf".to_string(),
        };
        assert_eq!(info.name, "Test");
        assert_eq!(info.weight, 400);
        assert!(read_font_bytes(&info).is_err());
    }

    #[test]
    #[ignore]
    fn test_init_font_db() {
        let db = init_font_db();
        let families = list_font_families(&db);
        assert!(!families.is_empty());
    }

    #[test]
    #[ignore]
    fn test_find_system_font() {
        let db = init_font_db();
        let result = find_font(&db, "DejaVu Sans", None, None);
        assert!(result.is_some());
    }

    #[test]
    #[ignore]
    fn test_list_family_variants_with_system() {
        let db = init_font_db();
        let families = list_font_families(&db);
        if let Some(family) = families.first() {
            let variants = list_family_variants(&db, family);
            assert!(!variants.is_empty());
        }
    }
}
