//! Hold command handler — remove a tension from the positioned sequence.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

#[derive(Serialize)]
struct HoldResult {
    id: String,
    previous_position: Option<i32>,
}

pub fn cmd_hold(output: &Output, id: String) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let (mut store, _hook_handle) = workspace.open_store_with_hooks()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);
    let tension = resolver.resolve(&id)?;

    let old_position = tension.position;

    let _ = store.begin_gesture(Some(&format!("hold {}", &tension.id)));
    store
        .update_position(&tension.id, None)
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    let result = HoldResult {
        id: tension.id.clone(),
        previous_position: old_position,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        match old_position {
            Some(p) => {
                output
                    .success(&format!("Held tension {} (was position {})", werk_shared::display_id(tension.short_code, &tension.id), p))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success(&format!("Tension {} is already held", werk_shared::display_id(tension.short_code, &tension.id)))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
    }

    Ok(())
}
