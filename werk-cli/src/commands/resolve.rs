//! Resolve command handler.

use crate::error::WerkError;
use crate::mutation_echo;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use werk_shared::HookEvent;

/// JSON output structure for resolve command.
#[derive(Serialize)]
struct ResolveResult {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_at: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    dry_run: bool,
}

pub fn cmd_resolve(
    output: &Output,
    id: String,
    actual_at: Option<String>,
    dry_run: bool,
    show_after: bool,
) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let (mut store, hook_handle) = workspace.open_store_with_hooks()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already resolved
    if old_status != werk_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot resolve tension with status {} (must be Active)",
            old_status
        )));
    }

    // Parse actual_at if provided
    let actual_at_dt = match &actual_at {
        Some(s) => Some(parse_actual_at(s)?),
        None => None,
    };

    // Dry run: validate and preview without mutating
    if dry_run {
        let result = ResolveResult {
            id: tension.id.clone(),
            status: "Resolved".to_string(),
            actual_at: actual_at.clone(),
            dry_run: true,
        };
        if output.is_structured() {
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            println!(
                "Would resolve tension {}",
                werk_shared::display_id(tension.short_code, &tension.id)
            );
            println!("  Status: {} -> Resolved", old_status);
            if let Some(at) = &actual_at {
                println!("  Actually done: {}", at);
            }
            println!("No changes made.");
        }
        return Ok(());
    }

    // Pre-hook check
    let event = HookEvent::status_change(
        &tension.id,
        &tension.desired,
        Some(&tension.actual),
        tension.parent_id.as_deref(),
        "Resolved",
    );
    if !hook_handle.runner.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Begin gesture for this resolve action
    let _ = store.begin_gesture(Some(&format!("resolve {}", &tension.id)));

    // Set actual_at if provided (when the resolution actually happened)
    if let Some(dt) = actual_at_dt {
        store.set_actual_at(dt);
    }

    // Update status via store (handles validation and mutation recording)
    store
        .update_status(&tension.id, werk_core::TensionStatus::Resolved)
        .map_err(WerkError::CoreError)?;

    store.clear_actual_at();
    store.end_gesture();
    // Post-hooks fire automatically via the HookBridge

    let result = ResolveResult {
        id: tension.id.clone(),
        status: "Resolved".to_string(),
        actual_at: actual_at.clone(),
        dry_run: false,
    };

    if output.is_structured() {
        let mut val =
            serde_json::to_value(&result).map_err(|e| WerkError::IoError(e.to_string()))?;
        if show_after && !dry_run {
            val["show"] = mutation_echo::build_json_echo(&store, &tension.id)?;
        }
        let json =
            serde_json::to_string_pretty(&val).map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("{}", json);
    } else {
        output
            .success(&format!(
                "Resolved tension {}",
                werk_shared::display_id(tension.short_code, &tension.id)
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Status: {} -> Resolved", old_status);
        if let Some(at) = &actual_at {
            println!("  Actually done: {}", at);
        }
        // Dry-run never mutates the store, so skip the echo path —
        // there's nothing new to show.
        if !dry_run {
            mutation_echo::print_human_echo(&store, &output.palette(), &tension.id)?;
        }
    }

    Ok(())
}

/// Parse a human-friendly actual_at value into a DateTime<Utc>.
///
/// Supported: "yesterday", "N days ago", "YYYY-MM-DD"
fn parse_actual_at(value: &str) -> Result<DateTime<Utc>, WerkError> {
    let v = value.trim().to_lowercase();
    let now = Utc::now();

    if v == "yesterday" {
        let yesterday = now - chrono::Duration::days(1);
        return Ok(yesterday);
    }

    if let Some(rest) = v.strip_suffix(" days ago") {
        let n: i64 = rest.trim().parse().map_err(|_| {
            WerkError::InvalidInput(format!(
                "invalid number in '{}': expected 'N days ago'",
                value
            ))
        })?;
        return Ok(now - chrono::Duration::days(n));
    }

    // Try ISO date
    if let Ok(date) = NaiveDate::parse_from_str(&v, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(12, 0, 0).unwrap().and_utc()); // ubs:ignore 12:00:00 is always valid
    }

    Err(WerkError::InvalidInput(format!(
        "cannot parse '{}' as a date. Try: 'yesterday', '3 days ago', or '2026-03-20'",
        value
    )))
}
