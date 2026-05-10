use std::fmt::Write;

use crate::ctx::Ctx;
use crate::engine::Engine;
use crate::error::SigilError;
use crate::logic::Logic;
use crate::scope::{Scope, ScopeKind};
use crate::sigil::Sigil;

const VIEWBOX: (f64, f64, f64, f64) = (0.0, 0.0, 600.0, 600.0);

#[derive(Debug, Clone)]
pub struct SheetLogic {
    pub inner_logic: Logic,
}

#[derive(Debug, Clone)]
pub struct CompositeLogic {
    pub rule: CompositionRule,
    pub pairs: Vec<(Scope, Logic)>,
}

#[derive(Debug, Clone)]
pub enum CompositionRule {
    Concentric,
    Overlay,
    SideBySide,
    Masked,
}

impl SheetLogic {
    pub fn render(&self, scope: Scope, ctx: &mut Ctx<'_>) -> Result<Sigil, SigilError> {
        self.render_with_depth(scope, ctx, 1)
    }

    pub fn render_with_depth(
        &self,
        scope: Scope,
        ctx: &mut Ctx<'_>,
        depth: usize,
    ) -> Result<Sigil, SigilError> {
        if depth > 4 {
            return Err(SigilError::recursion_limit(depth));
        }
        if scope.kind != ScopeKind::Union {
            return Err(SigilError::construction(
                "sheet expects union scope",
                1,
                1,
            ));
        }
        let mut sigils = Vec::new();
        for member in scope.members.iter() {
            let sigil = Engine::render(member.clone(), self.inner_logic.clone(), ctx)?;
            sigils.push(sigil);
        }
        let svg = compose_grid(&sigils)?;
        Ok(Sigil::new(svg.into_bytes()))
    }
}

impl CompositeLogic {
    pub fn render(&self, ctx: &mut Ctx<'_>) -> Result<Sigil, SigilError> {
        match self.rule {
            CompositionRule::Concentric => {
                let mut sigils = Vec::new();
                for (scope, logic) in self.pairs.iter() {
                    let sigil = Engine::render(scope.clone(), logic.clone(), ctx)?;
                    sigils.push(sigil);
                }
                let svg = compose_concentric(&sigils)?;
                Ok(Sigil::new(svg.into_bytes()))
            }
            CompositionRule::Overlay => Err(SigilError::unsupported("Overlay")),
            CompositionRule::SideBySide => Err(SigilError::unsupported("SideBySide")),
            CompositionRule::Masked => Err(SigilError::unsupported("Masked")),
        }
    }
}

fn compose_grid(sigils: &[Sigil]) -> Result<String, SigilError> {
    let count = sigils.len().max(1);
    let columns = (count as f64).sqrt().ceil() as usize;
    let rows = count.div_ceil(columns);
    let cell_w = VIEWBOX.2 / columns as f64;
    let cell_h = VIEWBOX.3 / rows as f64;
    let scale = (cell_w / VIEWBOX.2).min(cell_h / VIEWBOX.3);

    let mut svg = start_svg();
    for (idx, sigil) in sigils.iter().enumerate() {
        let col = idx % columns;
        let row = idx / columns;
        let cx = (col as f64 + 0.5) * cell_w;
        let cy = (row as f64 + 0.5) * cell_h;
        let body = extract_svg_body(&sigil.svg.0)?;
        let transform = format!(
            "translate({:.2} {:.2}) scale({:.3}) translate(-300 -300)",
            cx, cy, scale
        );
        writeln!(
            svg,
            r#"<g data-sigil-index="{idx}" transform="{transform}">"#
        )
        .map_err(|e| SigilError::render(e.to_string()))?;
        svg.push_str(&body);
        writeln!(svg, "</g>").ok();
    }
    finish_svg(svg)
}

fn compose_concentric(sigils: &[Sigil]) -> Result<String, SigilError> {
    let mut svg = start_svg();
    for (idx, sigil) in sigils.iter().enumerate() {
        let scale = (1.0 - idx as f64 * 0.2).max(0.2);
        let body = extract_svg_body(&sigil.svg.0)?;
        let transform = format!(
            "translate(300 300) scale({scale:.3}) translate(-300 -300)"
        );
        writeln!(
            svg,
            r#"<g data-sigil-index="{idx}" transform="{transform}">"#
        )
        .map_err(|e| SigilError::render(e.to_string()))?;
        svg.push_str(&body);
        writeln!(svg, "</g>").ok();
    }
    finish_svg(svg)
}

fn start_svg() -> String {
    let mut svg = String::new();
    let (vx, vy, vw, vh) = VIEWBOX;
    writeln!(&mut svg, r#"<?xml version="1.0" encoding="UTF-8"?>"#).ok();
    writeln!(
        &mut svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vx} {vy} {vw} {vh}" width="{vw}" height="{vh}">"#
    )
    .ok();
    svg
}

fn finish_svg(mut svg: String) -> Result<String, SigilError> {
    writeln!(svg, "</svg>").map_err(|e| SigilError::render(e.to_string()))?;
    Ok(svg)
}

fn extract_svg_body(svg: &[u8]) -> Result<String, SigilError> {
    let content = String::from_utf8(svg.to_vec())
        .map_err(|e| SigilError::render(format!("invalid svg bytes: {e}")))?;
    let start = content
        .find("<svg")
        .ok_or_else(|| SigilError::render("missing <svg>"))?;
    let after_svg = content[start..]
        .find('>')
        .ok_or_else(|| SigilError::render("malformed <svg>"))?
        + start
        + 1;
    let end = content
        .rfind("</svg>")
        .ok_or_else(|| SigilError::render("missing </svg>"))?;
    Ok(content[after_svg..end].to_string())
}
