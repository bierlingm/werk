//! Tree command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{DynamicsEngine, Forest, TensionStatus};
use werk_shared::truncate;
use serde::Serialize;

/// JSON output structure for a tension in tree.
#[derive(Serialize)]
struct TensionJson {
    id: String,
    desired: String,
    actual: String,
    status: String,
    parent_id: Option<String>,
    created_at: String,
    horizon: Option<String>,
    phase: String,
    movement: String,
    has_conflict: bool,
}

/// JSON output structure for tree.
#[derive(Serialize)]
struct TreeJson {
    tensions: Vec<TensionJson>,
    summary: TreeSummary,
}

#[derive(Serialize)]
struct TreeSummary {
    total: usize,
    active: usize,
    resolved: usize,
    released: usize,
}

/// Filter for tree display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Active,
    Resolved,
    Released,
}

pub fn cmd_tree(
    output: &Output,
    _open: bool,
    all: bool,
    resolved: bool,
    released: bool,
) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Create DynamicsEngine from store
    let mut engine = DynamicsEngine::with_store(store);

    // Get all tensions
    let tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    // Build forest
    let forest = Forest::from_tensions(tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // Determine filter
    let filter = if all {
        Filter::All
    } else if resolved {
        Filter::Resolved
    } else if released {
        Filter::Released
    } else {
        // Default: --open (active only)
        Filter::Active
    };

    // Filter tensions
    let filtered_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| match filter {
            Filter::All => true,
            Filter::Active => t.status == TensionStatus::Active,
            Filter::Resolved => t.status == TensionStatus::Resolved,
            Filter::Released => t.status == TensionStatus::Released,
        })
        .collect();

    // Handle empty forest
    if filtered_tensions.is_empty() {
        if output.is_structured() {
            let result = TreeJson {
                tensions: vec![],
                summary: TreeSummary {
                    total: 0,
                    active: 0,
                    resolved: 0,
                    released: 0,
                },
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .info("No tensions found")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
        return Ok(());
    }

    // Compute dynamics for each tension using DynamicsEngine
    let mut dynamics_map: std::collections::HashMap<String, (String, String, bool)> =
        std::collections::HashMap::new();

    for tension in &filtered_tensions {
        // Use DynamicsEngine to compute all dynamics (phase, conflict, tendency, etc.)
        let computed = engine.compute_full_dynamics_for_tension(&tension.id);

        let (phase_badge, movement_signal, has_conflict) = match computed {
            Some(cd) => {
                let phase = match cd.phase.phase {
                    sd_core::CreativeCyclePhase::Germination => "[G]",
                    sd_core::CreativeCyclePhase::Assimilation => "[A]",
                    sd_core::CreativeCyclePhase::Completion => "[C]",
                    sd_core::CreativeCyclePhase::Momentum => "[M]",
                };
                let movement = match cd.tendency.tendency {
                    sd_core::StructuralTendency::Advancing => "→",
                    sd_core::StructuralTendency::Oscillating => "↔",
                    sd_core::StructuralTendency::Stagnant => "○",
                };
                (phase, movement, cd.conflict.is_some())
            }
            None => ("[G]", "○", false),
        };

        dynamics_map.insert(
            tension.id.clone(),
            (
                phase_badge.to_string(),
                movement_signal.to_string(),
                has_conflict,
            ),
        );
    }

    // If JSON output, build JSON structure
    if output.is_structured() {
        let json_tensions: Vec<TensionJson> = filtered_tensions
            .iter()
            .map(|t| {
                let (phase, movement, has_conflict) = dynamics_map.get(&t.id).cloned().unwrap_or((
                    "[G]".to_string(),
                    "○".to_string(),
                    false,
                ));
                TensionJson {
                    id: t.id.clone(),
                    desired: t.desired.clone(),
                    actual: t.actual.clone(),
                    status: t.status.to_string(),
                    parent_id: t.parent_id.clone(),
                    created_at: t.created_at.to_rfc3339(),
                    horizon: t.horizon.as_ref().map(|h| h.to_string()),
                    phase: phase.replace(['[', ']'], ""),
                    movement: movement.to_string(),
                    has_conflict,
                }
            })
            .collect();

        // Count by status
        let active_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .count();
        let resolved_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Resolved)
            .count();
        let released_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Released)
            .count();

        let result = TreeJson {
            tensions: json_tensions,
            summary: TreeSummary {
                total: tensions.len(),
                active: active_count,
                resolved: resolved_count,
                released: released_count,
            },
        };

        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
        return Ok(());
    }

    // Human-readable tree output
    // Build filtered forest for display
    let filtered_ids: std::collections::HashSet<_> =
        filtered_tensions.iter().map(|t| t.id.as_str()).collect();

    // Traverse and render the forest
    fn render_tree(
        forest: &Forest,
        root_ids: &[String],
        filtered_ids: &std::collections::HashSet<&str>,
        dynamics_map: &std::collections::HashMap<String, (String, String, bool)>,
        _output: &Output,
        prefix: &str,
        lines: &mut Vec<String>,
    ) {
        let mut roots: Vec<_> = root_ids
            .iter()
            .filter(|id| filtered_ids.contains(id.as_str()))
            .filter_map(|id| forest.find(id))
            .collect();

        // Sort roots by horizon (earliest first, None last)
        roots.sort_by(|a, b| match (&a.tension.horizon, &b.tension.horizon) {
            (Some(ha), Some(hb)) => ha.cmp(hb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        for (i, node) in roots.iter().enumerate() {
            let is_last = i == roots.len() - 1;

            // Get dynamics
            let (phase, movement, has_conflict) = dynamics_map.get(node.id()).cloned().unwrap_or((
                "[G]".to_string(),
                "○".to_string(),
                false,
            ));

            // Build the line
            let connector = if is_last { "└── " } else { "├── " };

            // Conflict marker
            let conflict_marker = if has_conflict { "!" } else { " " };

            // Horizon annotation
            let horizon_annotation = match &node.tension.horizon {
                Some(h) => format!("[{}]", h),
                None => "[—]".to_string(),
            };

            // Format: prefix + connector + [badge] status id horizon movement desired
            let id_short = &node.id()[..8.min(node.id().len())];
            let line = format!(
                "{}{}{}{} {} {} {}{} {}",
                prefix,
                connector,
                &phase,
                &node.tension.status,
                id_short,
                &horizon_annotation,
                conflict_marker,
                movement,
                truncate(&node.tension.desired, 50)
            );
            lines.push(line);

            // Recurse for children (only those that pass the filter)
            let mut children: Vec<_> = node
                .children
                .iter()
                .filter(|id| filtered_ids.contains(id.as_str()))
                .filter_map(|id| forest.find(id))
                .collect();

            // Sort children by horizon as well
            children.sort_by(|a, b| match (&a.tension.horizon, &b.tension.horizon) {
                (Some(ha), Some(hb)) => ha.cmp(hb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });

            if !children.is_empty() {
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };
                render_tree(
                    forest,
                    &node.children,
                    filtered_ids,
                    dynamics_map,
                    _output,
                    &new_prefix,
                    lines,
                );
            }
        }
    }

    let mut lines = Vec::new();
    render_tree(
        &forest,
        forest.root_ids(),
        &filtered_ids,
        &dynamics_map,
        output,
        "",
        &mut lines,
    );

    // Print tree
    for line in &lines {
        println!("{}", line);
    }

    // Print summary footer
    let active_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();

    println!();
    println!(
        "Total: {}  Active: {}  Resolved: {}  Released: {}",
        tensions.len(), active_count, resolved_count, released_count
    );

    Ok(())
}
