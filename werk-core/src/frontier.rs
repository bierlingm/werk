//! Frontier of action — the structural projection of where NOW falls on the order stream.
//!
//! The frontier exists whether or not anyone is looking at it. It is the fact of where
//! accomplished meets remaining within a tension's theory of closure.
//!
//! The operating envelope is the frontier realized as an interaction surface — the set of
//! structurally relevant categories around the frontier: next step, overdue steps, held
//! steps, recently resolved steps, and the remaining theory.
//!
//! This module computes these categories from tree structure, temporal data, and mutation
//! history. Both CLI and TUI consume these projections.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mutation::Mutation;
use crate::store::EpochRecord;
use crate::tension::TensionStatus;
use crate::tree::Forest;

/// A step at the frontier, carrying its structural identity and overdue status.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrontierStep {
    /// Tension ID of this step.
    pub tension_id: String,
    /// Short code for display.
    pub short_code: Option<i32>,
    /// The step's desired outcome (truncated for display by consumers).
    pub desired: String,
    /// Whether this step is past its horizon.
    pub is_overdue: bool,
}

/// Closure progress — how much of the active theory has been executed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClosureProgress {
    /// Number of resolved children.
    pub resolved: usize,
    /// Number of children in the active theory (total - released).
    pub active: usize,
    /// Number of released (abandoned) children.
    pub released: usize,
    /// Total number of children.
    pub total: usize,
}

/// The frontier projection for a tension's theory of closure.
///
/// Partitions children into the structural categories that the operating envelope displays.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Frontier {
    /// The tension this frontier belongs to.
    pub tension_id: String,
    /// The next committed step — first unresolved positioned child by order of operations.
    /// Carries an `is_overdue` flag rather than appearing in the overdue list.
    pub next_step: Option<FrontierStep>,
    /// Positioned, unresolved steps past their horizon (excluding next_step).
    pub overdue: Vec<FrontierStep>,
    /// Children resolved since the last epoch (or all resolved if no epoch exists).
    pub recently_resolved: Vec<FrontierStep>,
    /// Unpositioned children — acknowledged but not committed to the sequence.
    pub held: Vec<FrontierStep>,
    /// Positioned, unresolved steps beyond next_step that are not overdue.
    pub remaining: Vec<FrontierStep>,
    /// All resolved children (the accomplished trace).
    pub resolved: Vec<FrontierStep>,
    /// Closure progress against the active theory.
    pub closure_progress: ClosureProgress,
    /// The position value where the frontier falls (position of next_step).
    pub frontier_position: Option<i32>,
}

/// Compute the frontier projection for a tension's children.
///
/// # Arguments
///
/// * `forest` — the full tension forest
/// * `tension_id` — the parent tension whose frontier to compute
/// * `now` — current time (for overdue detection)
/// * `epochs` — epochs for this tension (chronological), used to determine recency boundary
/// * `child_mutations` — mutations for all children, keyed by child tension ID;
///   used to find resolution timestamps for recently_resolved classification
///
/// If the tension has no children, returns a frontier with empty categories and zero progress.
pub fn compute_frontier(
    forest: &Forest,
    tension_id: &str,
    now: DateTime<Utc>,
    epochs: &[EpochRecord],
    child_mutations: &[(String, Vec<Mutation>)],
) -> Frontier {
    let children = forest.children(tension_id).unwrap_or_default();

    if children.is_empty() {
        return Frontier {
            tension_id: tension_id.to_string(),
            next_step: None,
            overdue: Vec::new(),
            recently_resolved: Vec::new(),
            held: Vec::new(),
            remaining: Vec::new(),
            resolved: Vec::new(),
            closure_progress: ClosureProgress {
                resolved: 0,
                active: 0,
                released: 0,
                total: 0,
            },
            frontier_position: None,
        };
    }

    // Recency boundary: timestamp of last epoch, or None (meaning all resolved are "recent")
    let recency_boundary: Option<DateTime<Utc>> = epochs.last().map(|e| e.timestamp);

    // Partition children
    let mut positioned: Vec<_> = Vec::new();
    let mut held = Vec::new();
    let mut all_resolved = Vec::new();
    let mut released_count: usize = 0;
    let mut resolved_count: usize = 0;

    for child in &children {
        let t = &child.tension;
        match t.status {
            TensionStatus::Released => {
                released_count += 1;
            }
            TensionStatus::Resolved => {
                resolved_count += 1;
                all_resolved.push(make_step(t));
            }
            TensionStatus::Active => {
                if t.position.is_some() {
                    positioned.push(t);
                } else {
                    held.push(make_step(t));
                }
            }
        }
    }

    // Sort positioned by position ascending
    positioned.sort_by_key(|t| t.position.unwrap_or(i32::MAX));

    // Walk positioned: first unresolved becomes next_step, rest partition into overdue/remaining
    let mut next_step: Option<FrontierStep> = None;
    let mut overdue = Vec::new();
    let mut remaining = Vec::new();

    for t in &positioned {
        let is_overdue = t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false);

        if next_step.is_none() {
            // First unresolved positioned child is the next step
            next_step = Some(FrontierStep {
                tension_id: t.id.clone(),
                short_code: t.short_code,
                desired: t.desired.clone(),
                is_overdue,
            });
        } else if is_overdue {
            overdue.push(make_step_with_overdue(t, true));
        } else {
            remaining.push(make_step(t));
        }
    }

    let frontier_position = next_step.as_ref().and_then(|ns| {
        positioned
            .iter()
            .find(|t| t.id == ns.tension_id)
            .and_then(|t| t.position)
    });

    // Recently resolved: resolved children whose resolution happened after the recency boundary
    let recently_resolved =
        classify_recently_resolved(&all_resolved, child_mutations, recency_boundary);

    let total = children.len();
    let active = total - released_count;

    Frontier {
        tension_id: tension_id.to_string(),
        next_step,
        overdue,
        recently_resolved,
        held,
        remaining,
        resolved: all_resolved,
        closure_progress: ClosureProgress {
            resolved: resolved_count,
            active,
            released: released_count,
            total,
        },
        frontier_position,
    }
}

/// Find resolved children whose resolution timestamp is after the recency boundary.
///
/// If no boundary exists (no epochs), all resolved children are considered recent.
fn classify_recently_resolved(
    resolved: &[FrontierStep],
    child_mutations: &[(String, Vec<Mutation>)],
    recency_boundary: Option<DateTime<Utc>>,
) -> Vec<FrontierStep> {
    resolved
        .iter()
        .filter(|step| {
            let resolution_time = find_resolution_time(&step.tension_id, child_mutations);
            match (recency_boundary, resolution_time) {
                // No epoch exists — all resolved are recent
                (None, _) => true,
                // Has epoch boundary, has resolution time — compare
                (Some(boundary), Some(resolved_at)) => resolved_at > boundary,
                // Has epoch boundary but no resolution mutation found — exclude
                (Some(_), None) => false,
            }
        })
        .cloned()
        .collect()
}

/// Find the timestamp when a tension was resolved by scanning its mutations.
fn find_resolution_time(
    tension_id: &str,
    child_mutations: &[(String, Vec<Mutation>)],
) -> Option<DateTime<Utc>> {
    child_mutations
        .iter()
        .find(|(id, _)| id == tension_id)
        .and_then(|(_, mutations)| {
            // Find the last status mutation that set to Resolved
            mutations
                .iter()
                .rev()
                .find(|m| m.field() == "status" && m.new_value() == "Resolved")
                .map(|m| m.timestamp())
        })
}

fn make_step(t: &crate::tension::Tension) -> FrontierStep {
    FrontierStep {
        tension_id: t.id.clone(),
        short_code: t.short_code,
        desired: t.desired.clone(),
        is_overdue: false,
    }
}

fn make_step_with_overdue(t: &crate::tension::Tension, is_overdue: bool) -> FrontierStep {
    FrontierStep {
        tension_id: t.id.clone(),
        short_code: t.short_code,
        desired: t.desired.clone(),
        is_overdue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tension::Tension;
    use chrono::{Datelike, Duration};

    fn make_tension(id: &str, status: TensionStatus, position: Option<i32>) -> Tension {
        Tension {
            id: id.to_string(),
            desired: format!("desired for {}", id),
            actual: format!("actual for {}", id),
            parent_id: Some("parent".to_string()),
            created_at: Utc::now() - Duration::days(10),
            status,
            horizon: None,
            position,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        }
    }

    fn make_tension_with_horizon(
        id: &str,
        status: TensionStatus,
        position: Option<i32>,
        horizon: crate::Horizon,
    ) -> Tension {
        let mut t = make_tension(id, status, position);
        t.horizon = Some(horizon);
        t
    }

    #[test]
    fn empty_children_returns_empty_frontier() {
        let forest = Forest::from_tensions(vec![]).unwrap();
        let f = compute_frontier(&forest, "nonexistent", Utc::now(), &[], &[]);
        assert!(f.next_step.is_none());
        assert_eq!(f.closure_progress.total, 0);
    }

    #[test]
    fn positioned_children_ordered_correctly() {
        let parent = make_tension("parent", TensionStatus::Active, None);
        let mut parent = parent;
        parent.parent_id = None;

        let c1 = make_tension("c1", TensionStatus::Active, Some(1));
        let c2 = make_tension("c2", TensionStatus::Active, Some(2));
        let c3 = make_tension("c3", TensionStatus::Active, Some(3));

        let forest = Forest::from_tensions(vec![parent, c1, c2, c3]).unwrap();
        let f = compute_frontier(&forest, "parent", Utc::now(), &[], &[]);

        assert_eq!(f.next_step.as_ref().unwrap().tension_id, "c1");
        assert_eq!(f.remaining.len(), 2);
        assert_eq!(f.remaining[0].tension_id, "c2");
        assert_eq!(f.remaining[1].tension_id, "c3");
        assert_eq!(f.closure_progress.resolved, 0);
        assert_eq!(f.closure_progress.active, 3);
    }

    #[test]
    fn held_children_partitioned() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let c1 = make_tension("c1", TensionStatus::Active, Some(1));
        let c2 = make_tension("c2", TensionStatus::Active, None); // held

        let forest = Forest::from_tensions(vec![parent, c1, c2]).unwrap();
        let f = compute_frontier(&forest, "parent", Utc::now(), &[], &[]);

        assert_eq!(f.next_step.as_ref().unwrap().tension_id, "c1");
        assert_eq!(f.held.len(), 1);
        assert_eq!(f.held[0].tension_id, "c2");
    }

    #[test]
    fn resolved_children_counted_and_listed() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let c1 = make_tension("c1", TensionStatus::Resolved, Some(1));
        let c2 = make_tension("c2", TensionStatus::Active, Some(2));

        let forest = Forest::from_tensions(vec![parent, c1, c2]).unwrap();
        let f = compute_frontier(&forest, "parent", Utc::now(), &[], &[]);

        assert_eq!(f.next_step.as_ref().unwrap().tension_id, "c2");
        assert_eq!(f.resolved.len(), 1);
        assert_eq!(f.closure_progress.resolved, 1);
        assert_eq!(f.closure_progress.active, 2);
    }

    #[test]
    fn released_children_excluded_from_active_count() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let c1 = make_tension("c1", TensionStatus::Active, Some(1));
        let c2 = make_tension("c2", TensionStatus::Released, Some(2));
        let c3 = make_tension("c3", TensionStatus::Resolved, Some(3));

        let forest = Forest::from_tensions(vec![parent, c1, c2, c3]).unwrap();
        let f = compute_frontier(&forest, "parent", Utc::now(), &[], &[]);

        assert_eq!(f.closure_progress.resolved, 1);
        assert_eq!(f.closure_progress.active, 2); // total(3) - released(1)
        assert_eq!(f.closure_progress.released, 1);
        assert_eq!(f.closure_progress.total, 3);
    }

    #[test]
    fn overdue_next_step_carries_flag() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let now = Utc::now();
        let past_horizon = {
            let d = (now - Duration::days(5)).date_naive();
            crate::Horizon::new_day(d.year(), d.month(), d.day()).unwrap()
        };
        let c1 = make_tension_with_horizon("c1", TensionStatus::Active, Some(1), past_horizon);

        let forest = Forest::from_tensions(vec![parent, c1]).unwrap();
        let f = compute_frontier(&forest, "parent", now, &[], &[]);

        assert!(f.next_step.as_ref().unwrap().is_overdue);
        assert!(f.overdue.is_empty()); // next_step is NOT in overdue list
    }

    #[test]
    fn overdue_non_next_steps_in_overdue_list() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let now = Utc::now();
        let past_horizon = {
            let d = (now - Duration::days(5)).date_naive();
            crate::Horizon::new_day(d.year(), d.month(), d.day()).unwrap()
        };

        let c1 = make_tension("c1", TensionStatus::Active, Some(1)); // next, not overdue
        let c2 = make_tension_with_horizon("c2", TensionStatus::Active, Some(2), past_horizon);

        let forest = Forest::from_tensions(vec![parent, c1, c2]).unwrap();
        let f = compute_frontier(&forest, "parent", now, &[], &[]);

        assert!(!f.next_step.as_ref().unwrap().is_overdue);
        assert_eq!(f.overdue.len(), 1);
        assert_eq!(f.overdue[0].tension_id, "c2");
    }

    #[test]
    fn recently_resolved_without_epochs_includes_all() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let c1 = make_tension("c1", TensionStatus::Resolved, Some(1));
        let c2 = make_tension("c2", TensionStatus::Active, Some(2));

        let forest = Forest::from_tensions(vec![parent, c1, c2]).unwrap();
        // No epochs, no mutations needed — all resolved are recent
        let f = compute_frontier(&forest, "parent", Utc::now(), &[], &[]);

        assert_eq!(f.recently_resolved.len(), 1);
        assert_eq!(f.recently_resolved[0].tension_id, "c1");
    }

    #[test]
    fn recently_resolved_with_epoch_filters_by_boundary() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let now = Utc::now();
        let epoch_time = now - Duration::days(3);

        let c1 = make_tension("c1", TensionStatus::Resolved, Some(1));
        let c2 = make_tension("c2", TensionStatus::Resolved, Some(2));
        let c3 = make_tension("c3", TensionStatus::Active, Some(3));

        let forest = Forest::from_tensions(vec![parent, c1, c2, c3]).unwrap();

        let epoch = EpochRecord {
            id: "epoch1".to_string(),
            tension_id: "parent".to_string(),
            timestamp: epoch_time,
            desire_snapshot: String::new(),
            reality_snapshot: String::new(),
            children_snapshot_json: None,
            trigger_gesture_id: None,
            epoch_type: None,
        };

        // c1 resolved before epoch, c2 resolved after
        let c1_mutations = vec![Mutation::new(
            "c1".to_string(),
            epoch_time - Duration::days(1), // before epoch
            "status".to_string(),
            Some("Active".to_string()),
            "Resolved".to_string(),
        )];
        let c2_mutations = vec![Mutation::new(
            "c2".to_string(),
            epoch_time + Duration::days(1), // after epoch
            "status".to_string(),
            Some("Active".to_string()),
            "Resolved".to_string(),
        )];

        let child_muts = vec![
            ("c1".to_string(), c1_mutations),
            ("c2".to_string(), c2_mutations),
        ];

        let f = compute_frontier(&forest, "parent", now, &[epoch], &child_muts);

        assert_eq!(f.recently_resolved.len(), 1);
        assert_eq!(f.recently_resolved[0].tension_id, "c2");
    }

    #[test]
    fn frontier_position_matches_next_step() {
        let mut parent = make_tension("parent", TensionStatus::Active, None);
        parent.parent_id = None;

        let c1 = make_tension("c1", TensionStatus::Resolved, Some(1));
        let c2 = make_tension("c2", TensionStatus::Active, Some(2));
        let c3 = make_tension("c3", TensionStatus::Active, Some(3));

        let forest = Forest::from_tensions(vec![parent, c1, c2, c3]).unwrap();
        let f = compute_frontier(&forest, "parent", Utc::now(), &[], &[]);

        assert_eq!(f.frontier_position, Some(2));
    }
}
