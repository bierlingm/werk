//! Merge command — combine tensions with provenance.
//!
//! Two modes:
//! - Asymmetric: `werk merge <id1> <id2> --into <id>` — survivor absorbs the other
//! - Symmetric:  `werk merge <id1> <id2> --as "new desire"` — both absorbed into new

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use sd_core::tension::TensionStatus;
use serde::Serialize;

#[derive(Serialize)]
struct MergeResult {
    mode: String,
    survivor_id: String,
    survivor_short_code: Option<i32>,
    absorbed: Vec<AbsorbedRef>,
    reparented_children: Vec<ReparentedChild>,
}

#[derive(Serialize)]
struct AbsorbedRef {
    id: String,
    short_code: Option<i32>,
    status: String,
}

#[derive(Serialize)]
struct ReparentedChild {
    id: String,
    short_code: Option<i32>,
}

pub fn cmd_merge(
    output: &Output,
    id1: String,
    id2: String,
    into: Option<String>,
    as_desire: Option<String>,
    desire: Option<String>,
    assign: Vec<String>,
    children_to_parent: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    // Must specify either --into or --as
    if into.is_none() && as_desire.is_none() {
        return Err(WerkError::InvalidInput(
            "merge requires either --into <id> (asymmetric) or --as \"desire\" (symmetric).\n\n\
             Asymmetric: werk merge 18 19 --into 18   (18 survives, 19 absorbed)\n\
             Symmetric:  werk merge 18 19 --as \"combined concern\"   (both absorbed into new)"
                .to_owned(),
        ));
    }

    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());

    let t1 = resolver.resolve(&id1)?;
    let t2 = resolver.resolve(&id2)?;

    if t1.id == t2.id {
        return Err(WerkError::InvalidInput(
            "cannot merge a tension with itself".to_owned(),
        ));
    }

    for t in [&t1, &t2] {
        if t.status != TensionStatus::Active {
            return Err(WerkError::InvalidInput(format!(
                "cannot merge {} tension #{}",
                t.status,
                t.short_code.unwrap_or(0)
            )));
        }
    }

    if let Some(ref as_d) = as_desire {
        // Symmetric merge: both absorbed into a new tension
        cmd_merge_symmetric(output, &mut store, &t1, &t2, as_d, &assign, children_to_parent, dry_run)
    } else {
        // Asymmetric merge: --into specifies the survivor
        let into_id = into.unwrap();
        let into_resolved = resolver.resolve(&into_id)?;

        if into_resolved.id != t1.id && into_resolved.id != t2.id {
            return Err(WerkError::InvalidInput(format!(
                "--into must be one of the merge arguments ({} or {})",
                werk_shared::display_id(t1.short_code, &t1.id),
                werk_shared::display_id(t2.short_code, &t2.id),
            )));
        }

        let (survivor, absorbed) = if into_resolved.id == t1.id {
            (&t1, &t2)
        } else {
            (&t2, &t1)
        };

        cmd_merge_asymmetric(output, &mut store, survivor, absorbed, desire.as_deref(), &assign, children_to_parent, dry_run)
    }
}

fn cmd_merge_asymmetric(
    output: &Output,
    store: &mut sd_core::Store,
    survivor: &sd_core::Tension,
    absorbed: &sd_core::Tension,
    new_desire: Option<&str>,
    assign: &[String],
    children_to_parent: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    let survivor_display = werk_shared::display_id(survivor.short_code, &survivor.id);
    let absorbed_display = werk_shared::display_id(absorbed.short_code, &absorbed.id);

    // Get children of absorbed tension
    let absorbed_children = store
        .get_children(&absorbed.id)
        .map_err(WerkError::StoreError)?;

    // Parse assignments
    let assignments = parse_assignments(assign)?;

    // Check: if absorbed has children and no strategy given
    if !absorbed_children.is_empty() && assignments.is_empty() && !children_to_parent {
        let child_list: Vec<String> = absorbed_children
            .iter()
            .map(|c| format!("  #{} {}", c.short_code.unwrap_or(0), werk_shared::truncate(&c.desired, 50)))
            .collect();
        return Err(WerkError::InvalidInput(format!(
            "{} has {} children that need assignment:\n{}\n\nUse --children-to-parent or --assign CHILD=survivor",
            absorbed_display,
            absorbed_children.len(),
            child_list.join("\n"),
        )));
    }

    if dry_run {
        println!("Dry run: would merge {} into {}.", absorbed_display, survivor_display);
        if let Some(d) = new_desire {
            println!("  Survivor desire updated to: {}", d);
        }
        if !absorbed_children.is_empty() {
            println!("  {} children of {} reparented.", absorbed_children.len(), absorbed_display);
        }
        println!("  {} would be resolved.", absorbed_display);
        return Ok(());
    }

    // Execute
    let gesture_id = store
        .begin_gesture(Some(&format!("merge {} into {}", absorbed_display, survivor_display)))
        .map_err(WerkError::StoreError)?;

    // Epoch on absorbed (merge_source)
    store
        .create_epoch_typed(
            &absorbed.id,
            &absorbed.desired,
            &absorbed.actual,
            None,
            Some(&gesture_id),
            Some("merge_source"),
        )
        .map_err(WerkError::StoreError)?;

    // Epoch on survivor (merge_target)
    store
        .create_epoch_typed(
            &survivor.id,
            &survivor.desired,
            &survivor.actual,
            None,
            Some(&gesture_id),
            Some("merge_target"),
        )
        .map_err(WerkError::StoreError)?;

    // Create merged_into edge: absorbed → survivor
    store
        .create_edge(&absorbed.id, &survivor.id, sd_core::EDGE_MERGED_INTO)
        .map_err(WerkError::StoreError)?;

    // Reparent absorbed children
    let mut reparented = Vec::new();
    for child in &absorbed_children {
        let new_parent = if children_to_parent {
            absorbed.parent_id.as_deref()
        } else {
            Some(survivor.id.as_str())
        };
        store
            .update_parent(&child.id, new_parent)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        reparented.push(ReparentedChild {
            id: child.id.clone(),
            short_code: child.short_code,
        });
    }

    // Update survivor desire if requested
    if let Some(d) = new_desire {
        store
            .update_desired(&survivor.id, d)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    // Resolve absorbed
    store
        .update_status(&absorbed.id, TensionStatus::Resolved)
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    // Record merge mutation
    store
        .record_mutation(&sd_core::Mutation::new(
            survivor.id.clone(),
            chrono::Utc::now(),
            "merge".to_owned(),
            None,
            serde_json::json!({
                "absorbed_id": absorbed.id,
                "absorbed_short_code": absorbed.short_code,
                "absorbed_desired": absorbed.desired,
            })
            .to_string(),
        ))
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    store.end_gesture();

    // Output
    if output.is_structured() {
        let result = MergeResult {
            mode: "asymmetric".to_owned(),
            survivor_id: survivor.id.clone(),
            survivor_short_code: survivor.short_code,
            absorbed: vec![AbsorbedRef {
                id: absorbed.id.clone(),
                short_code: absorbed.short_code,
                status: "resolved".to_owned(),
            }],
            reparented_children: reparented,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!(
                "Merged {} into {}",
                absorbed_display, survivor_display
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  {} resolved.", absorbed_display);
        if !reparented.is_empty() {
            println!("  {} children reparented.", reparented.len());
        }
    }

    Ok(())
}

fn cmd_merge_symmetric(
    output: &Output,
    store: &mut sd_core::Store,
    t1: &sd_core::Tension,
    t2: &sd_core::Tension,
    new_desire: &str,
    _assign: &[String],
    children_to_parent: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    let t1_display = werk_shared::display_id(t1.short_code, &t1.id);
    let t2_display = werk_shared::display_id(t2.short_code, &t2.id);

    // Get children of both
    let t1_children = store.get_children(&t1.id).map_err(WerkError::StoreError)?;
    let t2_children = store.get_children(&t2.id).map_err(WerkError::StoreError)?;
    let all_children: Vec<_> = t1_children.iter().chain(t2_children.iter()).collect();

    if !all_children.is_empty() && !children_to_parent {
        let child_list: Vec<String> = all_children
            .iter()
            .map(|c| format!("  #{} {}", c.short_code.unwrap_or(0), werk_shared::truncate(&c.desired, 50)))
            .collect();
        return Err(WerkError::InvalidInput(format!(
            "merged tensions have {} children that need assignment:\n{}\n\nUse --children-to-parent to float to parents, or the new tension will adopt them.",
            all_children.len(),
            child_list.join("\n"),
        )));
    }

    if dry_run {
        println!("Dry run: would merge {} and {} into new tension:", t1_display, t2_display);
        println!("  Desire: {}", new_desire);
        println!("  Both {} and {} would be resolved.", t1_display, t2_display);
        if !all_children.is_empty() {
            println!("  {} children reassigned.", all_children.len());
        }
        return Ok(());
    }

    // Determine parent: use t1's parent (arbitrary but stable)
    let parent_id = t1.parent_id.clone();

    let gesture_id = store
        .begin_gesture(Some(&format!("merge {} + {} into new", t1_display, t2_display)))
        .map_err(WerkError::StoreError)?;

    // Epochs on both sources
    for t in [t1, t2] {
        store
            .create_epoch_typed(
                &t.id,
                &t.desired,
                &t.actual,
                None,
                Some(&gesture_id),
                Some("merge_source"),
            )
            .map_err(WerkError::StoreError)?;
    }

    // Create new tension
    let new_t = store
        .create_tension_with_parent(new_desire, "", parent_id)
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    // Origin epoch on new tension
    store
        .create_epoch_typed(
            &new_t.id,
            new_desire,
            "",
            None,
            Some(&gesture_id),
            Some("merge_target"),
        )
        .map_err(WerkError::StoreError)?;

    // Create merged_into edges: both → new
    for t in [t1, t2] {
        store
            .create_edge(&t.id, &new_t.id, sd_core::EDGE_MERGED_INTO)
            .map_err(WerkError::StoreError)?;
    }

    // Reparent children
    let mut reparented = Vec::new();
    for child in &all_children {
        let new_parent = if children_to_parent {
            // Float to original parent
            None // This means "to the tension's own parent" — but we need the actual parent
        } else {
            Some(new_t.id.as_str())
        };

        // For children_to_parent, use the child's current grandparent
        let actual_parent = if children_to_parent {
            if t1_children.iter().any(|c| c.id == child.id) {
                t1.parent_id.as_deref()
            } else {
                t2.parent_id.as_deref()
            }
        } else {
            new_parent
        };

        store
            .update_parent(&child.id, actual_parent)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        reparented.push(ReparentedChild {
            id: child.id.clone(),
            short_code: child.short_code,
        });
    }

    // Resolve both sources
    for t in [t1, t2] {
        store
            .update_status(&t.id, TensionStatus::Resolved)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    // Record merge mutation on new tension
    store
        .record_mutation(&sd_core::Mutation::new(
            new_t.id.clone(),
            chrono::Utc::now(),
            "merge".to_owned(),
            None,
            serde_json::json!({
                "sources": [
                    {"id": t1.id, "short_code": t1.short_code, "desired": t1.desired},
                    {"id": t2.id, "short_code": t2.short_code, "desired": t2.desired},
                ]
            })
            .to_string(),
        ))
        .map_err(|e| WerkError::IoError(e.to_string()))?;

    store.end_gesture();

    let new_display = werk_shared::display_id(new_t.short_code, &new_t.id);

    if output.is_structured() {
        let result = MergeResult {
            mode: "symmetric".to_owned(),
            survivor_id: new_t.id.clone(),
            survivor_short_code: new_t.short_code,
            absorbed: vec![
                AbsorbedRef {
                    id: t1.id.clone(),
                    short_code: t1.short_code,
                    status: "resolved".to_owned(),
                },
                AbsorbedRef {
                    id: t2.id.clone(),
                    short_code: t2.short_code,
                    status: "resolved".to_owned(),
                },
            ],
            reparented_children: reparented,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!(
                "Merged {} and {} into {}",
                t1_display, t2_display, new_display
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  {} — {}", new_display, new_desire);
        println!("  Both {} and {} resolved.", t1_display, t2_display);
        if !reparented.is_empty() {
            println!("  {} children reparented.", reparented.len());
        }
    }

    Ok(())
}

fn parse_assignments(assign: &[String]) -> Result<std::collections::HashMap<i32, String>, WerkError> {
    let mut result = std::collections::HashMap::new();
    for a in assign {
        let parts: Vec<&str> = a.split('=').collect();
        if parts.len() != 2 {
            return Err(WerkError::InvalidInput(format!(
                "invalid --assign format: '{}'. Use CHILD_ID=TARGET",
                a
            )));
        }
        let child_code: i32 = parts[0]
            .trim_start_matches('#')
            .parse()
            .map_err(|_| WerkError::InvalidInput(format!("invalid child ID: '{}'", parts[0])))?;
        result.insert(child_code, parts[1].to_owned());
    }
    Ok(result)
}
