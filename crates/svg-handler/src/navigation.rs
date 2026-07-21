pub struct SvgNavigator;

pub struct ResolvedNode<'doc> {
    pub node: roxmltree::Node<'doc, 'doc>,
    pub tag_path: String,
    pub element_type: String,
}

struct Seg {
    name: String,
    index: Option<usize>,
    attribute: Option<(String, String)>,
}

impl SvgNavigator {
    pub fn resolve<'doc>(
        doc: &'doc roxmltree::Document,
        path: &str,
    ) -> Result<ResolvedNode<'doc>, String> {
        let segments = Self::parse_path(path)?;

        if segments.is_empty() || segments[0].name != "svg" {
            return Err("path must start with /svg".to_string());
        }

        let root = doc.root_element();
        if root.tag_name().name() != "svg" {
            return Err("root element is not <svg>".to_string());
        }

        let mut current = root;
        let mut tag_path = "/svg".to_string();

        for segment in &segments[1..] {
            let candidates: Vec<_> = current
                .children()
                .filter(|c| c.is_element())
                .filter(|c| segment.name == "*" || c.tag_name().name() == segment.name)
                .filter(|c| {
                    if let Some((key, val)) = &segment.attribute {
                        c.attribute(key.as_str()) == Some(val.as_str())
                    } else {
                        true
                    }
                })
                .collect();

            if candidates.is_empty() {
                let desc = seg_description(segment);
                return Err(format!("no element matching '{}' found at {}", desc, tag_path));
            }

            let idx = segment.index.unwrap_or(1).saturating_sub(1);
            if idx >= candidates.len() {
                return Err(format!(
                    "index {} out of range for '{}' at {}",
                    segment.index.unwrap_or(1),
                    segment.name,
                    tag_path
                ));
            }

            current = candidates[idx];
            let my_index = compute_index(&current);
            tag_path = format!("{}/{}[{}]", tag_path, current.tag_name().name(), my_index);
        }

        let element_type = current.tag_name().name().to_string();
        Ok(ResolvedNode {
            node: current,
            tag_path,
            element_type,
        })
    }

    pub fn find_by_id<'doc>(
        doc: &'doc roxmltree::Document,
        id: &str,
    ) -> Option<ResolvedNode<'doc>> {
        for node in doc.descendants() {
            if !node.is_element() {
                continue;
            }
            if node.attribute("id") == Some(id) {
                let tag_path = resolve_tag_path(&node);
                let element_type = node.tag_name().name().to_string();
                return Some(ResolvedNode {
                    node,
                    tag_path,
                    element_type,
                });
            }
        }
        None
    }

    pub fn find_all_by_type<'doc>(
        doc: &'doc roxmltree::Document,
        element_type: &str,
    ) -> Vec<ResolvedNode<'doc>> {
        let mut results = Vec::new();
        for node in doc.descendants() {
            if !node.is_element() {
                continue;
            }
            if node.tag_name().name() == element_type {
                let tag_path = resolve_tag_path(&node);
                results.push(ResolvedNode {
                    node,
                    tag_path,
                    element_type: element_type.to_string(),
                });
            }
        }
        results
    }

    fn parse_path(path: &str) -> Result<Vec<Seg>, String> {
        if !path.starts_with('/') {
            return Err(format!("path must start with /: {}", path));
        }
        let raw_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut segments = Vec::new();

        for raw in raw_segments {
            segments.push(parse_seg(raw)?);
        }

        Ok(segments)
    }
}

fn parse_seg(s: &str) -> Result<Seg, String> {
    let mut name = s.to_string();
    let mut index = None;
    let mut attribute = None;

    let mut bracket_start = 0;
    let remaining = s;

    while let Some(open) = remaining[bracket_start..].find('[') {
        let open_pos = bracket_start + open;
        if let Some(close) = remaining[open_pos..].find(']') {
            let close_pos = open_pos + close;
            let content = &remaining[open_pos + 1..close_pos];

            if name.len() > open_pos || name == s {
                name = remaining[..open_pos].to_string();
            }

            if let Some(attr_content) = content.strip_prefix('@') {
                if let Some(eq_pos) = attr_content.find('=') {
                    let attr_key = attr_content[..eq_pos].to_string();
                    let attr_val = attr_content[eq_pos + 1..].to_string();
                    attribute = Some((attr_key, attr_val));
                }
            } else if let Ok(idx) = content.parse::<usize>() {
                index = Some(idx);
            }

            bracket_start = close_pos + 1;
        } else {
            break;
        }
    }

    Ok(Seg {
        name,
        index,
        attribute,
    })
}

fn seg_description(seg: &Seg) -> String {
    let mut s = seg.name.clone();
    if let Some((k, v)) = &seg.attribute {
        s.push_str(&format!("[@{}={}]", k, v));
    }
    if let Some(idx) = seg.index {
        s.push_str(&format!("[{}]", idx));
    }
    s
}

fn compute_index(node: &roxmltree::Node) -> usize {
    let tag = node.tag_name().name();
    if let Some(parent) = node.parent() {
        let mut count = 0;
        for child in parent.children() {
            if child.is_element() && child.tag_name().name() == tag {
                count += 1;
                if child == *node {
                    return count;
                }
            }
        }
    }
    1
}

fn resolve_tag_path(node: &roxmltree::Node) -> String {
    let mut segments: Vec<String> = Vec::new();
    let mut current = *node;
    loop {
        let tag = current.tag_name().name();
        let idx = compute_index(&current);
        segments.push(format!("{}[{}]", tag, idx));
        if let Some(parent) = current.parent() {
            if parent.is_element() {
                current = parent;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    segments.reverse();
    let mut path = String::new();
    for s in &segments {
        path.push('/');
        path.push_str(s);
    }
    path
}
