mod encoder;
mod featurizer;
mod layouter;
mod renderer;
mod selector;
mod stylist;

pub use encoder::*;
pub use featurizer::*;
pub use layouter::*;
pub use renderer::*;
pub use selector::*;
pub use stylist::*;

use std::collections::HashMap;

use crate::registry::Primitive;

#[derive(Debug, Clone)]
pub enum ChannelValue {
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct MarkSpec {
    pub id: String,
    pub primitive: Primitive,
    pub channels: HashMap<String, ChannelValue>,
}

#[derive(Debug, Clone)]
pub struct PlacedMark {
    pub mark: MarkSpec,
    pub cx: f64,
    pub cy: f64,
    pub rotation: f64,
    pub scale: f64,
}

#[derive(Debug, Clone)]
pub struct StructuralMark {
    pub path: String,
    pub stroke_width: f64,
    pub opacity: f64,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub marks: Vec<PlacedMark>,
    pub structural: Vec<StructuralMark>,
}

#[derive(Debug, Clone)]
pub struct StyledScene {
    pub layout: Layout,
    pub background: Option<String>,
    pub stroke_color: String,
    pub fill_color: String,
    pub glyph_color: String,
    pub filter: Option<String>,
    pub palette_name: String,
    pub stroke_only: bool,
    pub glyph_mirror: bool,
}
