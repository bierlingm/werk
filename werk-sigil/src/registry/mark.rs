#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Circle,
    Ellipse,
    Glyph,
    Polygon,
}

impl Primitive {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "circle" => Some(Self::Circle),
            "ellipse" => Some(Self::Ellipse),
            "glyph" => Some(Self::Glyph),
            "polygon" => Some(Self::Polygon),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Circle => "circle",
            Self::Ellipse => "ellipse",
            Self::Glyph => "glyph",
            Self::Polygon => "polygon",
        }
    }
}
