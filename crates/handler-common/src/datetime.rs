use chrono::NaiveDate;

/// Parse a date string in various formats (ISO 8601, "2024-01-15", "January 15, 2024", "15/01/2024", etc.)
pub fn parse_date(input: &str) -> Result<NaiveDate, String> {
    let input = input.trim();

    if let Ok(d) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%Y/%m/%d") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%B %d, %Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%B %d %Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%b %d, %Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%b %d %Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%d/%m/%Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%m/%d/%Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%d-%m-%Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%m-%d-%Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%d.%m.%Y") {
        return Ok(d);
    }

    Err(format!("Unable to parse date: {}", input))
}

/// Format a date to a given OOXML-compatible string.
/// format_code examples: "YYYY-MM-DD", "DD/MM/YYYY", "MMMM DD, YYYY", "ddd, MMM DD"
pub fn format_date(date: &NaiveDate, format_code: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = format_code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if i + 4 <= len {
            let four: String = chars[i..i + 4].iter().collect();
            if four == "YYYY" {
                result.push_str(&date.format("%Y").to_string());
                i += 4;
                continue;
            }
            if four == "dddd" {
                result.push_str(&date.format("%A").to_string());
                i += 4;
                continue;
            }
            if four == "MMMM" {
                result.push_str(&date.format("%B").to_string());
                i += 4;
                continue;
            }
        }
        if i + 3 <= len {
            let three: String = chars[i..i + 3].iter().collect();
            if three == "ddd" {
                result.push_str(&date.format("%a").to_string());
                i += 3;
                continue;
            }
            if three == "MMM" {
                result.push_str(&date.format("%b").to_string());
                i += 3;
                continue;
            }
        }
        if i + 2 <= len {
            let two: String = chars[i..i + 2].iter().collect();
            match two.as_str() {
                "YY" => {
                    result.push_str(&date.format("%y").to_string());
                    i += 2;
                    continue;
                }
                "MM" => {
                    result.push_str(&date.format("%m").to_string());
                    i += 2;
                    continue;
                }
                "DD" => {
                    result.push_str(&date.format("%d").to_string());
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        match chars[i] {
            'Y' => {
                result.push_str(&date.format("%Y").to_string());
            }
            'M' => {
                result.push_str(&date.format("%-m").to_string());
            }
            'D' => {
                result.push_str(&date.format("%-d").to_string());
            }
            c => {
                result.push(c);
            }
        }
        i += 1;
    }

    result
}

/// Parse a duration string like "+30 days" or "-7d" or "+1m" or "+1y" and return
/// the number of days to add/subtract.
pub fn parse_duration(delta: &str) -> Result<chrono::Duration, String> {
    let delta = delta.trim();

    let (sign, rest) = match delta.chars().next() {
        Some('+') => (1i64, &delta[1..]),
        Some('-') => (-1i64, &delta[1..]),
        _ => (1i64, delta),
    };

    let rest = rest.trim();

    let num_end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    let num_str = rest[..num_end].trim();
    let unit_str = rest[num_end..].trim();

    let num: f64 = if num_str.is_empty() {
        1.0
    } else {
        num_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid number in duration: {}", delta))?
    };

    let days = match unit_str.to_lowercase().as_str() {
        "days" | "day" | "d" => num,
        "weeks" | "week" | "w" => num * 7.0,
        "months" | "month" | "m" => num * 30.0,
        "years" | "year" | "y" => num * 365.0,
        "" => num,
        other => {
            return Err(format!(
                "Unknown duration unit: '{}' in '{}'",
                other, delta
            ))
        }
    };

    let total_days = (days * sign as f64) as i64;
    Ok(chrono::Duration::days(total_days))
}

/// Add/subtract duration to a date string.
/// input: "2024-01-15", delta: "+30 days" or "-7d" or "+1m" or "+1y"
pub fn date_add(input: &str, delta: &str) -> Result<String, String> {
    let date = parse_date(input)?;
    let duration = parse_duration(delta)?;
    let new_date = date + duration;
    Ok(new_date.format("%Y-%m-%d").to_string())
}

/// Get current date as string in given format
pub fn today(format_code: &str) -> String {
    let today = chrono::Local::now().naive_local().date();
    format_date(&today, format_code)
}

/// Get the number of days between two dates (date2 - date1)
pub fn date_diff(date1: &str, date2: &str) -> Result<i64, String> {
    let d1 = parse_date(date1)?;
    let d2 = parse_date(date2)?;
    let diff = d2.signed_duration_since(d1);
    Ok(diff.num_days())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_iso() {
        let d = parse_date("2024-01-15").unwrap();
        assert_eq!(d.year(), 2024);
        assert_eq!(d.month(), 1);
        assert_eq!(d.day(), 15);
    }

    #[test]
    fn test_parse_date_us_format() {
        let d = parse_date("January 15, 2024").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_parse_date_short_month() {
        let d = parse_date("Jan 15, 2024").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_parse_date_dd_mm_yyyy() {
        let d = parse_date("15/01/2024").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_parse_date_mm_dd_yyyy() {
        let d = parse_date("01/15/2024").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_parse_date_dd_mm_yyyy_dash() {
        let d = parse_date("15-01-2024").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("not-a-date").is_err());
    }

    #[test]
    fn test_format_date_yyyy_mm_dd() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(format_date(&d, "YYYY-MM-DD"), "2024-01-15");
    }

    #[test]
    fn test_format_date_dd_mm_yyyy() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(format_date(&d, "DD/MM/YYYY"), "15/01/2024");
    }

    #[test]
    fn test_format_date_long_month() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(format_date(&d, "MMMM DD, YYYY"), "January 15, 2024");
    }

    #[test]
    fn test_format_date_short_weekday() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Monday
        assert_eq!(format_date(&d, "ddd, MMM DD"), "Mon, Jan 15");
    }

    #[test]
    fn test_parse_duration_days() {
        let dur = parse_duration("+30 days").unwrap();
        assert_eq!(dur.num_days(), 30);
    }

    #[test]
    fn test_parse_duration_neg_days() {
        let dur = parse_duration("-7d").unwrap();
        assert_eq!(dur.num_days(), -7);
    }

    #[test]
    fn test_parse_duration_month() {
        let dur = parse_duration("+1m").unwrap();
        assert_eq!(dur.num_days(), 30);
    }

    #[test]
    fn test_parse_duration_year() {
        let dur = parse_duration("+1y").unwrap();
        assert_eq!(dur.num_days(), 365);
    }

    #[test]
    fn test_parse_duration_week() {
        let dur = parse_duration("2w").unwrap();
        assert_eq!(dur.num_days(), 14);
    }

    #[test]
    fn test_date_add_days() {
        let result = date_add("2024-01-15", "+30 days").unwrap();
        assert_eq!(result, "2024-02-14");
    }

    #[test]
    fn test_date_add_subtract() {
        let result = date_add("2024-01-15", "-7d").unwrap();
        assert_eq!(result, "2024-01-08");
    }

    #[test]
    fn test_date_add_month() {
        let result = date_add("2024-01-15", "+1m").unwrap();
        assert_eq!(result, "2024-02-14");
    }

    #[test]
    fn test_today() {
        let result = today("YYYY-MM-DD");
        let d = parse_date(&result).unwrap();
        let now = chrono::Local::now().naive_local().date();
        assert_eq!(d, now);
    }

    #[test]
    fn test_date_diff_positive() {
        let diff = date_diff("2024-01-01", "2024-01-15").unwrap();
        assert_eq!(diff, 14);
    }

    #[test]
    fn test_date_diff_negative() {
        let diff = date_diff("2024-01-15", "2024-01-01").unwrap();
        assert_eq!(diff, -14);
    }

    #[test]
    fn test_date_diff_zero() {
        let diff = date_diff("2024-06-15", "2024-06-15").unwrap();
        assert_eq!(diff, 0);
    }
}
