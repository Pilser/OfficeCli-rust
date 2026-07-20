use image::ImageEncoder;
use plotters::coord::Shift;
use plotters::prelude::*;

/// Supported chart types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Scatter,
}

/// Configuration for a chart
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub chart_type: ChartType,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<(String, f64)>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub colors: Option<Vec<String>>,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            chart_type: ChartType::Bar,
            title: String::new(),
            width: 800,
            height: 600,
            data: Vec::new(),
            x_label: None,
            y_label: None,
            colors: None,
        }
    }
}

fn hex_to_rgb(hex: &str) -> Result<RGBColor, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(format!("invalid hex color '{}': expected 6 hex digits", hex));
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| format!("invalid hex: {}", e))?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| format!("invalid hex: {}", e))?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| format!("invalid hex: {}", e))?;
    Ok(RGBColor(r, g, b))
}

fn default_colors() -> Vec<RGBColor> {
    vec![
        RGBColor(66, 133, 244),
        RGBColor(234, 67, 53),
        RGBColor(52, 168, 83),
        RGBColor(251, 188, 4),
        RGBColor(171, 71, 188),
        RGBColor(0, 172, 193),
        RGBColor(255, 112, 67),
        RGBColor(158, 158, 158),
    ]
}

fn resolve_colors(config: &ChartConfig) -> Vec<RGBColor> {
    match &config.colors {
        Some(hex_colors) => hex_colors
            .iter()
            .map(|h| hex_to_rgb(h).unwrap_or_else(|_| RGBColor(66, 133, 244)))
            .collect(),
        None => default_colors(),
    }
}

/// Render chart to PNG bytes. Returns (png_bytes, width, height).
pub fn render_chart_png(config: &ChartConfig) -> Result<(Vec<u8>, u32, u32), String> {
    if config.data.is_empty() {
        return Err("cannot render chart with empty data".to_string());
    }

    let buf_size = (config.width * config.height * 3) as usize;
    let mut buffer = vec![0u8; buf_size];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        match config.chart_type {
            ChartType::Bar => draw_bar_chart(&root, config)?,
            ChartType::Line => draw_line_chart(&root, config)?,
            ChartType::Pie => draw_pie_chart(&root, config)?,
            ChartType::Scatter => draw_scatter_chart(&root, config)?,
        }

        root.present().map_err(|e| e.to_string())?;
    }

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(&buffer, config.width, config.height, image::ColorType::Rgb8.into())
        .map_err(|e| e.to_string())?;

    Ok((png_bytes, config.width, config.height))
}

/// Render chart to SVG string.
pub fn render_chart_svg(config: &ChartConfig) -> Result<String, String> {
    if config.data.is_empty() {
        return Err("cannot render chart with empty data".to_string());
    }

    let mut svg_string = String::new();
    {
        let root = SVGBackend::with_string(&mut svg_string, (config.width, config.height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        match config.chart_type {
            ChartType::Bar => draw_bar_chart(&root, config)?,
            ChartType::Line => draw_line_chart(&root, config)?,
            ChartType::Pie => draw_pie_chart(&root, config)?,
            ChartType::Scatter => draw_scatter_chart(&root, config)?,
        }

        root.present().map_err(|e| e.to_string())?;
    }

    Ok(svg_string)
}

fn draw_bar_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    config: &ChartConfig,
) -> Result<(), String> {
    let colors = resolve_colors(config);
    let n = config.data.len();
    let max_val = config.data.iter().map(|(_, v)| *v).fold(0.0, f64::max).max(1.0);

    let mut chart = ChartBuilder::on(root)
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption(&config.title, ("sans-serif", 30).into_font())
        .build_cartesian_2d((0..n).into_segmented(), 0.0..max_val * 1.1)
        .map_err(|e| e.to_string())?;

    chart
        .configure_mesh()
        .x_labels(n)
        .y_desc(config.y_label.as_deref().unwrap_or(""))
        .x_desc(config.x_label.as_deref().unwrap_or(""))
        .draw()
        .map_err(|e| e.to_string())?;

    chart
        .draw_series(config.data.iter().enumerate().map(|(i, (_, value))| {
            let color = colors[i % colors.len()];
            let x0 = SegmentValue::Exact(i);
            let x1 = SegmentValue::Exact(i + 1);
            let mut bar = Rectangle::new([(x0, 0.0), (x1, *value)], color.filled());
            bar.set_margin(0, 0, 10, 10);
            bar
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn draw_line_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    config: &ChartConfig,
) -> Result<(), String> {
    let colors = resolve_colors(config);
    let color = colors[0];
    let n = config.data.len();
    let max_val = config.data.iter().map(|(_, v)| *v).fold(0.0, f64::max).max(1.0);

    let mut chart = ChartBuilder::on(root)
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption(&config.title, ("sans-serif", 30).into_font())
        .build_cartesian_2d(0.0..n.max(1) as f64, 0.0..max_val * 1.1)
        .map_err(|e| e.to_string())?;

    chart
        .configure_mesh()
        .x_labels(n)
        .y_desc(config.y_label.as_deref().unwrap_or(""))
        .x_desc(config.x_label.as_deref().unwrap_or(""))
        .draw()
        .map_err(|e| e.to_string())?;

    chart
        .draw_series(LineSeries::new(
            config
                .data
                .iter()
                .enumerate()
                .map(|(i, (_, v))| (i as f64, *v)),
            color,
        ))
        .map_err(|e| e.to_string())?
        .label(&config.title)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn draw_pie_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    config: &ChartConfig,
) -> Result<(), String> {
    let colors = resolve_colors(config);
    let total: f64 = config.data.iter().map(|(_, v)| v).sum();
    if total == 0.0 {
        return Err("cannot draw pie chart with zero total".to_string());
    }

    let (w, h) = root.dim_in_pixel();
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let radius = (w.min(h) as f64 / 2.0) * 0.7;

    let mut start_angle = -90.0;

    for (i, (_, value)) in config.data.iter().enumerate() {
        let angle = (*value / total) * 360.0;
        let end_angle = start_angle + angle;
        let color = colors[i % colors.len()];

        let start_rad = start_angle.to_radians();
        let end_rad = end_angle.to_radians();

        let steps = (angle.abs() * 2.0).max(4.0) as usize;
        let mut points = Vec::with_capacity(steps + 2);
        points.push((cx as i32, cy as i32));

        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let rad = start_rad + (end_rad - start_rad) * t;
            let px = cx + radius * rad.cos();
            let py = cy + radius * rad.sin();
            points.push((px as i32, py as i32));
        }

        root.draw(&PathElement::new(points, color.filled()))
            .map_err(|e| e.to_string())?;

        start_angle = end_angle;
    }

    start_angle = -90.0;
    for (label, value) in &config.data {
        let mid_angle = start_angle + (*value / total) * 180.0;
        let mid_rad = mid_angle.to_radians();
        let label_radius = radius * 0.6;
        let lx = cx + label_radius * mid_rad.cos();
        let ly = cy + label_radius * mid_rad.sin();

        let pct = (*value / total) * 100.0;
        let text = if label.is_empty() {
            format!("{:.0}%", pct)
        } else {
            format!("{} ({:.0}%)", label, pct)
        };

        let font_size = (config.width.min(config.height) as f64 / 35.0).max(10.0);
        root.draw(&Text::new(
            text,
            (lx as i32, ly as i32),
            ("sans-serif", font_size).into_font().color(&BLACK),
        ))
        .map_err(|e| e.to_string())?;

        start_angle += (*value / total) * 360.0;
    }

    Ok(())
}

fn draw_scatter_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    config: &ChartConfig,
) -> Result<(), String> {
    let colors = resolve_colors(config);
    let color = colors[0];
    let n = config.data.len();
    let max_val = config.data.iter().map(|(_, v)| *v).fold(0.0, f64::max).max(1.0);
    let y_range = max_val * 1.1;
    let marker_size = y_range * 0.04;

    let mut chart = ChartBuilder::on(root)
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption(&config.title, ("sans-serif", 30).into_font())
        .build_cartesian_2d(0.0..n.max(1) as f64, 0.0..y_range)
        .map_err(|e| e.to_string())?;

    chart
        .configure_mesh()
        .x_labels(n)
        .y_desc(config.y_label.as_deref().unwrap_or(""))
        .x_desc(config.x_label.as_deref().unwrap_or(""))
        .draw()
        .map_err(|e| e.to_string())?;

    chart
        .draw_series(config.data.iter().enumerate().map(|(i, (_, v))| {
            let x = i as f64;
            let y = *v;
            Rectangle::new(
                [(x - 0.2, y - marker_size), (x + 0.2, y + marker_size)],
                color.filled(),
            )
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(chart_type: ChartType) -> ChartConfig {
        ChartConfig {
            chart_type,
            title: "Test Chart".to_string(),
            width: 400,
            height: 300,
            data: vec![
                ("A".to_string(), 10.0),
                ("B".to_string(), 25.0),
                ("C".to_string(), 15.0),
                ("D".to_string(), 30.0),
            ],
            x_label: Some("Category".to_string()),
            y_label: Some("Value".to_string()),
            colors: None,
        }
    }

    #[test]
    fn test_bar_chart_png() {
        let config = test_config(ChartType::Bar);
        let result = render_chart_png(&config);
        assert!(result.is_ok());
        let (bytes, w, h) = result.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(w, 400);
        assert_eq!(h, 300);
    }

    #[test]
    fn test_line_chart_png() {
        let config = test_config(ChartType::Line);
        let result = render_chart_png(&config);
        assert!(result.is_ok());
        let (bytes, w, h) = result.unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_pie_chart_png() {
        let config = test_config(ChartType::Pie);
        let result = render_chart_png(&config);
        assert!(result.is_ok());
        let (bytes, w, h) = result.unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_scatter_chart_png() {
        let config = test_config(ChartType::Scatter);
        let result = render_chart_png(&config);
        assert!(result.is_ok());
        let (bytes, w, h) = result.unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_bar_chart_svg() {
        let config = test_config(ChartType::Bar);
        let result = render_chart_svg(&config);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_empty_data() {
        let config = ChartConfig {
            data: vec![],
            ..Default::default()
        };
        assert!(render_chart_png(&config).is_err());
        assert!(render_chart_svg(&config).is_err());
    }

    #[test]
    fn test_hex_color() {
        let color = hex_to_rgb("#FF0000").unwrap();
        assert_eq!(color, RGBColor(255, 0, 0));

        let color = hex_to_rgb("00FF00").unwrap();
        assert_eq!(color, RGBColor(0, 255, 0));

        assert!(hex_to_rgb("xyz").is_err());
        assert!(hex_to_rgb("#FFF").is_err());
    }

    #[test]
    fn test_custom_colors() {
        let mut config = test_config(ChartType::Bar);
        config.colors = Some(vec![
            "#FF0000".to_string(),
            "#00FF00".to_string(),
        ]);
        let result = render_chart_png(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_colors_count() {
        let colors = default_colors();
        assert_eq!(colors.len(), 8);
    }
}
