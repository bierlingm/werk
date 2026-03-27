//! Restrained 6-color palette for the Operative Instrument.

use ftui::PackedRgba;
use ftui::style::Style;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// The palette: 6 colors only.
// ---------------------------------------------------------------------------

/// Normal text — active tensions, desire/reality content.
pub const CLR_DEFAULT: PackedRgba = PackedRgba::rgb(220, 220, 220);
/// Dim text — resolved, released, labels, separators, chrome.
pub const CLR_DIM: PackedRgba = PackedRgba::rgb(100, 100, 100);
/// Amber — neglect, stagnation, oscillation warnings.
pub const CLR_AMBER: PackedRgba = PackedRgba::rgb(200, 170, 60);
/// Red — conflict only.
pub const CLR_RED: PackedRgba = PackedRgba::rgb(220, 90, 90);
/// Cyan — agent, accents, selection highlight, gaze border.
pub const CLR_CYAN: PackedRgba = PackedRgba::rgb(80, 190, 210);
/// Green — recent positive change, advancing tendency.
pub const CLR_GREEN: PackedRgba = PackedRgba::rgb(80, 190, 120);

/// Background for selected line — very subtle.
pub const CLR_SELECTED_BG: PackedRgba = PackedRgba::rgb(35, 35, 42);
/// Background (terminal default).
pub const CLR_BG: PackedRgba = PackedRgba::rgb(0, 0, 0);

// ---------------------------------------------------------------------------
// Pre-computed styles.
// ---------------------------------------------------------------------------

pub struct Styles {
    /// Normal text.
    pub text: Style,
    /// Bold white (selected tension name).
    pub text_bold: Style,
    /// Subdued text — between normal and dim. For secondary content that should
    /// still be readable (e.g., reality in detail cards).
    pub subdued: Style,
    /// Dim text (labels, chrome, resolved).
    pub dim: Style,
    /// Amber for warnings.
    pub amber: Style,
    /// Red for conflict.
    pub red: Style,
    /// Cyan for accents.
    pub cyan: Style,
    /// Green for positive signals.
    pub green: Style,
    /// Selected line: white on subtle bg.
    pub selected: Style,
    /// Gaze section label (dim, aligned).
    pub label: Style,
    /// Lever / status line.
    pub lever: Style,
}

pub static STYLES: LazyLock<Styles> = LazyLock::new(|| Styles {
    text: Style::new().fg(CLR_DEFAULT),
    text_bold: Style::new().fg(PackedRgba::rgb(255, 255, 255)).bold(),
    subdued: Style::new().fg(PackedRgba::rgb(160, 160, 160)),
    dim: Style::new().fg(CLR_DIM),
    amber: Style::new().fg(CLR_AMBER),
    red: Style::new().fg(CLR_RED),
    cyan: Style::new().fg(CLR_CYAN),
    green: Style::new().fg(CLR_GREEN),
    selected: Style::new().fg(PackedRgba::rgb(255, 255, 255)).bg(CLR_SELECTED_BG),
    label: Style::new().fg(CLR_DIM),
    lever: Style::new().fg(CLR_DIM),
});
