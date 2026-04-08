//! Survey command handler — the Napoleonic field survey.
//!
//! Navigate time, see structure. Shows all steps across all tensions
//! organized by temporal urgency, answering: where across the field
//! of opportunities should energy flow?

use chrono::Utc;
use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{compute_urgency, TensionStatus};
use werk_shared::truncate;

/// A step in the survey view, annotated with its structural context.
#[derive(Serialize)]
struct SurveyItem {
    id: String,
    short_code: Option<i32>,
    desired: String,
    parent_id: Option<String>,
    parent_short_code: Option<i32>,
    parent_desired: Option<String>,
    deadline: Option<String>,
    urgency: Option<f64>,
    position: Option<i32>,
    category: String,
}

#[derive(Serialize)]
struct SurveyJson {
    overdue: Vec<SurveyItem>,
    due_soon: Vec<SurveyItem>,
    active: Vec<SurveyItem>,
    held: Vec<SurveyItem>,
    recently_resolved: Vec<SurveyItem>,
}

pub fn cmd_survey(output: &Output, days: i64) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let now = Utc::now();

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    if tensions.is_empty() {
        if output.is_structured() {
            let result = SurveyJson {
                overdue: vec![],
                due_soon: vec![],
                active: vec![],
                held: vec![],
                recently_resolved: vec![],
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

    // Build a lookup for parent context
    let parent_lookup: std::collections::HashMap<String, (Option<i32>, String)> = tensions
        .iter()
        .map(|t| (t.id.clone(), (t.short_code, t.desired.clone())))
        .collect();

    let mut overdue = Vec::new();
    let mut due_soon = Vec::new();
    let mut active = Vec::new();
    let mut held = Vec::new();
    let mut recently_resolved = Vec::new();

    let frame_end = now + chrono::Duration::days(days);

    for tension in &tensions {
        let urgency_val = compute_urgency(tension, now).map(|u| u.value);

        let (parent_short_code, parent_desired) = tension
            .parent_id
            .as_ref()
            .and_then(|pid| parent_lookup.get(pid))
            .map(|(sc, d)| (*sc, Some(d.clone())))
            .unwrap_or((None, None));

        let item = SurveyItem {
            id: tension.id.clone(),
            short_code: tension.short_code,
            desired: tension.desired.clone(),
            parent_id: tension.parent_id.clone(),
            parent_short_code,
            parent_desired,
            deadline: tension.horizon.as_ref().map(|h| h.to_string()),
            urgency: urgency_val,
            position: tension.position,
            category: String::new(), // filled below
        };

        match tension.status {
            TensionStatus::Resolved | TensionStatus::Released => {
                // Check if recently resolved (within frame)
                let mutations = store
                    .get_mutations(&tension.id)
                    .map_err(WerkError::StoreError)?;
                let resolved_recently = mutations.iter().any(|m| {
                    m.field() == "status"
                        && m.new_value().contains("Resolved")
                        && (now - m.timestamp()).num_days() <= days
                });
                if resolved_recently {
                    recently_resolved.push(SurveyItem {
                        category: "recently_resolved".to_string(),
                        ..item
                    });
                }
            }
            TensionStatus::Active => {
                let is_overdue = tension
                    .horizon
                    .as_ref()
                    .map(|h| h.is_past(now))
                    .unwrap_or(false);

                let is_due_soon = !is_overdue
                    && tension
                        .horizon
                        .as_ref()
                        .map(|h| h.range_end() <= frame_end)
                        .unwrap_or(false);

                let is_held = tension.position.is_none();

                if is_overdue {
                    overdue.push(SurveyItem {
                        category: "overdue".to_string(),
                        ..item
                    });
                } else if is_due_soon {
                    due_soon.push(SurveyItem {
                        category: "due_soon".to_string(),
                        ..item
                    });
                } else if is_held {
                    held.push(SurveyItem {
                        category: "held".to_string(),
                        ..item
                    });
                } else {
                    active.push(SurveyItem {
                        category: "active".to_string(),
                        ..item
                    });
                }
            }
        }
    }

    // Sort each category by urgency (most urgent first)
    let sort_by_urgency = |a: &SurveyItem, b: &SurveyItem| {
        let ua = a.urgency.unwrap_or(-1.0);
        let ub = b.urgency.unwrap_or(-1.0);
        ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
    };
    overdue.sort_by(sort_by_urgency);
    due_soon.sort_by(sort_by_urgency);

    if output.is_structured() {
        let result = SurveyJson {
            overdue,
            due_soon,
            active,
            held,
            recently_resolved,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let total = overdue.len() + due_soon.len() + active.len() + held.len();

        if total == 0 && recently_resolved.is_empty() {
            output
                .info("Field is clear — no active tensions")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            return Ok(());
        }

        if !overdue.is_empty() {
            println!("OVERDUE");
            for item in &overdue {
                print_survey_item(item);
            }
            println!();
        }

        if !due_soon.is_empty() {
            println!("Due within {} days", days);
            for item in &due_soon {
                print_survey_item(item);
            }
            println!();
        }

        if !active.is_empty() {
            println!("Active (positioned)");
            for item in &active {
                print_survey_item(item);
            }
            println!();
        }

        if !held.is_empty() {
            println!("Held across field");
            for item in &held {
                print_survey_item(item);
            }
            println!();
        }

        if !recently_resolved.is_empty() {
            println!("Recently resolved");
            for item in &recently_resolved {
                print_survey_item(item);
            }
            println!();
        }

        println!(
            "{} active  {} overdue  {} held  {} resolved",
            active.len() + due_soon.len(),
            overdue.len(),
            held.len(),
            recently_resolved.len()
        );
    }

    Ok(())
}

fn print_survey_item(item: &SurveyItem) {
    let id_display = match item.short_code {
        Some(c) => format!("#{}", c),
        None => item.id[..8.min(item.id.len())].to_string(),
    };

    let urgency_display = match item.urgency {
        Some(u) => format!("{:>3}%", (u * 100.0).round() as i32),
        None => "\u{2014}".to_string(),
    };

    let deadline_display = item
        .deadline
        .as_deref()
        .unwrap_or("\u{2014}");

    let parent_annotation = match (&item.parent_short_code, &item.parent_desired) {
        (Some(sc), Some(d)) => format!("  (#{} {})", sc, truncate(d, 25)),
        _ => String::new(),
    };

    println!(
        "  {:<6} {:>4}  {:<40} {:>10}{}",
        id_display,
        urgency_display,
        truncate(&item.desired, 40),
        deadline_display,
        parent_annotation
    );
}
