//! AdaptiveColor theme for the Operative Instrument.
//!
//! Color philosophy (see designs/tui-reimagined.md Section III):
//! - Monochrome foundation — text, muted, subtle form a luminance ramp.
//! - Color appears ONLY as exception signal. Amber = neglect/urgency.
//!   Red = conflict. Green = positive change. Cyan = agent/accent.
//! - Every slot has dark and light variants via AdaptiveColor.

use ftui::{AdaptiveColor, Color, PackedRgba, ResolvedTheme, Rgb, Theme};
use ftui::style::Style;

// ---------------------------------------------------------------------------
// Theme construction
// ---------------------------------------------------------------------------

/// Build the instrument's theme with dark/light adaptive colors.
pub fn instrument_theme() -> Theme {
    Theme::builder()
        // Monochrome foundation
        .text(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 40, g: 40, b: 40 }),     // light terminal
            Color::Rgb(Rgb { r: 220, g: 220, b: 220 }),  // dark terminal
        ))
        .text_muted(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 120, g: 120, b: 120 }),
            Color::Rgb(Rgb { r: 100, g: 100, b: 100 }),
        ))
        .text_subtle(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 100, g: 100, b: 100 }),
            Color::Rgb(Rgb { r: 160, g: 160, b: 160 }),
        ))
        // Accent — cyan
        .accent(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 30, g: 140, b: 170 }),   // darker on light
            Color::Rgb(Rgb { r: 80, g: 190, b: 210 }),   // bright on dark
        ))
        // Exception colors — signal only
        .warning(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 170, g: 120, b: 20 }),   // darker amber on light
            Color::Rgb(Rgb { r: 200, g: 170, b: 60 }),   // warm amber on dark
        ))
        .error(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 180, g: 50, b: 50 }),
            Color::Rgb(Rgb { r: 220, g: 90, b: 90 }),
        ))
        .success(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 40, g: 140, b: 80 }),
            Color::Rgb(Rgb { r: 80, g: 190, b: 120 }),
        ))
        // Surfaces
        .background(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 255, g: 255, b: 255 }),
            Color::Rgb(Rgb { r: 0, g: 0, b: 0 }),
        ))
        .surface(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 240, g: 240, b: 245 }),
            Color::Rgb(Rgb { r: 35, g: 35, b: 42 }),
        ))
        .border(AdaptiveColor::adaptive(
            Color::Rgb(Rgb { r: 200, g: 200, b: 210 }),
            Color::Rgb(Rgb { r: 60, g: 60, b: 70 }),
        ))
        .build()
}

// ---------------------------------------------------------------------------
// Color bridge
// ---------------------------------------------------------------------------

/// Convert an ftui Color to PackedRgba for the rendering layer.
pub fn resolve_color(color: Color) -> PackedRgba {
    match color {
        Color::Rgb(rgb) => PackedRgba::rgb(rgb.r, rgb.g, rgb.b),
        _ => PackedRgba::rgb(220, 220, 220),
    }
}

// ---------------------------------------------------------------------------
// Pre-resolved styles
// ---------------------------------------------------------------------------

/// All styles resolved to `PackedRgba` for direct rendering use.
///
/// Field names mirror the `Styles` API used across deck.rs, render.rs and
/// survey.rs so callers can reach for styles by role (amber, selected, dim…)
/// without knowing about theme resolution.
pub struct InstrumentStyles {
    // Styles (Style objects for Span/Line rendering)
    pub text: Style,
    pub text_bold: Style,
    pub subdued: Style,
    pub dim: Style,
    pub amber: Style,
    pub red: Style,
    pub cyan: Style,
    pub green: Style,
    pub selected: Style,
    pub label: Style,
    pub lever: Style,

    // Raw colors (PackedRgba for Cell manipulation, cursor styling, etc.)
    pub clr_text: PackedRgba,
    pub clr_dim: PackedRgba,
    pub clr_amber: PackedRgba,
    pub clr_red: PackedRgba,
    pub clr_cyan: PackedRgba,
    pub clr_green: PackedRgba,
    pub clr_selected_bg: PackedRgba,
    pub clr_bg: PackedRgba,
}

impl InstrumentStyles {
    /// Resolve all styles from a theme for the detected terminal mode.
    pub fn resolve(resolved: &ResolvedTheme) -> Self {
        let text_fg = resolve_color(resolved.text);
        let dim_fg = resolve_color(resolved.text_muted);
        let subdued_fg = resolve_color(resolved.text_subtle);
        let accent_fg = resolve_color(resolved.accent);
        let warn_fg = resolve_color(resolved.warning);
        let err_fg = resolve_color(resolved.error);
        let ok_fg = resolve_color(resolved.success);
        let surface_bg = resolve_color(resolved.surface);
        let bg = resolve_color(resolved.background);

        // text_bold: always white on dark, always black on light.
        // We derive this from the background: if bg is dark, bold is white.
        let bold_fg = if bg.r() < 128 {
            PackedRgba::rgb(255, 255, 255)
        } else {
            PackedRgba::rgb(0, 0, 0)
        };

        // selected: bold fg on surface bg
        let selected_fg = bold_fg;

        Self {
            text: Style::new().fg(text_fg),
            text_bold: Style::new().fg(bold_fg).bold(),
            subdued: Style::new().fg(subdued_fg),
            dim: Style::new().fg(dim_fg),
            amber: Style::new().fg(warn_fg),
            red: Style::new().fg(err_fg),
            cyan: Style::new().fg(accent_fg),
            green: Style::new().fg(ok_fg),
            selected: Style::new().fg(selected_fg).bg(surface_bg),
            label: Style::new().fg(dim_fg),
            lever: Style::new().fg(dim_fg),

            clr_text: text_fg,
            clr_dim: dim_fg,
            clr_amber: warn_fg,
            clr_red: err_fg,
            clr_cyan: accent_fg,
            clr_green: ok_fg,
            clr_selected_bg: surface_bg,
            clr_bg: bg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_produce_different_colors() {
        let theme = instrument_theme();
        let dark = theme.resolve(true);
        let light = theme.resolve(false);

        // Text should be different (light text on dark bg vs dark text on light bg)
        assert_ne!(dark.text, light.text, "text should differ");
        assert_ne!(dark.warning, light.warning, "warning should differ");
        assert_ne!(dark.accent, light.accent, "accent should differ");
        assert_ne!(dark.error, light.error, "error should differ");
        assert_ne!(dark.success, light.success, "success should differ");
        assert_ne!(dark.background, light.background, "background should differ");
        assert_ne!(dark.surface, light.surface, "surface should differ");
    }

    #[test]
    fn resolve_styles_produces_valid_fg_colors() {
        let theme = instrument_theme();
        for is_dark in [true, false] {
            let resolved = theme.resolve(is_dark);
            let styles = InstrumentStyles::resolve(&resolved);
            // All raw colors should be non-transparent
            assert_ne!(styles.clr_text, PackedRgba::TRANSPARENT);
            assert_ne!(styles.clr_dim, PackedRgba::TRANSPARENT);
            assert_ne!(styles.clr_cyan, PackedRgba::TRANSPARENT);
            assert_ne!(styles.clr_amber, PackedRgba::TRANSPARENT);
        }
    }

    #[test]
    fn bold_adapts_to_background() {
        let theme = instrument_theme();
        let dark_styles = InstrumentStyles::resolve(&theme.resolve(true));
        let light_styles = InstrumentStyles::resolve(&theme.resolve(false));
        // Bold on dark = white, bold on light = black
        assert_eq!(dark_styles.text_bold.fg, Some(PackedRgba::rgb(255, 255, 255)));
        assert_eq!(light_styles.text_bold.fg, Some(PackedRgba::rgb(0, 0, 0)));
    }
}
