//! Tree command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{Forest, TensionStatus, detect_containment_violations, detect_sequencing_pressure};
use serde::Serialize;
use werk_shared::truncate;

/// JSON output structure for a tension in tree.
#[derive(Serialize)]
struct TensionJson {
    id: String,
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: String,
    parent_id: Option<String>,
    created_at: String,
    horizon: Option<String>,
    overdue: bool,
    containment_violation: bool,
    sequencing_pressure: bool,
    closure_resolved: usize,
    closure_total: usize,
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
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    let forest = Forest::from_tensions(tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    let now = Utc::now();

    let filter = if all {
        Filter::All
    } else if resolved {
        Filter::Resolved
    } else if released {
        Filter::Released
    } else {
        Filter::Active
    };

    let filtered_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| match filter {
            Filter::All => true,
            Filter::Active => t.status == TensionStatus::Active,
            Filter::Resolved => t.status == TensionStatus::Resolved,
            Filter::Released => t.status == TensionStatus::Released,
        })
        .collect();

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

    // JSON output
    if output.is_structured() {
        let json_tensions: Vec<TensionJson> = filtered_tensions
            .iter()
            .map(|t| {
                let all_children = forest.children(&t.id).unwrap_or_default();
                let closure_resolved = all_children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Resolved)
                    .count();
                let overdue = t.status == TensionStatus::Active
                    && t.horizon
                        .as_ref()
                        .map(|h| h.is_past(now))
                        .unwrap_or(false);

                let containment_violation = t.status == TensionStatus::Active
                    && t.parent_id.as_ref().map_or(false, |pid| {
                        detect_containment_violations(&forest, pid)
                            .iter()
                            .any(|v| v.tension_id == t.id)
                    });

                let sequencing_pressure = t.status == TensionStatus::Active
                    && t.parent_id.as_ref().map_or(false, |pid| {
                        detect_sequencing_pressure(&forest, pid)
                            .iter()
                            .any(|p| p.tension_id == t.id)
                    });

                TensionJson {
                    id: t.id.clone(),
                    short_code: t.short_code,
                    desired: t.desired.clone(),
                    actual: t.actual.clone(),
                    status: t.status.to_string(),
                    parent_id: t.parent_id.clone(),
                    created_at: t.created_at.to_rfc3339(),
                    horizon: t.horizon.as_ref().map(|h| h.to_string()),
                    overdue,
                    containment_violation,
                    sequencing_pressure,
                    closure_resolved,
                    closure_total: all_children.len(),
                }
            })
            .collect();

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
    let filtered_ids: std::collections::HashSet<_> =
        filtered_tensions.iter().map(|t| t.id.as_str()).collect();

    fn render_tree(
        forest: &Forest,
        root_ids: &[String],
        filtered_ids: &std::collections::HashSet<&str>,
        now: chrono::DateTime<Utc>,
        prefix: &str,
        lines: &mut Vec<String>,
    ) {
        let mut roots: Vec<_> = root_ids
            .iter()
            .filter(|id| filtered_ids.contains(id.as_str()))
            .filter_map(|id| forest.find(id))
            .collect();

        // Sort by horizon (earliest first, None last)
        roots.sort_by(|a, b| match (&a.tension.horizon, &b.tension.horizon) {
            (Some(ha), Some(hb)) => ha.cmp(hb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        for (i, node) in roots.iter().enumerate() {
            let is_last = i == roots.len() - 1;
            let connector = if is_last { "└── " } else { "├── " };

            // Build line content
            let mut content = werk_shared::display_id(node.tension.short_code, node.id());

            // Status marker (only for non-Active)
            match node.tension.status {
                TensionStatus::Resolved => content.push_str(" ✓"),
                TensionStatus::Released => content.push_str(" ~"),
                TensionStatus::Active => {}
            }

            // Horizon and overdue
            if let Some(h) = &node.tension.horizon {
                content.push_str(&format!(" [{}]", h));
                if node.tension.status == TensionStatus::Active && h.is_past(now) {
                    content.push_str(" OVERDUE");
                }
            }

            // Containment violation (child deadline exceeds parent)
            if node.tension.status == TensionStatus::Active {
                if let Some(ref parent_id) = node.tension.parent_id {
                    let violations = detect_containment_violations(forest, parent_id);
                    if violations.iter().any(|v| v.tension_id == node.id()) {
                        content.push_str(" EXCEEDS_PARENT");
                    }
                }
                // Sequencing pressure
                if let Some(ref parent_id) = node.tension.parent_id {
                    let pressures = detect_sequencing_pressure(forest, parent_id);
                    if pressures.iter().any(|p| p.tension_id == node.id()) {
                        content.push_str(" PRESSURE");
                    }
                }
            }

            // Desired text
            content.push(' ');
            content.push_str(&truncate(&node.tension.desired, 50));

            // Theory of closure progress (from ALL children, not just filtered)
            let all_children = forest.children(node.id()).unwrap_or_default();
            if !all_children.is_empty() {
                let resolved_count = all_children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Resolved)
                    .count();
                content.push_str(&format!(" [{}/{}]", resolved_count, all_children.len()));
            }

            lines.push(format!("{}{}{}", prefix, connector, content));

            // Recurse for children (only those that pass the filter)
            let mut children: Vec<_> = node
                .children
                .iter()
                .filter(|id| filtered_ids.contains(id.as_str()))
                .filter_map(|id| forest.find(id))
                .collect();

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
                    now,
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
        now,
        "",
        &mut lines,
    );

    for line in &lines {
        println!("{}", line);
    }

    // Summary footer
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
        tensions.len(),
        active_count,
        resolved_count,
        released_count
    );

    Ok(())
}
