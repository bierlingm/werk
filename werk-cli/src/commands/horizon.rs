//! Horizon command handler.

use crate::error::WerkError;
use crate::output::{ColorStyle, Output};
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{compute_urgency, Horizon, HorizonKind, TensionStatus};
use serde::Serialize;

/// JSON output structure for horizon set.
#[derive(Serialize)]
struct HorizonResult {
    id: String,
    horizon: Option<String>,
    old_horizon: Option<String>,
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
    let store = workspace.open_store()?;

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
                        "Invalid horizon '{}': {}. Examples: 2026, 2026-05, 2026-05-15, 2026-05-15T14:00:00Z",
                        new_value, e
                    ))
                })?)
            };

            // Check status - only Active tensions can have horizon updated
            if tension.status != TensionStatus::Active {
                return Err(WerkError::InvalidInput(format!(
                    "cannot update horizon on {} tension (must be Active)",
                    tension.status
                )));
            }

            // Record old horizon
            let old_horizon = tension.horizon.as_ref().map(|h| h.to_string());

            // Update horizon
            store
                .update_horizon(&tension.id, horizon_parsed.clone())
                .map_err(WerkError::SdError)?;

            let result = HorizonResult {
                id: tension.id.clone(),
                horizon: horizon_parsed.as_ref().map(|h| h.to_string()),
                old_horizon,
            };

            if output.is_structured() {
                output
                    .print_structured(&result)
                    .map_err(WerkError::IoError)?;
            } else {
                let id_styled = output.styled(&tension.id, ColorStyle::Id);
                match &horizon_parsed {
                    Some(h) => {
                        output
                            .success(&format!(
                                "Set horizon for tension {} to {}",
                                id_styled,
                                output.styled(&h.to_string(), ColorStyle::Highlight)
                            ))
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    None => {
                        output
                            .success(&format!("Cleared horizon for tension {}", id_styled))
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                }
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
                let id_styled = output.styled(&tension.id, ColorStyle::Id);
                println!("Tension {}", id_styled);

                match &tension.horizon {
                    Some(h) => {
                        println!(
                            "  Horizon: {}",
                            output.styled(&h.to_string(), ColorStyle::Highlight)
                        );

                        // Human interpretation
                        let interpretation = match h.kind() {
                            HorizonKind::Year(y) => format!("Year {}", y),
                            HorizonKind::Month(y, m) => format!("{}-{:02}", y, m),
                            HorizonKind::Day(d) => d.format("%Y-%m-%d").to_string(),
                            HorizonKind::DateTime(_) => h.to_string(),
                        };
                        println!(
                            "  Interpreted: {}",
                            output.styled(&interpretation, ColorStyle::Muted)
                        );

                        if let Some(urg) = &urgency {
                            let urgency_pct = (urg.value * 100.0).min(999.0);
                            println!("  Urgency:    {:.0}% of time window elapsed", urgency_pct);
                        }

                        if let Some(days) = days_remaining {
                            println!(
                                "  Days remaining: {}",
                                output.styled(&format!("{}", days), ColorStyle::Highlight)
                            );
                        }
                    }
                    None => {
                        println!("  Horizon:    {}", output.styled("None", ColorStyle::Muted));
                    }
                }
            }

            Ok(())
        }
    }
}
