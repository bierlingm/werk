#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelName(pub &'static str);

pub const CHANNEL_NAMES: &[&str] = &[
    "primitive",
    "cx",
    "cy",
    "r",
    "stroke_width",
    "fill_opacity",
    "stroke_opacity",
    "glyph_family",
    "glyph_index",
    "scale",
    "rotation",
    "fill",
    "stroke",
    "opacity",
];
