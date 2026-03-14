//! Health command handler.
//!
//! System health summary: phase distribution, movement ratios, alerts.

use chrono::Utc;
use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{
    compute_urgency, CreativeCyclePhase, DynamicsEngine, StructuralTendency, TensionStatus,
};

/// JSON output structure for health.
#[derive(Serialize)]
struct HealthJson {
    active_count: usize,
    phase_distribution: PhaseDistribution,
    movement_ratios: MovementRatios,
    alerts: Alerts,
}

#[derive(Serialize)]
struct PhaseDistribution {
    germination: usize,
    assimilation: usize,
    completion: usize,
    momentum: usize,
}

#[derive(Serialize)]
struct MovementRatios {
    advancing: usize,
    oscillating: usize,
    stagnant: usize,
}

#[derive(Serialize)]
struct Alerts {
    urgent: usize,
    neglected: usize,
}

fn bar(count: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "\u{2591}".repeat(width);
    }
    let filled = (count as f64 / total as f64 * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    )
}

pub fn cmd_health(output: &Output) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let mut engine = DynamicsEngine::with_store(store);
    let now = Utc::now();

    let tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    // Filter active
    let active: Vec<_> = tensions
        .iter()
        .filter(|t| t.status != TensionStatus::Resolved && t.status != TensionStatus::Released)
        .collect();
    let total = active.len();

    // Phase distribution
    let mut germination = 0usize;
    let mut assimilation = 0usize;
    let mut completion = 0usize;
    let mut momentum = 0usize;

    // Movement distribution
    let mut advancing = 0usize;
    let mut oscillating = 0usize;
    let mut stagnant = 0usize;

    // Alerts
    let mut urgent = 0usize;
    let mut neglected = 0usize;

    for t in &active {
        if let Some(cd) = engine.compute_full_dynamics_for_tension(&t.id) {
            match cd.phase.phase {
                CreativeCyclePhase::Germination => germination += 1,
                CreativeCyclePhase::Assimilation => assimilation += 1,
                CreativeCyclePhase::Completion => completion += 1,
                CreativeCyclePhase::Momentum => momentum += 1,
            }
            match cd.tendency.tendency {
                StructuralTendency::Advancing => advancing += 1,
                StructuralTendency::Oscillating => oscillating += 1,
                StructuralTendency::Stagnant => stagnant += 1,
            }
            if cd.neglect.is_some() {
                neglected += 1;
            }
        }
        if let Some(u) = compute_urgency(t, now) {
            if u.value > 0.75 {
                urgent += 1;
            }
        }
    }

    if output.is_structured() {
        let result = HealthJson {
            active_count: total,
            phase_distribution: PhaseDistribution {
                germination,
                assimilation,
                completion,
                momentum,
            },
            movement_ratios: MovementRatios {
                advancing,
                oscillating,
                stagnant,
            },
            alerts: Alerts { urgent, neglected },
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let bar_width = 10;

        println!("System Health ({} active tensions)", total);
        println!();
        println!("Phase Distribution:");
        println!(
            "  Germination   {}  {}",
            bar(germination, total, bar_width),
            germination,
        );
        println!(
            "  Assimilation  {}  {}",
            bar(assimilation, total, bar_width),
            assimilation,
        );
        println!(
            "  Completion    {}  {}",
            bar(completion, total, bar_width),
            completion,
        );
        println!(
            "  Momentum      {}  {}",
            bar(momentum, total, bar_width),
            momentum,
        );
        println!();
        println!("Movement Ratios:");
        println!(
            "  Advancing     {}  {}",
            bar(advancing, total, bar_width),
            advancing,
        );
        println!(
            "  Oscillating   {}  {}",
            bar(oscillating, total, bar_width),
            oscillating,
        );
        println!(
            "  Stagnant      {}  {}",
            bar(stagnant, total, bar_width),
            stagnant,
        );

        if urgent > 0 || neglected > 0 {
            println!();
            println!("Alerts:");
            if urgent > 0 {
                println!("  ! {} urgent tension(s)", urgent);
            }
            if neglected > 0 {
                println!("  ! {} neglected tension(s)", neglected);
            }
        }
    }

    Ok(())
}
