use chrono::Utc;

use sd_core::{
    compute_urgency, ComputedDynamics, CreativeCyclePhase, DynamicsEngine,
    StructuralTendency, Tension, TensionStatus,
};
use crate::types::{DetailDynamics, TensionRow, UrgencyTier};

pub fn render_bar(value: f64, width: usize) -> String {
    let filled = ((value * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    )
}

pub fn phase_char(phase: CreativeCyclePhase) -> &'static str {
    match phase {
        CreativeCyclePhase::Germination => "G",
        CreativeCyclePhase::Assimilation => "A",
        CreativeCyclePhase::Completion => "C",
        CreativeCyclePhase::Momentum => "M",
    }
}

pub fn phase_name(phase: CreativeCyclePhase) -> &'static str {
    match phase {
        CreativeCyclePhase::Germination => "Germination",
        CreativeCyclePhase::Assimilation => "Assimilation",
        CreativeCyclePhase::Completion => "Completion",
        CreativeCyclePhase::Momentum => "Momentum",
    }
}

pub fn movement_char(tendency: StructuralTendency) -> &'static str {
    match tendency {
        StructuralTendency::Advancing => "\u{2192}",
        StructuralTendency::Oscillating => "\u{2194}",
        StructuralTendency::Stagnant => "\u{25CB}",
    }
}

pub fn movement_name(tendency: StructuralTendency) -> &'static str {
    match tendency {
        StructuralTendency::Advancing => "Advancing",
        StructuralTendency::Oscillating => "Oscillating",
        StructuralTendency::Stagnant => "Stagnant",
    }
}

pub fn format_horizon(tension: &Tension, now: chrono::DateTime<Utc>) -> String {
    match &tension.horizon {
        Some(h) => {
            let days = h.range_end().signed_duration_since(now).num_days();
            if days < 0 {
                format!("{}d past", -days)
            } else if days == 0 {
                "today".to_string()
            } else if days <= 30 {
                format!("{}d", days)
            } else {
                h.to_string()
            }
        }
        None => "\u{2014}".to_string(),
    }
}

pub fn compute_tier(
    tension: &Tension,
    urgency: Option<f64>,
    neglected: bool,
    now: chrono::DateTime<Utc>,
) -> UrgencyTier {
    if tension.status == TensionStatus::Resolved || tension.status == TensionStatus::Released {
        UrgencyTier::Resolved
    } else if urgency.map(|u| u > 0.75).unwrap_or(false)
        || tension
            .horizon
            .as_ref()
            .map(|h| h.range_end() < now)
            .unwrap_or(false)
    {
        UrgencyTier::Urgent
    } else if neglected {
        UrgencyTier::Neglected
    } else {
        UrgencyTier::Active
    }
}

pub fn build_detail_dynamics(cd: &ComputedDynamics) -> DetailDynamics {
    let phase = phase_name(cd.phase.phase).to_string();
    let movement = format!("{} {}", movement_char(cd.tendency.tendency), movement_name(cd.tendency.tendency));
    let magnitude = cd.structural_tension.as_ref().map(|st| st.magnitude);
    let urgency = cd.urgency.as_ref().map(|u| u.value);

    let neglect = cd.neglect.as_ref().map(|n| {
        let ntype = match n.neglect_type {
            sd_core::NeglectType::ParentNeglectsChildren => "Parent neglects children",
            sd_core::NeglectType::ChildrenNeglected => "Children neglected",
        };
        format!("{} (ratio: {:.2})", ntype, n.activity_ratio)
    });

    let conflict = cd.conflict.as_ref().map(|c| {
        let pattern = match c.pattern {
            sd_core::ConflictPattern::AsymmetricActivity => "Asymmetric activity",
            sd_core::ConflictPattern::CompetingTensions => "Competing tensions",
        };
        pattern.to_string()
    });

    let oscillation = cd.oscillation.as_ref().map(|o| {
        format!("{} reversals, magnitude {:.2}", o.reversals, o.magnitude)
    });

    let resolution = cd.resolution.as_ref().map(|r| {
        let trend = match r.trend {
            sd_core::ResolutionTrend::Accelerating => "accelerating",
            sd_core::ResolutionTrend::Steady => "steady",
            sd_core::ResolutionTrend::Decelerating => "decelerating",
        };
        format!("velocity {:.4}, {}", r.velocity, trend)
    });

    let orientation = cd.orientation.as_ref().map(|o| {
        let orient = match o.orientation {
            sd_core::Orientation::Creative => "Creative",
            sd_core::Orientation::ProblemSolving => "Problem-solving",
            sd_core::Orientation::ReactiveResponsive => "Reactive/Responsive",
        };
        format!(
            "{} (creative: {:.0}%, problem: {:.0}%, reactive: {:.0}%)",
            orient,
            o.evidence.creative_ratio * 100.0,
            o.evidence.problem_solving_ratio * 100.0,
            o.evidence.reactive_ratio * 100.0,
        )
    });

    let compensating_strategy = cd.compensating_strategy.as_ref().map(|cs| {
        let stype = match cs.strategy_type {
            sd_core::CompensatingStrategyType::TolerableConflict => "Tolerable conflict",
            sd_core::CompensatingStrategyType::ConflictManipulation => "Conflict manipulation",
            sd_core::CompensatingStrategyType::WillpowerManipulation => "Willpower manipulation",
        };
        format!("{}, persisted {}s", stype, cs.persistence_seconds)
    });

    let assimilation_depth = {
        let depth = match cd.assimilation.depth {
            sd_core::AssimilationDepth::Shallow => "Shallow",
            sd_core::AssimilationDepth::Deep => "Deep",
            sd_core::AssimilationDepth::None => "None",
        };
        if cd.assimilation.depth != sd_core::AssimilationDepth::None {
            Some(format!(
                "{} (freq: {:.2}, trend: {:.2})",
                depth, cd.assimilation.mutation_frequency, cd.assimilation.frequency_trend
            ))
        } else {
            None
        }
    };

    let horizon_drift = {
        let dtype = match cd.horizon_drift.drift_type {
            sd_core::HorizonDriftType::Stable => "Stable",
            sd_core::HorizonDriftType::Tightening => "Tightening",
            sd_core::HorizonDriftType::Postponement => "Postponement",
            sd_core::HorizonDriftType::RepeatedPostponement => "Repeated postponement",
            sd_core::HorizonDriftType::Loosening => "Loosening",
            sd_core::HorizonDriftType::Oscillating => "Oscillating",
        };
        if cd.horizon_drift.change_count > 0 {
            Some(format!(
                "{} ({} changes, net shift {}s)",
                dtype, cd.horizon_drift.change_count, cd.horizon_drift.net_shift_seconds
            ))
        } else {
            None
        }
    };

    DetailDynamics {
        phase,
        movement,
        magnitude,
        urgency,
        neglect,
        conflict,
        oscillation,
        resolution,
        orientation,
        compensating_strategy,
        assimilation_depth,
        horizon_drift,
        forecast_line: None,
    }
}

pub fn build_tension_row(
    engine: &mut DynamicsEngine,
    tension: &Tension,
    now: chrono::DateTime<Utc>,
) -> TensionRow {
    let computed = engine.compute_full_dynamics_for_tension(&tension.id);
    build_tension_row_from_computed(&computed, tension, now, vec![])
}

pub fn build_tension_row_from_computed(
    computed: &Option<ComputedDynamics>,
    tension: &Tension,
    now: chrono::DateTime<Utc>,
    activity: Vec<f64>,
) -> TensionRow {
    let short_id = tension.id.chars().take(6).collect::<String>();

    let (phase, movement, neglected, magnitude) = match computed {
        Some(cd) => {
            let p = phase_char(cd.phase.phase);
            let m = movement_char(cd.tendency.tendency);
            let negl = cd.neglect.is_some();
            let mag = cd.structural_tension.as_ref().map(|st| st.magnitude);
            (p, m, negl, mag)
        }
        None => ("?", "\u{25CB}", false, None),
    };

    let urgency = compute_urgency(tension, now).map(|u| u.value);
    let horizon_display = format_horizon(tension, now);
    let tier = compute_tier(tension, urgency, neglected, now);

    TensionRow {
        id: tension.id.clone(),
        short_id,
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        phase: phase.to_string(),
        movement: movement.to_string(),
        urgency,
        magnitude,
        neglected,
        horizon_display,
        tier,
        activity,
        trajectory: None,
    }
}
