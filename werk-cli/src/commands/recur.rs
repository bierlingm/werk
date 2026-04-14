//! Recur command handler.
//!
//! Set or clear a recurrence interval on a tension.
//! Usage:
//!   werk recur <id> +1d    — recur daily
//!   werk recur <id> +1w    — recur weekly
//!   werk recur <id> +2w    — recur biweekly
//!   werk recur <id> +1m    — recur monthly
//!   werk recur <id> --clear — remove recurrence

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use werk_core::Mutation;
use serde::Serialize;
use werk_shared::HookEvent;

/// JSON output structure for recur command.
#[derive(Serialize)]
struct RecurResult {
    id: String,
    recurrence: Option<String>,
    cleared: bool,
}

/// Validate a recurrence interval string.
/// Accepts formats like +1d, +1w, +2w, +1m, +3m, etc.
fn validate_interval(input: &str) -> Result<String, WerkError> {
    let rest = input.strip_prefix('+').ok_or_else(|| {
        WerkError::InvalidInput(format!(
            "invalid interval '{}': expected format like +1d, +1w, +1m",
            input
        ))
    })?;

    if rest.len() < 2 {
        return Err(WerkError::InvalidInput(format!(
            "invalid interval '{}': expected format like +1d, +1w, +1m",
            input
        )));
    }

    let (num_str, unit) = rest.split_at(rest.len() - 1);
    let _n: u32 = num_str.parse().map_err(|_| {
        WerkError::InvalidInput(format!(
            "invalid interval '{}': number part '{}' is not valid",
            input, num_str
        ))
    })?;

    match unit {
        "d" | "w" | "m" => {}
        _ => {
            return Err(WerkError::InvalidInput(format!(
                "invalid interval unit '{}': expected d (days), w (weeks), or m (months)",
                unit
            )));
        }
    }

    // Store the canonical form (without +)
    Ok(input.to_string())
}

pub fn cmd_recur(
    output: &Output,
    id: String,
    interval: Option<String>,
    clear: bool,
) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let (store, hook_handle) = workspace.open_store_with_hooks()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    if clear {
        let event = HookEvent::mutation(
            &tension.id,
            &tension.desired,
            Some(&tension.actual),
            tension.parent_id.as_deref(),
            "recurrence",
            None,
            "none",
        );
        if !hook_handle.runner.pre_mutation(&event) {
            return Err(WerkError::InvalidInput(
                "Blocked by pre_mutation hook".to_string(),
            ));
        }

        store
            .record_mutation(&Mutation::new(
                tension.id.clone(),
                Utc::now(),
                "recurrence".to_owned(),
                None,
                "none".to_owned(),
            ))
            .map_err(WerkError::CoreError)?;

        let result = RecurResult {
            id: tension.id.clone(),
            recurrence: None,
            cleared: true,
        };

        if output.is_structured() {
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .success(&format!("Cleared recurrence for tension {}", &tension.id))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
    } else {
        let interval_str = interval.ok_or_else(|| {
            WerkError::InvalidInput(
                "recurrence interval required (e.g., +1d, +1w, +1m); use --clear to remove"
                    .into(),
            )
        })?;
        let validated = validate_interval(&interval_str)?;

        let event = HookEvent::mutation(
            &tension.id,
            &tension.desired,
            Some(&tension.actual),
            tension.parent_id.as_deref(),
            "recurrence",
            None,
            &validated,
        );
        if !hook_handle.runner.pre_mutation(&event) {
            return Err(WerkError::InvalidInput(
                "Blocked by pre_mutation hook".to_string(),
            ));
        }

        store
            .record_mutation(&Mutation::new(
                tension.id.clone(),
                Utc::now(),
                "recurrence".to_owned(),
                None,
                validated.clone(),
            ))
            .map_err(WerkError::CoreError)?;

        let result = RecurResult {
            id: tension.id.clone(),
            recurrence: Some(validated.clone()),
            cleared: false,
        };

        if output.is_structured() {
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .success(&format!(
                    "Set recurrence for tension {} to {}",
                    &tension.id, &validated
                ))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
    }

    Ok(())
}
