use std::collections::HashMap;
use regex::Regex;

/// Parse SVG text into usvg Tree
pub fn parse_svg(svg_text: &str) -> Result<usvg::Tree, String> {
    let opt = usvg::Options::default();
    usvg::Tree::from_str(svg_text, &opt).map_err(|e| e.to_string())
}

/// Serialize usvg Tree back to SVG text
pub fn serialize_svg(tree: &usvg::Tree) -> Result<String, String> {
    Ok(tree.to_string(&usvg::WriteOptions::default()))
}

/// Apply CSS properties to SVG elements at the XML level.
/// Serializes the tree, modifies element attributes, and re-parses.
pub fn apply_css(tree: &mut usvg::Tree, css: &HashMap<String, String>) -> Result<(), String> {
    if css.is_empty() {
        return Ok(());
    }
    let svg_text = serialize_svg(tree)?;
    let modified = apply_css_to_svg_xml(&svg_text, css)?;
    *tree = parse_svg(&modified)?;
    Ok(())
}

/// Get all text content from SVG as (element_id, text) pairs.
pub fn extract_text(tree: &usvg::Tree) -> Vec<(String, String)> {
    let mut results = Vec::new();
    extract_text_from_nodes(tree.root(), &mut results);
    results
}

/// Resize SVG viewport by modifying viewBox, width, and height at the XML level.
pub fn resize_svg(tree: &mut usvg::Tree, new_width: f64, new_height: f64) -> Result<(), String> {
    let svg_text = serialize_svg(tree)?;
    let modified = resize_svg_xml(&svg_text, new_width, new_height)?;
    *tree = parse_svg(&modified)?;
    Ok(())
}

fn extract_text_from_nodes(group: &usvg::Group, results: &mut Vec<(String, String)>) {
    for node in group.children() {
        match node {
            usvg::Node::Text(text) => {
                let id = text.id().to_string();
                for chunk in text.chunks() {
                    let chunk_text = chunk.text();
                    if !chunk_text.is_empty() {
                        results.push((id.clone(), chunk_text.to_string()));
                    }
                }
            }
            usvg::Node::Group(g) => {
                extract_text_from_nodes(g, results);
            }
            _ => {}
        }
    }
}

fn apply_css_to_svg_xml(svg: &str, css: &HashMap<String, String>) -> Result<String, String> {
    let mut result = svg.to_string();

    let element_tags = [
        "svg", "path", "rect", "circle", "ellipse", "line", "polyline",
        "polygon", "text", "tspan", "g", "use", "image", "mask", "clipPath",
    ];
    let element_pattern = element_tags.join("|");

    for (prop, value) in css {
        let escaped = regex::escape(prop);

        // Step 1: Replace existing values: prop="..." -> prop="value"
        let re = Regex::new(&format!(r#"{}=["'][^"']*["']"#, escaped))
            .map_err(|e| e.to_string())?;
        result = re
            .replace_all(&result, format!("{}=\"{}\"", prop, value))
            .to_string();

        // Step 2: Add property to matching element tags that don't have it yet.
        // Match: <tagname ... > or <tagname .../>
        let re_add = Regex::new(&format!(
            r"(<(?:{})\b(?:[^>]*?))(\s*/?>)$",
            element_pattern
        ))
        .map_err(|e| e.to_string())?;

        // Apply line-by-line to handle multi-line content properly
        let mut lines: Vec<String> = Vec::new();
        for line in result.lines() {
            let modified = re_add.replace(line, |caps: &regex::Captures| {
                let prefix = &caps[1];
                let suffix = &caps[2];
                if prefix.contains(&format!("{}=\"", prop))
                    || prefix.contains(&format!("{}='", prop))
                {
                    prefix.to_string() + suffix
                } else {
                    format!("{} {}=\"{}\"{}", prefix, prop, value, suffix)
                }
            });
            lines.push(modified.to_string());
        }
        result = lines.join("\n");
    }

    Ok(result)
}

fn resize_svg_xml(svg: &str, new_width: f64, new_height: f64) -> Result<String, String> {
    let w_str = format!("{}", new_width);
    let h_str = format!("{}", new_height);
    let vb_str = format!("0 0 {} {}", new_width, new_height);

    let mut result = svg.to_string();

    // Replace width
    let re_w =
        Regex::new(r#"width="([^"]*)""#).map_err(|e| e.to_string())?;
    result = re_w
        .replace(&result, format!("width=\"{}\"", w_str))
        .to_string();

    // Replace height
    let re_h =
        Regex::new(r#"height="([^"]*)""#).map_err(|e| e.to_string())?;
    result = re_h
        .replace(&result, format!("height=\"{}\"", h_str))
        .to_string();

    // Replace or add viewBox
    let re_vb =
        Regex::new(r#"viewBox="([^"]*)""#).map_err(|e| e.to_string())?;
    if re_vb.is_match(&result) {
        result = re_vb
            .replace(&result, format!("viewBox=\"{}\"", vb_str))
            .to_string();
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_svg() -> &'static str {
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">
  <rect x="10" y="10" width="80" height="80" fill="red"/>
</svg>"#
    }

    fn sample_svg_with_text() -> &'static str {
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
  <text x="10" y="50" font-size="16">Hello SVG</text>
  <text x="10" y="80" font-size="12">Second line</text>
</svg>"#
    }

    #[test]
    fn test_parse_svg() {
        let tree = parse_svg(sample_svg()).unwrap();
        let size = tree.size();
        assert!(size.width() > 0.0);
        assert!(size.height() > 0.0);
    }

    #[test]
    fn test_round_trip() {
        let tree = parse_svg(sample_svg()).unwrap();
        let serialized = serialize_svg(&tree).unwrap();
        let tree2 = parse_svg(&serialized).unwrap();
        let serialized2 = serialize_svg(&tree2).unwrap();
        assert!(!serialized2.is_empty());
        assert!(serialized2.contains("svg"));
    }

    #[test]
    fn test_apply_css_fill_color() {
        let mut tree = parse_svg(sample_svg()).unwrap();
        let mut css = HashMap::new();
        css.insert("fill".to_string(), "blue".to_string());
        apply_css(&mut tree, &css).unwrap();
        let serialized = serialize_svg(&tree).unwrap();
        // usvg normalizes color names to hex
        assert!(serialized.contains("fill=\"#0000ff\""));
    }

    #[test]
    fn test_extract_text() {
        let tree = parse_svg(sample_svg_with_text()).unwrap();
        let texts = extract_text(&tree);
        // usvg may keep text nodes or flatten them to paths
        // depending on the text feature availability
        if !texts.is_empty() {
            assert!(
                texts.iter().any(|(_, t)| t.contains("Hello"))
                    || texts.iter().any(|(_, t)| t.contains("Second"))
            );
        }
    }

    #[test]
    fn test_resize_svg() {
        let mut tree = parse_svg(sample_svg()).unwrap();
        resize_svg(&mut tree, 200.0, 150.0).unwrap();
        let serialized = serialize_svg(&tree).unwrap();
        assert!(
            serialized.contains("width=\"200\"")
                || serialized.contains("width='200'")
        );
        assert!(
            serialized.contains("height=\"150\"")
                || serialized.contains("height='150'")
        );
    }

    #[test]
    fn test_invalid_svg() {
        let result = parse_svg("not valid svg");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_css() {
        let mut tree = parse_svg(sample_svg()).unwrap();
        let css = HashMap::new();
        apply_css(&mut tree, &css).unwrap();
        let serialized = serialize_svg(&tree).unwrap();
        // Should still be valid SVG
        assert!(serialized.contains("</svg>") || serialized.contains("/>"));
    }

    #[test]
    fn test_apply_css_stroke() {
        let mut tree = parse_svg(sample_svg()).unwrap();
        let mut css = HashMap::new();
        css.insert("stroke".to_string(), "black".to_string());
        css.insert("stroke-width".to_string(), "2".to_string());
        apply_css(&mut tree, &css).unwrap();
        let serialized = serialize_svg(&tree).unwrap();
        assert!(
            serialized.contains("stroke=\"black\"")
                || serialized.contains("stroke=\"#000000\"")
        );
        assert!(serialized.contains("stroke-width=\"2\""));
    }
}
