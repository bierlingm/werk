//! Shared dynamics computation module.
//!
//! This module provides a single set of JSON serialization types and a unified
//! `compute_all_dynamics()` function used by `show`, `context`, and `run` commands.
//! All dynamics functions receive the correct horizon parameter — no `None` pass-through.

use chrono::{DateTime, Utc};
use serde::Serialize;

use sd_core::{
    classify_creative_cycle_phase, classify_orientation, compute_structural_tension,
    compute_urgency, detect_compensating_strategy, detect_horizon_drift, detect_neglect,
    detect_oscillation, detect_resolution, detect_structural_conflict, measure_assimilation_depth,
    predict_structural_tendency, AssimilationDepthThresholds, CompensatingStrategyThresholds,
    ConflictThresholds, Forest, LifecycleThresholds, Mutation, NeglectThresholds,
    OrientationThresholds, OscillationThresholds, ResolutionThresholds, Tension, TensionStatus,
};

// ============================================================================
// JSON Serialization Types (shared across show, context, run)
// ============================================================================

/// All 10 dynamics plus horizon_drift in JSON format.
///
/// Used by the `show` command (serializes the phase field as `"phase"`).
#[derive(Serialize, Clone, Debug)]
pub struct DynamicsJson {
    pub structural_tension: Option<StructuralTensionJson>,
    pub structural_conflict: Option<ConflictJson>,
    pub oscillation: Option<OscillationJson>,
    pub resolution: Option<ResolutionJson>,
    pub phase: PhaseJson,
    pub orientation: Option<OrientationJson>,
    pub compensating_strategy: Option<CompensatingStrategyJson>,
    pub structural_tendency: TendencyJson,
    pub assimilation_depth: Option<AssimilationDepthJson>,
    pub neglect: Option<NeglectJson>,
    pub horizon_drift: HorizonDriftJson,
}

/// Dynamics JSON with `creative_cycle_phase` field name (used by `context` and `run` commands).
///
/// Identical data to [`DynamicsJson`] but serializes the phase field as
/// `"creative_cycle_phase"` for backward compatibility with agent consumers.
#[derive(Serialize, Clone, Debug)]
pub struct ContextDynamicsJson {
    pub structural_tension: Option<StructuralTensionJson>,
    pub structural_conflict: Option<ConflictJson>,
    pub oscillation: Option<OscillationJson>,
    pub resolution: Option<ResolutionJson>,
    pub creative_cycle_phase: PhaseJson,
    pub orientation: Option<OrientationJson>,
    pub compensating_strategy: Option<CompensatingStrategyJson>,
    pub structural_tendency: TendencyJson,
    pub assimilation_depth: Option<AssimilationDepthJson>,
    pub neglect: Option<NeglectJson>,
    pub horizon_drift: HorizonDriftJson,
}

impl From<DynamicsJson> for ContextDynamicsJson {
    fn from(d: DynamicsJson) -> Self {
        Self {
            structural_tension: d.structural_tension,
            structural_conflict: d.structural_conflict,
            oscillation: d.oscillation,
            resolution: d.resolution,
            creative_cycle_phase: d.phase,
            orientation: d.orientation,
            compensating_strategy: d.compensating_strategy,
            structural_tendency: d.structural_tendency,
            assimilation_depth: d.assimilation_depth,
            neglect: d.neglect,
            horizon_drift: d.horizon_drift,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct StructuralTensionJson {
    pub magnitude: f64,
    pub has_gap: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct ConflictJson {
    pub pattern: String,
    pub tension_ids: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct OscillationJson {
    pub reversals: usize,
    pub magnitude: f64,
    pub window_start: String,
    pub window_end: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResolutionJson {
    pub velocity: f64,
    pub trend: String,
    pub window_start: String,
    pub window_end: String,
    pub required_velocity: Option<f64>,
    pub is_sufficient: Option<bool>,
}

#[derive(Serialize, Clone, Debug)]
pub struct PhaseJson {
    pub phase: String,
    pub evidence: PhaseEvidenceJson,
}

#[derive(Serialize, Clone, Debug)]
pub struct PhaseEvidenceJson {
    pub mutation_count: usize,
    pub gap_closing: bool,
    pub convergence_ratio: f64,
    pub age_seconds: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct OrientationJson {
    pub orientation: String,
    pub creative_ratio: f64,
    pub problem_solving_ratio: f64,
    pub reactive_ratio: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct CompensatingStrategyJson {
    pub strategy_type: String,
    pub persistence_seconds: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct TendencyJson {
    pub tendency: String,
    pub has_conflict: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct AssimilationDepthJson {
    pub depth: String,
    pub mutation_frequency: f64,
    pub frequency_trend: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct NeglectJson {
    pub neglect_type: String,
    pub activity_ratio: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct HorizonDriftJson {
    pub drift_type: String,
    pub change_count: usize,
    pub net_shift_seconds: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct HorizonRangeJson {
    pub start: String,
    pub end: String,
}

/// Tension information with horizon data (used by context and run commands).
#[derive(Serialize, Clone, Debug)]
pub struct TensionInfo {
    pub id: String,
    pub desired: String,
    pub actual: String,
    pub status: String,
    pub created_at: String,
    pub parent_id: Option<String>,
    pub horizon: Option<String>,
    pub horizon_range: Option<HorizonRangeJson>,
    pub urgency: Option<f64>,
    pub pressure: Option<f64>,
    pub staleness_ratio: Option<f64>,
}

/// Mutation information for display.
#[derive(Serialize, Clone, Debug)]
pub struct MutationInfo {
    pub tension_id: String,
    pub timestamp: String,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: String,
}

// ============================================================================
// Computation
// ============================================================================

/// Compute all 10 dynamics + horizon_drift for a single tension.
///
/// This is the single entry point for dynamics computation in the CLI.
/// All dynamics functions receive the correct horizon parameter via
/// `tension.horizon.as_ref()` — no `None` pass-through.
pub fn compute_all_dynamics(
    tension: &Tension,
    mutations: &[Mutation],
    forest: &Forest,
    all_tensions: &[Tension],
    all_mutations: &[Mutation],
    now: DateTime<Utc>,
) -> DynamicsJson {
    let lifecycle_thresholds = LifecycleThresholds::default();
    let conflict_thresholds = ConflictThresholds::default();
    let oscillation_thresholds = OscillationThresholds::default();
    let resolution_thresholds = ResolutionThresholds::default();
    let orientation_thresholds = OrientationThresholds::default();
    let compensating_thresholds = CompensatingStrategyThresholds::default();
    let assimilation_thresholds = AssimilationDepthThresholds::default();
    let neglect_thresholds = NeglectThresholds::default();

    // Get resolved tensions for momentum phase detection
    let resolved_tensions: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .cloned()
        .collect();

    // 1. Structural Tension
    let structural_tension = compute_structural_tension(tension, now);

    // 2. Structural Conflict
    let conflict = detect_structural_conflict(
        forest,
        &tension.id,
        all_mutations,
        &conflict_thresholds,
        now,
    );

    // 3. Oscillation — passes tension.horizon.as_ref() (NOT None)
    let oscillation = detect_oscillation(
        &tension.id,
        mutations,
        &oscillation_thresholds,
        now,
        tension.horizon.as_ref(),
    );

    // 4. Resolution
    let resolution = detect_resolution(tension, mutations, &resolution_thresholds, now);

    // 5. Creative Cycle Phase
    let phase_result = classify_creative_cycle_phase(
        tension,
        mutations,
        &resolved_tensions,
        &lifecycle_thresholds,
        now,
    );

    // 6. Orientation (requires multiple tensions)
    let orientation =
        classify_orientation(all_tensions, all_mutations, &orientation_thresholds, now);

    // 7. Compensating Strategy — passes tension.horizon.as_ref() (NOT None)
    let compensating_strategy = detect_compensating_strategy(
        &tension.id,
        mutations,
        oscillation.as_ref(),
        &compensating_thresholds,
        now,
        tension.horizon.as_ref(),
    );

    // 8. Structural Tendency — passes Some(now) (NOT None)
    let has_conflict = conflict.is_some();
    let tendency_result = predict_structural_tendency(tension, has_conflict, Some(now), None);

    // 9. Assimilation Depth
    let assimilation = measure_assimilation_depth(
        &tension.id,
        mutations,
        tension,
        &assimilation_thresholds,
        now,
    );

    // 10. Neglect
    let neglect = detect_neglect(forest, &tension.id, all_mutations, &neglect_thresholds, now);

    // Horizon Drift
    let horizon_drift = detect_horizon_drift(&tension.id, mutations);

    // Build JSON output
    DynamicsJson {
        structural_tension: structural_tension.as_ref().map(|st| StructuralTensionJson {
            magnitude: st.magnitude,
            has_gap: st.has_gap,
        }),
        structural_conflict: conflict.as_ref().map(|c| ConflictJson {
            pattern: match c.pattern {
                sd_core::ConflictPattern::AsymmetricActivity => "AsymmetricActivity".to_string(),
                sd_core::ConflictPattern::CompetingTensions => "CompetingTensions".to_string(),
            },
            tension_ids: c.tension_ids.clone(),
        }),
        oscillation: oscillation.as_ref().map(|o| OscillationJson {
            reversals: o.reversals,
            magnitude: o.magnitude,
            window_start: o.window_start.to_rfc3339(),
            window_end: o.window_end.to_rfc3339(),
        }),
        resolution: resolution.as_ref().map(|r| ResolutionJson {
            velocity: r.velocity,
            trend: match r.trend {
                sd_core::ResolutionTrend::Accelerating => "Accelerating".to_string(),
                sd_core::ResolutionTrend::Steady => "Steady".to_string(),
                sd_core::ResolutionTrend::Decelerating => "Decelerating".to_string(),
            },
            window_start: r.window_start.to_rfc3339(),
            window_end: r.window_end.to_rfc3339(),
            required_velocity: r.required_velocity,
            is_sufficient: r.is_sufficient,
        }),
        phase: PhaseJson {
            phase: match phase_result.phase {
                sd_core::CreativeCyclePhase::Germination => "Germination".to_string(),
                sd_core::CreativeCyclePhase::Assimilation => "Assimilation".to_string(),
                sd_core::CreativeCyclePhase::Completion => "Completion".to_string(),
                sd_core::CreativeCyclePhase::Momentum => "Momentum".to_string(),
            },
            evidence: PhaseEvidenceJson {
                mutation_count: phase_result.evidence.mutation_count,
                gap_closing: phase_result.evidence.gap_closing,
                convergence_ratio: phase_result.evidence.convergence_ratio,
                age_seconds: phase_result.evidence.age_seconds,
            },
        },
        orientation: orientation.as_ref().map(|o| OrientationJson {
            orientation: match o.orientation {
                sd_core::Orientation::Creative => "Creative".to_string(),
                sd_core::Orientation::ProblemSolving => "ProblemSolving".to_string(),
                sd_core::Orientation::ReactiveResponsive => "ReactiveResponsive".to_string(),
            },
            creative_ratio: o.evidence.creative_ratio,
            problem_solving_ratio: o.evidence.problem_solving_ratio,
            reactive_ratio: o.evidence.reactive_ratio,
        }),
        compensating_strategy: compensating_strategy
            .as_ref()
            .map(|cs| CompensatingStrategyJson {
                strategy_type: match cs.strategy_type {
                    sd_core::CompensatingStrategyType::TolerableConflict => {
                        "TolerableConflict".to_string()
                    }
                    sd_core::CompensatingStrategyType::ConflictManipulation => {
                        "ConflictManipulation".to_string()
                    }
                    sd_core::CompensatingStrategyType::WillpowerManipulation => {
                        "WillpowerManipulation".to_string()
                    }
                },
                persistence_seconds: cs.persistence_seconds,
            }),
        structural_tendency: TendencyJson {
            tendency: match tendency_result.tendency {
                sd_core::StructuralTendency::Advancing => "Advancing".to_string(),
                sd_core::StructuralTendency::Oscillating => "Oscillating".to_string(),
                sd_core::StructuralTendency::Stagnant => "Stagnant".to_string(),
            },
            has_conflict: tendency_result.has_conflict,
        },
        assimilation_depth: if assimilation.depth == sd_core::AssimilationDepth::None
            && assimilation.evidence.total_mutations == 0
        {
            None
        } else {
            Some(AssimilationDepthJson {
                depth: match assimilation.depth {
                    sd_core::AssimilationDepth::Shallow => "Shallow".to_string(),
                    sd_core::AssimilationDepth::Deep => "Deep".to_string(),
                    sd_core::AssimilationDepth::None => "None".to_string(),
                },
                mutation_frequency: assimilation.mutation_frequency,
                frequency_trend: assimilation.frequency_trend,
            })
        },
        neglect: neglect.as_ref().map(|n| NeglectJson {
            neglect_type: match n.neglect_type {
                sd_core::NeglectType::ParentNeglectsChildren => {
                    "ParentNeglectsChildren".to_string()
                }
                sd_core::NeglectType::ChildrenNeglected => "ChildrenNeglected".to_string(),
            },
            activity_ratio: n.activity_ratio,
        }),
        horizon_drift: HorizonDriftJson {
            drift_type: match horizon_drift.drift_type {
                sd_core::HorizonDriftType::Stable => "Stable".to_string(),
                sd_core::HorizonDriftType::Tightening => "Tightening".to_string(),
                sd_core::HorizonDriftType::Postponement => "Postponement".to_string(),
                sd_core::HorizonDriftType::RepeatedPostponement => {
                    "RepeatedPostponement".to_string()
                }
                sd_core::HorizonDriftType::Loosening => "Loosening".to_string(),
                sd_core::HorizonDriftType::Oscillating => "Oscillating".to_string(),
            },
            change_count: horizon_drift.change_count,
            net_shift_seconds: horizon_drift.net_shift_seconds,
        },
    }
}

/// Compute tension info with horizon data for a forest node.
///
/// Used by context and run commands when building ancestor/sibling/children info.
pub fn node_to_tension_info(node: &sd_core::Node, now: DateTime<Utc>) -> TensionInfo {
    let horizon = node.tension.horizon.as_ref().map(|h| h.to_string());
    let horizon_range = node.tension.horizon.as_ref().map(|h| HorizonRangeJson {
        start: h.range_start().to_rfc3339(),
        end: h.range_end().to_rfc3339(),
    });
    let urgency = compute_urgency(&node.tension, now).map(|u| u.value);
    let structural_tension = compute_structural_tension(&node.tension, now);
    let pressure = structural_tension.as_ref().and_then(|st| st.pressure);

    TensionInfo {
        id: node.id().to_string(),
        desired: node.tension.desired.clone(),
        actual: node.tension.actual.clone(),
        status: node.tension.status.to_string(),
        created_at: node.tension.created_at.to_rfc3339(),
        parent_id: node.tension.parent_id.clone(),
        horizon,
        horizon_range,
        urgency,
        pressure,
        staleness_ratio: None, // Would need mutations to compute for siblings
    }
}

/// Build a TensionInfo for the primary tension (includes staleness_ratio).
pub fn tension_to_info(
    tension: &Tension,
    mutations: &[Mutation],
    now: DateTime<Utc>,
) -> TensionInfo {
    let horizon = tension.horizon.as_ref().map(|h| h.to_string());
    let horizon_range = tension.horizon.as_ref().map(|h| HorizonRangeJson {
        start: h.range_start().to_rfc3339(),
        end: h.range_end().to_rfc3339(),
    });
    let urgency = compute_urgency(tension, now).map(|u| u.value);
    let structural_tension = compute_structural_tension(tension, now);
    let pressure = structural_tension.as_ref().and_then(|st| st.pressure);

    // Staleness ratio: need last mutation timestamp
    let last_mutation_time = mutations.last().map(|m| m.timestamp());
    let staleness_ratio = match (&tension.horizon, last_mutation_time) {
        (Some(h), Some(last_time)) => Some(h.staleness(last_time, now)),
        _ => None,
    };

    TensionInfo {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        created_at: tension.created_at.to_rfc3339(),
        parent_id: tension.parent_id.clone(),
        horizon,
        horizon_range,
        urgency,
        pressure,
        staleness_ratio,
    }
}

/// Build a MutationInfo from a Mutation.
pub fn mutation_to_info(m: &Mutation) -> MutationInfo {
    MutationInfo {
        tension_id: m.tension_id().to_owned(),
        timestamp: m.timestamp().to_rfc3339(),
        field: m.field().to_owned(),
        old_value: m.old_value().map(|s| s.to_owned()),
        new_value: m.new_value().to_owned(),
    }
}
