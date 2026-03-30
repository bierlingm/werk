//! Calculus of Time — all temporal computations.
//!
//! This module contains every computation that derives from structure and time.
//! No inference, no speculation — only facts and signals.
//!
//! **From the foundation's two user-set primitives** (deadline/horizon, order/position):
//! - **Urgency**: elapsed fraction of a deadline window
//! - **Horizon drift**: pattern of deadline changes over time
//! - **Implied execution window**: temporal gap between predecessor and successor deadlines
//! - **Sequencing pressure**: order conflicts with deadline ordering
//! - **Critical path**: child deadline crowds parent deadline (recursive)
//! - **Containment violation**: child deadline exceeds parent deadline
//!
//! **Binary gap detection:**
//! - **Gap magnitude**: whether desired != actual (a fact, not a similarity measure)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::mutation::Mutation;
use crate::tension::{Tension, TensionStatus};
use crate::tree::Forest;

// ============================================================================
// Gap Detection (honest — binary, not text-similarity)
// ============================================================================

/// Returns 1.0 if desired != actual, 0.0 if identical.
///
/// This replaces the former text-similarity based magnitude computation
/// which pretended to quantify "how different" two text strings were.
/// The honest answer: either there's a gap or there isn't.
pub fn gap_magnitude(desired: &str, actual: &str) -> f64 {
    if desired == actual { 0.0 } else { 1.0 }
}

// ============================================================================
// Urgency
// ============================================================================

/// Urgency — the temporal pressure on a tension.
///
/// Only computable when a horizon is present. Represents the ratio
/// of elapsed time to total time window.
///
/// - `value = 0.0` → just created, full window ahead
/// - `value = 0.5` → halfway through the time window
/// - `value = 1.0` → at the horizon's end
/// - `value > 1.0` → past the horizon
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Urgency {
    /// The tension ID this urgency is computed for.
    pub tension_id: String,
    /// The urgency value: elapsed / total time window.
    pub value: f64,
    /// Seconds remaining until horizon.range_end().
    pub time_remaining: i64,
    /// Total seconds from created_at to horizon.range_end().
    pub total_window: i64,
}

/// Compute urgency as the ratio of elapsed time to total time window.
///
/// Urgency is only computable when a horizon is present. A tension
/// without a horizon is "outside the urgency frame entirely" — not
/// "not urgent" but genuinely absent.
pub fn compute_urgency(tension: &Tension, now: DateTime<Utc>) -> Option<Urgency> {
    let horizon = tension.horizon.as_ref()?;

    let time_elapsed = (now - tension.created_at).num_seconds().max(0);
    let total_window = (horizon.range_end() - tension.created_at).num_seconds();
    // Guard against zero/negative total window
    let total_window_guarded = total_window.max(1);

    let time_remaining = (horizon.range_end() - now).num_seconds().max(0);
    let value = time_elapsed as f64 / total_window_guarded as f64;

    Some(Urgency {
        tension_id: tension.id.clone(),
        value,
        time_remaining,
        total_window: total_window_guarded,
    })
}

// ============================================================================
// Horizon Drift
// ============================================================================

/// The type of horizon drift pattern detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HorizonDriftType {
    /// No horizon changes.
    Stable,
    /// Net shift earlier or to higher precision (Year → Month → Day).
    Tightening,
    /// Single shift later.
    Postponement,
    /// 3+ shifts later.
    RepeatedPostponement,
    /// Net shift later or to lower precision (Day → Month → Year).
    Loosening,
    /// Back and forth pattern (alternating directions).
    Oscillating,
}

/// Horizon drift — pattern of horizon changes over time.
///
/// Detected from mutations where field == "horizon".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HorizonDrift {
    /// The tension ID this drift is computed for.
    pub tension_id: String,
    /// The type of drift pattern detected.
    pub drift_type: HorizonDriftType,
    /// Number of horizon changes recorded.
    pub change_count: usize,
    /// Net shift in seconds (positive = postponed, negative = tightened).
    pub net_shift_seconds: i64,
    /// When this drift pattern first emerged (timestamp of the mutation that tipped the pattern).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onset: Option<DateTime<Utc>>,
}

/// Detect horizon drift pattern from mutation history.
///
/// Horizon drift is detected from mutations where field == "horizon".
/// The pattern reveals how the practitioner's temporal commitment
/// has evolved over time.
pub fn detect_horizon_drift(tension_id: &str, mutations: &[Mutation]) -> HorizonDrift {
    use crate::Horizon;

    // Filter to horizon mutations only
    let horizon_mutations: Vec<&Mutation> = mutations
        .iter()
        .filter(|m| m.tension_id() == tension_id && m.field() == "horizon")
        .collect();

    let change_count = horizon_mutations.len();

    // No horizon changes = Stable
    if change_count == 0 {
        return HorizonDrift {
            tension_id: tension_id.to_string(),
            drift_type: HorizonDriftType::Stable,
            change_count: 0,
            net_shift_seconds: 0,
            onset: None,
        };
    }

    // Parse horizon mutations and compute shifts, tracking timestamps for onset.
    let mut shifts: Vec<(i64, DateTime<Utc>)> = Vec::new(); // (shift_seconds, timestamp)
    let mut precision_tightenings = 0i32;
    let mut precision_loosenings = 0i32;

    for mutation in &horizon_mutations {
        let old_horizon = mutation.old_value().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Horizon::parse(s).ok()
            }
        });
        let new_horizon = if mutation.new_value().is_empty() {
            None
        } else {
            Horizon::parse(mutation.new_value()).ok()
        };

        match (old_horizon, new_horizon) {
            (None, Some(_new)) => {
                // Setting horizon for the first time - not a shift
            }
            (Some(old), Some(new)) => {
                let shift = (new.range_end() - old.range_end()).num_seconds();
                shifts.push((shift, mutation.timestamp()));

                let old_precision = old.precision_level();
                let new_precision = new.precision_level();
                if new_precision < old_precision {
                    precision_tightenings += 1;
                } else if new_precision > old_precision {
                    precision_loosenings += 1;
                }
            }
            (Some(_old), None) => {}
            (None, None) => {}
        }
    }

    let net_shift_seconds: i64 = shifts.iter().map(|(s, _)| s).sum();

    // Count direction changes for oscillation detection, tracking when 2nd change occurs.
    let mut direction_changes = 0;
    let mut last_positive: Option<bool> = None;
    let mut second_direction_change_ts: Option<DateTime<Utc>> = None;
    for (shift, ts) in &shifts {
        let is_positive = *shift >= 0;
        if let Some(was_positive) = last_positive
            && is_positive != was_positive
        {
            direction_changes += 1;
            if direction_changes == 2 {
                second_direction_change_ts = Some(*ts);
            }
        }
        last_positive = Some(is_positive);
    }

    // CRITICAL: Empty shifts (only None->Some assignments) = Stable baseline
    if shifts.is_empty() {
        return HorizonDrift {
            tension_id: tension_id.to_string(),
            drift_type: HorizonDriftType::Stable,
            change_count,
            net_shift_seconds: 0,
            onset: None,
        };
    }

    // Priority: Oscillating > Precision-based > Time-based
    let (drift_type, onset) = if direction_changes >= 2 {
        (HorizonDriftType::Oscillating, second_direction_change_ts)
    } else if precision_tightenings > precision_loosenings {
        // Onset: first shift (any direction — precision dominates)
        (HorizonDriftType::Tightening, Some(shifts[0].1))
    } else if precision_loosenings > precision_tightenings {
        (HorizonDriftType::Loosening, Some(shifts[0].1))
    } else if shifts.iter().all(|(s, _)| *s > 0) {
        if shifts.len() >= 3 {
            // RepeatedPostponement: onset = 3rd postponement
            (HorizonDriftType::RepeatedPostponement, Some(shifts[2].1))
        } else {
            // Postponement: onset = 1st postponement
            (HorizonDriftType::Postponement, Some(shifts[0].1))
        }
    } else if shifts.iter().all(|(s, _)| *s < 0) {
        (HorizonDriftType::Tightening, Some(shifts[0].1))
    } else if net_shift_seconds > 0 {
        (HorizonDriftType::Loosening, Some(shifts[0].1))
    } else if net_shift_seconds < 0 {
        (HorizonDriftType::Tightening, Some(shifts[0].1))
    } else {
        (HorizonDriftType::Stable, None)
    };

    HorizonDrift {
        tension_id: tension_id.to_string(),
        drift_type,
        change_count,
        net_shift_seconds,
        onset,
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Implied execution window — the temporal gap available for a positioned step.
///
/// For a positioned step with a horizon, this is the gap between the predecessor's
/// deadline and this step's deadline. If no predecessor has a horizon, the window
/// starts at `now`. If this step has no horizon but a successor does, the window
/// extends to the successor's deadline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpliedWindow {
    pub tension_id: String,
    /// Start of the implied window (predecessor's deadline, or now).
    pub window_start: DateTime<Utc>,
    /// End of the implied window (this step's deadline, or successor's deadline).
    pub window_end: DateTime<Utc>,
    /// Duration of the window in seconds.
    pub duration_seconds: i64,
}

/// Sequencing pressure — a step ordered later has an earlier deadline than a preceding step.
///
/// Not necessarily wrong (may reflect genuine real-world pressure) but always noteworthy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequencingPressure {
    /// The tension that has the pressure (ordered later, deadline earlier).
    pub tension_id: String,
    /// The preceding sibling whose deadline is later.
    pub predecessor_id: String,
    /// Short code of the predecessor (for display).
    pub predecessor_short_code: Option<i32>,
    /// How many seconds earlier this step's deadline is vs the predecessor's.
    pub gap_seconds: i64,
}

/// Critical path — a child whose deadline crowds the parent's deadline.
///
/// Recursive: if a critical-path child has children, their critical-path
/// children are also on the critical path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticalPath {
    /// The child tension on the critical path.
    pub tension_id: String,
    /// The parent tension whose deadline is crowded.
    pub parent_id: String,
    /// Fraction of the parent's remaining window consumed by this child's deadline.
    /// 1.0 = deadline matches parent. >1.0 = exceeds (also a containment violation).
    pub crowding_ratio: f64,
    /// Seconds between this child's deadline and the parent's deadline.
    pub slack_seconds: i64,
}

/// Containment violation — a child's deadline exceeds its parent's deadline.
///
/// The foundation says the instrument offers pathways: keep as-is, clip to parent,
/// promote to sibling, extend parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainmentViolation {
    /// The child tension whose deadline exceeds parent.
    pub tension_id: String,
    /// The parent tension.
    pub parent_id: String,
    /// How many seconds past the parent's deadline.
    pub excess_seconds: i64,
}

// ============================================================================
// Computation Functions
// ============================================================================

/// Compute implied execution windows for all positioned children of a given parent.
///
/// Siblings are sorted by position. For each positioned sibling with (or bounded by)
/// a horizon, the implied window is the gap between the previous positioned sibling's
/// deadline and this one's deadline.
pub fn compute_implied_windows(
    forest: &Forest,
    parent_id: &str,
    now: DateTime<Utc>,
) -> Vec<ImpliedWindow> {
    let children = match forest.children(parent_id) {
        Some(c) => c,
        None => return vec![],
    };

    // Get active, positioned children sorted by position
    let mut positioned: Vec<&Tension> = children
        .iter()
        .map(|n| &n.tension)
        .filter(|t| t.status == TensionStatus::Active && t.position.is_some())
        .collect();
    positioned.sort_by_key(|t| t.position.unwrap()); // ubs:ignore filter guarantees is_some()

    let mut results = Vec::new();

    for (i, tension) in positioned.iter().enumerate() {
        let this_end = tension.horizon.as_ref().map(|h| h.range_end());

        // Find predecessor deadline: walk backward through positioned siblings
        let pred_end = positioned[..i]
            .iter()
            .rev()
            .find_map(|t| t.horizon.as_ref().map(|h| h.range_end()));

        // Window start: predecessor's deadline or now
        let window_start = pred_end.unwrap_or(now);

        // Window end: this step's deadline, or next sibling's deadline
        let window_end = this_end.or_else(|| {
            positioned[i + 1..]
                .iter()
                .find_map(|t| t.horizon.as_ref().map(|h| h.range_end()))
        });

        if let Some(end) = window_end {
            let duration = (end - window_start).num_seconds();
            results.push(ImpliedWindow {
                tension_id: tension.id.clone(),
                window_start,
                window_end: end,
                duration_seconds: duration,
            });
        }
    }

    results
}

/// Detect sequencing pressure among positioned siblings of a parent.
///
/// Sequencing pressure exists when a step ordered later (higher position) has an
/// earlier deadline than a preceding step (lower position). The order says "wait"
/// but the deadline says "now."
pub fn detect_sequencing_pressure(forest: &Forest, parent_id: &str) -> Vec<SequencingPressure> {
    let children = match forest.children(parent_id) {
        Some(c) => c,
        None => return vec![],
    };

    // Get active, positioned children with horizons, sorted by position
    let mut positioned: Vec<&Tension> = children
        .iter()
        .map(|n| &n.tension)
        .filter(|t| {
            t.status == TensionStatus::Active && t.position.is_some() && t.horizon.is_some()
        })
        .collect();
    positioned.sort_by_key(|t| t.position.unwrap()); // ubs:ignore filter guarantees is_some()

    let mut results = Vec::new();

    for i in 1..positioned.len() {
        let current = positioned[i];
        let current_end = current.horizon.as_ref().unwrap().range_end(); // ubs:ignore filter guarantees is_some()

        // Check against all predecessors (not just immediate — pressure can skip)
        for pred in &positioned[..i] {
            let pred_end = pred.horizon.as_ref().unwrap().range_end(); // ubs:ignore filter guarantees is_some()

            if current_end < pred_end {
                let gap = (pred_end - current_end).num_seconds();
                results.push(SequencingPressure {
                    tension_id: current.id.clone(),
                    predecessor_id: pred.id.clone(),
                    predecessor_short_code: pred.short_code,
                    gap_seconds: gap,
                });
            }
        }
    }

    results
}

/// Detect critical path children for a given parent.
///
/// A child is on the critical path when its deadline crowds the parent's deadline.
/// "Crowds" means the child's deadline is within the last 20% of the remaining
/// window between now and the parent's deadline, or within 7 days — whichever
/// is larger.
///
/// Returns results sorted by crowding_ratio (most critical first).
pub fn detect_critical_path(
    forest: &Forest,
    parent_id: &str,
    now: DateTime<Utc>,
) -> Vec<CriticalPath> {
    let parent = match forest.find(parent_id) {
        Some(n) => &n.tension,
        None => return vec![],
    };

    let parent_end = match parent.horizon.as_ref() {
        Some(h) => h.range_end(),
        None => return vec![], // No parent deadline — no critical path possible
    };

    let children = match forest.children(parent_id) {
        Some(c) => c,
        None => return vec![],
    };

    let parent_remaining = (parent_end - now).num_seconds().max(1);

    let mut results: Vec<CriticalPath> = children
        .iter()
        .filter(|n| n.tension.status == TensionStatus::Active)
        .filter_map(|n| {
            let child_end = n.tension.horizon.as_ref()?.range_end();
            let slack = (parent_end - child_end).num_seconds();
            let crowding_ratio = 1.0 - (slack as f64 / parent_remaining as f64);

            // Critical if crowding_ratio >= 0.8 (within last 20% of parent window)
            // or slack <= 7 days
            let seven_days = 7 * 86400;
            if crowding_ratio >= 0.8 || slack <= seven_days {
                Some(CriticalPath {
                    tension_id: n.tension.id.clone(),
                    parent_id: parent_id.to_string(),
                    crowding_ratio,
                    slack_seconds: slack,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.crowding_ratio
            .partial_cmp(&a.crowding_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Recursively find all tensions on the critical path from a root.
///
/// Walks down the tree: if a child is on the critical path of its parent,
/// and that child has children, check those children against the child's deadline.
pub fn detect_critical_path_recursive(
    forest: &Forest,
    root_id: &str,
    now: DateTime<Utc>,
) -> Vec<CriticalPath> {
    let mut all = Vec::new();
    let mut stack = vec![root_id.to_string()];

    while let Some(parent_id) = stack.pop() {
        let critical = detect_critical_path(forest, &parent_id, now);
        for cp in &critical {
            stack.push(cp.tension_id.clone());
        }
        all.extend(critical);
    }

    all
}

/// Detect containment violations for all children of a given parent.
///
/// A containment violation exists when a child's deadline exceeds the parent's deadline.
pub fn detect_containment_violations(
    forest: &Forest,
    parent_id: &str,
) -> Vec<ContainmentViolation> {
    let parent = match forest.find(parent_id) {
        Some(n) => &n.tension,
        None => return vec![],
    };

    let parent_end = match parent.horizon.as_ref() {
        Some(h) => h.range_end(),
        None => return vec![], // No parent deadline — no violation possible
    };

    let children = match forest.children(parent_id) {
        Some(c) => c,
        None => return vec![],
    };

    children
        .iter()
        .filter(|n| n.tension.status == TensionStatus::Active)
        .filter_map(|n| {
            let child_end = n.tension.horizon.as_ref()?.range_end();
            if child_end > parent_end {
                let excess = (child_end - parent_end).num_seconds();
                Some(ContainmentViolation {
                    tension_id: n.tension.id.clone(),
                    parent_id: parent_id.to_string(),
                    excess_seconds: excess,
                })
            } else {
                None
            }
        })
        .collect()
}

// ============================================================================
// Convenience: compute all temporal signals for a tension and its children
// ============================================================================

/// All temporal signals relevant to a single tension viewed in context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemporalSignals {
    /// Implied execution window for this tension (if positioned within a parent).
    pub implied_window: Option<ImpliedWindow>,
    /// Sequencing pressure on this tension (if any).
    pub sequencing_pressures: Vec<SequencingPressure>,
    /// Children on the critical path (if this tension has children and a horizon).
    pub critical_path: Vec<CriticalPath>,
    /// Children with containment violations (if this tension has children and a horizon).
    pub containment_violations: Vec<ContainmentViolation>,
    /// Whether this tension itself is on the critical path of its parent.
    pub on_critical_path: bool,
    /// Whether this tension has a containment violation against its parent.
    pub has_containment_violation: bool,
}

/// Compute all temporal signals for a given tension.
pub fn compute_temporal_signals(
    forest: &Forest,
    tension_id: &str,
    now: DateTime<Utc>,
) -> TemporalSignals {
    let tension = match forest.find(tension_id) {
        Some(n) => &n.tension,
        None => return TemporalSignals::default(),
    };

    // Signals about this tension's children
    let critical_path = detect_critical_path(forest, tension_id, now);
    let containment_violations = detect_containment_violations(forest, tension_id);

    // Signals about this tension relative to its parent
    let (implied_window, sequencing_pressures, on_critical_path, has_containment_violation) =
        if let Some(parent_id) = &tension.parent_id {
            // Implied window: find this tension's window among siblings
            let windows = compute_implied_windows(forest, parent_id, now);
            let my_window = windows
                .into_iter()
                .find(|w| w.tension_id == tension_id);

            // Sequencing pressure: find pressures involving this tension
            let pressures = detect_sequencing_pressure(forest, parent_id);
            let my_pressures: Vec<_> = pressures
                .into_iter()
                .filter(|p| p.tension_id == tension_id)
                .collect();

            // Am I on my parent's critical path?
            let parent_critical = detect_critical_path(forest, parent_id, now);
            let on_cp = parent_critical.iter().any(|cp| cp.tension_id == tension_id);

            // Do I violate my parent's containment?
            let parent_violations = detect_containment_violations(forest, parent_id);
            let has_cv = parent_violations
                .iter()
                .any(|cv| cv.tension_id == tension_id);

            (my_window, my_pressures, on_cp, has_cv)
        } else {
            (None, vec![], false, false)
        };

    TemporalSignals {
        implied_window,
        sequencing_pressures,
        critical_path,
        containment_violations,
        on_critical_path,
        has_containment_violation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::Horizon;
    use crate::mutation::Mutation;
    use crate::tension::{Tension, TensionStatus};
    use chrono::TimeZone;

    fn make_tension(id: &str, parent: Option<&str>, position: Option<i32>, horizon: Option<&str>) -> Tension {
        let h = horizon.and_then(|s| Horizon::parse(s).ok());
        let mut t = Tension::new_full(
            &format!("desired {}", id),
            &format!("actual {}", id),
            parent.map(|s| s.to_string()),
            h,
        )
        .unwrap();
        t.id = id.to_string();
        t.position = position;
        t.short_code = Some(id.parse::<i32>().unwrap_or(0));
        t
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 22, 12, 0, 0).unwrap()
    }

    // === Implied Window Tests ===

    #[test]
    fn implied_window_basic() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-06")),
            make_tension("2", Some("1"), Some(1), Some("2026-04")),
            make_tension("3", Some("1"), Some(2), Some("2026-05")),
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let windows = compute_implied_windows(&forest, "1", now());

        assert_eq!(windows.len(), 2);
        // First step: window from now to its deadline (April)
        assert_eq!(windows[0].tension_id, "2");
        // Second step: window from April to May
        assert_eq!(windows[1].tension_id, "3");
        assert!(windows[1].window_start > now()); // starts after step 2's deadline
    }

    #[test]
    fn implied_window_no_positioned_children() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-06")),
            make_tension("2", Some("1"), None, None), // held, no horizon
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let windows = compute_implied_windows(&forest, "1", now());
        assert!(windows.is_empty());
    }

    #[test]
    fn implied_window_step_without_horizon_uses_successor() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-06")),
            make_tension("2", Some("1"), Some(1), None),         // no horizon
            make_tension("3", Some("1"), Some(2), Some("2026-05")),
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let windows = compute_implied_windows(&forest, "1", now());

        // Step 2 has no horizon but successor does — window extends to May
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].tension_id, "2");
        assert_eq!(windows[1].tension_id, "3");
    }

    // === Sequencing Pressure Tests ===

    #[test]
    fn sequencing_pressure_detected() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-06")),
            make_tension("2", Some("1"), Some(1), Some("2026-05")), // first, May deadline
            make_tension("3", Some("1"), Some(2), Some("2026-04")), // second, April deadline — pressure!
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let pressures = detect_sequencing_pressure(&forest, "1");

        assert_eq!(pressures.len(), 1);
        assert_eq!(pressures[0].tension_id, "3");
        assert_eq!(pressures[0].predecessor_id, "2");
        assert!(pressures[0].gap_seconds > 0);
    }

    #[test]
    fn no_sequencing_pressure_when_aligned() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-06")),
            make_tension("2", Some("1"), Some(1), Some("2026-04")),
            make_tension("3", Some("1"), Some(2), Some("2026-05")),
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let pressures = detect_sequencing_pressure(&forest, "1");
        assert!(pressures.is_empty());
    }

    // === Critical Path Tests ===

    #[test]
    fn critical_path_detected() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-04")),       // parent: April
            make_tension("2", Some("1"), Some(1), Some("2026-04")), // child: also April — critical!
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let critical = detect_critical_path(&forest, "1", now());

        assert_eq!(critical.len(), 1);
        assert_eq!(critical[0].tension_id, "2");
        assert!(critical[0].crowding_ratio >= 0.8);
    }

    #[test]
    fn critical_path_not_detected_for_distant_child() {
        // Parent deadline June, child deadline April — plenty of slack
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let tensions = vec![
            make_tension("1", None, None, Some("2026-12")),
            make_tension("2", Some("1"), Some(1), Some("2026-04")),
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let critical = detect_critical_path(&forest, "1", now);
        assert!(critical.is_empty());
    }

    #[test]
    fn critical_path_recursive() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-04")),
            make_tension("2", Some("1"), Some(1), Some("2026-04")),
            make_tension("3", Some("2"), Some(1), Some("2026-04")),
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let critical = detect_critical_path_recursive(&forest, "1", now());

        // Both child and grandchild should be on critical path
        assert!(critical.len() >= 2);
        assert!(critical.iter().any(|cp| cp.tension_id == "2"));
        assert!(critical.iter().any(|cp| cp.tension_id == "3"));
    }

    // === Containment Violation Tests ===

    #[test]
    fn containment_violation_detected() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-04")),       // parent: April
            make_tension("2", Some("1"), Some(1), Some("2026-06")), // child: June — violates!
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let violations = detect_containment_violations(&forest, "1");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].tension_id, "2");
        assert!(violations[0].excess_seconds > 0);
    }

    #[test]
    fn no_containment_violation_when_contained() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-06")),
            make_tension("2", Some("1"), Some(1), Some("2026-04")),
        ];
        let forest = Forest::from_tensions(tensions).unwrap();
        let violations = detect_containment_violations(&forest, "1");
        assert!(violations.is_empty());
    }

    // === TemporalSignals Integration Test ===

    #[test]
    fn temporal_signals_comprehensive() {
        let tensions = vec![
            make_tension("1", None, None, Some("2026-04")),
            make_tension("2", Some("1"), Some(1), Some("2026-05")), // May — sequencing ok but containment violation
            make_tension("3", Some("1"), Some(2), Some("2026-04")), // April — critical path, sequencing pressure vs #2
        ];
        let forest = Forest::from_tensions(tensions).unwrap();

        let signals = compute_temporal_signals(&forest, "1", now());
        // Parent sees containment violation on child #2
        assert_eq!(signals.containment_violations.len(), 1);
        assert_eq!(signals.containment_violations[0].tension_id, "2");

        // Parent sees critical path on child #3 (same deadline)
        assert!(!signals.critical_path.is_empty());

        // Child #3 has sequencing pressure (ordered after #2, but deadline before)
        let signals_3 = compute_temporal_signals(&forest, "3", now());
        assert!(!signals_3.sequencing_pressures.is_empty());
        assert!(signals_3.on_critical_path);

        // Child #2 has containment violation
        let signals_2 = compute_temporal_signals(&forest, "2", now());
        assert!(signals_2.has_containment_violation);
    }

    // === Urgency Tests ===

    #[test]
    fn test_compute_urgency_none_without_horizon() {
        let t = Tension::new("goal", "reality").unwrap();
        let now = Utc::now();
        let result = compute_urgency(&t, now);
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_urgency_at_zero_percent() {
        use chrono::Datelike;

        let now = Utc::now();
        let h = Horizon::new_month(now.year() + 1, 1).unwrap();
        let t = Tension {
            id: "test-0".to_string(),
            desired: "goal".to_string(),
            actual: "reality".to_string(),
            parent_id: None,
            created_at: now,
            status: TensionStatus::Active,
            horizon: Some(h),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        };

        let result = compute_urgency(&t, now).unwrap();
        assert!(
            (result.value - 0.0).abs() < 0.01,
            "urgency should be ~0.0, got {}",
            result.value
        );
        assert!(result.time_remaining > 0);
        assert!(result.total_window > 0);
    }

    #[test]
    fn test_compute_urgency_at_25_percent() {
        use chrono::Duration;

        let start = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let end = start + Duration::hours(4);
        let h = Horizon::new_datetime(end);

        let t = Tension {
            id: "test-25".to_string(),
            desired: "goal".to_string(),
            actual: "reality".to_string(),
            parent_id: None,
            created_at: start,
            status: TensionStatus::Active,
            horizon: Some(h),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        };

        let now = start + Duration::hours(1);
        let result = compute_urgency(&t, now).unwrap();
        assert!(
            (result.value - 0.25).abs() < 0.02,
            "urgency should be ~0.25, got {}",
            result.value
        );
    }

    #[test]
    fn test_compute_urgency_at_50_percent() {
        use chrono::Duration;

        let start = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        let end = start + Duration::hours(48);
        let h = Horizon::new_datetime(end);

        let t = Tension {
            id: "test-50".to_string(),
            desired: "goal".to_string(),
            actual: "reality".to_string(),
            parent_id: None,
            created_at: start,
            status: TensionStatus::Active,
            horizon: Some(h),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        };

        let now = start + Duration::hours(24);
        let result = compute_urgency(&t, now).unwrap();
        assert!(
            (result.value - 0.5).abs() < 0.02,
            "urgency should be ~0.5, got {}",
            result.value
        );
    }

    #[test]
    fn test_compute_urgency_at_75_percent() {
        use chrono::Duration;

        let start = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let end = start + Duration::hours(4);
        let h = Horizon::new_datetime(end);

        let t = Tension {
            id: "test-75".to_string(),
            desired: "goal".to_string(),
            actual: "reality".to_string(),
            parent_id: None,
            created_at: start,
            status: TensionStatus::Active,
            horizon: Some(h),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        };

        let now = start + Duration::hours(3);
        let result = compute_urgency(&t, now).unwrap();
        assert!(
            (result.value - 0.75).abs() < 0.02,
            "urgency should be ~0.75, got {}",
            result.value
        );
    }

    #[test]
    fn test_compute_urgency_at_100_percent() {
        use chrono::Duration;

        let start = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let end = start + Duration::hours(4);
        let h = Horizon::new_datetime(end);

        let t = Tension {
            id: "test-100".to_string(),
            desired: "goal".to_string(),
            actual: "reality".to_string(),
            parent_id: None,
            created_at: start,
            status: TensionStatus::Active,
            horizon: Some(h),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        };

        let now = end;
        let result = compute_urgency(&t, now).unwrap();
        assert!(
            (result.value - 1.0).abs() < 0.02,
            "urgency should be ~1.0, got {}",
            result.value
        );
    }

    #[test]
    fn test_compute_urgency_past_horizon() {
        use chrono::Duration;

        let start = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let end = start + Duration::hours(4);
        let h = Horizon::new_datetime(end);

        let t = Tension {
            id: "test-150".to_string(),
            desired: "goal".to_string(),
            actual: "reality".to_string(),
            parent_id: None,
            created_at: start,
            status: TensionStatus::Active,
            horizon: Some(h),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        };

        let now = end + Duration::hours(2);
        let result = compute_urgency(&t, now).unwrap();
        assert!(result.value > 1.0, "urgency should be > 1.0, got {}", result.value);
        assert!(
            (result.value - 1.5).abs() < 0.05,
            "urgency should be ~1.5, got {}",
            result.value
        );
    }

    #[test]
    fn test_compute_urgency_struct_fields() {
        use chrono::Duration;

        let start = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let end = start + Duration::hours(4);
        let h = Horizon::new_datetime(end);

        let t = Tension {
            id: "test-fields".to_string(),
            desired: "goal".to_string(),
            actual: "reality".to_string(),
            parent_id: None,
            created_at: start,
            status: TensionStatus::Active,
            horizon: Some(h),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        };

        let now = start + Duration::hours(1);
        let result = compute_urgency(&t, now).unwrap();

        assert_eq!(result.tension_id, "test-fields");
        assert!(result.value >= 0.0);
        assert!(result.time_remaining >= 0);
        assert!(result.total_window > 0);
    }

    // === Horizon Drift Tests ===

    #[test]
    fn test_detect_horizon_drift_stable() {
        let result = detect_horizon_drift("test-stable", &[]);
        assert_eq!(result.drift_type, HorizonDriftType::Stable);
        assert_eq!(result.change_count, 0);
        assert_eq!(result.net_shift_seconds, 0);
    }

    #[test]
    fn test_detect_horizon_drift_stable_with_non_horizon_mutations() {
        let m1 = Mutation::new(
            "test-stable".to_string(),
            Utc::now(),
            "actual".to_string(),
            Some("old".to_string()),
            "new".to_string(),
        );

        let result = detect_horizon_drift("test-stable", &[m1]);
        assert_eq!(result.drift_type, HorizonDriftType::Stable);
        assert_eq!(result.change_count, 0);
    }

    #[test]
    fn test_detect_horizon_drift_postponement() {
        let m1 = Mutation::new(
            "test-postpone".to_string(),
            Utc::now(),
            "horizon".to_string(),
            Some("2026-05".to_string()),
            "2026-06".to_string(),
        );

        let result = detect_horizon_drift("test-postpone", &[m1]);
        assert_eq!(result.drift_type, HorizonDriftType::Postponement);
        assert_eq!(result.change_count, 1);
        assert!(result.net_shift_seconds > 0);
    }

    #[test]
    fn test_detect_horizon_drift_repeated_postponement() {
        let m1 = Mutation::new(
            "test-rep".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-05".to_string()), "2026-06".to_string(),
        );
        let m2 = Mutation::new(
            "test-rep".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-06".to_string()), "2026-08".to_string(),
        );
        let m3 = Mutation::new(
            "test-rep".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-08".to_string()), "2026-12".to_string(),
        );

        let result = detect_horizon_drift("test-rep", &[m1, m2, m3]);
        assert_eq!(result.drift_type, HorizonDriftType::RepeatedPostponement);
        assert_eq!(result.change_count, 3);
        assert!(result.net_shift_seconds > 0);
    }

    #[test]
    fn test_detect_horizon_drift_tightening() {
        let m1 = Mutation::new(
            "test-tighten".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-06".to_string()), "2026-05".to_string(),
        );

        let result = detect_horizon_drift("test-tighten", &[m1]);
        assert_eq!(result.drift_type, HorizonDriftType::Tightening);
        assert!(result.net_shift_seconds < 0);
    }

    #[test]
    fn test_detect_horizon_drift_tightening_to_higher_precision() {
        let m1 = Mutation::new(
            "test-precision".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026".to_string()), "2026-05".to_string(),
        );

        let result = detect_horizon_drift("test-precision", &[m1]);
        assert_eq!(result.drift_type, HorizonDriftType::Tightening);
    }

    #[test]
    fn test_detect_horizon_drift_loosening() {
        let m1 = Mutation::new(
            "test-loosen".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-05-15".to_string()), "2026-06".to_string(),
        );

        let result = detect_horizon_drift("test-loosen", &[m1]);
        assert_eq!(result.drift_type, HorizonDriftType::Loosening);
        assert!(result.net_shift_seconds > 0);
    }

    #[test]
    fn test_detect_horizon_drift_oscillating() {
        let m1 = Mutation::new(
            "test-osc".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-05".to_string()), "2026-06".to_string(),
        );
        let m2 = Mutation::new(
            "test-osc".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-06".to_string()), "2026-04".to_string(),
        );
        let m3 = Mutation::new(
            "test-osc".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-04".to_string()), "2026-07".to_string(),
        );

        let result = detect_horizon_drift("test-osc", &[m1, m2, m3]);
        assert_eq!(result.drift_type, HorizonDriftType::Oscillating);
        assert_eq!(result.change_count, 3);
    }

    #[test]
    fn test_detect_horizon_drift_two_shifts_is_postponement() {
        let m1 = Mutation::new(
            "test-two".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-05".to_string()), "2026-06".to_string(),
        );
        let m2 = Mutation::new(
            "test-two".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-06".to_string()), "2026-08".to_string(),
        );

        let result = detect_horizon_drift("test-two", &[m1, m2]);
        assert_eq!(result.drift_type, HorizonDriftType::Postponement);
        assert_eq!(result.change_count, 2);
    }

    #[test]
    fn test_horizon_drift_struct_fields() {
        let m1 = Mutation::new(
            "test-fields".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-05".to_string()), "2026-06".to_string(),
        );

        let result = detect_horizon_drift("test-fields", &[m1]);
        assert_eq!(result.tension_id, "test-fields");
        assert!(result.change_count > 0);
    }

    // === Trait Tests ===

    #[test]
    fn test_types_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Urgency>();
        assert_send_sync::<HorizonDrift>();
        assert_send_sync::<HorizonDriftType>();
    }

    #[test]
    fn test_urgency_serializes_deserializes() {
        let u = Urgency {
            tension_id: "test-123".to_string(),
            value: 0.75,
            time_remaining: 1800,
            total_window: 7200,
        };
        let json = serde_json::to_string(&u).unwrap();
        let u2: Urgency = serde_json::from_str(&json).unwrap();
        assert_eq!(u, u2);

        let hd = HorizonDrift {
            tension_id: "test-456".to_string(),
            drift_type: HorizonDriftType::RepeatedPostponement,
            change_count: 5,
            net_shift_seconds: 12345,
            onset: None,
        };
        let json = serde_json::to_string(&hd).unwrap();
        let hd2: HorizonDrift = serde_json::from_str(&json).unwrap();
        assert_eq!(hd, hd2);
    }
}
