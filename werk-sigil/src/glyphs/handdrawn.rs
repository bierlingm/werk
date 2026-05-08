use super::GlyphFamily;

const GLYPHS: &[&str] = &[
    "M0 12 C4 2 8 2 12 12 C16 22 20 22 24 12",
    "M2 2 C6 10 10 14 14 10 C18 6 22 8 22 20",
    "M1 6 C6 0 12 0 17 6 C22 12 22 18 17 24",
    "M4 4 C8 8 12 12 16 8 C20 4 24 8 20 16",
    "M0 8 C6 4 10 12 16 8 C22 4 24 12 20 20",
    "M2 22 C6 18 10 14 14 18 C18 22 22 18 22 10",
    "M3 3 C9 5 15 9 21 3",
    "M3 21 C9 19 15 15 21 21",
    "M1 12 C5 6 9 18 13 12 C17 6 21 18 23 12",
    "M2 8 C8 6 12 10 18 8 C22 6 24 14 20 18",
    "M4 20 C8 14 12 10 16 14 C20 18 22 12 24 6",
    "M0 4 C6 8 12 6 18 10 C22 12 24 16 22 20",
];

#[derive(Debug, Default)]
pub struct HandDrawnFamily;

impl GlyphFamily for HandDrawnFamily {
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
    fn twelve_atoms_present() {
        let family = HandDrawnFamily;
        let mut seen = std::collections::HashSet::new();
        for idx in 0..12 {
            seen.insert(family.glyph(idx));
        }
        assert!(seen.len() >= 12);
    }
}
