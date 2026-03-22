//! Honest temporal computations.
//!
//! This module contains only computations that are directly derivable from
//! structure and time — no inference, no speculation.
//!
//! - **Urgency**: elapsed fraction of a horizon window (a fact)
//! - **HorizonDrift**: pattern of horizon changes over time (a fact)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::mutation::Mutation;
use crate::tension::Tension;

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
        };
    }

    // Parse horizon mutations and compute shifts
    let mut shifts: Vec<i64> = Vec::new(); // positive = later, negative = earlier
    let mut precision_tightenings = 0i32; // higher precision (DateTime < Day < Month < Year)
    let mut precision_loosenings = 0i32; // lower precision

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
                // Compute shift: new.range_end - old.range_end
                let shift = (new.range_end() - old.range_end()).num_seconds();
                shifts.push(shift);

                // Track precision changes (lower precision_level = higher precision)
                let old_precision = old.precision_level();
                let new_precision = new.precision_level();
                if new_precision < old_precision {
                    // Higher precision (e.g., Year → Month)
                    precision_tightenings += 1;
                } else if new_precision > old_precision {
                    // Lower precision (e.g., Day → Month)
                    precision_loosenings += 1;
                }
            }
            (Some(_old), None) => {
                // Clearing horizon - conceptual "infinity" shift, skip for computation
            }
            (None, None) => {
                // Both empty - shouldn't happen but skip
            }
        }
    }

    // Compute net shift
    let net_shift_seconds: i64 = shifts.iter().sum();

    // Count direction changes for oscillation detection
    let mut direction_changes = 0;
    let mut last_positive: Option<bool> = None;
    for shift in &shifts {
        let is_positive = *shift >= 0;
        if let Some(was_positive) = last_positive
            && is_positive != was_positive
        {
            direction_changes += 1;
        }
        last_positive = Some(is_positive);
    }

    // Determine drift type
    // CRITICAL: Empty shifts (only None->Some assignments) = Stable baseline
    if shifts.is_empty() {
        return HorizonDrift {
            tension_id: tension_id.to_string(),
            drift_type: HorizonDriftType::Stable,
            change_count,
            net_shift_seconds: 0,
        };
    }

    // Priority: Oscillating > Precision-based > Time-based
    let drift_type = if direction_changes >= 2 {
        HorizonDriftType::Oscillating
    } else if precision_tightenings > precision_loosenings {
        HorizonDriftType::Tightening
    } else if precision_loosenings > precision_tightenings {
        HorizonDriftType::Loosening
    } else if shifts.iter().all(|s| *s > 0) {
        if shifts.len() >= 3 {
            HorizonDriftType::RepeatedPostponement
        } else {
            HorizonDriftType::Postponement
        }
    } else if shifts.iter().all(|s| *s < 0) {
        HorizonDriftType::Tightening
    } else if net_shift_seconds > 0 {
        HorizonDriftType::Loosening
    } else if net_shift_seconds < 0 {
        HorizonDriftType::Tightening
    } else {
        HorizonDriftType::Stable
    };

    HorizonDrift {
        tension_id: tension_id.to_string(),
        drift_type,
        change_count,
        net_shift_seconds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tension::{Tension, TensionStatus};

    // ── Urgency Tests ────────────────────────────────────────────────────

    #[test]
    fn test_compute_urgency_none_without_horizon() {
        let t = Tension::new("goal", "reality").unwrap();
        let now = Utc::now();
        let result = compute_urgency(&t, now);
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_urgency_at_zero_percent() {
        use crate::Horizon;
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
        use crate::Horizon;
        use chrono::{Duration, TimeZone};

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
        use crate::Horizon;
        use chrono::{Duration, TimeZone};

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
        use crate::Horizon;
        use chrono::{Duration, TimeZone};

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
        use crate::Horizon;
        use chrono::{Duration, TimeZone};

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
        use crate::Horizon;
        use chrono::{Duration, TimeZone};

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
        use crate::Horizon;
        use chrono::{Duration, TimeZone};

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

    // ── Horizon Drift Tests ──────────────────────────────────────────────

    #[test]
    fn test_detect_horizon_drift_stable() {
        let result = detect_horizon_drift("test-stable", &[]);
        assert_eq!(result.drift_type, HorizonDriftType::Stable);
        assert_eq!(result.change_count, 0);
        assert_eq!(result.net_shift_seconds, 0);
    }

    #[test]
    fn test_detect_horizon_drift_stable_with_non_horizon_mutations() {
        use crate::mutation::Mutation;

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
        use crate::mutation::Mutation;

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
        use crate::mutation::Mutation;

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
        use crate::mutation::Mutation;

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
        use crate::mutation::Mutation;

        let m1 = Mutation::new(
            "test-precision".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026".to_string()), "2026-05".to_string(),
        );

        let result = detect_horizon_drift("test-precision", &[m1]);
        assert_eq!(result.drift_type, HorizonDriftType::Tightening);
    }

    #[test]
    fn test_detect_horizon_drift_loosening() {
        use crate::mutation::Mutation;

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
        use crate::mutation::Mutation;

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
        use crate::mutation::Mutation;

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
        use crate::mutation::Mutation;

        let m1 = Mutation::new(
            "test-fields".to_string(), Utc::now(),
            "horizon".to_string(), Some("2026-05".to_string()), "2026-06".to_string(),
        );

        let result = detect_horizon_drift("test-fields", &[m1]);
        assert_eq!(result.tension_id, "test-fields");
        assert!(result.change_count > 0);
    }

    // ── Trait Tests ──────────────────────────────────────────────────────

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
        };
        let json = serde_json::to_string(&hd).unwrap();
        let hd2: HorizonDrift = serde_json::from_str(&json).unwrap();
        assert_eq!(hd, hd2);
    }
}
