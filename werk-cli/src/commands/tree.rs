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

    /// Compute the display width of a string (ASCII chars = 1, most Unicode = 1).
    /// This is a simple approximation — good enough for box-drawing and common text.
    fn display_width(s: &str) -> usize {
        s.chars().count()
    }

    fn render_tree(
        forest: &Forest,
        root_ids: &[String],
        filtered_ids: &std::collections::HashSet<&str>,
        now: chrono::DateTime<Utc>,
        prefix: &str,
        term_width: usize,
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

            // === Left side (plan-facing): id, status, horizon, signals, desired text ===
            let mut left_prefix_parts = werk_shared::display_id(node.tension.short_code, node.id());

            // Status marker (only for non-Active)
            match node.tension.status {
                TensionStatus::Resolved => left_prefix_parts.push_str(" ✓"),
                TensionStatus::Released => left_prefix_parts.push_str(" ~"),
                TensionStatus::Active => {}
            }

            // Horizon and overdue
            if let Some(h) = &node.tension.horizon {
                left_prefix_parts.push_str(&format!(" [{}]", h));
                if node.tension.status == TensionStatus::Active && h.is_past(now) {
                    left_prefix_parts.push_str(" OVERDUE");
                }
            }

            // Containment violation
            if node.tension.status == TensionStatus::Active {
                if let Some(ref parent_id) = node.tension.parent_id {
                    let violations = detect_containment_violations(forest, parent_id);
                    if violations.iter().any(|v| v.tension_id == node.id()) {
                        left_prefix_parts.push_str(" EXCEEDS_PARENT");
                    }
                }
                // Sequencing pressure
                if let Some(ref parent_id) = node.tension.parent_id {
                    let pressures = detect_sequencing_pressure(forest, parent_id);
                    if pressures.iter().any(|p| p.tension_id == node.id()) {
                        left_prefix_parts.push_str(" PRESSURE");
                    }
                }
            }

            left_prefix_parts.push(' ');

            // === Right side (trace-facing): closure progress, released ===
            let all_children = forest.children(node.id()).unwrap_or_default();
            let right = if !all_children.is_empty() {
                let resolved_count = all_children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Resolved)
                    .count();
                let released_count = all_children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Released)
                    .count();
                let active_count = all_children.len() - released_count;
                if released_count > 0 {
                    format!(" [{}/{}] ({} released)", resolved_count, active_count, released_count)
                } else {
                    format!(" [{}/{}]", resolved_count, active_count)
                }
            } else {
                String::new()
            };

            // Compute available space for desired text
            let tree_prefix_width = display_width(prefix) + display_width(connector);
            let left_prefix_width = display_width(&left_prefix_parts);
            let right_width = display_width(&right);
            // Reserve: tree prefix + left prefix + at least 10 chars of text + gap + right
            let min_text = 10;
            let used = tree_prefix_width + left_prefix_width + right_width + 1; // +1 for gap
            let available_for_text = if term_width > used {
                term_width - used
            } else {
                min_text
            };
            let text_max = available_for_text.max(min_text);
            let desired_text = truncate(&node.tension.desired, text_max);
            let desired_width = display_width(&desired_text);

            // Assemble: fill gap between left content and right-aligned trace
            let left_total = tree_prefix_width + left_prefix_width + desired_width;
            let line = if !right.is_empty() && term_width > left_total + right_width {
                let gap = term_width - left_total - right_width;
                format!(
                    "{}{}{}{}{}{}",
                    prefix, connector, left_prefix_parts, desired_text,
                    " ".repeat(gap), right
                )
            } else {
                // Narrow terminal or no right content — just concatenate
                format!("{}{}{}{}{}", prefix, connector, left_prefix_parts, desired_text, right)
            };

            lines.push(line);

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
                    term_width,
                    lines,
                );
            }
        }
    }

    let term_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    let mut lines = Vec::new();
    render_tree(
        &forest,
        forest.root_ids(),
        &filtered_ids,
        now,
        "",
        term_width,
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
