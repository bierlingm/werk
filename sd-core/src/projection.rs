//! Structural projection engine — mutation pattern extraction and trajectory classification.
//!
//! Extrapolates observed engagement patterns forward to classify
//! per-tension trajectories. Layer 1 extracts mutation patterns from
//! a tension's history; layer 2 combines them into structural trajectory
//! projections per tension.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::dynamics::{compute_urgency, gap_magnitude};
use crate::mutation::Mutation;
use crate::tension::{Tension, TensionStatus};

/// Engagement pattern extracted from a tension's mutation history.
#[derive(Debug, Clone)]
pub struct MutationPattern {
    pub tension_id: String,
    /// Average time between mutations in seconds. None if < 2 mutations.
    pub mean_interval_seconds: Option<f64>,
    /// Total mutation count in the analysis window.
    pub mutation_count: usize,
    /// Mutations per day in the analysis window.
    pub frequency_per_day: f64,
    /// Frequency trend: positive = accelerating, negative = declining.
    /// Computed by comparing first-half vs second-half mutation counts.
    pub frequency_trend: f64,
    /// Direction of gap change: negative = closing, positive = opening.
    /// Linear slope of gap_samples.
    pub gap_trend: f64,
    /// Recent gap magnitude samples (oldest to newest, max ~10).
    /// Sampled at each "actual"-field mutation.
    pub gap_samples: Vec<f64>,
    /// Whether there's enough data to project (>= 2 gap samples).
    pub is_projectable: bool,
}

/// Extract a mutation pattern from a tension's mutation history.
///
/// Filters mutations to those within `[now - window_seconds, now]`, then
/// computes engagement metrics: frequency, trend, gap trajectory, and
/// projectability.
pub fn extract_mutation_pattern(
    tension: &Tension,
    mutations: &[Mutation],
    window_seconds: i64,
    now: DateTime<Utc>,
) -> MutationPattern {
    let window_start = now - chrono::Duration::seconds(window_seconds);

    // Filter and sort mutations within the window.
    let mut filtered: Vec<&Mutation> = mutations
        .iter()
        .filter(|m| m.timestamp() >= window_start && m.timestamp() <= now)
        .collect();
    filtered.sort_by_key(|m| m.timestamp());

    let mutation_count = filtered.len();

    // Mean interval between consecutive mutations.
    let mean_interval_seconds = if mutation_count >= 2 {
        let total: f64 = filtered
            .windows(2)
            .map(|w| {
                (w[1].timestamp() - w[0].timestamp())
                    .num_seconds()
                    .unsigned_abs() as f64
            })
            .sum();
        Some(total / (mutation_count - 1) as f64)
    } else {
        None
    };

    // Frequency: mutations per day.
    let window_days = window_seconds as f64 / 86400.0;
    let frequency_per_day = if window_days > 0.0 {
        mutation_count as f64 / window_days
    } else {
        0.0
    };

    // Frequency trend: first half vs second half of the window.
    let window_midpoint = window_start + chrono::Duration::seconds(window_seconds / 2);
    let first_half_count = filtered
        .iter()
        .filter(|m| m.timestamp() < window_midpoint)
        .count() as f64;
    let second_half_count = filtered
        .iter()
        .filter(|m| m.timestamp() >= window_midpoint)
        .count() as f64;
    let frequency_trend = (second_half_count - first_half_count) / first_half_count.max(1.0);

    // Gap samples: for each "actual" mutation, compute gap magnitude.
    let actual_mutations: Vec<&&Mutation> = filtered
        .iter()
        .filter(|m| m.field() == "actual")
        .collect();
    // Take the last 10.
    let recent_actual: Vec<&&Mutation> = if actual_mutations.len() > 10 {
        actual_mutations[actual_mutations.len() - 10..].to_vec()
    } else {
        actual_mutations
    };
    let gap_samples: Vec<f64> = recent_actual
        .iter()
        .map(|m| gap_magnitude(&tension.desired, m.new_value()))
        .collect();

    // Gap trend: linear regression slope over gap_samples by index.
    let gap_trend = if gap_samples.len() >= 2 {
        linear_slope(&gap_samples)
    } else {
        0.0
    };

    let is_projectable = gap_samples.len() >= 2;

    MutationPattern {
        tension_id: tension.id.clone(),
        mean_interval_seconds,
        mutation_count,
        frequency_per_day,
        frequency_trend,
        gap_trend,
        gap_samples,
        is_projectable,
    }
}

/// Simple linear regression slope: y = gap_samples, x = 0..n-1.
///
/// slope = (n * sum(x*y) - sum(x) * sum(y)) / (n * sum(x^2) - sum(x)^2)
fn linear_slope(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;

    for (i, &y) in values.iter().enumerate() {
        let x = i as f64;
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
    }

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return 0.0;
    }

    (n * sum_xy - sum_x * sum_y) / denom
}

/// Time horizon for projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectionHorizon {
    OneWeek,
    OneMonth,
    ThreeMonths,
    Custom(i64), // arbitrary seconds
}

impl ProjectionHorizon {
    pub fn seconds(&self) -> i64 {
        match self {
            Self::OneWeek => 7 * 86400,
            Self::OneMonth => 30 * 86400,
            Self::ThreeMonths => 90 * 86400,
            Self::Custom(s) => *s,
        }
    }
}

/// Project gap magnitude at a future time point.
/// Linear extrapolation from gap_trend, clamped to [0.0, 1.0].
pub fn project_gap_at(
    pattern: &MutationPattern,
    current_gap: f64,
    seconds_forward: i64,
) -> f64 {
    if !pattern.is_projectable || pattern.gap_trend == 0.0 {
        return current_gap.clamp(0.0, 1.0);
    }

    // gap_trend is slope per sample index. Convert to per-second rate
    // using mean_interval_seconds (fall back to 1.0 to avoid division by zero).
    let interval = pattern.mean_interval_seconds.unwrap_or(1.0);
    let rate_per_second = pattern.gap_trend / interval;
    let projected = current_gap + rate_per_second * seconds_forward as f64;
    projected.clamp(0.0, 1.0)
}

/// Project engagement frequency at a future time point.
/// Extrapolates frequency_trend, clamped to >= 0.
pub fn project_frequency_at(
    pattern: &MutationPattern,
    seconds_forward: i64,
) -> f64 {
    if !pattern.is_projectable {
        return pattern.frequency_per_day.max(0.0);
    }

    let days_forward = seconds_forward as f64 / 86400.0;
    // frequency_trend is a ratio (e.g. 0.5 = 50% increase over the analysis window).
    // We don't know the exact analysis window length, but we can use the total
    // observation span from mean_interval * mutation_count as an approximation.
    let analysis_window_days = pattern
        .mean_interval_seconds
        .map(|i| i * pattern.mutation_count.max(1) as f64 / 86400.0)
        .unwrap_or(1.0)
        .max(f64::EPSILON);

    let projected = pattern.frequency_per_day
        * (1.0 + pattern.frequency_trend * days_forward / analysis_window_days);
    projected.max(0.0)
}

/// Estimate time-to-resolution in seconds.
/// Returns None if gap is not closing (gap_trend >= 0) or not projectable.
pub fn estimate_time_to_resolution(
    pattern: &MutationPattern,
    current_gap: f64,
) -> Option<i64> {
    if !pattern.is_projectable || pattern.gap_trend >= 0.0 {
        return None;
    }

    let interval = pattern.mean_interval_seconds.unwrap_or(1.0);
    let rate_per_second = pattern.gap_trend.abs() / interval;
    if rate_per_second < f64::EPSILON {
        return None;
    }

    let seconds = current_gap / rate_per_second;
    Some(seconds as i64)
}

/// Structural trajectory classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trajectory {
    /// Gap closing, engagement present — on track to resolve.
    Resolving,
    /// Low or zero engagement, or engagement declining — stalling out.
    Stalling,
    /// Engagement present but gap not closing — effort without progress.
    Drifting,
    /// Gap samples show alternating up/down — advance-then-regress pattern.
    Oscillating,
}

/// Configurable thresholds for projection.
#[derive(Debug, Clone)]
pub struct ProjectionThresholds {
    /// Analysis window for mutation patterns (default: 30 days in seconds).
    pub pattern_window_seconds: i64,
    /// Frequency below this = neglect risk (default: 0.1 per day).
    pub neglect_frequency_threshold: f64,
    /// Gap sample variance above this = oscillation risk (default: 0.02).
    pub oscillation_gap_variance: f64,
    /// Gap below this considered "resolved" (default: 0.05).
    pub resolution_gap_threshold: f64,
}

impl Default for ProjectionThresholds {
    fn default() -> Self {
        Self {
            pattern_window_seconds: 30 * 86400,
            neglect_frequency_threshold: 0.1,
            oscillation_gap_variance: 0.02,
            resolution_gap_threshold: 0.05,
        }
    }
}

/// Full projection for a single tension at a specific horizon.
#[derive(Debug, Clone)]
pub struct TensionProjection {
    pub tension_id: String,
    pub horizon: ProjectionHorizon,
    pub projected_gap: f64,
    pub current_gap: f64,
    /// projected - current (negative = improving).
    pub gap_delta: f64,
    /// Whether the tension can resolve before its horizon deadline (if any).
    pub will_resolve: Option<bool>,
    /// Projected urgency at this horizon's time point.
    pub projected_urgency: Option<f64>,
    /// Whether oscillation pattern is detected/predicted.
    pub oscillation_risk: bool,
    /// Whether engagement is declining toward neglect.
    pub neglect_risk: bool,
    /// Structural trajectory classification.
    pub trajectory: Trajectory,
    /// Estimated seconds to resolution (None if not converging).
    pub time_to_resolution: Option<i64>,
}

/// Classify the structural trajectory of a tension from its mutation pattern.
fn classify_trajectory(pattern: &MutationPattern, thresholds: &ProjectionThresholds) -> Trajectory {
    // Check for oscillation: 3+ gap samples with alternating diffs.
    if pattern.gap_samples.len() >= 3 {
        let diffs: Vec<f64> = pattern
            .gap_samples
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();
        // Check if consecutive diffs alternate sign (ignore zero diffs).
        let non_zero_diffs: Vec<f64> = diffs.iter().copied().filter(|d| d.abs() > f64::EPSILON).collect();
        if non_zero_diffs.len() >= 2 {
            let all_alternate = non_zero_diffs
                .windows(2)
                .all(|w| w[0].signum() != w[1].signum());
            if all_alternate {
                return Trajectory::Oscillating;
            }
        }
    }

    // Check for stalling: low frequency or sharply declining engagement.
    if pattern.frequency_per_day < thresholds.neglect_frequency_threshold
        || pattern.frequency_trend < -0.5
    {
        return Trajectory::Stalling;
    }

    // Check for drifting: engaged but gap not closing.
    if pattern.frequency_per_day > 0.0 && pattern.gap_trend >= -0.001 {
        return Trajectory::Drifting;
    }

    // Default: engaged and gap closing.
    Trajectory::Resolving
}

/// Compute the variance of a slice of f64 values.
fn variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
}

/// Project a tension across all three standard horizons.
///
/// Returns a `Vec<TensionProjection>` with one entry per horizon
/// (OneWeek, OneMonth, ThreeMonths).
pub fn project_tension(
    tension: &Tension,
    mutations: &[Mutation],
    thresholds: &ProjectionThresholds,
    now: DateTime<Utc>,
) -> Vec<TensionProjection> {
    let pattern = extract_mutation_pattern(
        tension,
        mutations,
        thresholds.pattern_window_seconds,
        now,
    );
    let current_gap = gap_magnitude(&tension.desired, &tension.actual);
    let trajectory = classify_trajectory(&pattern, thresholds);
    let ttr = estimate_time_to_resolution(&pattern, current_gap);

    let horizons = [
        ProjectionHorizon::OneWeek,
        ProjectionHorizon::OneMonth,
        ProjectionHorizon::ThreeMonths,
    ];

    horizons
        .iter()
        .map(|&horizon| {
            let secs = horizon.seconds();
            let projected_gap = project_gap_at(&pattern, current_gap, secs);
            let gap_delta = projected_gap - current_gap;

            // Projected urgency at this horizon's future time point.
            let projected_urgency = compute_urgency(
                tension,
                now + chrono::Duration::seconds(secs),
            )
            .map(|u| u.value);

            // Will resolve: only meaningful if the tension has a horizon deadline.
            let will_resolve = tension.horizon.as_ref().map(|h| {
                let deadline_secs = (h.range_end() - now).num_seconds();
                match ttr {
                    Some(t) => t < deadline_secs,
                    None => false,
                }
            });

            // Oscillation risk: variance of gap samples above threshold AND engaged.
            let gap_var = variance(&pattern.gap_samples);
            let oscillation_risk = gap_var > thresholds.oscillation_gap_variance
                && pattern.frequency_per_day > thresholds.neglect_frequency_threshold;

            // Neglect risk: projected frequency falls below threshold.
            let neglect_risk =
                project_frequency_at(&pattern, secs) < thresholds.neglect_frequency_threshold;

            TensionProjection {
                tension_id: tension.id.clone(),
                horizon,
                projected_gap,
                current_gap,
                gap_delta,
                will_resolve,
                projected_urgency,
                oscillation_risk,
                neglect_risk,
                trajectory,
                time_to_resolution: ttr,
            }
        })
        .collect()
}

/// Trajectory distribution counts at a specific horizon.
#[derive(Debug, Clone, Default)]
pub struct TrajectoryBuckets {
    pub resolving: usize,
    pub stalling: usize,
    pub drifting: usize,
    pub oscillating: usize,
    pub total: usize,
}

/// Window where multiple tensions have high urgency simultaneously.
#[derive(Debug, Clone)]
pub struct UrgencyCollision {
    pub tension_ids: Vec<String>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub peak_combined_urgency: f64,
}

/// Full field-wide projection result.
#[derive(Debug, Clone)]
pub struct FieldProjection {
    pub computed_at: DateTime<Utc>,
    /// Per-tension projections: (tension_id, vec of projections at each horizon).
    pub tension_projections: Vec<(String, Vec<TensionProjection>)>,
    /// Trajectory distribution per horizon.
    pub funnel: HashMap<ProjectionHorizon, TrajectoryBuckets>,
    /// Upcoming urgency collision windows.
    pub urgency_collisions: Vec<UrgencyCollision>,
}

/// Project the entire field of tensions at once.
///
/// Filters to active tensions, computes per-tension projections,
/// aggregates trajectory distributions per horizon, and detects
/// urgency collision windows.
pub fn project_field(
    tensions: &[Tension],
    mutations: &[Mutation],
    thresholds: &ProjectionThresholds,
    now: DateTime<Utc>,
) -> FieldProjection {
    // 1. Filter to active tensions only.
    let active: Vec<&Tension> = tensions
        .iter()
        .filter(|t| t.status != TensionStatus::Resolved && t.status != TensionStatus::Released)
        .collect();

    // 2. Per-tension projections.
    let mut tension_projections: Vec<(String, Vec<TensionProjection>)> = Vec::new();
    for t in &active {
        let t_mutations: Vec<Mutation> = mutations
            .iter()
            .filter(|m| m.tension_id() == t.id)
            .cloned()
            .collect();
        let projs = project_tension(t, &t_mutations, thresholds, now);
        tension_projections.push((t.id.clone(), projs));
    }

    // 3. Aggregate trajectory buckets per horizon.
    let horizons = [
        ProjectionHorizon::OneWeek,
        ProjectionHorizon::OneMonth,
        ProjectionHorizon::ThreeMonths,
    ];
    let mut funnel: HashMap<ProjectionHorizon, TrajectoryBuckets> = HashMap::new();
    for &h in &horizons {
        let mut buckets = TrajectoryBuckets::default();
        for (_id, projs) in &tension_projections {
            if let Some(p) = projs.iter().find(|p| p.horizon == h) {
                match p.trajectory {
                    Trajectory::Resolving => buckets.resolving += 1,
                    Trajectory::Stalling => buckets.stalling += 1,
                    Trajectory::Drifting => buckets.drifting += 1,
                    Trajectory::Oscillating => buckets.oscillating += 1,
                }
                buckets.total += 1;
            }
        }
        funnel.insert(h, buckets);
    }

    // 4. Urgency collision detection.
    //    Sample at weekly intervals from now to now + 90 days.
    let weeks = 13; // ~90 days
    let week_secs = 7 * 86400_i64;
    let mut urgency_collisions: Vec<UrgencyCollision> = Vec::new();

    for week_idx in 0..weeks {
        let sample_time = now + chrono::Duration::seconds(week_secs * week_idx);
        let window_end_time = now + chrono::Duration::seconds(week_secs * (week_idx + 1));

        let mut high_urgency: Vec<(String, f64)> = Vec::new();
        for t in &active {
            if let Some(u) = compute_urgency(t, sample_time) {
                if u.value > 0.7 {
                    high_urgency.push((t.id.clone(), u.value));
                }
            }
        }

        if high_urgency.len() >= 2 {
            let peak: f64 = high_urgency.iter().map(|(_, v)| v).sum();
            let ids: Vec<String> = high_urgency.into_iter().map(|(id, _)| id).collect();
            urgency_collisions.push(UrgencyCollision {
                tension_ids: ids,
                window_start: sample_time,
                window_end: window_end_time,
                peak_combined_urgency: peak,
            });
        }
    }

    FieldProjection {
        computed_at: now,
        tension_projections,
        funnel,
        urgency_collisions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tension::Tension;

    /// Helper: build a Tension with controlled id, desired, actual.
    fn make_tension(id: &str, desired: &str, actual: &str) -> Tension {
        let mut t = Tension::new(desired, actual).unwrap();
        t.id = id.to_owned();
        t
    }

    /// Helper: build an "actual" mutation at a given offset from `base`.
    fn actual_mutation(tension_id: &str, base: DateTime<Utc>, offset_secs: i64, new_val: &str) -> Mutation {
        Mutation::new(
            tension_id.to_owned(),
            base + chrono::Duration::seconds(offset_secs),
            "actual".to_owned(),
            Some("old".to_owned()),
            new_val.to_owned(),
        )
    }

    /// Helper: build a generic mutation (non-actual field).
    fn generic_mutation(tension_id: &str, base: DateTime<Utc>, offset_secs: i64) -> Mutation {
        Mutation::new(
            tension_id.to_owned(),
            base + chrono::Duration::seconds(offset_secs),
            "desired".to_owned(),
            Some("old".to_owned()),
            "new".to_owned(),
        )
    }

    // ── Zero mutations ──────────────────────────────────────────────

    #[test]
    fn test_zero_mutations() {
        let t = make_tension("t1", "write a novel", "have an outline");
        let now = Utc::now();
        let pattern = extract_mutation_pattern(&t, &[], 86400, now);

        assert_eq!(pattern.tension_id, "t1");
        assert_eq!(pattern.mutation_count, 0);
        assert!(pattern.mean_interval_seconds.is_none());
        assert_eq!(pattern.frequency_per_day, 0.0);
        assert_eq!(pattern.frequency_trend, 0.0);
        assert_eq!(pattern.gap_trend, 0.0);
        assert!(pattern.gap_samples.is_empty());
        assert!(!pattern.is_projectable);
    }

    // ── One actual mutation ─────────────────────────────────────────

    #[test]
    fn test_one_actual_mutation_not_projectable() {
        let t = make_tension("t1", "write a novel", "have an outline");
        let now = Utc::now();
        let mutations = vec![actual_mutation("t1", now, -100, "started writing")];
        let pattern = extract_mutation_pattern(&t, &mutations, 86400, now);

        assert_eq!(pattern.mutation_count, 1);
        assert!(pattern.mean_interval_seconds.is_none());
        assert_eq!(pattern.gap_samples.len(), 1);
        assert!(!pattern.is_projectable);
    }

    // ── Steady engagement (evenly spaced) ───────────────────────────

    #[test]
    fn test_steady_engagement_frequency_trend_near_zero() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        // 10 mutations evenly across a 1000s window
        let window = 1000_i64;
        let mutations: Vec<Mutation> = (0..10)
            .map(|i| generic_mutation("t1", now, -window + i * (window / 10) + 1))
            .collect();
        let pattern = extract_mutation_pattern(&t, &mutations, window, now);

        assert_eq!(pattern.mutation_count, 10);
        // 5 in each half, trend should be 0
        assert!(
            pattern.frequency_trend.abs() < 0.01,
            "expected ~0, got {}",
            pattern.frequency_trend
        );
    }

    // ── Accelerating engagement ─────────────────────────────────────

    #[test]
    fn test_accelerating_engagement_positive_trend() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        let window = 1000_i64;
        // 1 mutation in first half, 5 in second half
        let mut mutations = vec![generic_mutation("t1", now, -900)];
        for i in 0..5 {
            mutations.push(generic_mutation("t1", now, -400 + i * 50));
        }
        let pattern = extract_mutation_pattern(&t, &mutations, window, now);

        assert!(
            pattern.frequency_trend > 0.0,
            "expected positive trend, got {}",
            pattern.frequency_trend
        );
    }

    // ── Gap closing ─────────────────────────────────────────────────

    #[test]
    fn test_gap_closing_negative_trend() {
        let t = make_tension("t1", "xyz", "start");
        let now = Utc::now();
        // Actual values that progressively approach "xyz"
        let mutations = vec![
            actual_mutation("t1", now, -300, "aaa"),  // far from "xyz"
            actual_mutation("t1", now, -200, "xya"),  // closer
            actual_mutation("t1", now, -100, "xyz"),  // identical = 0 gap
        ];
        let pattern = extract_mutation_pattern(&t, &mutations, 86400, now);

        assert!(pattern.is_projectable);
        assert!(
            pattern.gap_trend < 0.0,
            "expected negative gap_trend (closing), got {}",
            pattern.gap_trend
        );
    }

    // ── Gap opening ─────────────────────────────────────────────────

    #[test]
    fn test_gap_opening_positive_trend() {
        let t = make_tension("t1", "xyz", "start");
        let now = Utc::now();
        // Actual values that move away from "xyz"
        let mutations = vec![
            actual_mutation("t1", now, -300, "xyz"),                   // identical = 0
            actual_mutation("t1", now, -200, "xya"),                   // small gap
            actual_mutation("t1", now, -100, "completely different"),   // large gap
        ];
        let pattern = extract_mutation_pattern(&t, &mutations, 86400, now);

        assert!(pattern.is_projectable);
        assert!(
            pattern.gap_trend > 0.0,
            "expected positive gap_trend (opening), got {}",
            pattern.gap_trend
        );
    }

    // ── Window filtering ────────────────────────────────────────────

    #[test]
    fn test_window_filtering_excludes_outside() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        let window = 100_i64;
        let mutations = vec![
            generic_mutation("t1", now, -200), // outside window
            generic_mutation("t1", now, -50),  // inside
            generic_mutation("t1", now, -10),  // inside
        ];
        let pattern = extract_mutation_pattern(&t, &mutations, window, now);

        assert_eq!(pattern.mutation_count, 2);
    }

    // ── Mean interval ───────────────────────────────────────────────

    #[test]
    fn test_mean_interval_two_mutations() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        let mutations = vec![
            generic_mutation("t1", now, -200),
            generic_mutation("t1", now, -100),
        ];
        let pattern = extract_mutation_pattern(&t, &mutations, 86400, now);

        assert_eq!(pattern.mutation_count, 2);
        let interval = pattern.mean_interval_seconds.unwrap();
        assert!(
            (interval - 100.0).abs() < 1.0,
            "expected ~100, got {}",
            interval
        );
    }

    // ── Frequency per day ───────────────────────────────────────────

    #[test]
    fn test_frequency_per_day() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        // 10 mutations in a 1-day window
        let mutations: Vec<Mutation> = (0..10)
            .map(|i| generic_mutation("t1", now, -86400 + i * 8640 + 1))
            .collect();
        let pattern = extract_mutation_pattern(&t, &mutations, 86400, now);

        assert!(
            (pattern.frequency_per_day - 10.0).abs() < 0.01,
            "expected ~10, got {}",
            pattern.frequency_per_day
        );
    }

    // ── Gap samples capped at 10 ───────────────────────────────────

    #[test]
    fn test_gap_samples_capped_at_10() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        // 15 actual mutations
        let mutations: Vec<Mutation> = (0..15)
            .map(|i| actual_mutation("t1", now, -1500 + i * 100 + 1, &format!("val{}", i)))
            .collect();
        let pattern = extract_mutation_pattern(&t, &mutations, 86400, now);

        assert_eq!(pattern.gap_samples.len(), 10);
    }

    // ── Linear slope helper ─────────────────────────────────────────

    #[test]
    fn test_linear_slope_constant() {
        let slope = linear_slope(&[5.0, 5.0, 5.0, 5.0]);
        assert!(slope.abs() < f64::EPSILON, "expected 0, got {}", slope);
    }

    #[test]
    fn test_linear_slope_increasing() {
        let slope = linear_slope(&[1.0, 2.0, 3.0, 4.0]);
        assert!(
            (slope - 1.0).abs() < f64::EPSILON,
            "expected 1.0, got {}",
            slope
        );
    }

    #[test]
    fn test_linear_slope_decreasing() {
        let slope = linear_slope(&[4.0, 3.0, 2.0, 1.0]);
        assert!(
            (slope - (-1.0)).abs() < f64::EPSILON,
            "expected -1.0, got {}",
            slope
        );
    }

    // ── ProjectionHorizon ───────────────────────────────────────────

    #[test]
    fn test_projection_horizon_seconds() {
        assert_eq!(ProjectionHorizon::OneWeek.seconds(), 7 * 86400);
        assert_eq!(ProjectionHorizon::OneMonth.seconds(), 30 * 86400);
        assert_eq!(ProjectionHorizon::ThreeMonths.seconds(), 90 * 86400);
        assert_eq!(ProjectionHorizon::Custom(12345).seconds(), 12345);
    }

    // ── Helpers for projection tests ────────────────────────────────

    fn make_pattern(
        gap_trend: f64,
        gap_samples: Vec<f64>,
        mean_interval: Option<f64>,
        frequency_per_day: f64,
        frequency_trend: f64,
        mutation_count: usize,
    ) -> MutationPattern {
        let is_projectable = gap_samples.len() >= 2;
        MutationPattern {
            tension_id: "test".to_owned(),
            mean_interval_seconds: mean_interval,
            mutation_count,
            frequency_per_day,
            frequency_trend,
            gap_trend,
            gap_samples,
            is_projectable,
        }
    }

    // ── project_gap_at ──────────────────────────────────────────────

    #[test]
    fn test_closing_gap_projects_lower() {
        // gap_trend = -0.1 per sample, mean_interval = 100s
        // rate = -0.1 / 100 = -0.001 per second
        // projected = 0.5 + (-0.001) * 200 = 0.3
        let p = make_pattern(-0.1, vec![0.6, 0.5], Some(100.0), 1.0, 0.0, 5);
        let projected = project_gap_at(&p, 0.5, 200);
        assert!(
            (projected - 0.3).abs() < 1e-9,
            "expected 0.3, got {}",
            projected
        );
    }

    #[test]
    fn test_opening_gap_projects_higher_capped() {
        // gap_trend = 0.2 per sample, mean_interval = 100s
        // rate = 0.2 / 100 = 0.002 per second
        // projected = 0.8 + 0.002 * 500 = 1.8 -> clamped to 1.0
        let p = make_pattern(0.2, vec![0.6, 0.8], Some(100.0), 1.0, 0.0, 5);
        let projected = project_gap_at(&p, 0.8, 500);
        assert!(
            (projected - 1.0).abs() < 1e-9,
            "expected 1.0 (clamped), got {}",
            projected
        );
    }

    #[test]
    fn test_zero_gap_stays_zero() {
        let p = make_pattern(0.0, vec![0.0, 0.0], Some(100.0), 1.0, 0.0, 5);
        let projected = project_gap_at(&p, 0.0, 1000);
        assert!(
            projected.abs() < 1e-9,
            "expected 0.0, got {}",
            projected
        );
    }

    #[test]
    fn test_stable_gap_trend_zero_unchanged() {
        let p = make_pattern(0.0, vec![0.5, 0.5], Some(100.0), 1.0, 0.0, 5);
        let projected = project_gap_at(&p, 0.5, 5000);
        assert!(
            (projected - 0.5).abs() < 1e-9,
            "expected 0.5, got {}",
            projected
        );
    }

    // ── project_frequency_at ────────────────────────────────────────

    #[test]
    fn test_declining_frequency_projects_toward_zero() {
        // frequency_per_day = 2.0, frequency_trend = -1.0 (halving)
        // mean_interval = 43200 (12h), mutation_count = 4
        // analysis_window_days = 43200 * 4 / 86400 = 2.0
        // days_forward for 4 days = 4.0
        // projected = 2.0 * (1.0 + (-1.0) * 4.0 / 2.0) = 2.0 * (1 - 2) = -2.0 -> clamped to 0.0
        let p = make_pattern(0.0, vec![0.5, 0.5], Some(43200.0), 2.0, -1.0, 4);
        let projected = project_frequency_at(&p, 4 * 86400);
        assert!(
            projected.abs() < 1e-9,
            "expected 0.0 (clamped), got {}",
            projected
        );
    }

    // ── estimate_time_to_resolution ─────────────────────────────────

    #[test]
    fn test_estimate_time_to_resolution_gap_opening_returns_none() {
        let p = make_pattern(0.1, vec![0.3, 0.4], Some(100.0), 1.0, 0.0, 5);
        assert!(estimate_time_to_resolution(&p, 0.5).is_none());
    }

    #[test]
    fn test_estimate_time_to_resolution_gap_closing_returns_some() {
        // gap_trend = -0.1 per sample, mean_interval = 100s
        // rate = 0.1 / 100 = 0.001 per second
        // time = 0.5 / 0.001 = 500 seconds
        let p = make_pattern(-0.1, vec![0.6, 0.5], Some(100.0), 1.0, 0.0, 5);
        let result = estimate_time_to_resolution(&p, 0.5);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 500);
    }

    // ── classify_trajectory ─────────────────────────────────────────

    #[test]
    fn test_trajectory_resolving() {
        // Engaged (freq > threshold) and gap closing (gap_trend < -0.001).
        let p = make_pattern(-0.05, vec![0.8, 0.6, 0.4], Some(100.0), 1.0, 0.0, 10);
        let thresholds = ProjectionThresholds::default();
        assert_eq!(classify_trajectory(&p, &thresholds), Trajectory::Resolving);
    }

    #[test]
    fn test_trajectory_stalling_zero_mutations() {
        // Zero frequency → stalling.
        let p = make_pattern(0.0, vec![], None, 0.0, 0.0, 0);
        let thresholds = ProjectionThresholds::default();
        assert_eq!(classify_trajectory(&p, &thresholds), Trajectory::Stalling);
    }

    #[test]
    fn test_trajectory_stalling_declining() {
        // Frequency trend sharply declining (< -0.5).
        let p = make_pattern(-0.01, vec![0.5, 0.45], Some(100.0), 0.5, -0.8, 5);
        let thresholds = ProjectionThresholds::default();
        assert_eq!(classify_trajectory(&p, &thresholds), Trajectory::Stalling);
    }

    #[test]
    fn test_trajectory_drifting() {
        // Engaged (freq > threshold) but gap essentially flat (>= -0.001).
        let p = make_pattern(0.0, vec![0.5, 0.5, 0.5], Some(100.0), 1.0, 0.0, 10);
        let thresholds = ProjectionThresholds::default();
        assert_eq!(classify_trajectory(&p, &thresholds), Trajectory::Drifting);
    }

    #[test]
    fn test_trajectory_oscillating() {
        // Gap samples alternate up/down.
        let p = make_pattern(0.0, vec![0.5, 0.7, 0.4, 0.8], Some(100.0), 1.0, 0.0, 10);
        let thresholds = ProjectionThresholds::default();
        assert_eq!(classify_trajectory(&p, &thresholds), Trajectory::Oscillating);
    }

    // ── project_tension ─────────────────────────────────────────────

    #[test]
    fn test_project_tension_returns_three_horizons() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(&t, &[], &thresholds, now);
        assert_eq!(projections.len(), 3);
        assert_eq!(projections[0].horizon, ProjectionHorizon::OneWeek);
        assert_eq!(projections[1].horizon, ProjectionHorizon::OneMonth);
        assert_eq!(projections[2].horizon, ProjectionHorizon::ThreeMonths);
    }

    #[test]
    fn test_project_tension_steady_closing_resolving() {
        let t = make_tension("t1", "xyz", "start");
        let now = Utc::now();
        let window = 30 * 86400_i64;
        // Create many mutations spread evenly across the window to ensure
        // frequency_per_day is well above the neglect threshold.
        let step = window / 20;
        let mut mutations: Vec<Mutation> = (0..20)
            .map(|i| generic_mutation("t1", now, -window + step * i + 1))
            .collect();
        // Add actual mutations showing gap closing.
        mutations.push(actual_mutation("t1", now, -window + 1000, "aaa"));
        mutations.push(actual_mutation("t1", now, -window / 2, "xya"));
        mutations.push(actual_mutation("t1", now, -1000, "xyz"));
        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(&t, &mutations, &thresholds, now);
        for p in &projections {
            assert_eq!(p.trajectory, Trajectory::Resolving);
        }
    }

    #[test]
    fn test_project_tension_zero_mutations_stalling_neglect() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(&t, &[], &thresholds, now);
        for p in &projections {
            assert_eq!(p.trajectory, Trajectory::Stalling);
            assert!(p.neglect_risk, "expected neglect_risk = true");
        }
    }

    #[test]
    fn test_project_tension_high_frequency_flat_gap_drifting() {
        let t = make_tension("t1", "goal", "start");
        let now = Utc::now();
        let window = 30 * 86400_i64;
        // Many mutations spread evenly across the window (high frequency)
        // but actual values never change → flat gap.
        let step = window / 30;
        let mut mutations: Vec<Mutation> = (0..30)
            .map(|i| generic_mutation("t1", now, -window + step * i + 1))
            .collect();
        // Actual mutations all with the same value (flat gap).
        for i in 0..5 {
            mutations.push(actual_mutation(
                "t1",
                now,
                -window + (i + 1) * (window / 6),
                "start",
            ));
        }
        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(&t, &mutations, &thresholds, now);
        for p in &projections {
            assert_eq!(p.trajectory, Trajectory::Drifting);
        }
    }

    #[test]
    fn test_project_tension_oscillating_gap() {
        let t = make_tension("t1", "xyz", "start");
        let now = Utc::now();
        let window = 30 * 86400_i64;
        // Alternating closer / further from desired.
        let mutations = vec![
            actual_mutation("t1", now, -window + 1000, "aaa"),  // far
            actual_mutation("t1", now, -window + 2000, "xyz"),  // close
            actual_mutation("t1", now, -window + 3000, "aaa"),  // far
            actual_mutation("t1", now, -window + 4000, "xyz"),  // close
        ];
        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(&t, &mutations, &thresholds, now);
        for p in &projections {
            assert_eq!(p.trajectory, Trajectory::Oscillating);
        }
    }

    #[test]
    fn test_project_tension_will_resolve_true() {
        use crate::horizon::Horizon;
        let mut t = make_tension("t1", "xyz", "start");
        // Set a horizon far in the future (1 year from now).
        let future = now_plus_days(365);
        t.horizon = Some(Horizon::parse(&future).unwrap());
        let now = Utc::now();
        let window = 30 * 86400_i64;
        // Rapid gap closing: should resolve well before 1 year.
        let mutations = vec![
            actual_mutation("t1", now, -window + 1000, "aaa"),
            actual_mutation("t1", now, -window + 2000, "xya"),
            actual_mutation("t1", now, -window + 3000, "xyy"),
            actual_mutation("t1", now, -window + 4000, "xyz"),
        ];
        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(&t, &mutations, &thresholds, now);
        // At least the first projection should say will_resolve = Some(true).
        assert_eq!(projections[0].will_resolve, Some(true));
    }

    #[test]
    fn test_project_tension_will_resolve_false() {
        use crate::horizon::Horizon;
        let mut t = make_tension("t1", "very-far-desired-state", "completely-different-actual");
        // Set a horizon very soon (tomorrow).
        let tomorrow = now_plus_days(1);
        t.horizon = Some(Horizon::parse(&tomorrow).unwrap());
        let now = Utc::now();
        let window = 30 * 86400_i64;
        // Very slow gap closing — gap_trend barely negative, gap still large.
        let mutations = vec![
            actual_mutation("t1", now, -window + 1000, "aaaaaaaaaaaaaaaaaaa"),
            actual_mutation("t1", now, -window + 2000, "aaaaaaaaaaaaaaaaaa"),
        ];
        let thresholds = ProjectionThresholds::default();
        let projections = project_tension(&t, &mutations, &thresholds, now);
        // Horizon is tomorrow — insufficient velocity to resolve.
        // will_resolve should be Some(false).
        assert_eq!(projections[0].will_resolve, Some(false));
    }

    /// Helper: produce a horizon-parseable date string N days from now.
    fn now_plus_days(days: i64) -> String {
        let future = Utc::now() + chrono::Duration::days(days);
        future.format("%Y-%m").to_string()
    }

    // ── project_field tests ───────────────────────────────────────

    #[test]
    fn test_project_field_empty_tensions() {
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();
        let result = project_field(&[], &[], &thresholds, now);

        assert!(result.tension_projections.is_empty());
        assert!(result.urgency_collisions.is_empty());
        // Funnel should still have 3 horizon entries, all zero.
        assert_eq!(result.funnel.len(), 3);
        for (_, buckets) in &result.funnel {
            assert_eq!(buckets.total, 0);
        }
    }

    #[test]
    fn test_project_field_mixed_trajectories() {
        let now = Utc::now();
        let window = 30 * 86400_i64;
        let thresholds = ProjectionThresholds::default();

        // Tension A: resolving (closing gap, high frequency).
        let t_a = make_tension("a", "xyz", "start");
        let step = window / 20;
        let mut muts_a: Vec<Mutation> = (0..20)
            .map(|i| generic_mutation("a", now, -window + step * i + 1))
            .collect();
        muts_a.push(actual_mutation("a", now, -window + 1000, "aaa"));
        muts_a.push(actual_mutation("a", now, -window / 2, "xya"));
        muts_a.push(actual_mutation("a", now, -1000, "xyz"));

        // Tension B: stalling (zero mutations).
        let t_b = make_tension("b", "goal", "start");

        let all_mutations = muts_a;
        let tensions = vec![t_a, t_b];

        let result = project_field(&tensions, &all_mutations, &thresholds, now);

        assert_eq!(result.tension_projections.len(), 2);

        // Check OneWeek funnel.
        let week_buckets = result.funnel.get(&ProjectionHorizon::OneWeek).unwrap();
        assert_eq!(week_buckets.total, 2);
        assert_eq!(week_buckets.resolving, 1);
        assert_eq!(week_buckets.stalling, 1);
    }

    #[test]
    fn test_project_field_excludes_resolved_and_released() {
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();

        let t_active = make_tension("active", "goal", "start");
        let mut t_resolved = make_tension("resolved", "goal", "start");
        t_resolved.status = crate::tension::TensionStatus::Resolved;
        let mut t_released = make_tension("released", "goal", "start");
        t_released.status = crate::tension::TensionStatus::Released;

        let tensions = vec![t_active, t_resolved, t_released];
        let result = project_field(&tensions, &[], &thresholds, now);

        assert_eq!(result.tension_projections.len(), 1);
        assert_eq!(result.tension_projections[0].0, "active");
    }

    #[test]
    fn test_project_field_funnel_has_three_horizons() {
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();
        let t = make_tension("t1", "goal", "start");
        let result = project_field(&[t], &[], &thresholds, now);

        assert!(result.funnel.contains_key(&ProjectionHorizon::OneWeek));
        assert!(result.funnel.contains_key(&ProjectionHorizon::OneMonth));
        assert!(result.funnel.contains_key(&ProjectionHorizon::ThreeMonths));
        assert_eq!(result.funnel.len(), 3);
    }

    #[test]
    fn test_project_field_no_collisions_without_horizons() {
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();

        // Tensions without horizons → compute_urgency returns None → no collisions.
        let t1 = make_tension("t1", "goal1", "start1");
        let t2 = make_tension("t2", "goal2", "start2");
        let tensions = vec![t1, t2];

        let result = project_field(&tensions, &[], &thresholds, now);
        assert!(result.urgency_collisions.is_empty());
    }

    #[test]
    fn test_project_field_urgency_collision_detected() {
        use crate::horizon::Horizon;

        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();

        // Two tensions with horizons set so urgency > 0.7 at the same sample point.
        // Urgency = time_elapsed / total_window. We need urgency > 0.7 at some
        // sample point in [now, now+90d].
        //
        // If created_at = now - 100 days, horizon_end = now + 10 days:
        //   total_window = 110 days
        //   At now (sample week 0): elapsed = 100d, urgency = 100/110 ≈ 0.91 > 0.7 ✓
        let created = now - chrono::Duration::days(100);
        let horizon_end = now + chrono::Duration::days(10);
        let horizon_str = horizon_end.format("%Y-%m-%d").to_string();

        let mut t1 = make_tension("t1", "goal1", "start1");
        t1.created_at = created;
        t1.horizon = Some(Horizon::parse(&horizon_str).unwrap());

        let mut t2 = make_tension("t2", "goal2", "start2");
        t2.created_at = created;
        t2.horizon = Some(Horizon::parse(&horizon_str).unwrap());

        let tensions = vec![t1, t2];
        let result = project_field(&tensions, &[], &thresholds, now);

        assert!(
            !result.urgency_collisions.is_empty(),
            "expected at least one urgency collision"
        );
        let collision = &result.urgency_collisions[0];
        assert_eq!(collision.tension_ids.len(), 2);
        assert!(collision.peak_combined_urgency > 1.4); // both > 0.7
    }
}
