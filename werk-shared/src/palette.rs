//! Pathway palettes — shared data types, builders, and applicators.
//!
//! Palette *presentation* is surface-specific (CLI reads stdin, TUI uses
//! key-driven selection). This module provides everything except presentation:
//! the data types, the palette builders (signal → options), and the applicators
//! (choice → store mutation).

use crate::error::WerkError;
use sd_core::{
    ContainmentViolation, Forest, SequencingPressure, Store, Tension,
};
use serde::Serialize;

// ── Data types ─────────────────────────────────────────────────────────

/// A single option in a pathway palette.
#[derive(Debug, Clone, Serialize)]
pub struct PaletteOption {
    /// 1-based index for user selection.
    pub index: usize,
    /// Short label describing the action.
    pub label: String,
    /// Machine-readable action key (for JSON consumers / programmatic handling).
    pub action: String,
}

/// A pathway palette — a signal with response options.
#[derive(Debug, Clone, Serialize)]
pub struct Palette {
    /// What structural signal was detected.
    pub signal: String,
    /// Human-readable description of the signal.
    pub description: String,
    /// Available response options (always includes dismiss as option 1).
    pub options: Vec<PaletteOption>,
}

/// Result of presenting a palette to the user.
pub enum PaletteChoice {
    /// User selected an option (0-based index into options vec).
    Selected(usize),
    /// User dismissed the palette.
    Dismissed,
}

// ── Containment violation ──────────────────────────────────────────────

/// Build a palette for a containment violation.
///
/// Signal: child deadline exceeds parent deadline.
/// Options: keep as-is, clip child to parent, extend parent to match child.
pub fn containment_palette(
    violation: &ContainmentViolation,
    child: &Tension,
    parent: &Tension,
) -> Palette {
    let child_display = crate::display_id(child.short_code, &child.id);
    let parent_display = crate::display_id(parent.short_code, &parent.id);
    let excess_days = violation.excess_seconds / 86400;

    let parent_horizon_str = parent
        .horizon
        .as_ref()
        .map(|h| h.to_string())
        .unwrap_or_else(|| "none".to_string());
    let child_horizon_str = child
        .horizon
        .as_ref()
        .map(|h| h.to_string())
        .unwrap_or_else(|| "none".to_string());

    Palette {
        signal: "containment_violation".to_string(),
        description: format!(
            "Deadline for {} ({}) exceeds parent {} deadline ({}) by ~{} days",
            child_display, child_horizon_str, parent_display, parent_horizon_str, excess_days
        ),
        options: vec![
            PaletteOption {
                index: 1,
                label: "Keep as-is".to_string(),
                action: "dismiss".to_string(),
            },
            PaletteOption {
                index: 2,
                label: format!(
                    "Clip {} to parent deadline ({})",
                    child_display, parent_horizon_str
                ),
                action: "clip_child".to_string(),
            },
            PaletteOption {
                index: 3,
                label: format!(
                    "Extend {} deadline to match ({})",
                    parent_display, child_horizon_str
                ),
                action: "extend_parent".to_string(),
            },
        ],
    }
}

/// Apply a containment violation palette choice to the store.
///
/// Returns a human-readable description of what was done, or None if dismissed.
pub fn apply_containment_choice(
    store: &mut Store,
    child: &Tension,
    parent: &Tension,
    choice: &PaletteChoice,
) -> Result<Option<String>, WerkError> {
    match choice {
        PaletteChoice::Dismissed | PaletteChoice::Selected(0) => Ok(None),
        PaletteChoice::Selected(1) => {
            // Clip child to parent deadline
            if let Some(ref parent_h) = parent.horizon {
                let _ = store.begin_gesture(Some(&format!(
                    "palette: clip {} to parent deadline",
                    &child.id
                )));
                store
                    .update_horizon(&child.id, Some(parent_h.clone()))
                    .map_err(WerkError::SdError)?;
                store.end_gesture();
                let child_display = crate::display_id(child.short_code, &child.id);
                Ok(Some(format!(
                    "Clipped {} deadline to {}",
                    child_display, parent_h
                )))
            } else {
                Ok(None)
            }
        }
        PaletteChoice::Selected(2) => {
            // Extend parent to match child
            if let Some(ref child_h) = child.horizon {
                let _ = store.begin_gesture(Some(&format!(
                    "palette: extend {} deadline to match child",
                    &parent.id
                )));
                store
                    .update_horizon(&parent.id, Some(child_h.clone()))
                    .map_err(WerkError::SdError)?;
                store.end_gesture();
                let parent_display = crate::display_id(parent.short_code, &parent.id);
                Ok(Some(format!(
                    "Extended {} deadline to {}",
                    parent_display, child_h
                )))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

// ── Sequencing pressure ────────────────────────────────────────────────

/// Build a palette for sequencing pressure.
///
/// Signal: a tension is ordered later but has an earlier deadline than its predecessor.
/// Options: keep as-is, swap positions.
pub fn sequencing_palette(
    pressure: &SequencingPressure,
    tension: &Tension,
    predecessor: &Tension,
) -> Palette {
    let t_display = crate::display_id(tension.short_code, &tension.id);
    let p_display = crate::display_id(predecessor.short_code, &predecessor.id);
    let gap_days = pressure.gap_seconds.abs() / 86400;

    let t_pos = tension
        .position
        .map(|p| p.to_string())
        .unwrap_or_else(|| "held".to_string());
    let p_pos = predecessor
        .position
        .map(|p| p.to_string())
        .unwrap_or_else(|| "held".to_string());

    Palette {
        signal: "sequencing_pressure".to_string(),
        description: format!(
            "{} (position {}) has a deadline ~{} days before predecessor {} (position {})",
            t_display, t_pos, gap_days, p_display, p_pos
        ),
        options: vec![
            PaletteOption {
                index: 1,
                label: "Keep as-is (noteworthy, not necessarily wrong)".to_string(),
                action: "dismiss".to_string(),
            },
            PaletteOption {
                index: 2,
                label: format!(
                    "Swap positions: {} to {}, {} to {}",
                    t_display, p_pos, p_display, t_pos
                ),
                action: "swap_positions".to_string(),
            },
        ],
    }
}

/// Apply a sequencing pressure palette choice to the store.
pub fn apply_sequencing_choice(
    store: &mut Store,
    tension: &Tension,
    predecessor: &Tension,
    choice: &PaletteChoice,
) -> Result<Option<String>, WerkError> {
    match choice {
        PaletteChoice::Dismissed | PaletteChoice::Selected(0) => Ok(None),
        PaletteChoice::Selected(1) => {
            // Swap positions
            let t_pos = tension.position;
            let p_pos = predecessor.position;
            if let (Some(tp), Some(pp)) = (t_pos, p_pos) {
                let _ = store.begin_gesture(Some(&format!(
                    "palette: swap positions {} and {}",
                    &tension.id, &predecessor.id
                )));
                store
                    .update_position(&tension.id, Some(pp))
                    .map_err(WerkError::SdError)?;
                store
                    .update_position(&predecessor.id, Some(tp))
                    .map_err(WerkError::SdError)?;
                store.end_gesture();
                let t_display = crate::display_id(tension.short_code, &tension.id);
                let p_display = crate::display_id(predecessor.short_code, &predecessor.id);
                Ok(Some(format!(
                    "Swapped: {} now at position {}, {} at position {}",
                    t_display, pp, p_display, tp
                )))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

// ── Detection helpers ──────────────────────────────────────────────────

/// Detect containment violations after a horizon change.
///
/// Checks both directions: this tension as child (violating parent),
/// and this tension as parent (children now violating).
///
/// Returns (palettes, tensions_snapshot) — the caller provides presentation
/// and feeds choices back through the apply functions.
pub fn detect_containment_palettes(
    store: &Store,
    tension_id: &str,
) -> Result<Vec<(Palette, PaletteContext)>, WerkError> {
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let forest = Forest::from_tensions(tensions.clone())
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    let tension = match tensions.iter().find(|t| t.id == tension_id) {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let mut results = Vec::new();

    // Check if this tension's deadline violates its parent's
    if let Some(ref parent_id) = tension.parent_id {
        let violations = sd_core::detect_containment_violations(&forest, parent_id);
        for v in &violations {
            if v.tension_id == tension_id {
                if let Some(parent) = tensions.iter().find(|t| t.id == *parent_id) {
                    let palette = containment_palette(v, tension, parent);
                    let ctx = PaletteContext::Containment {
                        child: tension.clone(),
                        parent: parent.clone(),
                    };
                    results.push((palette, ctx));
                }
            }
        }
    }

    // Check if this tension is a parent whose children now violate
    let violations = sd_core::detect_containment_violations(&forest, tension_id);
    for v in &violations {
        if let Some(child) = tensions.iter().find(|t| t.id == v.tension_id) {
            let palette = containment_palette(v, child, tension);
            let ctx = PaletteContext::Containment {
                child: child.clone(),
                parent: tension.clone(),
            };
            results.push((palette, ctx));
        }
    }

    Ok(results)
}

/// Detect sequencing pressure after a position change.
pub fn detect_sequencing_palettes(
    store: &Store,
    tension_id: &str,
) -> Result<Vec<(Palette, PaletteContext)>, WerkError> {
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let forest = Forest::from_tensions(tensions.clone())
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    let tension = match tensions.iter().find(|t| t.id == tension_id) {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let mut results = Vec::new();

    if let Some(ref parent_id) = tension.parent_id {
        let pressures = sd_core::detect_sequencing_pressure(&forest, parent_id);
        for p in &pressures {
            if p.tension_id == tension_id {
                if let Some(predecessor) = tensions.iter().find(|t| t.id == p.predecessor_id) {
                    let palette = sequencing_palette(p, tension, predecessor);
                    let ctx = PaletteContext::Sequencing {
                        tension: tension.clone(),
                        predecessor: predecessor.clone(),
                    };
                    results.push((palette, ctx));
                }
            }
        }
    }

    Ok(results)
}

/// Context needed to apply a palette choice. Passed back to apply functions
/// so the caller doesn't need to re-derive the tensions.
pub enum PaletteContext {
    Containment {
        child: Tension,
        parent: Tension,
    },
    Sequencing {
        tension: Tension,
        predecessor: Tension,
    },
}

/// Apply a choice given its context. Dispatches to the right apply function.
pub fn apply_choice(
    store: &mut Store,
    ctx: &PaletteContext,
    choice: &PaletteChoice,
) -> Result<Option<String>, WerkError> {
    match ctx {
        PaletteContext::Containment { child, parent } => {
            apply_containment_choice(store, child, parent, choice)
        }
        PaletteContext::Sequencing {
            tension,
            predecessor,
        } => apply_sequencing_choice(store, tension, predecessor, choice),
    }
}
