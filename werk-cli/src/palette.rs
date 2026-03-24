//! CLI-specific pathway palette presentation.
//!
//! The shared palette types, builders, and applicators live in werk-shared.
//! This module provides CLI presentation: printing palettes, reading stdin
//! selections, and orchestrating the detect → present → apply flow.

use crate::error::WerkError;
use crate::output::Output;
use sd_core::Store;
use std::io::{self, BufRead, Write as IoWrite};
// Re-export shared types so command handlers can use `palette::Palette`.
pub use werk_shared::palette::{Palette, PaletteChoice};

/// Present a palette to the user and read their choice.
///
/// Three modes:
/// - JSON/structured: returns Dismissed (palette data included in response JSON)
/// - Non-interactive (piped stdin): prints signal to stderr, returns Dismissed
/// - Interactive terminal: prints signal + numbered options, reads selection
fn present_palette(output: &Output, palette: &Palette) -> PaletteChoice {
    if output.is_structured() {
        return PaletteChoice::Dismissed;
    }

    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        eprintln!("  signal: {}", palette.description);
        return PaletteChoice::Dismissed;
    }

    // Interactive terminal
    println!();
    println!("  \u{26a1} {}", palette.description);
    println!();

    for opt in &palette.options {
        println!("    [{}] {}", opt.index, opt.label);
    }

    print!("  choice (enter to dismiss): ");
    let _ = io::stdout().flush();

    let stdin = io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_ok() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return PaletteChoice::Dismissed;
        }
        if let Ok(n) = trimmed.parse::<usize>() {
            if n == 1 {
                return PaletteChoice::Dismissed;
            }
            if n >= 2 && n <= palette.options.len() {
                return PaletteChoice::Selected(n - 1);
            }
        }
        PaletteChoice::Dismissed
    } else {
        PaletteChoice::Dismissed
    }
}

/// After a horizon mutation, detect containment violations and present palettes.
///
/// Returns palette data (for JSON inclusion) regardless of user interaction.
pub fn check_containment_after_horizon(
    output: &Output,
    store: &mut Store,
    tension_id: &str,
) -> Result<Vec<Palette>, WerkError> {
    let detected = werk_shared::detect_containment_palettes(store, tension_id)?;

    let mut palettes = Vec::new();
    for (palette, ctx) in detected {
        let choice = present_palette(output, &palette);
        werk_shared::apply_choice(store, &ctx, &choice)?;
        palettes.push(palette);
    }

    Ok(palettes)
}

/// After a position mutation, detect sequencing pressure and present palettes.
pub fn check_sequencing_after_position(
    output: &Output,
    store: &mut Store,
    tension_id: &str,
) -> Result<Vec<Palette>, WerkError> {
    let detected = werk_shared::detect_sequencing_palettes(store, tension_id)?;

    let mut palettes = Vec::new();
    for (palette, ctx) in detected {
        let choice = present_palette(output, &palette);
        werk_shared::apply_choice(store, &ctx, &choice)?;
        palettes.push(palette);
    }

    Ok(palettes)
}
