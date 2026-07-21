#[derive(Debug, Clone, PartialEq)]
pub enum SvgNodeType {
    Svg,
    Group,
    Rect,
    Circle,
    Ellipse,
    Line,
    Polyline,
    Polygon,
    Path,
    Text,
    TSpan,
    Image,
    Use,
    ClipPath,
    Mask,
    Defs,
    LinearGradient,
    RadialGradient,
    Unknown(String),
}

impl SvgNodeType {
    pub fn from_tag(tag: &str) -> Self {
        match tag {
            "svg" => SvgNodeType::Svg,
            "g" => SvgNodeType::Group,
            "rect" => SvgNodeType::Rect,
            "circle" => SvgNodeType::Circle,
            "ellipse" => SvgNodeType::Ellipse,
            "line" => SvgNodeType::Line,
            "polyline" => SvgNodeType::Polyline,
            "polygon" => SvgNodeType::Polygon,
            "path" => SvgNodeType::Path,
            "text" => SvgNodeType::Text,
            "tspan" => SvgNodeType::TSpan,
            "image" => SvgNodeType::Image,
            "use" => SvgNodeType::Use,
            "clipPath" => SvgNodeType::ClipPath,
            "mask" => SvgNodeType::Mask,
            "defs" => SvgNodeType::Defs,
            "linearGradient" => SvgNodeType::LinearGradient,
            "radialGradient" => SvgNodeType::RadialGradient,
            other => SvgNodeType::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            SvgNodeType::Svg => "svg",
            SvgNodeType::Group => "g",
            SvgNodeType::Rect => "rect",
            SvgNodeType::Circle => "circle",
            SvgNodeType::Ellipse => "ellipse",
            SvgNodeType::Line => "line",
            SvgNodeType::Polyline => "polyline",
            SvgNodeType::Polygon => "polygon",
            SvgNodeType::Path => "path",
            SvgNodeType::Text => "text",
            SvgNodeType::TSpan => "tspan",
            SvgNodeType::Image => "image",
            SvgNodeType::Use => "use",
            SvgNodeType::ClipPath => "clipPath",
            SvgNodeType::Mask => "mask",
            SvgNodeType::Defs => "defs",
            SvgNodeType::LinearGradient => "linearGradient",
            SvgNodeType::RadialGradient => "radialGradient",
            SvgNodeType::Unknown(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SvgStats {
    pub total_elements: usize,
    pub rects: usize,
    pub circles: usize,
    pub ellipses: usize,
    pub paths: usize,
    pub texts: usize,
    pub images: usize,
    pub groups: usize,
    pub width: f64,
    pub height: f64,
}
