//! Position command handler — set the order of operations position for a tension.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

#[derive(Serialize)]
struct PositionResult {
    id: String,
    previous_position: Option<i32>,
    new_position: i32,
}

pub fn cmd_position(output: &Output, id: String, n: i32) -> Result<(), WerkError> {
    if n < 1 {
        return Err(WerkError::InvalidInput(
            "position must be >= 1".to_string(),
        ));
    }

    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);
    let tension = resolver.resolve(&id)?;

    let old_position = tension.position;

    let _ = store.begin_gesture(Some(&format!("position {} at {}", &tension.id, n)));
    store
        .update_position(&tension.id, Some(n))
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    let result = PositionResult {
        id: tension.id.clone(),
        previous_position: old_position,
        new_position: n,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        match old_position {
            Some(p) => {
                output
                    .success(&format!(
                        "Positioned tension {} at {} (was {})",
                        werk_shared::display_id(tension.short_code, &tension.id), n, p
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success(&format!(
                        "Positioned tension {} at {} (was held)",
                        werk_shared::display_id(tension.short_code, &tension.id), n
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
    }

    Ok(())
}
