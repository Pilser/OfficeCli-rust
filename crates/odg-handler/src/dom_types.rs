#[derive(Debug, Clone, PartialEq)]
pub enum OdgNodeType {
    Page,
    Rect,
    Circle,
    Ellipse,
    Line,
    Polyline,
    Polygon,
    Path,
    TextBox,
    Image,
    Group,
    Connector,
    Unknown(String),
}

impl OdgNodeType {
    pub fn from_tag(tag: &str) -> Self {
        match tag.to_lowercase().as_str() {
            "page" => Self::Page,
            "rect" | "rectangle" => Self::Rect,
            "circle" => Self::Circle,
            "ellipse" => Self::Ellipse,
            "line" => Self::Line,
            "polyline" => Self::Polyline,
            "polygon" => Self::Polygon,
            "path" => Self::Path,
            "text-box" | "textbox" => Self::TextBox,
            "image" => Self::Image,
            "group" => Self::Group,
            "connector" => Self::Connector,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Page => "page",
            Self::Rect => "rect",
            Self::Circle => "circle",
            Self::Ellipse => "ellipse",
            Self::Line => "line",
            Self::Polyline => "polyline",
            Self::Polygon => "polygon",
            Self::Path => "path",
            Self::TextBox => "text-box",
            Self::Image => "image",
            Self::Group => "group",
            Self::Connector => "connector",
            Self::Unknown(s) => s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OdgStats {
    pub total_pages: usize,
    pub total_elements: usize,
    pub rects: usize,
    pub circles: usize,
    pub paths: usize,
    pub texts: usize,
    pub images: usize,
    pub groups: usize,
    pub width: f64,
    pub height: f64,
}
