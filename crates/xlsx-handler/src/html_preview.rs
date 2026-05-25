use handler_common::HandlerError;
use crate::helpers;
use oxml::OxmlPackage;

/// Render the Excel workbook as HTML for browser preview.
pub fn view_as_html(package: &OxmlPackage) -> Result<String, HandlerError> {
    let model = helpers::build_workbook_model(package)
        .map_err(|e| HandlerError::OperationFailed(e))?;

    let mut sheets_html = String::new();

    for ws in &model.sheets {
        sheets_html.push_str(&format!("<h2 data-path=\"/{}\">{}</h2>\n", ws.name, ws.name));
        sheets_html.push_str(&render_sheet_table(ws));
        sheets_html.push_str("\n");
    }

    Ok(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
body {{ font-family: "Segoe UI", Arial, sans-serif; margin: 40px; }}
table {{ border-collapse: collapse; margin: 1em 0; font-size: 0.9em; }}
td, th {{ border: 1px solid #ddd; padding: 4px 8px; }}
th {{ background: #4472c4; color: white; text-align: center; font-weight: normal; }}
td {{ text-align: left; }}
td.number {{ text-align: right; }}
.sheet-tab {{ display: inline-block; padding: 6px 12px; background: #f0f0f0; border: 1px solid #ccc; cursor: pointer; margin-right: 2px; }}
.sheet-tab.active {{ background: #4472c4; color: white; }}
</style>
</head>
<body>
<h1>Excel Workbook</h1>
<div class="sheet-tabs">
{}
</div>
{}
</body>
</html>"#,
        model.sheets.iter()
            .map(|ws| format!("<span class=\"sheet-tab\" data-path=\"/{}\">{}</span>", ws.name, ws.name))
            .collect::<Vec<_>>()
            .join("\n"),
        sheets_html
    ))
}

fn render_sheet_table(ws: &crate::dom_types::Worksheet) -> String {
    let max_row = ws.max_row.min(100);
    let max_col = ws.max_col.min(26);

    let mut html = String::from("<table>\n<tr><th></th>");

    // Column headers (A, B, C, ...)
    for col in 1..=max_col {
        html.push_str(&format!("<th>{}</th>", crate::dom_types::col_num_to_letters(col)));
    }
    html.push_str("</tr>\n");

    // Rows
    for row in 1..=max_row {
        html.push_str(&format!("<tr><th>{}</th>", row));
        for col in 1..=max_col {
            if let Some(cell) = ws.cells.get(&(row, col)) {
                let class = if cell.value_type == crate::dom_types::CellValueType::Number { "number" } else { "" };
                let text = html_escape(&cell.display_value);
                html.push_str(&format!("<td class=\"{}\" data-path=\"/{}/{}\">{}</td>", class, ws.name, cell.ref_str, text));
            } else {
                html.push_str("<td></td>");
            }
        }
        html.push_str("</tr>\n");
    }

    html.push_str("</table>\n");
    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}