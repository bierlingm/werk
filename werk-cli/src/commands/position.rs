//! Position command handler — set the order of operations position for a tension,
//! or compact existing positions among the children of a parent.

use crate::error::WerkError;
use crate::output::Output;
use crate::palette;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_core::TensionStatus;

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

#[derive(Serialize)]
struct RenumberEntry {
    id: String,
    short_code: Option<i32>,
    from: i32,
    to: i32,
}

#[derive(Serialize)]
struct RenumberResult {
    parent_id: String,
    parent_short_code: Option<i32>,
    changes: Vec<RenumberEntry>,
    unchanged: Vec<RenumberEntry>,
    held: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    dry_run: bool,
}

pub fn cmd_position(
    output: &Output,
    id: Option<String>,
    n: Option<i32>,
    renumber: Option<String>,
    dry_run: bool,
) -> Result<(), WerkError> {
    if let Some(parent_id) = renumber {
        return cmd_renumber(output, parent_id, dry_run);
    }

    let id = id.ok_or_else(|| {
        WerkError::InvalidInput(
            "position requires <id> <n> (or --renumber <parent_id>)".to_string(),
        )
    })?;
    let n = n.ok_or_else(|| {
        WerkError::InvalidInput("position requires a position number <n>".to_string())
    })?;
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

fn cmd_renumber(output: &Output, parent_id: String, dry_run: bool) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let (mut store, _hook_handle) = workspace.open_store_with_hooks()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());
    let parent = resolver.resolve(&parent_id)?;

    // Gather children: positioned (sorted by current position) keep their relative order.
    let mut positioned: Vec<&werk_core::Tension> = tensions
        .iter()
        .filter(|t| {
            t.parent_id.as_deref() == Some(parent.id.as_str())
                && t.status == TensionStatus::Active
                && t.position.is_some()
        })
        .collect();
    positioned.sort_by_key(|t| t.position.unwrap_or(i32::MAX));

    let held_count = tensions
        .iter()
        .filter(|t| {
            t.parent_id.as_deref() == Some(parent.id.as_str())
                && t.status == TensionStatus::Active
                && t.position.is_none()
        })
        .count();

    let mut changes: Vec<RenumberEntry> = Vec::new();
    let mut unchanged: Vec<RenumberEntry> = Vec::new();
    for (idx, t) in positioned.iter().enumerate() {
        let from = t.position.unwrap_or(0);
        let to = (idx + 1) as i32;
        let entry = RenumberEntry {
            id: t.id.clone(),
            short_code: t.short_code,
            from,
            to,
        };
        if from == to {
            unchanged.push(entry);
        } else {
            changes.push(entry);
        }
    }

    let parent_display = werk_shared::display_id(parent.short_code, &parent.id);

    if dry_run || changes.is_empty() {
        if !output.is_structured() {
            if changes.is_empty() {
                println!(
                    "No renumber needed under {} ({} positioned, {} held)",
                    parent_display,
                    positioned.len(),
                    held_count
                );
            } else {
                println!(
                    "Would renumber {} children under {}:",
                    changes.len(),
                    parent_display
                );
                for c in &changes {
                    let cd = werk_shared::display_id(c.short_code, &c.id);
                    println!("  {} : {} -> {}", cd, c.from, c.to);
                }
                println!("No changes made.");
            }
        }
        if output.is_structured() {
            let result = RenumberResult {
                parent_id: parent.id.clone(),
                parent_short_code: parent.short_code,
                changes,
                unchanged,
                held: held_count,
                dry_run: true,
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        }
        return Ok(());
    }

    let change_count = changes.len();
    let _ = store.begin_gesture(Some(&format!(
        "renumber children of {} (1..{})",
        parent.id,
        positioned.len()
    )));
    for c in &changes {
        store
            .update_position(&c.id, Some(c.to))
            .map_err(WerkError::CoreError)?;
    }
    store.end_gesture();

    if !output.is_structured() {
        output
            .success(&format!(
                "Renumbered {} children under {} (1..{})",
                change_count,
                parent_display,
                positioned.len()
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        for c in &changes {
            let cd = werk_shared::display_id(c.short_code, &c.id);
            println!("  {} : {} -> {}", cd, c.from, c.to);
        }
    } else {
        let result = RenumberResult {
            parent_id: parent.id.clone(),
            parent_short_code: parent.short_code,
            changes,
            unchanged,
            held: held_count,
            dry_run: false,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    }

    Ok(())
}
