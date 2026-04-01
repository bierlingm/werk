//! Split command — divide a tension into N new tensions with provenance.
//!
//! `werk split <id> "desire 1" "desire 2" [..."desire N"]`
//!
//! Creates N new tensions from one source. The source is resolved by default.
//! Each new tension gets a `split_from` edge pointing to the source.
//! Cross-tension epochs link via shared gesture ID.
//!
//! Child assignment: if the source has children, they must be assigned
//! to one of the successors (or floated to the source's parent).

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use sd_core::tension::TensionStatus;
use serde::Serialize;

#[derive(Serialize)]
struct SplitResult {
    source_id: String,
    source_short_code: Option<i32>,
    source_status: String,
    new_tensions: Vec<NewTensionRef>,
    reparented_children: Vec<ReparentedChild>,
}

#[derive(Serialize)]
struct NewTensionRef {
    id: String,
    short_code: Option<i32>,
    desired: String,
}

#[derive(Serialize)]
struct ReparentedChild {
    id: String,
    short_code: Option<i32>,
    new_parent_short_code: Option<i32>,
}

pub fn cmd_split(
    output: &Output,
    id: String,
    desires: Vec<String>,
    assign: Vec<String>,
    children_to_parent: bool,
    children_to: Option<usize>,
    keep: bool,
    release: bool,
    hold: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    if desires.len() < 2 {
        return Err(WerkError::InvalidInput(
            "split requires at least 2 desires (e.g., werk split 42 \"concern A\" \"concern B\")"
                .to_owned(),
        ));
    }

    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());
    let source = resolver.resolve(&id)?;

    if source.status != TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot split {} tension #{}",
            source.status,
            source.short_code.unwrap_or(0)
        )));
    }

    let source_id = source.id.clone();
    let source_display =
        werk_shared::display_id(source.short_code, &source_id);
    let parent_id = source.parent_id.clone();

    // Get children of the source
    let children = store
        .get_children(&source_id)
        .map_err(WerkError::StoreError)?;

    // Parse --assign flags: "30=1" means child #30 goes to successor 1
    let mut assignments: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
    for a in &assign {
        let parts: Vec<&str> = a.split('=').collect();
        if parts.len() != 2 {
            return Err(WerkError::InvalidInput(format!(
                "invalid --assign format: '{}'. Use CHILD_ID=TARGET_NUM (e.g., 30=1)",
                a
            )));
        }
        let child_code: i32 = parts[0]
            .trim_start_matches('#')
            .parse()
            .map_err(|_| WerkError::InvalidInput(format!("invalid child ID: '{}'", parts[0])))?;
        let target: usize = parts[1]
            .parse()
            .map_err(|_| WerkError::InvalidInput(format!("invalid target number: '{}'", parts[1])))?;
        if target < 1 || target > desires.len() {
            return Err(WerkError::InvalidInput(format!(
                "target {} out of range (1-{})",
                target,
                desires.len()
            )));
        }
        assignments.insert(child_code, target);
    }

    // Determine assignment strategy for unassigned children
    if !children.is_empty() && assignments.is_empty() && !children_to_parent && children_to.is_none()
    {
        // No assignment strategy — error with helpful message
        let child_list: Vec<String> = children
            .iter()
            .map(|c| {
                format!(
                    "  #{} {}",
                    c.short_code.unwrap_or(0),
                    werk_shared::truncate(&c.desired, 50)
                )
            })
            .collect();

        return Err(WerkError::InvalidInput(format!(
            "{} has {} children that need assignment:\n{}\n\nUse one of:\n  --assign {}=1    Assign specific children\n  --children-to-parent    Float all to parent\n  --children-to=1         All to successor 1",
            source_display,
            children.len(),
            child_list.join("\n"),
            children.first().and_then(|c| c.short_code).unwrap_or(0),
        )));
    }

    if let Some(target) = children_to {
        if target < 1 || target > desires.len() {
            return Err(WerkError::InvalidInput(format!(
                "--children-to {} out of range (1-{})",
                target,
                desires.len()
            )));
        }
    }

    if dry_run {
        println!("Dry run: would split {} into {} tensions:", source_display, desires.len());
        for (i, d) in desires.iter().enumerate() {
            println!("  {}. {}", i + 1, d);
        }
        if !children.is_empty() {
            println!("\nChild assignment:");
            for child in &children {
                let cc = child.short_code.unwrap_or(0);
                let target = if let Some(&t) = assignments.get(&cc) {
                    format!("successor {}", t)
                } else if children_to_parent {
                    "parent".to_owned()
                } else if let Some(t) = children_to {
                    format!("successor {}", t)
                } else {
                    "unassigned".to_owned()
                };
                println!("  #{} → {}", cc, target);
            }
        }
        let disposition = if keep {
            "kept active"
        } else if release {
            "released"
        } else if hold {
            "held"
        } else {
            "resolved"
        };
        println!("\nSource {} would be {}.", source_display, disposition);
        return Ok(());
    }

    // Execute the split
    let gesture_id = store
        .begin_gesture(Some(&format!("split {}", source_display)))
        .map_err(WerkError::StoreError)?;

    // 1. Create epoch on source (final epoch, type "split_source")
    store
        .create_epoch_typed(
            &source_id,
            &source.desired,
            &source.actual,
            None,
            Some(&gesture_id),
            Some("split_source"),
        )
        .map_err(WerkError::StoreError)?;

    // 2. Create new tensions
    let mut new_tensions: Vec<sd_core::Tension> = Vec::new();
    for desire in &desires {
        let t = store
            .create_tension_with_parent(desire, "", parent_id.clone())
            .map_err(|e| WerkError::IoError(e.to_string()))?;

        // Create split_from edge: new → source
        store
            .create_edge(&t.id, &source_id, sd_core::EDGE_SPLIT_FROM)
            .map_err(WerkError::StoreError)?;

        // Create origin epoch on new tension
        store
            .create_epoch_typed(
                &t.id,
                desire,
                "",
                None,
                Some(&gesture_id),
                Some("split_target"),
            )
            .map_err(WerkError::StoreError)?;

        new_tensions.push(t);
    }

    // 3. Reparent children
    let mut reparented: Vec<ReparentedChild> = Vec::new();
    for child in &children {
        let cc = child.short_code.unwrap_or(0);
        let target_idx = if let Some(&t) = assignments.get(&cc) {
            Some(t - 1) // 1-based to 0-based
        } else if children_to_parent {
            None // goes to grandparent
        } else if let Some(t) = children_to {
            Some(t - 1)
        } else {
            None // shouldn't happen — checked above
        };

        let new_parent = match target_idx {
            Some(idx) => Some(new_tensions[idx].id.as_str()),
            None => parent_id.as_deref(),
        };

        store
            .update_parent(&child.id, new_parent)
            .map_err(|e| WerkError::IoError(e.to_string()))?;

        reparented.push(ReparentedChild {
            id: child.id.clone(),
            short_code: child.short_code,
            new_parent_short_code: match target_idx {
                Some(idx) => new_tensions[idx].short_code,
                None => tensions
                    .iter()
                    .find(|t| Some(&t.id) == parent_id.as_ref())
                    .and_then(|t| t.short_code),
            },
        });
    }

    // 4. Resolve/release/hold the source
    let source_status = if keep {
        // Keep active — no status change
        "active".to_owned()
    } else if release {
        store
            .update_status(&source_id, TensionStatus::Released)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        "released".to_owned()
    } else {
        // Default: resolve
        store
            .update_status(&source_id, TensionStatus::Resolved)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        "resolved".to_owned()
    };

    // Record the split mutation on source
    store
        .record_mutation(&sd_core::Mutation::new(
            source_id.clone(),
            chrono::Utc::now(),
            "split".to_owned(),
            Some(source.desired.clone()),
            serde_json::to_string(
                &new_tensions
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "id": t.id,
                            "short_code": t.short_code,
                            "desired": t.desired,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
        ))
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    store.end_gesture();

    // Output
    if output.is_structured() {
        let result = SplitResult {
            source_id: source_id.clone(),
            source_short_code: source.short_code,
            source_status,
            new_tensions: new_tensions
                .iter()
                .map(|t| NewTensionRef {
                    id: t.id.clone(),
                    short_code: t.short_code,
                    desired: t.desired.clone(),
                })
                .collect(),
            reparented_children: reparented,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!("Split {} into {} tensions:", source_display, new_tensions.len()))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        for (i, t) in new_tensions.iter().enumerate() {
            let display = werk_shared::display_id(t.short_code, &t.id);
            println!(
                "  {}. {} — {}",
                i + 1,
                display,
                werk_shared::truncate(&t.desired, 60)
            );
        }
        if !reparented.is_empty() {
            println!("\n  {} children reparented.", reparented.len());
        }
        let disposition = if keep { "kept active" } else if release { "released" } else { "resolved" };
        println!("  Source {} {}.", source_display, disposition);
    }

    Ok(())
}
