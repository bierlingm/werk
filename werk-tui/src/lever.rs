//! Lever computation — find the single highest-leverage structural move.
//!
//! The lever is the tension where a single action would produce the most
//! structural movement across the entire system.

use chrono::Utc;
use sd_core::{
    ComputedDynamics, ConflictPattern, DynamicsEngine, Forest, ProjectionHorizon,
    ProjectionThresholds, StructuralTendency, Tension, TensionStatus, Trajectory,
};

/// The result of lever computation.
#[derive(Debug, Clone)]
pub struct LeverResult {
    /// The tension ID of the highest-leverage move.
    pub tension_id: String,
    /// The desired state of the lever tension.
    pub tension_desired: String,
    /// The overall lever score (0.0 to 1.0).
    pub score: f64,
    /// The recommended action.
    pub action: LeverAction,
    /// Human-readable reasoning for the recommendation.
    pub reasoning: String,
    /// Per-component score breakdown.
    pub breakdown: LeverBreakdown,
    /// Number of downstream tensions that would benefit.
    pub cascade_count: usize,
    /// Downstream tensions (id, desired) that would benefit.
    pub cascade_tensions: Vec<(String, String)>,
}

/// The recommended lever action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeverAction {
    UpdateReality,
    BreakOscillation,
    RedirectAttention,
    AdjustHorizon,
    ResolveConflict,
    UnblockDownstream,
    DeepenAssimilation,
    AddressCompensation,
    SetHorizon,
}

impl LeverAction {
    /// Human-readable label for the action.
    pub fn label(&self) -> &'static str {
        match self {
            Self::UpdateReality => "Update reality",
            Self::BreakOscillation => "Break oscillation",
            Self::RedirectAttention => "Redirect attention",
            Self::AdjustHorizon => "Adjust horizon",
            Self::ResolveConflict => "Resolve conflict",
            Self::UnblockDownstream => "Unblock downstream",
            Self::DeepenAssimilation => "Deepen assimilation",
            Self::AddressCompensation => "Address compensation",
            Self::SetHorizon => "Set horizon",
        }
    }
}

/// Per-component breakdown of the lever score.
#[derive(Debug, Clone)]
pub struct LeverBreakdown {
    pub temporal_pressure: f64,
    pub gap_magnitude: f64,
    pub combined_pressure: f64,
    pub stuck_energy: f64,
    pub sibling_imbalance: f64,
    pub workaround_duration: f64,
    pub stalled_potential: f64,
    pub cascade_potential: f64,
    pub falling_behind: f64,
    pub systemic_blocker: f64,
    pub horizon_integrity: f64,
    pub trajectory_urgency: f64,
}

/// Compute the single highest-leverage structural move.
///
/// Returns `None` if there are no active tensions.
pub fn compute_lever(engine: &mut DynamicsEngine) -> Option<LeverResult> {
    let tensions = engine.store().list_tensions().ok()?;

    // Filter to non-resolved tensions
    let active_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| t.status != TensionStatus::Resolved && t.status != TensionStatus::Released)
        .collect();

    if active_tensions.is_empty() {
        return None;
    }

    // Build forest for descendant counting
    let all_tensions = tensions.clone();
    let forest = Forest::from_tensions(all_tensions).ok()?;

    // Score each tension
    let mut best: Option<(f64, LeverResult)> = None;

    for tension in &active_tensions {
        let computed = engine.compute_full_dynamics_for_tension(&tension.id);
        let cd = match computed {
            Some(ref c) => c,
            None => continue,
        };

        let breakdown = compute_breakdown(cd, tension, &forest, engine);
        let score = weighted_score(&breakdown);

        // Get descendants for cascade info
        let descendants = forest.descendants(&tension.id).unwrap_or_default();
        let cascade_tensions: Vec<(String, String)> = descendants
            .iter()
            .filter(|n| {
                n.tension.status != TensionStatus::Resolved
                    && n.tension.status != TensionStatus::Released
            })
            .map(|n| (n.tension.id.clone(), n.tension.desired.clone()))
            .collect();
        let cascade_count = cascade_tensions.len();

        // Determine action from highest-scoring component
        let action = determine_action(&breakdown);

        // Generate reasoning
        let reasoning = generate_reasoning(&breakdown, &action, cascade_count);

        let result = LeverResult {
            tension_id: tension.id.clone(),
            tension_desired: tension.desired.clone(),
            score,
            action,
            reasoning,
            breakdown,
            cascade_count,
            cascade_tensions,
        };

        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, result));
        }
    }

    best.map(|(_, result)| result)
}

fn compute_breakdown(
    cd: &ComputedDynamics,
    tension: &Tension,
    forest: &Forest,
    engine: &DynamicsEngine,
) -> LeverBreakdown {
    let tension_id = &tension.id;
    // temporal_pressure: urgency value capped at 1.0
    let temporal_pressure = cd
        .urgency
        .as_ref()
        .map(|u| u.value.min(1.0))
        .unwrap_or(0.0);

    // gap_magnitude: structural tension magnitude
    let gap_magnitude = cd
        .structural_tension
        .as_ref()
        .map(|st| st.magnitude)
        .unwrap_or(0.0);

    // combined_pressure: structural tension pressure capped at 1.0
    let combined_pressure = cd
        .structural_tension
        .as_ref()
        .and_then(|st| st.pressure)
        .map(|p| p.min(1.0))
        .unwrap_or(0.0);

    // stuck_energy: oscillation reversals / 10, capped at 1.0
    let stuck_energy = cd
        .oscillation
        .as_ref()
        .map(|o| (o.reversals as f64 / 10.0).min(1.0))
        .unwrap_or(0.0);

    // sibling_imbalance: neglect activity ratio capped at 1.0
    let sibling_imbalance = cd
        .neglect
        .as_ref()
        .map(|n| n.activity_ratio.min(1.0))
        .unwrap_or(0.0);

    // workaround_duration: compensating persistence / 604800 (1 week), capped at 1.0
    let workaround_duration = cd
        .compensating_strategy
        .as_ref()
        .map(|cs| (cs.persistence_seconds as f64 / 604800.0).min(1.0))
        .unwrap_or(0.0);

    // stalled_potential: 1.0 if tendency is Stagnant
    let stalled_potential = if cd.tendency.tendency == StructuralTendency::Stagnant {
        1.0
    } else {
        0.0
    };

    // cascade_potential: active descendant count / 10, capped at 1.0
    let descendants = forest.descendants(tension_id).unwrap_or_default();
    let active_descendants = descendants
        .iter()
        .filter(|n| {
            n.tension.status != TensionStatus::Resolved
                && n.tension.status != TensionStatus::Released
        })
        .count();
    let cascade_potential = (active_descendants as f64 / 10.0).min(1.0);

    // falling_behind: 1.0 if resolution.is_sufficient == Some(false)
    let falling_behind = cd
        .resolution
        .as_ref()
        .and_then(|r| r.is_sufficient)
        .map(|s| if s { 0.0 } else { 1.0 })
        .unwrap_or(0.0);

    // systemic_blocker: 1.0 if conflict pattern is CompetingTensions
    let systemic_blocker = cd
        .conflict
        .as_ref()
        .map(|c| {
            if c.pattern == ConflictPattern::CompetingTensions {
                1.0
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    // horizon_integrity: 1.0 if no horizon set (urgency is None)
    let horizon_integrity = if cd.urgency.is_none() { 1.0 } else { 0.0 };

    // trajectory_urgency: how structurally concerning the tension's trajectory is
    let now = Utc::now();
    let tension_mutations = engine.store().get_mutations(tension_id).unwrap_or_default();
    let thresholds = ProjectionThresholds::default();
    let projections = sd_core::project_tension(tension, &tension_mutations, &thresholds, now);

    let trajectory_urgency =
        if let Some(proj_1m) = projections.iter().find(|p| p.horizon == ProjectionHorizon::OneMonth)
        {
            match proj_1m.trajectory {
                Trajectory::Stalling | Trajectory::Oscillating => {
                    // Worse if tension has approaching horizon
                    if temporal_pressure > 0.5 {
                        1.0
                    } else {
                        0.7
                    }
                }
                Trajectory::Drifting => 0.5,
                Trajectory::Resolving => 0.0,
            }
        } else {
            0.0
        };

    LeverBreakdown {
        temporal_pressure,
        gap_magnitude,
        combined_pressure,
        stuck_energy,
        sibling_imbalance,
        workaround_duration,
        stalled_potential,
        cascade_potential,
        falling_behind,
        systemic_blocker,
        horizon_integrity,
        trajectory_urgency,
    }
}

fn weighted_score(b: &LeverBreakdown) -> f64 {
    b.temporal_pressure * 0.13
        + b.gap_magnitude * 0.13
        + b.combined_pressure * 0.10
        + b.stuck_energy * 0.10
        + b.sibling_imbalance * 0.08
        + b.workaround_duration * 0.05
        + b.stalled_potential * 0.08
        + b.cascade_potential * 0.10
        + b.falling_behind * 0.05
        + b.systemic_blocker * 0.05
        + b.horizon_integrity * 0.05
        + b.trajectory_urgency * 0.08
}

fn determine_action(b: &LeverBreakdown) -> LeverAction {
    let components: [(f64, LeverAction); 10] = [
        (b.stalled_potential, LeverAction::UpdateReality),
        (b.stuck_energy, LeverAction::BreakOscillation),
        (b.sibling_imbalance, LeverAction::RedirectAttention),
        (b.temporal_pressure, LeverAction::AdjustHorizon),
        (b.systemic_blocker, LeverAction::ResolveConflict),
        (b.cascade_potential, LeverAction::UnblockDownstream),
        (b.horizon_integrity, LeverAction::SetHorizon),
        (b.workaround_duration, LeverAction::AddressCompensation),
        (b.falling_behind, LeverAction::DeepenAssimilation),
        (b.trajectory_urgency, LeverAction::UpdateReality), // trajectory concern → check reality
    ];

    components
        .iter()
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, action)| *action)
        .unwrap_or(LeverAction::UpdateReality)
}

fn generate_reasoning(b: &LeverBreakdown, action: &LeverAction, cascade_count: usize) -> String {
    let mut parts = Vec::new();

    if b.temporal_pressure > 0.5 {
        parts.push(format!("High urgency ({:.0}%)", b.temporal_pressure * 100.0));
    }
    if b.stalled_potential > 0.0 {
        parts.push("stagnant movement".to_string());
    }
    if b.stuck_energy > 0.3 {
        parts.push(format!(
            "oscillating ({} reversals)",
            (b.stuck_energy * 10.0) as usize
        ));
    }
    if cascade_count > 0 {
        parts.push(format!("{} children blocked", cascade_count));
    }
    if b.systemic_blocker > 0.0 {
        parts.push("competing tensions detected".to_string());
    }
    if b.horizon_integrity > 0.0 {
        parts.push("no horizon set".to_string());
    }
    if b.trajectory_urgency > 0.3 {
        parts.push("trajectory at risk".to_string());
    }

    let context = if parts.is_empty() {
        "Active tension".to_string()
    } else {
        parts.join(", ")
    };

    let advice = match action {
        LeverAction::UpdateReality => "update reality to check progress",
        LeverAction::BreakOscillation => "break the oscillation pattern",
        LeverAction::RedirectAttention => "redirect attention to neglected area",
        LeverAction::AdjustHorizon => "review and adjust the horizon",
        LeverAction::ResolveConflict => "resolve the competing tensions",
        LeverAction::UnblockDownstream => "resolving cascades movement downstream",
        LeverAction::DeepenAssimilation => "deepen assimilation of progress",
        LeverAction::AddressCompensation => "address the workaround pattern",
        LeverAction::SetHorizon => "set a horizon to create temporal structure",
    };

    format!("{} \u{2014} {}", context, advice)
}
