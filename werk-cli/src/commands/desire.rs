//! Desire command handler.
//!
//! Desire updates are epoch boundaries. Before applying the new desire,
//! the current state (desire, reality, children) is snapshotted as an epoch —
//! the delta that's ending. This builds the ghost geometry through normal use.
//!
//! Use --no-epoch for minor corrections that don't warrant a new delta.

use crate::error::WerkError;
use crate::mutation_echo;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::HookEvent;

/// JSON output structure for desire command.
#[derive(Serialize)]
struct DesireResult {
    id: String,
    desired: String,
    old_desired: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    epoch_id: Option<String>,
}

pub fn cmd_desire(
    output: &Output,
    id: String,
    value: Option<String>,
    no_epoch: bool,
    show_after: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let (mut store, hook_handle) = workspace.open_store_with_hooks()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);
    let tension = resolver.resolve(&id)?;

    let new_value = match value {
        Some(v) => v,
        None => {
            if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                return Err(WerkError::InvalidInput(
                    "value required in non-interactive mode (no TTY). Usage:\n  werk desire <id> \"new desired state\"".to_string(),
                ));
            }
            let edited = crate::edit_content(&tension.desired)?;
            match edited {
                Some(v) => v,
                None => {
                    if output.is_structured() {
                        let result = DesireResult {
                            id: tension.id.clone(),
                            desired: tension.desired.clone(),
                            old_desired: tension.desired.clone(),
                            epoch_id: None,
                        };
                        output
                            .print_structured(&result)
                            .map_err(WerkError::IoError)?;
                    } else {
                        output
                            .info("No changes made (editor cancelled or content unchanged)")
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    return Ok(());
                }
            }
        }
    };

    if new_value.is_empty() {
        return Err(WerkError::InvalidInput(
            "desired state cannot be empty".to_string(),
        ));
    }

    let old_desired = tension.desired.clone();

    let event = HookEvent::mutation(
        &tension.id,
        &tension.desired,
        Some(&tension.actual),
        tension.parent_id.as_deref(),
        "desired",
        Some(&old_desired),
        &new_value,
    );
    if !hook_handle.runner.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Begin gesture — encompasses both epoch snapshot and desire update
    let _ = store.begin_gesture(Some(&format!("update desire {}", &tension.id)));

    // Epoch: snapshot the ending delta BEFORE applying the update
    let epoch_id = if !no_epoch {
        let children = store
            .get_children(&tension.id)
            .map_err(WerkError::StoreError)?;
        let children_snapshot: Vec<serde_json::Value> = children
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "desired": c.desired,
                    "actual": c.actual,
                    "status": c.status.to_string(),
                    "position": c.position,
                })
            })
            .collect();
        let children_json = serde_json::to_string(&children_snapshot)
            .map_err(|e| WerkError::IoError(e.to_string()))?;

        let eid = store
            .create_epoch(
                &tension.id,
                &old_desired,
                &tension.actual,
                Some(&children_json),
                store.active_gesture().as_deref(),
            )
            .map_err(WerkError::StoreError)?;
        Some(eid)
    } else {
        None
    };

    // Apply the desire update
    store
        .update_desired(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;
    store.end_gesture();
    // Post-hooks fire automatically via the HookBridge

    let tension_display =
        werk_shared::display_id(tension.short_code, &tension.id);

    let result = DesireResult {
        id: tension.id.clone(),
        desired: new_value,
        old_desired,
        epoch_id: epoch_id.clone(),
    };

    if output.is_structured() {
        let mut val = serde_json::to_value(&result)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        if show_after {
            val["show"] = mutation_echo::build_json_echo(&store, &tension.id)?;
        }
        let json = serde_json::to_string_pretty(&val)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("{}", json);
    } else {
        output
            .success(&format!(
                "Updated desired for tension {}",
                tension_display
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Old:  {}", &result.old_desired);
        println!("  New:  {}", &result.desired);
        if epoch_id.is_some() {
            let epoch_count = store
                .get_epochs(&tension.id)
                .map_err(WerkError::StoreError)?
                .len();
            println!("  Epoch {} recorded (epoch boundary)", epoch_count);
        }
        mutation_echo::print_human_echo(&store, &output.palette(), &tension.id)?;
    }

    Ok(())
}
