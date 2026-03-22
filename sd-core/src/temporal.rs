//! Calculus of Time — computed temporal properties.
//!
//! The foundation defines two user-set primitives (deadline/horizon, order/position)
//! and six computed temporal properties. This module implements four of them:
//!
//! - **Implied execution window**: the temporal gap between predecessor and successor deadlines
//! - **Sequencing pressure**: order conflicts with deadline ordering
//! - **Critical path**: child deadline crowds parent deadline (recursive)
//! - **Containment violation**: child deadline exceeds parent deadline
//!
//! These are facts and signals, not speculative dynamics. They derive directly
//! from the data without inference.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tension::{Tension, TensionStatus};
use crate::tree::Forest;

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
    positioned.sort_by_key(|t| t.position.unwrap());

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
    positioned.sort_by_key(|t| t.position.unwrap());

    let mut results = Vec::new();

    for i in 1..positioned.len() {
        let current = positioned[i];
        let current_end = current.horizon.as_ref().unwrap().range_end();

        // Check against all predecessors (not just immediate — pressure can skip)
        for pred in &positioned[..i] {
            let pred_end = pred.horizon.as_ref().unwrap().range_end();

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
    use crate::tension::Tension;
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
}
