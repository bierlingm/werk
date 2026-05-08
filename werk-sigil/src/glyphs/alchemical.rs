use super::GlyphFamily;

const GLYPHS: &[&str] = &[
    "M10 2 L14 10 L22 10 L16 15 L18 24 L10 18 L2 24 L4 15 L-2 10 L6 10 Z",
    "M12 0 A12 12 0 1 0 12 24 A12 12 0 1 0 12 0 M12 6 A6 6 0 1 1 12 18 A6 6 0 1 1 12 6",
    "M12 0 L12 24 M0 12 L24 12 M4 4 L20 20 M20 4 L4 20",
    "M2 2 L22 2 L22 22 L2 22 Z M6 6 L18 6 L18 18 L6 18 Z",
    "M12 1 L23 12 L12 23 L1 12 Z",
    "M3 12 C3 5 9 1 12 1 C15 1 21 5 21 12 C21 19 15 23 12 23 C9 23 3 19 3 12 Z",
    "M4 2 L20 2 L12 22 Z",
    "M6 2 L18 2 L22 12 L18 22 L6 22 L2 12 Z",
    "M12 0 L22 6 L22 18 L12 24 L2 18 L2 6 Z",
    "M12 4 L20 20 L4 20 Z",
    "M4 8 L20 8 L20 16 L4 16 Z",
    "M12 0 L24 12 L12 24 L0 12 Z M12 6 L18 12 L12 18 L6 12 Z",
    "M0 12 A12 12 0 0 1 24 12 A12 12 0 0 1 0 12 Z",
    "M6 0 L18 0 L24 12 L18 24 L6 24 L0 12 Z M6 6 L18 6 L18 18 L6 18 Z",
    "M12 2 L22 12 L12 22 L2 12 Z M12 6 L18 12 L12 18 L6 12 Z",
    "M2 2 L22 2 L12 12 Z M2 22 L22 22 L12 12 Z",
];

#[derive(Debug, Default)]
pub struct AlchemicalFamily;

impl GlyphFamily for AlchemicalFamily {
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
    fn sixteen_distinct_glyphs() {
        let family = AlchemicalFamily;
        let mut seen = std::collections::HashSet::new();
        for idx in 0..16 {
            seen.insert(family.glyph(idx));
        }
        assert!(seen.len() >= 16);
    }
}
