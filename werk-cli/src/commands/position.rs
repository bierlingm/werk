//! Position command handler — set the order of operations position for a tension.

use crate::error::WerkError;
use crate::output::Output;
use crate::palette;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

#[derive(Serialize)]
struct PositionResult {
    id: String,
    previous_position: Option<i32>,
    new_position: i32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    dry_run: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signals: Vec<palette::Palette>,
}

pub fn cmd_position(
    output: &Output,
    id: String,
    n: i32,
    dry_run: bool,
) -> Result<(), WerkError> {
    if n < 1 {
        return Err(WerkError::InvalidInput("position must be >= 1".to_string()));
    }

    let workspace = Workspace::discover()?;
    let (mut store, _hook_handle) = workspace.open_store_with_hooks()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);
    let tension = resolver.resolve(&id)?;

    let old_position = tension.position;

    if dry_run {
        let unchanged = old_position == Some(n);
        if output.is_structured() {
            let result = PositionResult {
                id: tension.id.clone(),
                previous_position: old_position,
                new_position: n,
                dry_run: true,
                signals: Vec::new(),
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            let display = werk_shared::display_id(tension.short_code, &tension.id);
            if unchanged {
                println!("Would leave tension {} at position {} (unchanged)", display, n);
            } else {
                match old_position {
                    Some(p) => println!("Would position tension {} at {} (was {})", display, n, p),
                    None => println!("Would position tension {} at {} (was held)", display, n),
                }
            }
            println!("No changes made.");
        }
        return Ok(());
    }

    let _ = store.begin_gesture(Some(&format!("position {} at {}", &tension.id, n)));
    let changed = store
        .update_position(&tension.id, Some(n))
        .map_err(WerkError::CoreError)?;
    store.end_gesture();

    // Print success message before palette (human mode)
    if !output.is_structured() {
        if !changed {
            output
                .success(&format!(
                    "Tension {} is already at position {}",
                    werk_shared::display_id(tension.short_code, &tension.id),
                    n
                ))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        } else {
            match old_position {
                Some(p) => {
                    output
                        .success(&format!(
                            "Positioned tension {} at {} (was {})",
                            werk_shared::display_id(tension.short_code, &tension.id),
                            n,
                            p
                        ))
                        .map_err(|e| WerkError::IoError(e.to_string()))?;
                }
                None => {
                    output
                        .success(&format!(
                            "Positioned tension {} at {} (was held)",
                            werk_shared::display_id(tension.short_code, &tension.id),
                            n
                        ))
                        .map_err(|e| WerkError::IoError(e.to_string()))?;
                }
            }
        }
    }

    // Pathway palette: detect sequencing pressure after position change
    let signals = palette::check_sequencing_after_position(output, &mut store, &tension.id)?;

    if output.is_structured() {
        let result = PositionResult {
            id: tension.id.clone(),
            previous_position: old_position,
            new_position: n,
            dry_run: false,
            signals,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    }

    Ok(())
}
