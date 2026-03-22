//! List command handler.
//!
//! Flat listing of tensions with rich filtering and sorting options.

use chrono::Utc;
use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{compute_urgency, TensionStatus};
use werk_shared::truncate;

/// JSON output structure for a tension in list.
#[derive(Serialize)]
struct ListTensionJson {
    id: String,
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: String,
    urgency: Option<f64>,
    horizon: Option<String>,
    overdue: bool,
    tier: String,
}

/// JSON output structure for list.
#[derive(Serialize)]
struct ListJson {
    tensions: Vec<ListTensionJson>,
    count: usize,
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
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: TensionStatus,
    urgency: Option<f64>,
    horizon_display: String,
    horizon_raw: Option<String>,
    overdue: bool,
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
    let now = Utc::now();

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

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

    // Build rows
    let mut rows: Vec<TensionRow> = Vec::new();

    for tension in &tensions {
        let urgency_val = compute_urgency(tension, now).map(|u| u.value);
        let horizon_display = format_horizon(tension, now);

        let overdue = tension.status == TensionStatus::Active
            && tension
                .horizon
                .as_ref()
                .map(|h| h.is_past(now))
                .unwrap_or(false);

        let tier = if tension.status == TensionStatus::Resolved
            || tension.status == TensionStatus::Released
        {
            "resolved"
        } else if overdue || urgency_val.map(|u| u > 0.75).unwrap_or(false) {
            "urgent"
        } else {
            "active"
        };

        rows.push(TensionRow {
            id: tension.id.clone(),
            short_code: tension.short_code,
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            status: tension.status,
            urgency: urgency_val,
            horizon_display,
            horizon_raw: tension.horizon.as_ref().map(|h| h.to_string()),
            overdue,
            tier: tier.to_string(),
        });
    }

    // Apply filters
    if !all {
        rows.retain(|r| r.status == TensionStatus::Active);
    }

    if urgent {
        rows.retain(|r| r.tier == "urgent");
    }

    // --neglected and --stagnant still filter but no longer depend on old dynamics
    // They now filter on overdue (as a proxy for neglect/stagnation)
    if neglected || stagnant {
        rows.retain(|r| r.overdue);
    }

    if let Some(ref _phase_filter) = phase {
        // Phase filtering removed — old dynamics phases are not honest.
        // This is a no-op until phase computation is rebuilt.
    }

    // Sort
    match sort.as_str() {
        "urgency" => {
            rows.sort_by(|a, b| {
                let ua = a.urgency.unwrap_or(-1.0);
                let ub = b.urgency.unwrap_or(-1.0);
                ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        "name" => {
            rows.sort_by(|a, b| a.desired.to_lowercase().cmp(&b.desired.to_lowercase()));
        }
        "horizon" => {
            rows.sort_by(|a, b| match (&a.horizon_raw, &b.horizon_raw) {
                (Some(ha), Some(hb)) => ha.cmp(hb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
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
                short_code: r.short_code,
                desired: r.desired.clone(),
                actual: r.actual.clone(),
                status: r.status.to_string(),
                urgency: r.urgency,
                horizon: r.horizon_raw.clone(),
                overdue: r.overdue,
                tier: r.tier.clone(),
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
            let id_display = match row.short_code {
                Some(c) => format!("#{:<4}", c),
                None => format!("{:<8}", &row.id[..8.min(row.id.len())]),
            };

            let overdue_marker = if row.overdue { " OVERDUE" } else { "" };

            let urgency_display = match row.urgency {
                Some(u) => format!("{:>3.0}%", u * 100.0),
                None => " \u{2014} ".to_string(),
            };

            println!(
                "{}  {:<30}  {:>8}{}  {}",
                id_display,
                truncate(&row.desired, 30),
                row.horizon_display,
                overdue_marker,
                urgency_display,
            );
        }

        println!();
        println!("{} tension(s)", rows.len());
    }

    Ok(())
}
