//! Style unsupported hints — surfaces curated guidance when a `set` operation
//! rejects properties that belong to the /styles tree. Matches the C#
//! StyleUnsupportedHints that recommends curated alternatives instead of
//! pushing users to the raw-set escape hatch.

/// Curated style properties that the Rust port recognizes at the document-level.
/// Mirrors the C# KnownProps list.
pub const KNOWN_STYLE_PROPS: &[&str] = &[
    "name",
    "basedOn",
    "next",
    "qFormat",
    "uiPriority",
    "hidden",
    "semiHidden",
    "unhideWhenUsed",
    "link",
    "locked",
    "pStyle",
    "rStyle",
    "tblStyle",
    // Paragraph-level
    "alignment",
    "jc",
    "indentLeft",
    "indentRight",
    "firstLine",
    "hanging",
    "spacingBefore",
    "spacingAfter",
    "lineSpacing",
    "keepLines",
    "keepNext",
    "pageBreakBefore",
    "widowControl",
    "outlineLevel",
    // Run-level
    "bold",
    "b",
    "italic",
    "i",
    "underline",
    "u",
    "strike",
    "font",
    "fontFamily",
    "size",
    "fontSize",
    "color",
    "fontColor",
    "bgColor",
    "highlight",
    "caps",
    "smallCaps",
    "vanish",
    "hidden",
    "kern",
    "spacing",
    "characterSpacing",
    // Table-level
    "tblStyle",
    "tblW",
    "tblInd",
    "tblLayout",
    "tblLook",
    "firstRow",
    "lastRow",
    "firstCol",
    "lastCol",
];

/// Format a list of unsupported property names with hints.
/// Returns None if there are no unsupported properties.
pub fn format(unsupported: &[String]) -> Option<String> {
    if unsupported.is_empty() {
        return None;
    }

    let mut hints = Vec::new();
    for prop in unsupported {
        if let Some(suggestion) = suggest_property(prop) {
            hints.push(format!("{} (did you mean: {}?)", prop, suggestion));
        } else {
            hints.push(prop.clone());
        }
    }

    Some(format!(
        "UNSUPPORTED props: {}. Use 'officecli help <format>-set' for available \
         properties, or use raw-set for direct XML manipulation.",
        hints.join(", ")
    ))
}

/// Suggest a close match using Levenshtein distance.
pub fn suggest_property(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    let mut best: Option<(String, usize)> = None;

    for &prop in KNOWN_STYLE_PROPS {
        let dist = levenshtein(&lower, &prop.to_lowercase());
        let max_dist = (input.len() / 3).max(2);
        if dist > 0 && dist <= max_dist {
            match &best {
                Some((_, cur_dist)) if dist >= *cur_dist => {}
                _ => best = Some((prop.to_string(), dist)),
            }
        }
    }

    best.map(|(s, _)| s)
}

fn levenshtein(s: &str, t: &str) -> usize {
    if s.is_empty() {
        return t.chars().count();
    }
    if t.is_empty() {
        return s.chars().count();
    }

    let s: Vec<char> = s.chars().collect();
    let t: Vec<char> = t.chars().collect();
    let (m, n) = (s.len(), t.len());

    let mut d = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in d[0].iter_mut().enumerate() {
        *val = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if s[i - 1] == t[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
        }
    }

    d[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_empty() {
        assert!(format(&[]).is_none());
    }

    #[test]
    fn test_format_with_unsupported() {
        let msg = format(&["xyz".to_string()]).unwrap();
        assert!(msg.contains("UNSUPPORTED props: xyz"));
    }

    #[test]
    fn test_suggest_property_typo() {
        assert_eq!(suggest_property("algnment").as_deref(), Some("alignment"));
        assert_eq!(suggest_property("blod").as_deref(), Some("bold"));
    }

    #[test]
    fn test_suggest_property_no_match() {
        assert_eq!(suggest_property("xyz123"), None);
    }
}
