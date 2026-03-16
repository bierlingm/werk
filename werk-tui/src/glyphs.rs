//! Symbol vocabulary for the Operative Instrument.

use sd_core::{CreativeCyclePhase, StructuralTendency, TensionStatus};

/// Phase glyph: ◇ Germination, ◆ Assimilation, ◈ Completion, ◉ Momentum.
pub fn phase_glyph(phase: CreativeCyclePhase) -> &'static str {
    match phase {
        CreativeCyclePhase::Germination => "\u{25C7}",  // ◇
        CreativeCyclePhase::Assimilation => "\u{25C6}", // ◆
        CreativeCyclePhase::Completion => "\u{25C8}",   // ◈
        CreativeCyclePhase::Momentum => "\u{25C9}",     // ◉
    }
}

/// Status-aware glyph: ✦ resolved, · released, else phase glyph.
pub fn status_glyph(status: TensionStatus, phase: CreativeCyclePhase) -> &'static str {
    match status {
        TensionStatus::Resolved => "\u{2726}", // ✦
        TensionStatus::Released => "\u{00B7}", // ·
        TensionStatus::Active => phase_glyph(phase),
    }
}

/// Tendency arrow.
pub fn tendency_char(tendency: StructuralTendency) -> &'static str {
    match tendency {
        StructuralTendency::Advancing => "\u{2192}",   // →
        StructuralTendency::Oscillating => "\u{2194}",  // ↔
        StructuralTendency::Stagnant => "\u{25CB}",     // ○
    }
}

/// Tendency as a word (for Gaze children summary).
pub fn tendency_word(tendency: StructuralTendency) -> &'static str {
    match tendency {
        StructuralTendency::Advancing => "advancing",
        StructuralTendency::Oscillating => "oscillating",
        StructuralTendency::Stagnant => "stagnant",
    }
}

/// Phase as a word.
pub fn phase_word(phase: CreativeCyclePhase) -> &'static str {
    match phase {
        CreativeCyclePhase::Germination => "germination",
        CreativeCyclePhase::Assimilation => "assimilation",
        CreativeCyclePhase::Completion => "completion",
        CreativeCyclePhase::Momentum => "momentum",
    }
}

/// Activity trail: ○● dots showing weekly mutation activity.
/// Each dot represents one time bucket: ● = active, ○ = quiet.
pub fn trail(activity: &[f64], max_dots: usize) -> String {
    if activity.is_empty() {
        return String::new();
    }
    activity
        .iter()
        .rev()
        .take(max_dots)
        .map(|&v| if v > 0.0 { "\u{25CF}" } else { "\u{25CB}" }) // ● or ○
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// Gap bar: ████░░░░
pub fn gap_bar(magnitude: f64, width: usize) -> String {
    let filled = ((magnitude * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    )
}

// Separator constants
pub const LIGHT_RULE: char = '\u{2504}'; // ┄
pub const RULE: char = '\u{2500}';       // ─
pub const HEAVY_RULE: char = '\u{2501}'; // ━
