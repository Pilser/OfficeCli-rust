pub fn detect_prefix(xml: &str) -> String {
    if let Some(pos) = xml.find("worksheet") {
        if let Some(lt_pos) = xml[..pos].rfind('<') {
            let prefix = &xml[lt_pos + 1..pos];
            if !prefix.is_empty() && prefix.ends_with(':') {
                return prefix.to_string();
            }
        }
    }
    String::new()
}

fn insert_or_replace_cols(sheet_xml: &str, new_cols_xml: &str) -> String {
    let p = detect_prefix(sheet_xml);
    let cols_open = format!("<{}cols", p);
    let cols_close = format!("</{}cols>", p);

    if let Some(start) = sheet_xml.find(&cols_open) {
        let end = sheet_xml[start..].find(&cols_close)
            .map(|i| start + i + cols_close.len())
            .unwrap_or(sheet_xml.len());
        format!("{}{}{}", &sheet_xml[..start], new_cols_xml, &sheet_xml[end..])
    } else {
        let sd_open = format!("<{}sheetData", p);
        if let Some(pos) = sheet_xml.find(&sd_open) {
            format!("{}{}{}", &sheet_xml[..pos], new_cols_xml, &sheet_xml[pos..])
        } else {
            sheet_xml.to_string()
        }
    }
}

pub fn set_column_width(sheet_xml: &str, col: u32, width: f64) -> String {
    let p = detect_prefix(sheet_xml);
    let mut existing_cols = Vec::new();
    let cols_open = format!("<{}cols", p);
    let cols_close = format!("</{}cols>", p);

    if let Some(start) = sheet_xml.find(&cols_open) {
        let end = sheet_xml[start..].find(&cols_close)
            .map(|i| start + i + cols_close.len())
            .unwrap_or(sheet_xml.len());
        let block = &sheet_xml[start..end];
        let col_open = format!("<{}col", p);
        let mut cursor = 0;
        while let Some(cs) = block[cursor..].find(&col_open) {
            let abs_cs = cursor + cs;
            let ce = block[abs_cs..].find("/>").map(|i| abs_cs + i + 2).unwrap_or(abs_cs + 10);
            let entry = &block[abs_cs..ce];
            let min = extract_attr(entry, "min").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            let max = extract_attr(entry, "max").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            existing_cols.push((min, max, entry.to_string()));
            cursor = ce;
        }
    }

    if !existing_cols.is_empty() {
        let mut new_cols_xml = format!("<{}cols>", p);
        for (min, max, ref entry) in &existing_cols {
            if *min <= col && *max >= col {
                new_cols_xml.push_str(&format!(
                    "<{}col min=\"{}\" max=\"{}\" width=\"{}\" customWidth=\"1\"/>",
                    p, min, max, width
                ));
            } else {
                new_cols_xml.push_str(entry);
            }
        }
        new_cols_xml.push_str(&format!("</{}cols>", p));
        insert_or_replace_cols(sheet_xml, &new_cols_xml)
    } else {
        let new_cols_xml = format!(
            "<{}cols><{}col min=\"{}\" max=\"{}\" width=\"{}\" customWidth=\"1\"/></{}cols>",
            p, p, col, col, width, p
        );
        insert_or_replace_cols(sheet_xml, &new_cols_xml)
    }
}

pub fn set_row_height(sheet_xml: &str, row: u32, height: f64) -> String {
    let p = detect_prefix(sheet_xml);
    let row_open = format!("<{}row r=\"{}\"", p, row);

    if let Some(start) = sheet_xml.find(&row_open) {
        let gt = sheet_xml[start..].find('>').map(|i| start + i).unwrap_or(sheet_xml.len());
        let tag = &sheet_xml[start..gt];

        let mut new_tag = format!("<{}row r=\"{}\" ht=\"{}\" customHeight=\"1\"", p, row, height);
        let mut rest = String::new();
        let mut saw_ht = false;
        let mut saw_ch = false;

        let mut i = tag.find(' ').unwrap_or(tag.len());
        while i < tag.len() {
            while i < tag.len() && tag.as_bytes()[i] == b' ' { i += 1; }
            if i >= tag.len() { break; }
            let eq = tag[i..].find('=').map(|p| i + p).unwrap_or(tag.len());
            let key = tag[i..eq].trim();
            let val_start = eq + 1;
            if val_start < tag.len() && tag.as_bytes()[val_start] == b'"' {
                let val_end = tag[val_start + 1..].find('"').map(|p| val_start + 1 + p).unwrap_or(tag.len());
                let val = &tag[val_start + 1..val_end];
                match key {
                    "r" => {}
                    "ht" => { saw_ht = true; }
                    "customHeight" => { saw_ch = true; }
                    _ => { rest.push_str(&format!(" {}=\"{}\"", key, val)); }
                }
                i = val_end + 1;
            } else {
                i = eq + 1;
            }
        }

        if !saw_ht { rest.push_str(&format!(" ht=\"{}\"", height)); }
        if !saw_ch { rest.push_str(" customHeight=\"1\""); }
        new_tag.push_str(&rest);
        new_tag.push_str(">");

        format!("{}{}", sheet_xml[..start].to_string() + &new_tag, &sheet_xml[gt..])
    } else {
        let sd_open = format!("<{}sheetData", p);
        if let Some(sd_start) = sheet_xml.find(&sd_open) {
            let sd_gt = sheet_xml[sd_start..].find('>').map(|i| sd_start + i + 1).unwrap_or(sheet_xml.len());
            let new_row = format!("<{}row r=\"{}\" ht=\"{}\" customHeight=\"1\"/>", p, row, height);
            format!("{}{}{}", &sheet_xml[..sd_gt], new_row, &sheet_xml[sd_gt..])
        } else {
            sheet_xml.to_string()
        }
    }
}

pub fn set_column_hidden(sheet_xml: &str, col: u32, hidden: bool) -> String {
    let p = detect_prefix(sheet_xml);
    let mut result = sheet_xml.to_string();
    let cols_open = format!("<{}cols", p);
    let cols_close = format!("</{}cols>", p);

    if let Some(start) = result.find(&cols_open) {
        let end = result[start..].find(&cols_close)
            .map(|i| start + i + cols_close.len())
            .unwrap_or(result.len());
        let block = &result[start..end].to_string();
        let col_open = format!("<{}col", p);
        let mut new_block = String::new();
        let mut cursor = 0;
        let mut found = false;
        while let Some(cs) = block[cursor..].find(&col_open) {
            let abs_cs = cursor + cs;
            let ce = block[abs_cs..].find("/>").map(|i| abs_cs + i + 2).unwrap_or(block.len());
            let entry = &block[abs_cs..ce];
            let min = extract_attr(entry, "min").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            let max = extract_attr(entry, "max").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            if min <= col && max >= col {
                found = true;
                let mut new_entry = format!("<{}col min=\"{}\" max=\"{}\"", p, min, max);
                let mut rest = String::new();
                let mut saw_hidden = false;
                let mut i = entry.find(' ').unwrap_or(entry.len());
                while i < entry.len() {
                    while i < entry.len() && entry.as_bytes()[i] == b' ' { i += 1; }
                    if i >= entry.len() { break; }
                    let eq = entry[i..].find('=').map(|p| i + p).unwrap_or(entry.len());
                    let key = entry[i..eq].trim();
                    let val_start = eq + 1;
                    if val_start < entry.len() && entry.as_bytes()[val_start] == b'"' {
                        let val_end = entry[val_start + 1..].find('"').map(|p| val_start + 1 + p).unwrap_or(entry.len());
                        let val = &entry[val_start + 1..val_end];
                        match key {
                            "min" | "max" => {}
                            "hidden" => { saw_hidden = true; }
                            _ => { rest.push_str(&format!(" {}=\"{}\"", key, val)); }
                        }
                        i = val_end + 1;
                    } else {
                        i = eq + 1;
                    }
                }
                if hidden { rest.push_str(" hidden=\"1\""); }
                if !saw_hidden && !hidden { rest.push_str(" hidden=\"0\""); }
                new_entry.push_str(&rest);
                new_entry.push_str("/>");
                new_block.push_str(&new_entry);
            } else {
                new_block.push_str(&block[abs_cs..ce]);
            }
            cursor = ce;
        }
        if found {
            let new_cols = format!("<{}cols>{}</{}cols>", p, new_block, p);
            result = format!("{}{}{}", &result[..start], new_cols, &result[end..]);
        }
    } else if hidden {
        let new_cols = format!(
            "<{}cols><{}col min=\"{}\" max=\"{}\" hidden=\"1\"/></{}cols>",
            p, p, col, col, p
        );
        result = insert_or_replace_cols(&result, &new_cols);
    }

    result
}

pub fn set_row_hidden(sheet_xml: &str, row: u32, hidden: bool) -> String {
    let p = detect_prefix(sheet_xml);
    let row_open = format!("<{}row r=\"{}\"", p, row);

    if let Some(start) = sheet_xml.find(&row_open) {
        let gt = sheet_xml[start..].find('>').map(|i| start + i).unwrap_or(sheet_xml.len());
        let tag = &sheet_xml[start..gt];

        let mut new_tag = format!("<{}row r=\"{}\"", p, row);
        let mut rest = String::new();
        let mut saw_hidden = false;

        let mut i = tag.find(' ').unwrap_or(tag.len());
        while i < tag.len() {
            while i < tag.len() && tag.as_bytes()[i] == b' ' { i += 1; }
            if i >= tag.len() { break; }
            let eq = tag[i..].find('=').map(|p| i + p).unwrap_or(tag.len());
            let key = tag[i..eq].trim();
            let val_start = eq + 1;
            if val_start < tag.len() && tag.as_bytes()[val_start] == b'"' {
                let val_end = tag[val_start + 1..].find('"').map(|p| val_start + 1 + p).unwrap_or(tag.len());
                let val = &tag[val_start + 1..val_end];
                match key {
                    "r" => {}
                    "hidden" => { saw_hidden = true; }
                    _ => { rest.push_str(&format!(" {}=\"{}\"", key, val)); }
                }
                i = val_end + 1;
            } else {
                i = eq + 1;
            }
        }

        if hidden { rest.push_str(" hidden=\"1\""); }
        if !saw_hidden && !hidden { rest.push_str(" hidden=\"0\""); }

        new_tag.push_str(&rest);
        let after_gt = &sheet_xml[gt..];
        if after_gt.starts_with("/>") {
            new_tag.push_str("/>");
            format!("{}{}", &sheet_xml[..start], new_tag)
        } else {
            new_tag.push('>');
            format!("{}{}{}", &sheet_xml[..start], new_tag, after_gt)
        }
    } else {
        sheet_xml.to_string()
    }
}

pub fn freeze_panes(sheet_xml: &str, cell_ref: &str) -> String {
    let p = detect_prefix(sheet_xml);

    let (col, row) = parse_cell_ref(cell_ref);
    let pane_xml = format!(
        "<{}pane xSplit=\"{}\" ySplit=\"{}\" topLeftCell=\"{}\" activePane=\"bottomRight\" state=\"frozen\"/>",
        p, col, row, cell_ref
    );

    let sheet_views_open = format!("<{}sheetViews", p);
    let sheet_views_close = format!("</{}sheetViews>", p);
    let _pane_with_views = format!("{}{}\n  {}\n  {}", pane_xml, "", "", "");

    if let Some(sv_start) = sheet_xml.find(&sheet_views_open) {
        let sv_end = sheet_xml[sv_start..].find(&sheet_views_close)
            .map(|i| sv_start + i + sheet_views_close.len())
            .unwrap_or(sheet_xml.len());
        let inner = &sheet_xml[sv_start..sv_end];
        let sv_tag_end = inner.find('>').map(|i| sv_start + i + 1).unwrap_or(sv_start);
        let new_inner = if inner.contains(&format!("<{}pane", p)) {
            let pane_start = inner.find(&format!("<{}pane", p)).unwrap();
            let pane_end = inner[pane_start..].find("/>").map(|i| pane_start + i + 2).unwrap_or(0);
            format!(
                "{}{}{}",
                &sheet_xml[sv_start..sv_tag_end],
                &pane_xml,
                &sheet_xml[sv_start + pane_end..sv_end]
            )
        } else {
            format!(
                "{}{}\n  {}",
                &sheet_xml[sv_start..sv_tag_end],
                pane_xml,
                &sheet_xml[sv_tag_end..sv_end],
            )
        };
        format!("{}{}", new_inner, &sheet_xml[sv_end..])
    } else {
        let p1 = detect_prefix(sheet_xml);
        let new_views = format!(
            "<{}sheetViews><{}sheetView tabSelected=\"1\" workbookViewId=\"0\">{}</{}sheetView></{}sheetViews>",
            p1, p1, pane_xml, p1, p1
        );
        let ws_open = format!("<{}worksheet", p1);
        if let Some(ws_start) = sheet_xml.find(&ws_open) {
            let ws_gt = sheet_xml[ws_start..].find('>').map(|i| ws_start + i + 1).unwrap_or(0);
            format!("{}{}{}", &sheet_xml[..ws_gt], new_views, &sheet_xml[ws_gt..])
        } else {
            sheet_xml.to_string()
        }
    }
}

pub fn set_auto_filter(sheet_xml: &str, range: &str) -> String {
    let p = detect_prefix(sheet_xml);
    let filter_xml = format!("<{}autoFilter ref=\"{}\"/>", p, range);

    if sheet_xml.contains(&format!("<{}autoFilter", p)) {
        let start = sheet_xml.find(&format!("<{}autoFilter", p)).unwrap();
        let end = sheet_xml[start..].find("/>").map(|i| start + i + 2).unwrap_or(sheet_xml.len());
        format!("{}{}{}", &sheet_xml[..start], filter_xml, &sheet_xml[end..])
    } else if let Some(pos) = sheet_xml.find("</worksheet>") {
        format!("{}{}{}", &sheet_xml[..pos], filter_xml, &sheet_xml[pos..])
    } else {
        sheet_xml.to_string()
    }
}

fn extract_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = xml.find(&pattern)?;
    let val_start = start + pattern.len();
    let end = xml[val_start..].find('"')?;
    Some(xml[val_start..val_start + end].to_string())
}

fn parse_cell_ref(s: &str) -> (u32, u32) {
    let bytes = s.as_bytes();
    let mut col = 0u32;
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        col = col * 26 + (bytes[i].to_ascii_uppercase() as u32 - b'A' as u32 + 1);
        i += 1;
    }
    let row: u32 = s[i..].parse().unwrap_or(0);
    (col, row)
}


