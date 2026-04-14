//! Horizon command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::palette;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use werk_core::{compute_urgency, Horizon, HorizonKind, TensionStatus};
use serde::Serialize;
use werk_shared::HookEvent;

/// JSON output structure for horizon set.
#[derive(Serialize)]
struct HorizonResult {
    id: String,
    horizon: Option<String>,
    old_horizon: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signals: Vec<palette::Palette>,
}

/// JSON output structure for horizon display.
#[derive(Serialize)]
struct HorizonDisplayResult {
    id: String,
    horizon: Option<String>,
    urgency: Option<f64>,
    days_remaining: Option<i64>,
}

pub fn cmd_horizon(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let (mut store, hook_handle) = workspace.open_store_with_hooks()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    match value {
        Some(new_value) => {
            // Set or clear horizon
            let horizon_parsed = if new_value.to_lowercase() == "none" {
                None
            } else {
                Some(Horizon::parse(&new_value).map_err(|e| {
                    WerkError::InvalidInput(format!(
                        "Invalid deadline '{}': {}. Examples: 2026, 2026-05, 2026-05-15, 2026-05-15T14:00:00Z",
                        new_value, e
                    ))
                })?)
            };

            // Check status - only Active tensions can have horizon updated
            if tension.status != TensionStatus::Active {
                return Err(WerkError::InvalidInput(format!(
                    "cannot update deadline on {} tension (must be Active)",
                    tension.status
                )));
            }

            // Record old horizon
            let old_horizon = tension.horizon.as_ref().map(|h| h.to_string());

            // Pre-hook check
            let new_horizon_str = horizon_parsed.as_ref().map(|h| h.to_string()).unwrap_or_else(|| "none".to_string());
            let event = HookEvent::mutation(
                &tension.id,
                &tension.desired,
                Some(&tension.actual),
                tension.parent_id.as_deref(),
                "horizon",
                old_horizon.as_deref(),
                &new_horizon_str,
            );
            if !hook_handle.runner.pre_mutation(&event) {
                return Err(WerkError::InvalidInput("Blocked by pre_mutation hook".to_string()));
            }

            // Update horizon
            let _ = store.begin_gesture(Some(&format!("update horizon {}", &tension.id)));
            store
                .update_horizon(&tension.id, horizon_parsed.clone())
                .map_err(WerkError::CoreError)?;
            store.end_gesture();
            // Post-hooks fire automatically via the HookBridge

            // Print success message before palette (human mode)
            if !output.is_structured() {
                match &horizon_parsed {
                    Some(h) => {
                        output
                            .success(&format!(
                                "Set deadline for tension {} to {}",
                                werk_shared::display_id(tension.short_code, &tension.id), h
                            ))
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    None => {
                        output
                            .success(&format!("Cleared deadline for tension {}", werk_shared::display_id(tension.short_code, &tension.id)))
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                }
            }

            // Pathway palette: detect containment violations after horizon change
            let signals = if horizon_parsed.is_some() {
                palette::check_containment_after_horizon(output, &mut store, &tension.id)?
            } else {
                vec![]
            };

            if output.is_structured() {
                let result = HorizonResult {
                    id: tension.id.clone(),
                    horizon: horizon_parsed.as_ref().map(|h| h.to_string()),
                    old_horizon,
                    signals,
                };
                output
                    .print_structured(&result)
                    .map_err(WerkError::IoError)?;
            }

            Ok(())
        }
        None => {
            // Display current horizon with urgency
            let now = Utc::now();
            let urgency = compute_urgency(tension, now);

            let days_remaining = tension.horizon.as_ref().and_then(|h| {
                let remaining = h.range_end().signed_duration_since(now).num_days();
                if remaining >= 0 {
                    Some(remaining)
                } else {
                    None
                }
            });

            let result = HorizonDisplayResult {
                id: tension.id.clone(),
                horizon: tension.horizon.as_ref().map(|h| h.to_string()),
                urgency: urgency.as_ref().map(|u| u.value),
                days_remaining,
            };

            if output.is_structured() {
                output
                    .print_structured(&result)
                    .map_err(WerkError::IoError)?;
            } else {
                println!("Tension {}", werk_shared::display_id(tension.short_code, &tension.id));

                match &tension.horizon {
                    Some(h) => {
                        println!("  Deadline: {}", h);

                        // Human interpretation
                        let interpretation = match h.kind() {
                            HorizonKind::Year(y) => format!("Year {}", y),
                            HorizonKind::Month(y, m) => format!("{}-{:02}", y, m),
                            HorizonKind::Day(d) => d.format("%Y-%m-%d").to_string(),
                            HorizonKind::DateTime(_) => h.to_string(),
                        };
                        println!("  Interpreted: {}", &interpretation);

                        if let Some(urg) = &urgency {
                            let urgency_pct = (urg.value * 100.0).min(999.0);
                            println!("  Urgency:    {:.0}% of time window elapsed", urgency_pct);
                        }

                        if let Some(days) = days_remaining {
                            println!("  Days remaining: {}", days);
                        }
                    }
                    None => {
                        println!("  Deadline:   None");
                    }
                }
            }

            Ok(())
        }
    }
}
