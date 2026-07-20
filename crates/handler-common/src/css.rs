use std::collections::HashMap;

const NAMED_COLORS: &[(&str, &str)] = &[
    ("red", "FF0000"),
    ("blue", "0000FF"),
    ("green", "008000"),
    ("yellow", "FFFF00"),
    ("white", "FFFFFF"),
    ("black", "000000"),
    ("gray", "808080"),
    ("grey", "808080"),
    ("orange", "FFA500"),
    ("purple", "800080"),
    ("pink", "FFC0CB"),
    ("brown", "A52A2A"),
    ("navy", "000080"),
    ("teal", "008080"),
    ("maroon", "800000"),
    ("lime", "00FF00"),
    ("aqua", "00FFFF"),
    ("silver", "C0C0C0"),
    ("fuchsia", "FF00FF"),
    ("indigo", "4B0082"),
    ("coral", "FF7F50"),
    ("crimson", "DC143C"),
    ("darkblue", "00008B"),
    ("darkgray", "A9A9A9"),
    ("darkgreen", "006400"),
    ("darkorange", "FF8C00"),
    ("darkred", "8B0000"),
    ("darkviolet", "9400D3"),
    ("gold", "FFD700"),
    ("lightblue", "ADD8E6"),
    ("lightgray", "D3D3D3"),
    ("lightgreen", "90EE90"),
    ("lightyellow", "FFFFE0"),
    ("skyblue", "87CEEB"),
    ("tomato", "FF6347"),
    ("turquoise", "40E0D0"),
    ("violet", "EE82EE"),
    ("wheat", "F5DEB3"),
    ("transparent", "none"),
];

pub fn css_named_color(name: &str) -> Option<&'static str> {
    let name = name.trim().to_lowercase();
    NAMED_COLORS
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
}

pub fn css_length_to_twips(value: &str) -> Option<i64> {
    parse_css_length(value).map(|(v, unit)| match unit {
        "pt" => (v * 20.0) as i64,
        "px" => (v * 15.0) as i64,
        "in" => (v * 1440.0) as i64,
        "cm" => (v * 567.0) as i64,
        "mm" => (v * 56.7) as i64,
        "pc" => (v * 240.0) as i64,
        "em" => (v * 240.0) as i64,
        _ => v as i64,
    })
}

pub fn css_length_to_half_points(value: &str) -> Option<i64> {
    parse_css_length(value).map(|(v, unit)| match unit {
        "pt" => (v * 2.0) as i64,
        "px" => (v * 1.5) as i64,
        "em" => (v * 24.0) as i64,
        _ => v as i64,
    })
}

fn parse_css_length(value: &str) -> Option<(f64, &str)> {
    let value = value.trim();
    let units = &[
        "pt", "px", "in", "cm", "mm", "pc", "em", "rem", "ex", "ch", "vw", "vh", "%",
    ];
    for unit in units {
        if let Some(stripped) = value.strip_suffix(unit) {
            let num = stripped.trim().parse::<f64>().ok()?;
            return Some((num, unit));
        }
    }
    value.parse::<f64>().ok().map(|v| (v, ""))
}

pub fn parse_css_color(color: &str) -> Option<String> {
    let c = color.trim();
    if c.is_empty() {
        return None;
    }
    if c.starts_with('#') {
        let hex = &c[1..];
        if hex.len() == 3 || hex.len() == 6 {
            if crate::color::hex_to_rgb(hex).is_some() {
                return Some(hex.to_uppercase());
            }
        }
        if hex.len() == 8 {
            let hex6 = &hex[..6];
            if crate::color::hex_to_rgb(hex6).is_some() {
                return Some(hex6.to_uppercase());
            }
        }
        return None;
    }
    if c.starts_with("rgb(") && c.ends_with(')') {
        return parse_rgb_function(c);
    }
    if c.starts_with("rgba(") && c.ends_with(')') {
        return parse_rgb_function(c);
    }
    if c.starts_with("hsl(") && c.ends_with(')') {
        return parse_hsl_function(c);
    }
    if c.starts_with("hsla(") && c.ends_with(')') {
        return parse_hsl_function(c);
    }
    css_named_color(c).map(|s| s.to_string())
}

fn parse_rgb_function(input: &str) -> Option<String> {
    let paren = input.find('(')?;
    let inner = input[paren + 1..input.len() - 1].trim();
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let r = parse_rgb_component(parts[0].trim())?;
    let g = parse_rgb_component(parts[1].trim())?;
    let b = parse_rgb_component(parts[2].trim())?;
    Some(crate::color::rgb_to_hex(r, g, b))
}

fn parse_rgb_component(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.ends_with('%') {
        let pct = s[..s.len() - 1].parse::<f64>().ok()?;
        return Some((pct.clamp(0.0, 100.0) / 100.0 * 255.0).round() as u8);
    }
    s.parse::<u8>().ok()
}

fn parse_hsl_function(input: &str) -> Option<String> {
    let paren = input.find('(')?;
    let inner = input[paren + 1..input.len() - 1].trim();
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let h = parts[0].trim().parse::<f64>().ok()? % 360.0;
    let s = parse_percentage(parts[1].trim())? / 100.0;
    let l = parse_percentage(parts[2].trim())? / 100.0;
    let (r, g, b) = hsl_to_rgb(h / 360.0, s, l);
    Some(crate::color::rgb_to_hex(r, g, b))
}

fn parse_percentage(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.ends_with('%') {
        s[..s.len() - 1].parse::<f64>().ok()
    } else {
        s.parse::<f64>().ok().map(|v| v * 100.0)
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }
    let hue_to_rgb = |p: f64, q: f64, mut t: f64| -> f64 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

pub fn parse_css(css: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for decl in css.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        let colon = match decl.find(':') {
            Some(c) => c,
            None => continue,
        };
        let prop = decl[..colon].trim();
        let value = decl[colon + 1..].trim();
        if prop.is_empty() || value.is_empty() {
            continue;
        }
        let normalized = normalize_property(prop);
        process_property(&normalized, value, &mut result);
    }
    result
}

fn normalize_property(prop: &str) -> String {
    let trimmed = prop.trim().replace('_', " ").replace('-', " ");
    let mut result = String::with_capacity(trimmed.len() + 4);
    for c in trimmed.chars() {
        if c.is_uppercase() {
            result.push('-');
            for lower in c.to_lowercase() {
                result.push(lower);
            }
        } else if c == ' ' {
            result.push('-');
        } else {
            result.push(c);
        }
    }
    result
}

fn process_property(normalized: &str, value: &str, result: &mut HashMap<String, String>) {
    let v = value.trim();
    match normalized {
        "font-weight" => {
            match v {
                "bold" => {
                    result.insert("bold".to_string(), "true".to_string());
                }
                "normal" => {
                    result.insert("bold".to_string(), "false".to_string());
                }
                _ => {
                    if let Ok(num) = v.parse::<i32>() {
                        if num >= 700 {
                            result.insert("bold".to_string(), "true".to_string());
                        } else if num <= 400 {
                            result.insert("bold".to_string(), "false".to_string());
                        }
                    }
                }
            }
        },
        "font-style" => match v {
            "italic" => {
                result.insert("italic".to_string(), "true".to_string());
            }
            "normal" => {
                result.insert("italic".to_string(), "false".to_string());
            }
            _ => {}
        },
        "font-size" => {
            if let Some(hp) = css_length_to_half_points(v) {
                result.insert("size".to_string(), hp.to_string());
            }
        }
        "color" => {
            if let Some(hex) = parse_css_color(v) {
                result.insert("color".to_string(), hex);
            }
        }
        "font-family" => {
            let first = v.split(',').next().unwrap_or(v).trim().trim_matches('\'');
            if !first.is_empty() {
                result.insert("font".to_string(), first.to_string());
            }
        }
        "text-decoration" => {
            let parts: Vec<&str> = v.split_whitespace().collect();
            for part in parts {
                match part {
                    "underline" => {
                        result.insert("underline".to_string(), "single".to_string());
                    }
                    "line-through" => {
                        result.insert("strike".to_string(), "true".to_string());
                    }
                    _ => {}
                }
            }
        }
        "text-align" => {
            result.insert("alignment".to_string(), v.to_string());
        }
        "vertical-align" => match v {
            "super" => {
                result.insert("vertAlign".to_string(), "superscript".to_string());
            }
            "sub" => {
                result.insert("vertAlign".to_string(), "subscript".to_string());
            }
            _ => {
                result.insert("valign".to_string(), v.to_string());
            }
        },
        "background" | "background-color" => {
            if let Some(hex) = parse_css_color(v) {
                result.insert("shading".to_string(), hex.clone());
                result.insert("fill".to_string(), hex);
            }
        }
        "border" => {
            parse_border_shorthand(v, None, result);
        }
        "border-top" => {
            parse_border_shorthand(v, Some("top"), result);
        }
        "border-bottom" => {
            parse_border_shorthand(v, Some("bottom"), result);
        }
        "border-left" => {
            parse_border_shorthand(v, Some("left"), result);
        }
        "border-right" => {
            parse_border_shorthand(v, Some("right"), result);
        }
        "width" => {
            if let Some(twips) = css_length_to_twips(v) {
                result.insert("width".to_string(), twips.to_string());
            }
        }
        "height" => {
            if let Some(twips) = css_length_to_twips(v) {
                result.insert("height".to_string(), twips.to_string());
            }
        }
        "padding" => {
            let twips: Vec<i64> = v
                .split_whitespace()
                .filter_map(|p| css_length_to_twips(p))
                .collect();
            if twips.is_empty() {
                return;
            }
            let (top, right, bottom, left) = match twips.len() {
                1 => (twips[0], twips[0], twips[0], twips[0]),
                2 => (twips[0], twips[1], twips[0], twips[1]),
                3 => (twips[0], twips[1], twips[2], twips[1]),
                4 => (twips[0], twips[1], twips[2], twips[3]),
                _ => return,
            };
            result.insert(
                "cellMargins".to_string(),
                format!("top={};bottom={};left={};right={}", top, bottom, left, right),
            );
        }
        "margin" => {
            if let Some(twips) = css_length_to_twips(v) {
                result.insert("spacing".to_string(), twips.to_string());
            }
        }
        "opacity" => {
            if let Ok(op) = v.parse::<f64>() {
                result.insert("opacity".to_string(), format!("{:.2}", op.clamp(0.0, 1.0)));
            }
        }
        "font-variant" => {
            if v == "small-caps" {
                result.insert("smallCaps".to_string(), "true".to_string());
            }
        }
        "text-transform" => {
            if v == "uppercase" {
                result.insert("caps".to_string(), "true".to_string());
            }
        }
        "white-space" => {
            if v == "nowrap" {
                result.insert("wrap".to_string(), "false".to_string());
            }
        }
        "line-height" => {
            if let Ok(num) = v.parse::<f64>() {
                let spacing = (num * 240.0) as i64;
                result.insert("spacing".to_string(), spacing.to_string());
            } else if let Some(twips) = css_length_to_twips(v) {
                result.insert("spacing".to_string(), twips.to_string());
            }
        }
        "letter-spacing" => {
            if let Some(twips) = css_length_to_twips(v) {
                result.insert("characterSpacing".to_string(), twips.to_string());
            }
        }
        _ => {
            result.insert(normalized.to_string(), v.to_string());
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn parse_border_shorthand(
    value: &str,
    side: Option<&str>,
    result: &mut HashMap<String, String>,
) {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    let mut color = String::new();
    let mut size = String::from("4");
    let mut style = "none";

    for part in &parts {
        if let Some(hex) = parse_css_color(part) {
            color = hex;
        } else if matches!(
            *part,
            "solid"
                | "dotted"
                | "dashed"
                | "double"
                | "none"
                | "hidden"
                | "groove"
                | "ridge"
                | "inset"
                | "outset"
        ) {
            style = "single";
        } else if let Some(twips) = css_length_to_twips(part) {
            let eighth_pts = (twips * 8 / 20).max(1);
            size = eighth_pts.to_string();
        } else {
            match *part {
                "thin" => size = "2".to_string(),
                "medium" => size = "4".to_string(),
                "thick" => size = "6".to_string(),
                _ => {}
            }
        }
    }

    let border_value = format!("color={};size={};space=1;val={}", color, size, style);

    if let Some(s) = side {
        let key = format!("border{}", capitalize(s));
        result.insert(key, border_value);
        // backward-compat old keys
        result.insert(format!("border{}", s), style.to_string());
        if !color.is_empty() {
            result.insert(format!("borderColor{}", s), color);
        }
    } else {
        for s in &["Top", "Bottom", "Left", "Right"] {
            result.insert(format!("border{}", s), border_value.clone());
        }
        // backward-compat old keys
        if style == "single" {
            result.insert("border".to_string(), "single".to_string());
        }
        if !color.is_empty() {
            result.insert("borderColor".to_string(), color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_colors() {
        assert_eq!(css_named_color("red"), Some("FF0000"));
        assert_eq!(css_named_color("RED"), Some("FF0000"));
        assert_eq!(css_named_color("transparent"), Some("none"));
        assert_eq!(css_named_color("unknown"), None);
    }

    #[test]
    fn test_parse_css_color_hex() {
        assert_eq!(parse_css_color("#FF0000"), Some("FF0000".to_string()));
        assert_eq!(parse_css_color("#f00"), Some("F00".to_string()));
    }

    #[test]
    fn test_parse_css_color_named() {
        assert_eq!(parse_css_color("red"), Some("FF0000".to_string()));
    }

    #[test]
    fn test_parse_css_color_rgb() {
        assert_eq!(
            parse_css_color("rgb(255, 0, 0)"),
            Some("FF0000".to_string())
        );
    }

    #[test]
    fn test_parse_css_color_rgba() {
        assert_eq!(
            parse_css_color("rgba(255, 0, 0, 0.5)"),
            Some("FF0000".to_string())
        );
    }

    #[test]
    fn test_parse_css_color_hsl() {
        let result = parse_css_color("hsl(0, 100%, 50%)");
        assert_eq!(result, Some("FF0000".to_string()));
    }

    #[test]
    fn test_css_length_to_twips() {
        assert_eq!(css_length_to_twips("1pt"), Some(20));
        assert_eq!(css_length_to_twips("1px"), Some(15));
        assert_eq!(css_length_to_twips("1in"), Some(1440));
    }

    #[test]
    fn test_css_length_to_half_points() {
        assert_eq!(css_length_to_half_points("12pt"), Some(24));
        assert_eq!(css_length_to_half_points("16px"), Some(24));
    }

    #[test]
    fn test_parse_css_font_weight() {
        let m = parse_css("font-weight: bold");
        assert_eq!(m.get("bold").map(|s| s.as_str()), Some("true"));
        let m = parse_css("font-weight: 400");
        assert_eq!(m.get("bold").map(|s| s.as_str()), Some("false"));
    }

    #[test]
    fn test_parse_css_font_style() {
        let m = parse_css("font-style: italic");
        assert_eq!(m.get("italic").map(|s| s.as_str()), Some("true"));
    }

    #[test]
    fn test_parse_css_font_size() {
        let m = parse_css("font-size: 12pt");
        assert_eq!(m.get("size").map(|s| s.as_str()), Some("24"));
    }

    #[test]
    fn test_parse_css_color_prop() {
        let m = parse_css("color: #FF0000");
        assert_eq!(m.get("color").map(|s| s.as_str()), Some("FF0000"));
    }

    #[test]
    fn test_parse_css_font_family() {
        let m = parse_css("font-family: Arial, sans-serif");
        assert_eq!(m.get("font").map(|s| s.as_str()), Some("Arial"));
    }

    #[test]
    fn test_parse_css_text_decoration() {
        let m = parse_css("text-decoration: underline line-through");
        assert_eq!(m.get("underline").map(|s| s.as_str()), Some("single"));
        assert_eq!(m.get("strike").map(|s| s.as_str()), Some("true"));
    }

    #[test]
    fn test_parse_css_text_align() {
        let m = parse_css("text-align: center");
        assert_eq!(m.get("alignment").map(|s| s.as_str()), Some("center"));
    }

    #[test]
    fn test_parse_css_background() {
        let m = parse_css("background: #FFF");
        assert!(m.contains_key("shading"));
        assert!(m.contains_key("fill"));
    }

    #[test]
    fn test_parse_css_border() {
        let m = parse_css("border: 1px solid red");
        assert_eq!(m.get("border").map(|s| s.as_str()), Some("single"));
        assert_eq!(m.get("borderColor").map(|s| s.as_str()), Some("FF0000"));
        assert_eq!(
            m.get("borderTop").map(|s| s.as_str()),
            Some("color=FF0000;size=6;space=1;val=single")
        );
    }

    #[test]
    fn test_parse_css_margin() {
        let m = parse_css("margin: 10px");
        assert_eq!(m.get("spacing").map(|s| s.as_str()), Some("150"));
    }

    #[test]
    fn test_parse_css_opacity() {
        let m = parse_css("opacity: 0.5");
        assert_eq!(m.get("opacity").map(|s| s.as_str()), Some("0.50"));
    }

    #[test]
    fn test_parse_css_multiple() {
        let m = parse_css("font-weight: bold; font-size: 14pt; color: blue");
        assert_eq!(m.get("bold").map(|s| s.as_str()), Some("true"));
        assert_eq!(m.get("size").map(|s| s.as_str()), Some("28"));
        assert_eq!(m.get("color").map(|s| s.as_str()), Some("0000FF"));
    }

    #[test]
    fn test_parse_css_empty() {
        let m = parse_css("");
        assert!(m.is_empty());
    }

    #[test]
    fn test_parse_css_malformed() {
        let m = parse_css("no-colon");
        assert!(m.is_empty());
    }

    #[test]
    fn test_css_length_to_twips_cm() {
        let result = css_length_to_twips("1cm");
        assert!(result.is_some());
    }

    #[test]
    fn test_css_line_height_number() {
        let m = parse_css("line-height: 1.5");
        assert_eq!(m.get("spacing").map(|s| s.as_str()), Some("360"));
    }

    #[test]
    fn test_css_letter_spacing() {
        let m = parse_css("letter-spacing: 1px");
        assert_eq!(m.get("characterSpacing").map(|s| s.as_str()), Some("15"));
    }

    #[test]
    fn test_parse_css_border_top() {
        let m = parse_css("border-top: 1px solid red");
        assert_eq!(
            m.get("borderTop").map(|s| s.as_str()),
            Some("color=FF0000;size=6;space=1;val=single")
        );
        assert_eq!(m.get("bordertop").map(|s| s.as_str()), Some("single"));
        assert_eq!(m.get("borderColortop").map(|s| s.as_str()), Some("FF0000"));
    }

    #[test]
    fn test_css_font_weight_numeric() {
        let m = parse_css("font-weight: 700");
        assert_eq!(m.get("bold").map(|s| s.as_str()), Some("true"));
        let m = parse_css("font-weight: 400");
        assert_eq!(m.get("bold").map(|s| s.as_str()), Some("false"));
        let m = parse_css("font-weight: 900");
        assert_eq!(m.get("bold").map(|s| s.as_str()), Some("true"));
        let m = parse_css("font-weight: 300");
        assert_eq!(m.get("bold").map(|s| s.as_str()), Some("false"));
    }

    #[test]
    fn test_css_vertical_align() {
        let m = parse_css("vertical-align: super");
        assert_eq!(
            m.get("vertAlign").map(|s| s.as_str()),
            Some("superscript")
        );
        let m = parse_css("vertical-align: sub");
        assert_eq!(m.get("vertAlign").map(|s| s.as_str()), Some("subscript"));
        let m = parse_css("vertical-align: top");
        assert_eq!(m.get("valign").map(|s| s.as_str()), Some("top"));
    }

    #[test]
    fn test_css_padding() {
        let m = parse_css("padding: 10px");
        assert_eq!(
            m.get("cellMargins").map(|s| s.as_str()),
            Some("top=150;bottom=150;left=150;right=150")
        );
    }

    #[test]
    fn test_css_line_height_length() {
        let m = parse_css("line-height: 20pt");
        assert_eq!(m.get("spacing").map(|s| s.as_str()), Some("400"));
    }
}
