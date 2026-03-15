// Color constants and semantic theme system for the TUI.

use ftui::PackedRgba;
use ftui::style::Style;
use sd_core::CreativeCyclePhase;
use sd_core::StructuralTendency;

use crate::types::UrgencyTier;

// ---------------------------------------------------------------------------
// Legacy color constants — referenced across 15+ files, do not remove.
// ---------------------------------------------------------------------------

pub const CLR_WHITE: PackedRgba = PackedRgba::rgb(255, 255, 255);
pub const CLR_LIGHT_GRAY: PackedRgba = PackedRgba::rgb(200, 200, 200);
pub const CLR_MID_GRAY: PackedRgba = PackedRgba::rgb(120, 120, 120);
pub const CLR_DIM_GRAY: PackedRgba = PackedRgba::rgb(100, 100, 100);
pub const CLR_RED: PackedRgba = PackedRgba::rgb(255, 80, 80);
pub const CLR_RED_SOFT: PackedRgba = PackedRgba::rgb(255, 100, 100);
pub const CLR_GREEN: PackedRgba = PackedRgba::rgb(80, 200, 120);
pub const CLR_YELLOW: PackedRgba = PackedRgba::rgb(255, 200, 60);
pub const CLR_YELLOW_SOFT: PackedRgba = PackedRgba::rgb(200, 180, 80);
pub const CLR_CYAN: PackedRgba = PackedRgba::rgb(80, 200, 220);
pub const CLR_BG_DARK: PackedRgba = PackedRgba::rgb(30, 30, 30);

// ---------------------------------------------------------------------------
// Semantic theme
// ---------------------------------------------------------------------------

/// Semantic color mappings for the entire TUI.
///
/// Every field is a `PackedRgba` so it can be passed directly to
/// `Style::fg()` / `Style::bg()` without conversion.
#[derive(Debug, Clone, Copy)]
pub struct WerkTheme {
    // Text hierarchy
    pub text: PackedRgba,
    pub text_muted: PackedRgba,
    pub text_subtle: PackedRgba,
    pub text_disabled: PackedRgba,

    // Backgrounds
    pub bg: PackedRgba,
    pub surface: PackedRgba,

    // Accent
    pub accent: PackedRgba,

    // Semantic status
    pub success: PackedRgba,
    pub warning: PackedRgba,
    pub warning_bright: PackedRgba,
    pub error: PackedRgba,
    pub error_soft: PackedRgba,

    // Chrome
    pub border: PackedRgba,

    // Selection & focus
    pub highlight: PackedRgba,
    pub surface_selected: PackedRgba,
    pub text_accent: PackedRgba,
    pub border_active: PackedRgba,

    // Movement-specific colors
    pub advancing: PackedRgba,
    pub oscillating: PackedRgba,
    pub stagnant: PackedRgba,

    // Creative-cycle phase colors
    pub phase_germination: PackedRgba,
    pub phase_assimilation: PackedRgba,
    pub phase_completion: PackedRgba,
    pub phase_momentum: PackedRgba,
}

/// The application-wide theme instance.
pub const WERK_THEME: WerkTheme = WerkTheme {
    text: CLR_WHITE,
    text_muted: CLR_LIGHT_GRAY,
    text_subtle: CLR_MID_GRAY,
    text_disabled: CLR_DIM_GRAY,

    bg: CLR_BG_DARK,
    surface: PackedRgba::rgb(40, 40, 45),

    accent: CLR_CYAN,

    success: CLR_GREEN,
    warning: CLR_YELLOW_SOFT,
    warning_bright: CLR_YELLOW,
    error: CLR_RED,
    error_soft: CLR_RED_SOFT,

    border: CLR_DIM_GRAY,

    highlight: PackedRgba::rgb(45, 45, 55),
    surface_selected: PackedRgba::rgb(50, 50, 60),
    text_accent: PackedRgba::rgb(140, 180, 220),
    border_active: CLR_CYAN,

    advancing: CLR_GREEN,
    oscillating: CLR_YELLOW,
    stagnant: CLR_MID_GRAY,

    phase_germination: PackedRgba::rgb(0, 180, 180),
    phase_assimilation: PackedRgba::rgb(80, 140, 220),
    phase_completion: CLR_GREEN,
    phase_momentum: PackedRgba::rgb(160, 120, 220),
};

// ---------------------------------------------------------------------------
// Pre-computed composite styles
// ---------------------------------------------------------------------------

/// Pre-computed styles to avoid per-frame `Style::new().fg().bold()` chains.
pub struct WerkStyles {
    pub label: Style,
    pub value: Style,
    pub value_bold: Style,
    pub muted: Style,
    pub accent: Style,
    pub accent_bold: Style,
    pub danger: Style,
    pub warn: Style,
    pub success: Style,
    pub status_bar: Style,
    pub hint_key: Style,
    pub hint_desc: Style,
}

use std::sync::LazyLock;

pub static STYLES: LazyLock<WerkStyles> = LazyLock::new(|| WerkStyles {
    label: Style::new().fg(CLR_MID_GRAY),
    value: Style::new().fg(CLR_LIGHT_GRAY),
    value_bold: Style::new().fg(CLR_WHITE).bold(),
    muted: Style::new().fg(CLR_DIM_GRAY),
    accent: Style::new().fg(CLR_CYAN),
    accent_bold: Style::new().fg(CLR_CYAN).bold(),
    danger: Style::new().fg(CLR_RED_SOFT),
    warn: Style::new().fg(CLR_YELLOW_SOFT),
    success: Style::new().fg(CLR_GREEN),
    status_bar: Style::new().fg(CLR_LIGHT_GRAY).bold(),
    hint_key: Style::new().fg(CLR_CYAN),
    hint_desc: Style::new().fg(CLR_DIM_GRAY),
});

// ---------------------------------------------------------------------------
// Semantic style helpers
// ---------------------------------------------------------------------------

/// Return the color associated with a creative-cycle phase.
pub fn phase_color(phase: CreativeCyclePhase) -> PackedRgba {
    match phase {
        CreativeCyclePhase::Germination => WERK_THEME.phase_germination,
        CreativeCyclePhase::Assimilation => WERK_THEME.phase_assimilation,
        CreativeCyclePhase::Completion => WERK_THEME.phase_completion,
        CreativeCyclePhase::Momentum => WERK_THEME.phase_momentum,
    }
}

/// Return the color associated with an urgency tier.
pub fn tier_color(tier: UrgencyTier) -> PackedRgba {
    match tier {
        UrgencyTier::Urgent => WERK_THEME.error_soft,
        UrgencyTier::Active => WERK_THEME.text_muted,
        UrgencyTier::Neglected => WERK_THEME.warning,
        UrgencyTier::Resolved => WERK_THEME.text_disabled,
    }
}

/// Return the color associated with a structural tendency.
pub fn movement_color(tendency: StructuralTendency) -> PackedRgba {
    match tendency {
        StructuralTendency::Advancing => WERK_THEME.advancing,
        StructuralTendency::Oscillating => WERK_THEME.oscillating,
        StructuralTendency::Stagnant => WERK_THEME.stagnant,
    }
}
