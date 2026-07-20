pub fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    cssparser::color::parse_hash_color(s.as_bytes())
        .ok()
        .map(|(r, g, b, _)| (r, g, b))
}

pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("{:02X}{:02X}{:02X}", r, g, b)
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    let af = a as f64;
    let bf = b as f64;
    (af + (bf - af) * t).round().clamp(0.0, 255.0) as u8
}

pub fn lighten(hex: &str, amount: f64) -> String {
    let (r, g, b) = hex_to_rgb(hex).unwrap_or((0, 0, 0));
    let t = amount.clamp(0.0, 1.0);
    rgb_to_hex(lerp(r, 255, t), lerp(g, 255, t), lerp(b, 255, t))
}

pub fn darken(hex: &str, amount: f64) -> String {
    let (r, g, b) = hex_to_rgb(hex).unwrap_or((0, 0, 0));
    let t = amount.clamp(0.0, 1.0);
    rgb_to_hex(lerp(r, 0, t), lerp(g, 0, t), lerp(b, 0, t))
}

pub fn mix(color1: &str, color2: &str, ratio: f64) -> String {
    let (r1, g1, b1) = hex_to_rgb(color1).unwrap_or((0, 0, 0));
    let (r2, g2, b2) = hex_to_rgb(color2).unwrap_or((0, 0, 0));
    let t = ratio.clamp(0.0, 1.0);
    rgb_to_hex(lerp(r1, r2, t), lerp(g1, g2, t), lerp(b1, b2, t))
}

fn luminance(r: u8, g: u8, b: u8) -> f64 {
    fn linearize(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

pub fn is_light(hex: &str) -> bool {
    hex_to_rgb(hex).map_or(false, |(r, g, b)| luminance(r, g, b) > 0.5)
}

pub fn complementary(hex: &str) -> String {
    let (r, g, b) = hex_to_rgb(hex).unwrap_or((0, 0, 0));
    rgb_to_hex(255 - r, 255 - g, 255 - b)
}

pub fn palette(hex: &str) -> Vec<String> {
    let (r, g, b) = hex_to_rgb(hex).unwrap_or((128, 128, 128));
    let steps: [f64; 5] = [-0.4, -0.2, 0.0, 0.2, 0.4];
    steps
        .iter()
        .map(|&t| {
            let _rf = (r as f64 + (255.0 - r as f64) * t.max(0.0)).round().clamp(0.0, 255.0) as u8;
            let _gf = (g as f64 + (255.0 - g as f64) * t.max(0.0)).round().clamp(0.0, 255.0) as u8;
            let _bf = (b as f64 + (255.0 - b as f64) * t.max(0.0)).round().clamp(0.0, 255.0) as u8;
            let (dr, dg, db) = if t < 0.0 {
                let rf = (r as f64 * (1.0 + t)).round().clamp(0.0, 255.0) as u8;
                let gf = (g as f64 * (1.0 + t)).round().clamp(0.0, 255.0) as u8;
                let bf = (b as f64 * (1.0 + t)).round().clamp(0.0, 255.0) as u8;
                (rf, gf, bf)
            } else {
                let rf = (r as f64 + (255.0 - r as f64) * t).round().clamp(0.0, 255.0) as u8;
                let gf = (g as f64 + (255.0 - g as f64) * t).round().clamp(0.0, 255.0) as u8;
                let bf = (b as f64 + (255.0 - b as f64) * t).round().clamp(0.0, 255.0) as u8;
                (rf, gf, bf)
            };
            rgb_to_hex(dr, dg, db)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgb_6digit() {
        assert_eq!(hex_to_rgb("#FF0000"), Some((255, 0, 0)));
        assert_eq!(hex_to_rgb("00FF00"), Some((0, 255, 0)));
    }

    #[test]
    fn test_hex_to_rgb_3digit() {
        assert_eq!(hex_to_rgb("#F00"), Some((255, 0, 0)));
        assert_eq!(hex_to_rgb("#0F0"), Some((0, 255, 0)));
    }

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(rgb_to_hex(255, 0, 0), "FF0000");
        assert_eq!(rgb_to_hex(0, 255, 0), "00FF00");
    }

    #[test]
    fn test_lighten() {
        assert_eq!(lighten("000000", 0.0), "000000");
        assert_eq!(lighten("000000", 1.0), "FFFFFF");
    }

    #[test]
    fn test_darken() {
        assert_eq!(darken("FFFFFF", 0.0), "FFFFFF");
        assert_eq!(darken("FFFFFF", 1.0), "000000");
    }

    #[test]
    fn test_mix() {
        assert_eq!(mix("FF0000", "0000FF", 0.0), "FF0000");
        assert_eq!(mix("FF0000", "0000FF", 1.0), "0000FF");
    }

    #[test]
    fn test_is_light() {
        assert!(is_light("FFFFFF"));
        assert!(!is_light("000000"));
    }

    #[test]
    fn test_complementary() {
        assert_eq!(complementary("FF0000"), "00FFFF");
        assert_eq!(complementary("000000"), "FFFFFF");
    }

    #[test]
    fn test_palette_len() {
        assert_eq!(palette("FF0000").len(), 5);
    }

    #[test]
    fn test_invalid_hex() {
        assert!(hex_to_rgb("xyz").is_none());
    }
}
