//! Health command handler.
//!
//! System health summary: closure progress, urgency distribution, alerts.

use chrono::Utc;
use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{compute_urgency, TensionStatus};

/// JSON output structure for health.
#[derive(Serialize)]
struct HealthJson {
    active_count: usize,
    with_children: usize,
    leaf_count: usize,
    closure: ClosureStats,
    alerts: Alerts,
}

#[derive(Serialize)]
struct ClosureStats {
    total_children: usize,
    resolved_children: usize,
}

#[derive(Serialize)]
struct Alerts {
    urgent: usize,
    overdue: usize,
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
    let now = Utc::now();

    let tensions = store
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    // Filter active
    let active: Vec<_> = tensions
        .iter()
        .filter(|t| t.status != TensionStatus::Resolved && t.status != TensionStatus::Released)
        .collect();
    let total = active.len();

    // Closure stats
    let mut with_children = 0usize;
    let mut leaf_count = 0usize;
    let mut total_children = 0usize;
    let mut resolved_children = 0usize;

    for t in &active {
        let children: Vec<_> = tensions
            .iter()
            .filter(|c| c.parent_id.as_deref() == Some(&t.id))
            .collect();
        if children.is_empty() {
            leaf_count += 1;
        } else {
            with_children += 1;
            total_children += children.len();
            resolved_children += children
                .iter()
                .filter(|c| c.status == TensionStatus::Resolved)
                .count();
        }
    }

    // Alerts
    let mut urgent = 0usize;
    let mut overdue = 0usize;

    for t in &active {
        if let Some(u) = compute_urgency(t, now) {
            if u.value > 1.0 {
                overdue += 1;
            } else if u.value > 0.75 {
                urgent += 1;
            }
        }
    }

    if output.is_structured() {
        let result = HealthJson {
            active_count: total,
            with_children,
            leaf_count,
            closure: ClosureStats {
                total_children,
                resolved_children,
            },
            alerts: Alerts { urgent, overdue },
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let bar_width = 10;

        println!("System Health ({} active tensions)", total);
        println!();
        println!("Structure:");
        println!(
            "  With children  {}  {}",
            bar(with_children, total, bar_width),
            with_children,
        );
        println!(
            "  Leaf tensions  {}  {}",
            bar(leaf_count, total, bar_width),
            leaf_count,
        );
        if total_children > 0 {
            println!();
            println!(
                "Closure: {}/{} children resolved across {} parents",
                resolved_children, total_children, with_children
            );
        }

        if urgent > 0 || overdue > 0 {
            println!();
            println!("Alerts:");
            if overdue > 0 {
                println!("  ! {} overdue tension(s)", overdue);
            }
            if urgent > 0 {
                println!("  ! {} urgent tension(s) (>75% of horizon elapsed)", urgent);
            }
        }
    }

    Ok(())
}
