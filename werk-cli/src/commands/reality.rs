//! Reality command handler.
//!
//! Reality updates are epoch boundaries. Before applying the new reality,
//! the current state (desire, reality, children) is snapshotted as an epoch —
//! the delta that's ending. This builds the ghost geometry through normal use.
//!
//! Use --no-epoch for minor corrections that don't warrant a new delta.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::{Config, HookEvent, HookRunner};

/// JSON output structure for reality command.
#[derive(Serialize)]
struct RealityResult {
    id: String,
    actual: String,
    old_actual: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    epoch_id: Option<String>,
}

pub fn cmd_reality(
    output: &Output,
    id: String,
    value: Option<String>,
    no_epoch: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);
    let tension = resolver.resolve(&id)?;

    // Get the new value - either from argument or editor
    let new_value = match value {
        Some(v) => v,
        None => {
            let edited = crate::edit_content(&tension.actual)?;
            match edited {
                Some(v) => v,
                None => {
                    if output.is_structured() {
                        let result = RealityResult {
                            id: tension.id.clone(),
                            actual: tension.actual.clone(),
                            old_actual: tension.actual.clone(),
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
            "actual state cannot be empty".to_string(),
        ));
    }

    let old_actual = tension.actual.clone();

    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());
    let event = HookEvent::mutation(
        &tension.id,
        &tension.desired,
        "actual",
        Some(&old_actual),
        &new_value,
    );
    if !hooks.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Begin gesture — encompasses both epoch snapshot and reality update
    let _ = store.begin_gesture(Some(&format!("update reality {}", &tension.id)));

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
                &tension.desired,
                &old_actual,
                Some(&children_json),
                store.active_gesture().as_deref(),
            )
            .map_err(WerkError::StoreError)?;
        Some(eid)
    } else {
        None
    };

    // Apply the reality update
    store
        .update_actual(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    hooks.post_mutation(&event);

    let tension_display =
        werk_shared::display_id(tension.short_code, &tension.id);

    let result = RealityResult {
        id: tension.id.clone(),
        actual: new_value,
        old_actual,
        epoch_id: epoch_id.clone(),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!("Updated reality for tension {}", tension_display))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Old:  {}", &result.old_actual);
        println!("  New:  {}", &result.actual);
        if epoch_id.is_some() {
            let epoch_count = store
                .get_epochs(&tension.id)
                .map_err(WerkError::StoreError)?
                .len();
            println!("  Epoch {} recorded (epoch boundary)", epoch_count);
        }
    }

    Ok(())
}
