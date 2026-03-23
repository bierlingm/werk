//! Compose-up command handler.
//!
//! Creates a parent tension for one or more existing tensions.
//! The inverse of decomposing — reveals implicit coherence by
//! composing existing structure upward.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

/// JSON output structure for compose-up command.
#[derive(Serialize)]
struct ComposeUpResult {
    parent_id: String,
    parent_desired: String,
    parent_actual: String,
    children: Vec<String>,
}

pub fn cmd_compose_up(
    output: &Output,
    desired: String,
    actual: String,
    children_ids: Vec<String>,
) -> Result<(), WerkError> {
    if children_ids.is_empty() {
        return Err(WerkError::InvalidInput(
            "at least one child tension ID is required".to_string(),
        ));
    }

    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());

    // Resolve all child IDs
    let mut resolved_children = Vec::new();
    for child_prefix in &children_ids {
        let child = resolver.resolve(child_prefix)?;
        resolved_children.push(child.clone());
    }

    // All children must share the same parent (or all be roots)
    let first_parent = resolved_children[0].parent_id.clone();
    for child in &resolved_children {
        if child.parent_id != first_parent {
            return Err(WerkError::InvalidInput(
                "all children must share the same parent (or all be roots)".to_string(),
            ));
        }
    }

    // Check no duplicates
    let mut seen = std::collections::HashSet::new();
    for child in &resolved_children {
        if !seen.insert(&child.id) {
            return Err(WerkError::InvalidInput(format!(
                "duplicate child ID: {}",
                werk_shared::display_id(child.short_code, &child.id)
            )));
        }
    }

    // The new parent goes where the children currently sit (under first_parent, or root)
    let _ = store.begin_gesture(Some("compose up"));

    // Create the new parent tension under the same parent the children had
    let new_parent = store
        .create_tension_full(&desired, &actual, first_parent.clone(), None)
        .map_err(WerkError::SdError)?;

    // Reparent each child under the new parent
    for child in &resolved_children {
        store
            .update_parent(&child.id, Some(&new_parent.id))
            .map_err(WerkError::SdError)?;
    }

    store.end_gesture();

    let child_displays: Vec<String> = resolved_children
        .iter()
        .map(|c| werk_shared::display_id(c.short_code, &c.id))
        .collect();

    let result = ComposeUpResult {
        parent_id: new_parent.id.clone(),
        parent_desired: new_parent.desired.clone(),
        parent_actual: new_parent.actual.clone(),
        children: resolved_children.iter().map(|c| c.id.clone()).collect(),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!(
                "Composed up: created {} as parent of {}",
                werk_shared::display_id(new_parent.short_code, &new_parent.id),
                child_displays.join(", ")
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Desired: {}", &new_parent.desired);
        println!("  Reality: {}", &new_parent.actual);
        if let Some(pid) = &first_parent {
            let parent_display = tensions
                .iter()
                .find(|t| &t.id == pid)
                .map(|t| werk_shared::display_id(t.short_code, &t.id))
                .unwrap_or_else(|| pid[..8.min(pid.len())].to_string());
            println!("  Under:   {}", parent_display);
        }
    }

    Ok(())
}
