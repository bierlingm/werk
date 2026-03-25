//! Snooze command handler.
//!
//! Hide a tension until a future date.
//! Usage:
//!   werk snooze <id> +3d    — snooze until 3 days from now
//!   werk snooze <id> +2w    — snooze until 2 weeks from now
//!   werk snooze <id> +1m    — snooze until 1 month from now
//!   werk snooze <id> 2026-04-01  — snooze until a specific date
//!   werk snooze <id> --clear — remove snooze

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::{Duration, NaiveDate, Utc};
use sd_core::Mutation;
use serde::Serialize;
use werk_shared::{Config, HookEvent, HookRunner};

/// JSON output structure for snooze command.
#[derive(Serialize)]
struct SnoozeResult {
    id: String,
    snoozed_until: Option<String>,
    cleared: bool,
}

/// Parse a date string that can be:
/// - Relative: +3d, +2w, +1m (days, weeks, months)
/// - Absolute ISO: 2026-04-01
fn parse_snooze_date(input: &str) -> Result<NaiveDate, WerkError> {
    // Try relative format: +Nd, +Nw, +Nm
    if let Some(rest) = input.strip_prefix('+') {
        let (num_str, unit) = rest.split_at(rest.len().saturating_sub(1));
        let n: i64 = num_str.parse().map_err(|_| {
            WerkError::InvalidInput(format!(
                "invalid relative date '{}': expected format like +3d, +2w, +1m",
                input
            ))
        })?;

        let today = Utc::now().date_naive();
        let target = match unit {
            "d" => today + Duration::days(n),
            "w" => today + Duration::weeks(n),
            "m" => {
                // Approximate month as 30 days
                today + Duration::days(n * 30)
            }
            _ => {
                return Err(WerkError::InvalidInput(format!(
                    "invalid date unit '{}': expected d (days), w (weeks), or m (months)",
                    unit
                )));
            }
        };
        return Ok(target);
    }

    // Try ISO date: YYYY-MM-DD
    NaiveDate::parse_from_str(input, "%Y-%m-%d").map_err(|_| {
        WerkError::InvalidInput(format!(
            "invalid date '{}': expected +Nd, +Nw, +Nm, or YYYY-MM-DD",
            input
        ))
    })
}

pub fn cmd_snooze(
    output: &Output,
    id: String,
    date: Option<String>,
    clear: bool,
) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Hook infrastructure
    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());

    if clear {
        let event = HookEvent::mutation(
            &tension.id,
            &tension.desired,
            Some(&tension.actual),
            tension.parent_id.as_deref(),
            "snoozed_until",
            None,
            "cleared",
        );
        if !hooks.pre_mutation(&event) {
            return Err(WerkError::InvalidInput(
                "Blocked by pre_mutation hook".to_string(),
            ));
        }

        store
            .record_mutation(&Mutation::new(
                tension.id.clone(),
                Utc::now(),
                "snoozed_until".to_owned(),
                None,
                "cleared".to_owned(),
            ))
            .map_err(WerkError::SdError)?;

        hooks.post_mutation(&event);

        let result = SnoozeResult {
            id: tension.id.clone(),
            snoozed_until: None,
            cleared: true,
        };

        if output.is_structured() {
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .success(&format!("Unsnoozed tension {}", &tension.id))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
    } else {
        let date_str =
            date.ok_or_else(|| WerkError::InvalidInput("snooze date required (e.g., +3d, +2w, +1m, or YYYY-MM-DD); use --clear to remove snooze".into()))?;
        let parsed = parse_snooze_date(&date_str)?;
        let formatted = parsed.format("%Y-%m-%d").to_string();

        let event = HookEvent::mutation(
            &tension.id,
            &tension.desired,
            Some(&tension.actual),
            tension.parent_id.as_deref(),
            "snoozed_until",
            None,
            &formatted,
        );
        if !hooks.pre_mutation(&event) {
            return Err(WerkError::InvalidInput(
                "Blocked by pre_mutation hook".to_string(),
            ));
        }

        store
            .record_mutation(&Mutation::new(
                tension.id.clone(),
                Utc::now(),
                "snoozed_until".to_owned(),
                None,
                formatted.clone(),
            ))
            .map_err(WerkError::SdError)?;

        hooks.post_mutation(&event);

        let result = SnoozeResult {
            id: tension.id.clone(),
            snoozed_until: Some(formatted.clone()),
            cleared: false,
        };

        if output.is_structured() {
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .success(&format!(
                    "Snoozed tension {} until {}",
                    &tension.id, &formatted
                ))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
    }

    Ok(())
}
