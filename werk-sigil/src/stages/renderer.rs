use crate::ctx::Ctx;
use crate::error::SigilError;
use crate::glyphs::{AlchemicalFamily, GeomanticFamily, GlyphFamily, HandDrawnFamily};
use crate::logic::Logic;
use crate::registry::Primitive;
use crate::sigil::Sigil;
use crate::stages::{ChannelValue, StyledScene};
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct SvgRenderer {
    pub viewbox: (f64, f64, f64, f64),
    pub margin: f64,
    pub embed_metadata: bool,
}

impl SvgRenderer {
    pub fn render(
        &self,
        logic: &Logic,
        scope_canonical: &str,
        seed: u64,
        scene: StyledScene,
        ctx: &Ctx<'_>,
    ) -> Result<Sigil, SigilError> {
        let mut svg = String::new();
        let (vx, vy, vw, vh) = self.viewbox;
        writeln!(svg, r#"<?xml version="1.0" encoding="UTF-8"?>"#)
            .map_err(|e| SigilError::render(e.to_string()))?;
        writeln!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vx} {vy} {vw} {vh}" width="{vw}" height="{vh}">"#
        )
        .map_err(|e| SigilError::render(e.to_string()))?;

        if self.embed_metadata {
            writeln!(svg, "<metadata>").ok();
            writeln!(svg, "<werk-sigil>").ok();
            writeln!(svg, "<scope>{}</scope>", scope_canonical).ok();
            writeln!(svg, "<logic>{}</logic>", logic.canonical()).ok();
            writeln!(svg, "<seed>{}</seed>", seed).ok();
            writeln!(svg, "<generated>{}</generated>", ctx.now.to_rfc3339()).ok();
            let mut warnings = ctx.diagnostics.warnings();
            warnings.sort();
            writeln!(svg, r#"<warnings count="{}">"#, warnings.len()).ok();
            for warning in warnings {
                writeln!(svg, "<warning>{}</warning>", warning).ok();
            }
            writeln!(svg, "</warnings>").ok();
            writeln!(svg, "</werk-sigil>").ok();
            writeln!(svg, "</metadata>").ok();
        }

        if let Some(background) = &scene.background {
            writeln!(
                svg,
                r#"<rect x="{vx}" y="{vy}" width="{vw}" height="{vh}" fill="{background}" />"#
            )
            .ok();
        }

        if let Some(filter) = &scene.filter {
            writeln!(svg, "<defs>").ok();
            writeln!(
                svg,
                r#"<filter id="{filter}"><feTurbulence type="fractalNoise" baseFrequency="0.02" numOctaves="2" /><feDisplacementMap in="SourceGraphic" scale="3"/></filter>"#
            )
            .ok();
            writeln!(svg, "</defs>").ok();
        }

        let mut marks = scene.layout.marks.clone();
        marks.sort_by(|a, b| a.mark.id.cmp(&b.mark.id));

        for mark in marks {
            render_mark(&mut svg, &mark, &scene).ok();
        }

        for structural in &scene.layout.structural {
            writeln!(
                svg,
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" opacity="{}" />"#,
                structural.path, scene.stroke_color, structural.stroke_width, structural.opacity
            )
            .ok();
        }

        writeln!(svg, "</svg>").ok();
        Ok(Sigil::new(svg.into_bytes()))
    }
}

fn render_mark(
    svg: &mut String,
    mark: &crate::stages::PlacedMark,
    scene: &StyledScene,
) -> std::fmt::Result {
    let mut primitive = mark.mark.primitive;
    if scene.palette_name == "glyphic" && primitive != Primitive::Glyph {
        let hash_input = if let Some(short_code) = channel_number(&mark.mark.channels, "short_code")
        {
            format!("short-code-{short_code}")
        } else {
            mark.mark.id.clone()
        };
        let hash = blake3::hash(hash_input.as_bytes());
        if hash.as_bytes()[0] % 10 < 7 {
            primitive = Primitive::Glyph;
        }
    }

    let r = channel_number(&mark.mark.channels, "r").unwrap_or(8.0);
    let stroke_width = channel_number(&mark.mark.channels, "stroke_width").unwrap_or(1.0);
    let fill_opacity = channel_number(&mark.mark.channels, "fill_opacity").unwrap_or(0.8);
    let stroke_opacity = channel_number(&mark.mark.channels, "stroke_opacity").unwrap_or(1.0);

    let fill_override = channel_text(&mark.mark.channels, "fill");
    let stroke_override = channel_text(&mark.mark.channels, "stroke");
    let fill = if scene.stroke_only {
        "none"
    } else {
        fill_override.unwrap_or(scene.fill_color.as_str())
    };
    let stroke = stroke_override.unwrap_or(scene.stroke_color.as_str());
    match primitive {
        Primitive::Circle => {
            writeln!(
                svg,
                r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="{:.2}" fill-opacity="{:.2}" stroke-opacity="{:.2}" />"#,
                mark.cx, mark.cy, r, fill, stroke, stroke_width, fill_opacity, stroke_opacity
            )?;
        }
        Primitive::Ellipse => {
            writeln!(
                svg,
                r#"<ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" fill="{}" stroke="{}" stroke-width="{:.2}" fill-opacity="{:.2}" stroke-opacity="{:.2}" />"#,
                mark.cx,
                mark.cy,
                r,
                r * 0.7,
                fill,
                stroke,
                stroke_width,
                fill_opacity,
                stroke_opacity
            )?;
        }
        Primitive::Polygon => {
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                mark.cx,
                mark.cy - r,
                mark.cx - r,
                mark.cy + r,
                mark.cx + r,
                mark.cy + r
            );
            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{:.2}" fill-opacity="{:.2}" stroke-opacity="{:.2}" />"#,
                points, fill, stroke, stroke_width, fill_opacity, stroke_opacity
            )?;
        }
        Primitive::Glyph => {
            let family_name =
                channel_text(&mark.mark.channels, "glyph_family").unwrap_or("alchemical");
            let index = channel_number(&mark.mark.channels, "glyph_index").unwrap_or(0.0) as usize;
            let path = glyph_path(family_name, index);
            if scene.glyph_mirror {
                writeln!(
                    svg,
                    r#"<g class="glyph-mirror" transform="translate({:.2},{:.2}) scale(-1,1)"><path d="{}" fill="{}" stroke="{}" stroke-width="{:.2}" /></g>"#,
                    mark.cx, mark.cy, path, scene.glyph_color, stroke, stroke_width
                )?;
            } else {
                writeln!(
                    svg,
                    r#"<path class="glyph" transform="translate({:.2},{:.2})" d="{}" fill="{}" stroke="{}" stroke-width="{:.2}" />"#,
                    mark.cx, mark.cy, path, scene.glyph_color, stroke, stroke_width
                )?;
            }
        }
    }
    Ok(())
}

fn channel_number(
    channels: &std::collections::HashMap<String, ChannelValue>,
    key: &str,
) -> Option<f64> {
    match channels.get(key) {
        Some(ChannelValue::Number(value)) => Some(*value),
        _ => None,
    }
}

fn channel_text<'a>(
    channels: &'a std::collections::HashMap<String, ChannelValue>,
    key: &str,
) -> Option<&'a str> {
    match channels.get(key) {
        Some(ChannelValue::Text(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn glyph_path(name: &str, idx: usize) -> &'static str {
    match name {
        "geomantic" => GeomanticFamily.glyph(idx),
        "handdrawn" => HandDrawnFamily.glyph(idx),
        _ => AlchemicalFamily.glyph(idx),
    }
}
