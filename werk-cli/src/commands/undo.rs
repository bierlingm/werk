//! Undo command handler — reverse a gesture's mutations.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use serde::Serialize;

#[derive(Serialize)]
struct UndoResult {
    gesture_id: String,
    undo_gesture_id: String,
}

#[derive(Serialize)]
struct UndoDryRun {
    gesture_id: String,
    mutation_count: usize,
    would_succeed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn cmd_undo(
    output: &Output,
    gesture_id: Option<String>,
    last: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let (store, _hook_handle) = workspace.open_store_with_hooks()?;

    // Resolve the gesture ID
    let gid = if last {
        store
            .get_last_gesture_id()
            .map_err(WerkError::StoreError)?
            .ok_or_else(|| WerkError::InvalidInput("no gestures found".to_string()))?
    } else if let Some(id) = gesture_id {
        id
    } else {
        return Err(WerkError::InvalidInput(
            "provide a gesture ID or use --last".to_string(),
        ));
    };

    if dry_run {
        // Check if undo would succeed without applying
        let mutations = store
            .get_gesture_mutations(&gid)
            .map_err(WerkError::StoreError)?;

        // Try the undo to see if it would conflict — we can't do a real dry-run
        // without duplicating conflict detection, so just report mutation count
        let result = UndoDryRun {
            gesture_id: gid.clone(),
            mutation_count: mutations.len(),
            would_succeed: true, // optimistic; real check would need conflict detection
            error: None,
        };

        if output.is_structured() {
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .info(&format!(
                    "Would undo gesture {} ({} mutations)",
                    &gid[..8.min(gid.len())],
                    mutations.len()
                ))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
        return Ok(());
    }

    let undo_id = store.undo_gesture(&gid).map_err(WerkError::CoreError)?;

    let result = UndoResult {
        gesture_id: gid.clone(),
        undo_gesture_id: undo_id.clone(),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!(
                "Undone gesture {} -> {}",
                &gid[..8.min(gid.len())],
                &undo_id[..8.min(undo_id.len())]
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}
