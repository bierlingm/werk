//! Pathway palettes — shared data types, builders, and applicators.
//!
//! Palette *presentation* is surface-specific (CLI reads stdin, TUI uses
//! key-driven selection). This module provides everything except presentation:
//! the data types, the palette builders (signal → options), and the applicators
//! (choice → store mutation).

use crate::error::WerkError;
use serde::Serialize;
use werk_core::{ContainmentViolation, Forest, SequencingPressure, Store, Tension};

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
/// Options: keep as-is, clip child to parent, extend parent to match child,
///          promote child to sibling, remove child deadline.
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

    // Promote option: reparent child to grandparent (or root if parent is root)
    let promote_label = if parent.parent_id.is_some() {
        format!(
            "Promote {} to sibling of {} (reparent under grandparent)",
            child_display, parent_display
        )
    } else {
        format!(
            "Promote {} to root (escape containment frame)",
            child_display
        )
    };

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
            PaletteOption {
                index: 4,
                label: promote_label,
                action: "promote_child".to_string(),
            },
            PaletteOption {
                index: 5,
                label: format!("Remove {} deadline entirely", child_display),
                action: "remove_child_deadline".to_string(),
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
                    .map_err(WerkError::CoreError)?;
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
                    .map_err(WerkError::CoreError)?;
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
        PaletteChoice::Selected(3) => {
            // Promote child to sibling (reparent to grandparent, or root)
            let new_parent = parent.parent_id.as_deref();
            let _ = store.begin_gesture(Some(&format!(
                "palette: promote {} to sibling of {}",
                &child.id, &parent.id
            )));
            store
                .update_parent(&child.id, new_parent)
                .map_err(WerkError::CoreError)?;
            store.end_gesture();
            let child_display = crate::display_id(child.short_code, &child.id);
            let parent_display = crate::display_id(parent.short_code, &parent.id);
            match new_parent {
                Some(_) => Ok(Some(format!(
                    "Promoted {} to sibling of {}",
                    child_display, parent_display
                ))),
                None => Ok(Some(format!("Promoted {} to root", child_display))),
            }
        }
        PaletteChoice::Selected(4) => {
            // Remove child deadline entirely
            let _ = store.begin_gesture(Some(&format!("palette: remove {} deadline", &child.id)));
            store
                .update_horizon(&child.id, None)
                .map_err(WerkError::CoreError)?;
            store.end_gesture();
            let child_display = crate::display_id(child.short_code, &child.id);
            Ok(Some(format!("Removed {} deadline", child_display)))
        }
        _ => Ok(None),
    }
}

// ── Sequencing pressure ────────────────────────────────────────────────

/// Build a palette for sequencing pressure.
///
/// Signal: a tension is ordered later but has an earlier deadline than its predecessor.
/// Options: keep as-is, swap positions, move before predecessor, hold tension.
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
            PaletteOption {
                index: 3,
                label: format!(
                    "Move {} before {} (position {})",
                    t_display, p_display, p_pos
                ),
                action: "move_before".to_string(),
            },
            PaletteOption {
                index: 4,
                label: format!("Hold {} (remove position)", t_display),
                action: "hold_tension".to_string(),
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
                    .map_err(WerkError::CoreError)?;
                store
                    .update_position(&predecessor.id, Some(tp))
                    .map_err(WerkError::CoreError)?;
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
        PaletteChoice::Selected(2) => {
            // Move before predecessor: tension takes predecessor's position,
            // predecessor shifts down by one
            let p_pos = predecessor.position;
            if let Some(pp) = p_pos {
                let _ = store.begin_gesture(Some(&format!(
                    "palette: move {} before {}",
                    &tension.id, &predecessor.id
                )));
                store
                    .update_position(&tension.id, Some(pp))
                    .map_err(WerkError::CoreError)?;
                store
                    .update_position(&predecessor.id, Some(pp + 1))
                    .map_err(WerkError::CoreError)?;
                store.end_gesture();
                let t_display = crate::display_id(tension.short_code, &tension.id);
                let p_display = crate::display_id(predecessor.short_code, &predecessor.id);
                Ok(Some(format!(
                    "Moved {} to position {}, {} shifted to {}",
                    t_display,
                    pp,
                    p_display,
                    pp + 1
                )))
            } else {
                Ok(None)
            }
        }
        PaletteChoice::Selected(3) => {
            // Hold tension (remove position)
            let _ = store.begin_gesture(Some(&format!("palette: hold {}", &tension.id)));
            store
                .update_position(&tension.id, None)
                .map_err(WerkError::CoreError)?;
            store.end_gesture();
            let t_display = crate::display_id(tension.short_code, &tension.id);
            Ok(Some(format!("Held {} (removed position)", t_display)))
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
    let forest =
        Forest::from_tensions(tensions.clone()).map_err(|e| WerkError::IoError(e.to_string()))?;

    let tension = match tensions.iter().find(|t| t.id == tension_id) {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let mut results = Vec::new();

    // Check if this tension's deadline violates its parent's
    if let Some(ref parent_id) = tension.parent_id {
        let violations = werk_core::detect_containment_violations(&forest, parent_id);
        for v in &violations {
            if v.tension_id == tension_id
                && let Some(parent) = tensions.iter().find(|t| t.id == *parent_id)
            {
                let palette = containment_palette(v, tension, parent);
                let ctx = PaletteContext::Containment {
                    child: tension.clone(),
                    parent: parent.clone(),
                };
                results.push((palette, ctx));
            }
        }
    }

    // Check if this tension is a parent whose children now violate
    let violations = werk_core::detect_containment_violations(&forest, tension_id);
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
    let forest =
        Forest::from_tensions(tensions.clone()).map_err(|e| WerkError::IoError(e.to_string()))?;

    let tension = match tensions.iter().find(|t| t.id == tension_id) {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let mut results = Vec::new();

    if let Some(ref parent_id) = tension.parent_id {
        let pressures = werk_core::detect_sequencing_pressure(&forest, parent_id);
        for p in &pressures {
            if p.tension_id == tension_id
                && let Some(predecessor) = tensions.iter().find(|t| t.id == p.predecessor_id)
            {
                let palette = sequencing_palette(p, tension, predecessor);
                let ctx = PaletteContext::Sequencing {
                    tension: tension.clone(),
                    predecessor: predecessor.clone(),
                };
                results.push((palette, ctx));
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

#[cfg(test)]
mod tests {
    use super::*;
    use werk_core::{Horizon, Store};

    /// Helper: create a parent with deadline and a child with deadline, return their IDs.
    fn setup_containment_scenario(
        store: &mut Store,
        parent_horizon: &str,
        child_horizon: &str,
    ) -> (String, String) {
        let parent = store
            .create_tension_full(
                "parent desired",
                "parent actual",
                None,
                Some(Horizon::parse(parent_horizon).unwrap()),
            )
            .unwrap();
        let child = store
            .create_tension_full(
                "child desired",
                "child actual",
                Some(parent.id.clone()),
                Some(Horizon::parse(child_horizon).unwrap()),
            )
            .unwrap();
        (parent.id, child.id)
    }

    /// Helper: create a parent with two positioned children, return (parent_id, child1_id, child2_id).
    fn setup_sequencing_scenario(
        store: &mut Store,
        c1_pos: i32,
        c1_horizon: &str,
        c2_pos: i32,
        c2_horizon: &str,
    ) -> (String, String, String) {
        let parent = store
            .create_tension_full(
                "parent",
                "parent actual",
                None,
                Some(Horizon::parse("2026-12").unwrap()),
            )
            .unwrap();
        let c1 = store
            .create_tension_full(
                "child1",
                "c1 actual",
                Some(parent.id.clone()),
                Some(Horizon::parse(c1_horizon).unwrap()),
            )
            .unwrap();
        store.update_position(&c1.id, Some(c1_pos)).unwrap();
        let c2 = store
            .create_tension_full(
                "child2",
                "c2 actual",
                Some(parent.id.clone()),
                Some(Horizon::parse(c2_horizon).unwrap()),
            )
            .unwrap();
        store.update_position(&c2.id, Some(c2_pos)).unwrap();
        (parent.id, c1.id, c2.id)
    }

    // ── Containment palette builder tests ─────────────────────────────

    #[test]
    fn containment_palette_has_five_options() {
        let mut store = Store::new_in_memory().unwrap();
        let (parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        assert_eq!(
            palettes.len(),
            1,
            "should detect exactly one containment violation"
        );
        let (palette, _) = &palettes[0];
        assert_eq!(palette.signal, "containment_violation");
        assert_eq!(palette.options.len(), 5);
        assert_eq!(palette.options[0].action, "dismiss");
        assert_eq!(palette.options[1].action, "clip_child");
        assert_eq!(palette.options[2].action, "extend_parent");
        assert_eq!(palette.options[3].action, "promote_child");
        assert_eq!(palette.options[4].action, "remove_child_deadline");

        // Verify parent ID is mentioned in the description
        let parent = store.get_tension(&parent_id).unwrap().unwrap();
        let parent_display = crate::display_id(parent.short_code, &parent.id);
        assert!(palette.description.contains(&parent_display));
    }

    #[test]
    fn no_containment_palette_when_child_within_parent_deadline() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, child_id) = setup_containment_scenario(&mut store, "2026-12", "2026-06");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        assert!(
            palettes.is_empty(),
            "no violation when child deadline is within parent"
        );
    }

    #[test]
    fn no_containment_palette_for_root_tension() {
        let store = Store::new_in_memory().unwrap();
        let t = store
            .create_tension_full(
                "root",
                "actual",
                None,
                Some(Horizon::parse("2026-06").unwrap()),
            )
            .unwrap();
        let palettes = detect_containment_palettes(&store, &t.id).unwrap();
        assert!(palettes.is_empty(), "root tension has no parent to violate");
    }

    #[test]
    fn containment_detected_from_parent_perspective() {
        let mut store = Store::new_in_memory().unwrap();
        let (parent_id, _child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        // Detect from the parent's perspective (checks children that violate)
        let palettes = detect_containment_palettes(&store, &parent_id).unwrap();
        assert_eq!(palettes.len(), 1, "parent should see child's violation");
    }

    // ── Containment applicator tests ──────────────────────────────────

    #[test]
    fn dismiss_containment_does_nothing() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Dismissed).unwrap();
        assert!(result.is_none());

        // Child deadline unchanged
        let child = store.get_tension(&child_id).unwrap().unwrap();
        assert_eq!(child.horizon.unwrap().to_string(), "2026-09");
    }

    #[test]
    fn clip_child_sets_child_to_parent_deadline() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(1)).unwrap();
        assert!(result.is_some(), "clip should return description");
        assert!(result.unwrap().contains("Clipped"));

        let child = store.get_tension(&child_id).unwrap().unwrap();
        assert_eq!(child.horizon.unwrap().to_string(), "2026-06");
    }

    #[test]
    fn extend_parent_sets_parent_to_child_deadline() {
        let mut store = Store::new_in_memory().unwrap();
        let (parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(2)).unwrap();
        assert!(result.is_some(), "extend should return description");
        assert!(result.unwrap().contains("Extended"));

        let parent = store.get_tension(&parent_id).unwrap().unwrap();
        assert_eq!(parent.horizon.unwrap().to_string(), "2026-09");
    }

    #[test]
    fn promote_child_reparents_to_grandparent() {
        let mut store = Store::new_in_memory().unwrap();
        let grandparent = store
            .create_tension_full(
                "grandparent",
                "gp actual",
                None,
                Some(Horizon::parse("2026-12").unwrap()),
            )
            .unwrap();
        let parent = store
            .create_tension_full(
                "parent",
                "p actual",
                Some(grandparent.id.clone()),
                Some(Horizon::parse("2026-06").unwrap()),
            )
            .unwrap();
        let child = store
            .create_tension_full(
                "child",
                "c actual",
                Some(parent.id.clone()),
                Some(Horizon::parse("2026-09").unwrap()),
            )
            .unwrap();

        let palettes = detect_containment_palettes(&store, &child.id).unwrap();
        assert_eq!(palettes.len(), 1);
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(3)).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Promoted"));

        let updated_child = store.get_tension(&child.id).unwrap().unwrap();
        assert_eq!(
            updated_child.parent_id.as_deref(),
            Some(grandparent.id.as_str())
        );
    }

    #[test]
    fn promote_child_to_root_when_parent_is_root() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(3)).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("root"));

        let updated_child = store.get_tension(&child_id).unwrap().unwrap();
        assert!(updated_child.parent_id.is_none());
    }

    #[test]
    fn remove_child_deadline_clears_horizon() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(4)).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Removed"));

        let updated_child = store.get_tension(&child_id).unwrap().unwrap();
        assert!(updated_child.horizon.is_none());
    }

    // ── Sequencing palette builder tests ──────────────────────────────

    #[test]
    fn sequencing_palette_has_four_options() {
        let mut store = Store::new_in_memory().unwrap();
        // c1 at position 1 with later deadline, c2 at position 2 with earlier deadline
        // c2 is ordered later but has earlier deadline → sequencing pressure on c2
        let (_parent_id, _c1_id, c2_id) =
            setup_sequencing_scenario(&mut store, 1, "2026-09", 2, "2026-03");
        let palettes = detect_sequencing_palettes(&store, &c2_id).unwrap();
        assert_eq!(palettes.len(), 1, "should detect sequencing pressure");
        let (palette, _) = &palettes[0];
        assert_eq!(palette.signal, "sequencing_pressure");
        assert_eq!(palette.options.len(), 4);
        assert_eq!(palette.options[0].action, "dismiss");
        assert_eq!(palette.options[1].action, "swap_positions");
        assert_eq!(palette.options[2].action, "move_before");
        assert_eq!(palette.options[3].action, "hold_tension");
    }

    #[test]
    fn no_sequencing_pressure_when_order_matches_deadlines() {
        let mut store = Store::new_in_memory().unwrap();
        // c1 at pos 1 with earlier deadline, c2 at pos 2 with later — no pressure
        let (_parent_id, _c1_id, c2_id) =
            setup_sequencing_scenario(&mut store, 1, "2026-03", 2, "2026-09");
        let palettes = detect_sequencing_palettes(&store, &c2_id).unwrap();
        assert!(
            palettes.is_empty(),
            "no pressure when order matches deadlines"
        );
    }

    // ── Sequencing applicator tests ───────────────────────────────────

    #[test]
    fn swap_positions_swaps_correctly() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, c1_id, c2_id) =
            setup_sequencing_scenario(&mut store, 1, "2026-09", 2, "2026-03");
        let palettes = detect_sequencing_palettes(&store, &c2_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(1)).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Swapped"));

        let c1 = store.get_tension(&c1_id).unwrap().unwrap();
        let c2 = store.get_tension(&c2_id).unwrap().unwrap();
        assert_eq!(c1.position, Some(2));
        assert_eq!(c2.position, Some(1));
    }

    #[test]
    fn move_before_predecessor_shifts_positions() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, c1_id, c2_id) =
            setup_sequencing_scenario(&mut store, 1, "2026-09", 2, "2026-03");
        let palettes = detect_sequencing_palettes(&store, &c2_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(2)).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Moved"));

        let c1 = store.get_tension(&c1_id).unwrap().unwrap();
        let c2 = store.get_tension(&c2_id).unwrap().unwrap();
        // c2 takes c1's position (1), c1 shifts to 2
        assert_eq!(c2.position, Some(1));
        assert_eq!(c1.position, Some(2));
    }

    #[test]
    fn hold_tension_removes_position() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, _c1_id, c2_id) =
            setup_sequencing_scenario(&mut store, 1, "2026-09", 2, "2026-03");
        let palettes = detect_sequencing_palettes(&store, &c2_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(3)).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Held"));

        let c2 = store.get_tension(&c2_id).unwrap().unwrap();
        assert!(c2.position.is_none());
    }

    #[test]
    fn dismiss_sequencing_leaves_positions_unchanged() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, c1_id, c2_id) =
            setup_sequencing_scenario(&mut store, 1, "2026-09", 2, "2026-03");
        let palettes = detect_sequencing_palettes(&store, &c2_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Dismissed).unwrap();
        assert!(result.is_none());

        let c1 = store.get_tension(&c1_id).unwrap().unwrap();
        let c2 = store.get_tension(&c2_id).unwrap().unwrap();
        assert_eq!(c1.position, Some(1));
        assert_eq!(c2.position, Some(2));
    }

    // ── Edge cases ────────────────────────────────────────────────────

    #[test]
    fn nonexistent_tension_returns_empty_palettes() {
        let store = Store::new_in_memory().unwrap();
        let palettes = detect_containment_palettes(&store, "nonexistent").unwrap();
        assert!(palettes.is_empty());
        let palettes = detect_sequencing_palettes(&store, "nonexistent").unwrap();
        assert!(palettes.is_empty());
    }

    #[test]
    fn tension_without_deadline_no_containment_palette() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension_full(
                "parent",
                "actual",
                None,
                Some(Horizon::parse("2026-06").unwrap()),
            )
            .unwrap();
        let child = store
            .create_tension_full("child", "actual", Some(parent.id.clone()), None)
            .unwrap();
        let palettes = detect_containment_palettes(&store, &child.id).unwrap();
        assert!(
            palettes.is_empty(),
            "no violation when child has no deadline"
        );
    }

    #[test]
    fn parent_without_deadline_no_containment_palette() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension_full("parent", "actual", None, None)
            .unwrap();
        let child = store
            .create_tension_full(
                "child",
                "actual",
                Some(parent.id.clone()),
                Some(Horizon::parse("2026-06").unwrap()),
            )
            .unwrap();
        let palettes = detect_containment_palettes(&store, &child.id).unwrap();
        assert!(
            palettes.is_empty(),
            "no violation when parent has no deadline"
        );
    }

    #[test]
    fn unpositioned_tension_no_sequencing_palette() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension_full("parent", "actual", None, None)
            .unwrap();
        let child = store
            .create_tension_full(
                "child",
                "actual",
                Some(parent.id.clone()),
                Some(Horizon::parse("2026-03").unwrap()),
            )
            .unwrap();
        // child has no position — should not trigger sequencing pressure
        let palettes = detect_sequencing_palettes(&store, &child.id).unwrap();
        assert!(palettes.is_empty(), "no pressure for unpositioned tension");
    }

    #[test]
    fn palette_choice_selected_0_acts_as_dismiss() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        let (_, ctx) = &palettes[0];

        // Selected(0) maps to "option 1" which is dismiss
        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(0)).unwrap();
        assert!(result.is_none());

        let child = store.get_tension(&child_id).unwrap().unwrap();
        assert_eq!(child.horizon.unwrap().to_string(), "2026-09");
    }

    #[test]
    fn out_of_range_selection_acts_as_dismiss() {
        let mut store = Store::new_in_memory().unwrap();
        let (_parent_id, child_id) = setup_containment_scenario(&mut store, "2026-06", "2026-09");
        let palettes = detect_containment_palettes(&store, &child_id).unwrap();
        let (_, ctx) = &palettes[0];

        let result = apply_choice(&mut store, ctx, &PaletteChoice::Selected(99)).unwrap();
        assert!(result.is_none());
    }
}
