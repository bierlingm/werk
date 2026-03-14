//! List command handler.
//!
//! Flat listing of tensions with rich filtering and sorting options.

use chrono::Utc;
use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{
    compute_urgency, CreativeCyclePhase, DynamicsEngine, StructuralTendency, TensionStatus,
};
use werk_shared::truncate;

/// JSON output structure for a tension in list.
#[derive(Serialize)]
struct ListTensionJson {
    id: String,
    desired: String,
    actual: String,
    status: String,
    phase: String,
    movement: String,
    urgency: Option<f64>,
    magnitude: Option<f64>,
    horizon: Option<String>,
    tier: String,
    neglected: bool,
}

/// JSON output structure for list.
#[derive(Serialize)]
struct ListJson {
    tensions: Vec<ListTensionJson>,
    count: usize,
}

fn phase_char(phase: CreativeCyclePhase) -> &'static str {
    match phase {
        CreativeCyclePhase::Germination => "G",
        CreativeCyclePhase::Assimilation => "A",
        CreativeCyclePhase::Completion => "C",
        CreativeCyclePhase::Momentum => "M",
    }
}

fn movement_char(tendency: StructuralTendency) -> &'static str {
    match tendency {
        StructuralTendency::Advancing => "\u{2192}",
        StructuralTendency::Oscillating => "\u{2194}",
        StructuralTendency::Stagnant => "\u{25CB}",
    }
}

fn format_horizon(tension: &sd_core::Tension, now: chrono::DateTime<Utc>) -> String {
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

/// Computed row data for filtering and sorting.
struct TensionRow {
    id: String,
    desired: String,
    actual: String,
    status: TensionStatus,
    phase: CreativeCyclePhase,
    phase_str: String,
    movement: StructuralTendency,
    movement_str: String,
    urgency: Option<f64>,
    magnitude: Option<f64>,
    horizon_display: String,
    horizon_raw: Option<String>,
    neglected: bool,
    tier: String,
}

pub fn cmd_list(
    output: &Output,
    all: bool,
    urgent: bool,
    neglected: bool,
    stagnant: bool,
    phase: Option<String>,
    sort: String,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let mut engine = DynamicsEngine::with_store(store);
    let now = Utc::now();

    let tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    if tensions.is_empty() {
        if output.is_structured() {
            let result = ListJson {
                tensions: vec![],
                count: 0,
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .info("No tensions found")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
        return Ok(());
    }

    // Build rows with computed dynamics
    let mut rows: Vec<TensionRow> = Vec::new();

    for tension in &tensions {
        let computed = engine.compute_full_dynamics_for_tension(&tension.id);

        let (phase_val, movement_val, is_neglected, mag) = match &computed {
            Some(cd) => (
                cd.phase.phase,
                cd.tendency.tendency,
                cd.neglect.is_some(),
                cd.structural_tension.as_ref().map(|st| st.magnitude),
            ),
            None => (
                CreativeCyclePhase::Germination,
                StructuralTendency::Stagnant,
                false,
                None,
            ),
        };

        let urgency_val = compute_urgency(tension, now).map(|u| u.value);
        let horizon_display = format_horizon(tension, now);

        // Compute tier
        let tier = if tension.status == TensionStatus::Resolved
            || tension.status == TensionStatus::Released
        {
            "resolved"
        } else if urgency_val.map(|u| u > 0.75).unwrap_or(false)
            || tension
                .horizon
                .as_ref()
                .map(|h| h.range_end() < now)
                .unwrap_or(false)
        {
            "urgent"
        } else if is_neglected {
            "neglected"
        } else {
            "active"
        };

        rows.push(TensionRow {
            id: tension.id.clone(),
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            status: tension.status,
            phase: phase_val,
            phase_str: phase_char(phase_val).to_string(),
            movement: movement_val,
            movement_str: movement_char(movement_val).to_string(),
            urgency: urgency_val,
            magnitude: mag,
            horizon_display,
            horizon_raw: tension.horizon.as_ref().map(|h| h.to_string()),
            neglected: is_neglected,
            tier: tier.to_string(),
        });
    }

    // Apply filters
    // By default, show only active tensions (not resolved/released)
    if !all {
        rows.retain(|r| r.status == TensionStatus::Active);
    }

    if urgent {
        rows.retain(|r| r.tier == "urgent");
    }

    if neglected {
        rows.retain(|r| r.neglected);
    }

    if stagnant {
        rows.retain(|r| r.movement == StructuralTendency::Stagnant);
    }

    if let Some(ref phase_filter) = phase {
        let phase_upper = phase_filter.to_uppercase();
        rows.retain(|r| r.phase_str == phase_upper);
    }

    // Sort
    match sort.as_str() {
        "urgency" => {
            rows.sort_by(|a, b| {
                // Higher urgency first, None last
                let ua = a.urgency.unwrap_or(-1.0);
                let ub = b.urgency.unwrap_or(-1.0);
                ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        "phase" => {
            rows.sort_by(|a, b| {
                let phase_order = |p: CreativeCyclePhase| match p {
                    CreativeCyclePhase::Completion => 0,
                    CreativeCyclePhase::Momentum => 1,
                    CreativeCyclePhase::Assimilation => 2,
                    CreativeCyclePhase::Germination => 3,
                };
                phase_order(a.phase).cmp(&phase_order(b.phase))
            });
        }
        "name" => {
            rows.sort_by(|a, b| a.desired.to_lowercase().cmp(&b.desired.to_lowercase()));
        }
        "horizon" => {
            rows.sort_by(|a, b| {
                // Tensions with horizons first (sorted by horizon string), then without
                match (&a.horizon_raw, &b.horizon_raw) {
                    (Some(ha), Some(hb)) => ha.cmp(hb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
        _ => {
            // Default to urgency
            rows.sort_by(|a, b| {
                let ua = a.urgency.unwrap_or(-1.0);
                let ub = b.urgency.unwrap_or(-1.0);
                ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    // Output
    if output.is_structured() {
        let json_tensions: Vec<ListTensionJson> = rows
            .iter()
            .map(|r| ListTensionJson {
                id: r.id.clone(),
                desired: r.desired.clone(),
                actual: r.actual.clone(),
                status: r.status.to_string(),
                phase: r.phase_str.clone(),
                movement: r.movement_str.clone(),
                urgency: r.urgency,
                magnitude: r.magnitude,
                horizon: r.horizon_raw.clone(),
                tier: r.tier.clone(),
                neglected: r.neglected,
            })
            .collect();

        let count = json_tensions.len();
        let result = ListJson {
            tensions: json_tensions,
            count,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        if rows.is_empty() {
            output
                .info("No tensions match the filter")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            return Ok(());
        }

        for row in &rows {
            let urgency_display = match row.urgency {
                Some(u) => format!("{:>3.0}%", u * 100.0),
                None => " \u{2014} ".to_string(),
            };
            println!(
                "[{}] {}  {:<30}  {:>8}  {}",
                row.phase_str,
                row.movement_str,
                truncate(&row.desired, 30),
                row.horizon_display,
                urgency_display,
            );
        }

        println!();
        println!("{} tension(s)", rows.len());
    }

    Ok(())
}
