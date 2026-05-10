use crate::ctx::Ctx;
use crate::error::SigilError;
use crate::stages::{Layout, StyledScene};

pub trait Stylist {
    fn style(&self, layout: Layout, ctx: &mut Ctx<'_>) -> Result<StyledScene, SigilError>;
}

#[derive(Debug, Clone)]
pub struct InkBrush {
    pub palette: String,
    pub background: String,
    pub stroke_color: String,
    pub fill_color: String,
    pub glyph_color: String,
    pub filter_mode: String,
}

#[derive(Debug, Clone)]
pub struct MinimalLine;

#[derive(Debug, Clone)]
pub struct Glyphic {
    pub mirror: bool,
}

impl Stylist for InkBrush {
    fn style(&self, layout: Layout, _ctx: &mut Ctx<'_>) -> Result<StyledScene, SigilError> {
        let filter = if self.filter_mode == "filter" {
            Some("ink-bleed".to_string())
        } else {
            None
        };
        Ok(StyledScene {
            layout,
            background: Some(self.background.clone()),
            stroke_color: self.stroke_color.clone(),
            fill_color: self.fill_color.clone(),
            glyph_color: self.glyph_color.clone(),
            filter,
            palette_name: self.palette.clone(),
            stroke_only: false,
            glyph_mirror: false,
        })
    }
}

impl Stylist for MinimalLine {
    fn style(&self, layout: Layout, _ctx: &mut Ctx<'_>) -> Result<StyledScene, SigilError> {
        Ok(StyledScene {
            layout,
            background: None,
            stroke_color: "#1a1a1a".into(),
            fill_color: "none".into(),
            glyph_color: "#1a1a1a".into(),
            filter: None,
            palette_name: "mono".into(),
            stroke_only: true,
            glyph_mirror: false,
        })
    }
}

impl Stylist for Glyphic {
    fn style(&self, layout: Layout, _ctx: &mut Ctx<'_>) -> Result<StyledScene, SigilError> {
        Ok(StyledScene {
            layout,
            background: Some("#f8f6f0".into()),
            stroke_color: "#2a2727".into(),
            fill_color: "#2a2727".into(),
            glyph_color: "#2a2727".into(),
            filter: None,
            palette_name: "glyphic".into(),
            stroke_only: false,
            glyph_mirror: self.mirror,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx::Ctx;
    use crate::registry::Primitive;
    use crate::stages::{Layout, MarkSpec, PlacedMark, StructuralMark};
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;
    use werk_core::store::Store;

    fn sample_layout() -> Layout {
        let mark = MarkSpec {
            id: "a".into(),
            primitive: Primitive::Circle,
            channels: HashMap::new(),
        };
        Layout {
            marks: vec![PlacedMark {
                mark,
                cx: 10.0,
                cy: 10.0,
                rotation: 0.0,
                scale: 1.0,
            }],
            structural: vec![StructuralMark {
                path: "M0 0 L10 10".into(),
                stroke_width: 0.4,
                opacity: 0.5,
            }],
        }
    }

    #[test]
    fn applies_palette_and_filter() {
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );
        let stylist = InkBrush {
            palette: "ink_on_cream".into(),
            background: "#f5efe1".into(),
            stroke_color: "#1a1818".into(),
            fill_color: "#1a1818".into(),
            glyph_color: "#1a1818".into(),
            filter_mode: "filter".into(),
        };
        let styled = stylist.style(sample_layout(), &mut ctx).unwrap();
        assert_eq!(styled.background.as_deref(), Some("#f5efe1"));
        assert!(styled.filter.is_some());
    }

    #[test]
    fn stroke_only_palette() {
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );
        let stylist = MinimalLine;
        let styled = stylist.style(sample_layout(), &mut ctx).unwrap();
        assert!(styled.stroke_only);
        assert!(styled.filter.is_none());
    }

    #[test]
    fn glyph_majority() {
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );
        let stylist = Glyphic { mirror: true };
        let styled = stylist.style(sample_layout(), &mut ctx).unwrap();
        assert!(styled.glyph_mirror);
    }
}
