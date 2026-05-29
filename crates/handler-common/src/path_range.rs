use serde::{Deserialize, Serialize};

/// Represents a segment in a cross-node highlighted path range,
/// with optional start and end character offsets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathRangeSegment {
    /// The unique path ID of the target document element
    pub path: String,
    /// Starting character offset within the element text (inclusive)
    pub start: Option<usize>,
    /// Ending character offset within the element text (exclusive)
    pub end: Option<usize>,
}

/// Parses the multi-path range DSL into a list of structured `PathRangeSegment`s.
/// Supports formats like:
///   `"/page[1]/text[2][2..],/page[1]/text[3],/page[1]/text[4][..3]"`
pub fn parse_range_paths(input: &str) -> Result<Vec<PathRangeSegment>, String> {
    let mut segments = Vec::new();

    // Split by comma to separate segments
    for token in input.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Find the last '[' and its matching ']' at the end
        if trimmed.ends_with(']') {
            if let Some(open_bracket_idx) = trimmed.rfind('[') {
                let range_part = &trimmed[open_bracket_idx + 1..trimmed.len() - 1];

                // If it contains "..", it's a range bracket (e.g. "2.." or "..3" or "1..4")
                if range_part.contains("..") {
                    let path = trimmed[..open_bracket_idx].to_string();

                    let parts: Vec<&str> = range_part.split("..").collect();
                    if parts.len() == 2 {
                        let start = if parts[0].is_empty() {
                            None
                        } else {
                            Some(
                                parts[0]
                                    .parse::<usize>()
                                    .map_err(|e| format!("invalid start offset: {}", e))?,
                            )
                        };
                        let end = if parts[1].is_empty() {
                            None
                        } else {
                            Some(
                                parts[1]
                                    .parse::<usize>()
                                    .map_err(|e| format!("invalid end offset: {}", e))?,
                            )
                        };

                        segments.push(PathRangeSegment { path, start, end });
                        continue;
                    } else {
                        return Err(format!("invalid range format: [{}]", range_part));
                    }
                }
            }
        }

        // Default: entire element highlighted (no range brackets or non-range brackets)
        segments.push(PathRangeSegment {
            path: trimmed.to_string(),
            start: None,
            end: None,
        });
    }

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_paths() {
        let input = "/page[1]/text[2][2..], /page[1]/text[3], /page[1]/text[4][..3], /page[1]/text[5][1..4]";
        let parsed = parse_range_paths(input).unwrap();

        assert_eq!(parsed.len(), 4);

        assert_eq!(
            parsed[0],
            PathRangeSegment {
                path: "/page[1]/text[2]".to_string(),
                start: Some(2),
                end: None,
            }
        );

        assert_eq!(
            parsed[1],
            PathRangeSegment {
                path: "/page[1]/text[3]".to_string(),
                start: None,
                end: None,
            }
        );

        assert_eq!(
            parsed[2],
            PathRangeSegment {
                path: "/page[1]/text[4]".to_string(),
                start: None,
                end: Some(3),
            }
        );

        assert_eq!(
            parsed[3],
            PathRangeSegment {
                path: "/page[1]/text[5]".to_string(),
                start: Some(1),
                end: Some(4),
            }
        );
    }
}
