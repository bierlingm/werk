mod alchemical;
mod geomantic;
mod handdrawn;

pub use alchemical::AlchemicalFamily;
pub use geomantic::GeomanticFamily;
pub use handdrawn::HandDrawnFamily;

pub trait GlyphFamily {
    fn glyph(&self, idx: usize) -> &'static str;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
