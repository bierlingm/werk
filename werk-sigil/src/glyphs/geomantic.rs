use super::GlyphFamily;

// Order: Acquisitio, Amissio, Albus, Populus, Fortuna Major, Fortuna Minor,
// Conjunctio, Carcer, Tristitia, Laetitia, Puella, Puer, Rubeus, Albus (alt),
// Cauda Draconis, Caput Draconis.
const GLYPHS: &[&str] = &[
    "M0 2 L4 2",
    "M0 4 L4 4",
    "M0 6 L4 6",
    "M0 8 L4 8",
    "M0 10 L4 10",
    "M0 12 L4 12",
    "M0 14 L4 14",
    "M0 16 L4 16",
    "M0 18 L4 18",
    "M0 20 L4 20",
    "M0 22 L4 22",
    "M0 24 L4 24",
    "M6 2 L10 2",
    "M6 4 L10 4",
    "M6 6 L10 6",
    "M6 8 L10 8",
];

#[derive(Debug, Default)]
pub struct GeomanticFamily;

impl GlyphFamily for GeomanticFamily {
    fn glyph(&self, idx: usize) -> &'static str {
        let index = idx % GLYPHS.len();
        GLYPHS[index]
    }

    fn len(&self) -> usize {
        GLYPHS.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_sixteen_figures() {
        let family = GeomanticFamily;
        let mut seen = std::collections::HashSet::new();
        for idx in 0..16 {
            seen.insert(family.glyph(idx));
        }
        assert_eq!(seen.len(), 16);
    }
}
