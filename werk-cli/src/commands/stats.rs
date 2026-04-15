//! Stats command handler — field-level summaries, aggregates, and analysis.
//!
//! The sole analysis surface. Default output: field vitals. Use flags to add
//! sections (temporal, attention, changes, trajectory, engagement, drift, health).

use chrono::{DateTime, Datelike, Utc};
use serde::Serialize;
use std::collections::HashMap;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use werk_core::{
    Forest, TensionStatus, compute_urgency, detect_containment_violations,
    detect_critical_path_recursive, detect_horizon_drift, detect_sequencing_pressure,
    project_field,
};
use werk_shared::cli_display::{Palette, glyphs};
use werk_shared::{format_short_code, truncate};

// ── JSON output ────────────────────────────────────────────────────

#[derive(Serialize)]
struct StatsJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    vitals: Option<VitalsJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temporal: Option<TemporalJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attention: Option<AttentionJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changes: Option<ChangesJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trajectory: Option<TrajectoryJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    engagement: Option<EngagementJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drift: Option<Vec<DriftEntryJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<HealthJson>,
}

#[derive(Serialize)]
struct VitalsJson {
    active: usize,
    resolved: usize,
    released: usize,
    deadlined: usize,
    overdue: usize,
    positioned: usize,
    held: usize,
    mutations: usize,
    tensions_touched: usize,
    avg_per_day: f64,
    period_days: i64,
}

#[derive(Serialize)]
struct ApproachingJson {
    short_code: Option<i32>,
    desired: String,
    deadline: String,
    urgency: f64,
}

#[derive(Serialize)]
struct CriticalPathJson {
    parent_short_code: Option<i32>,
    child_short_code: Option<i32>,
    child_desired: String,
    slack_days: i64,
}

#[derive(Serialize)]
struct PressureJson {
    short_code: Option<i32>,
    desired: String,
    predecessor_short_code: Option<i32>,
}

#[derive(Serialize)]
struct ViolationJson {
    short_code: Option<i32>,
    desired: String,
    parent_short_code: Option<i32>,
    excess_days: i64,
}

#[derive(Serialize)]
struct TemporalJson {
    approaching: Vec<ApproachingJson>,
    critical_path: Vec<CriticalPathJson>,
    sequencing_pressure: Vec<PressureJson>,
    containment_violations: Vec<ViolationJson>,
}

#[derive(Serialize)]
struct BranchJson {
    short_code: Option<i32>,
    desired: String,
    mutations: usize,
    tensions_touched: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    untouched_children: Option<usize>,
}

#[derive(Serialize)]
struct RootAttentionJson {
    short_code: Option<i32>,
    desired: String,
    total_mutations: usize,
    descendants_touched: usize,
    branches: Vec<BranchJson>,
}

#[derive(Serialize)]
struct AttentionJson {
    roots: Vec<RootAttentionJson>,
}

#[derive(Serialize)]
struct ChangesJson {
    epochs: Vec<ChangeEntryJson>,
    resolutions: Vec<ChangeEntryJson>,
    new_tensions: Vec<ChangeEntryJson>,
    reality_shifts: Vec<RealityShiftJson>,
}

#[derive(Serialize)]
struct ChangeEntryJson {
    short_code: Option<i32>,
    desired: String,
}

#[derive(Serialize)]
struct RealityShiftJson {
    short_code: Option<i32>,
    desired: String,
    preview: String,
}

#[derive(Serialize)]
struct TrajectoryJson {
    resolving: usize,
    drifting: usize,
    stalling: usize,
    oscillating: usize,
    collisions: Vec<CollisionJson>,
}

#[derive(Serialize)]
struct CollisionJson {
    tension_short_codes: Vec<Option<i32>>,
    window: String,
    peak_urgency: f64,
}

#[derive(Serialize)]
struct EngagementJson {
    field_frequency: f64,
    field_trend: String,
    most_engaged: Option<EngagedTensionJson>,
    least_engaged_with_deadline: Option<EngagedTensionJson>,
    /// Mutations per day of week, Monday-first (length 7).
    activity_by_weekday: [usize; 7],
}

#[derive(Serialize)]
struct EngagedTensionJson {
    short_code: Option<i32>,
    desired: String,
    frequency: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    deadline: Option<String>,
}

#[derive(Serialize)]
struct DriftEntryJson {
    short_code: Option<i32>,
    desired: String,
    drift_type: String,
    changes: usize,
    net_shift_days: i64,
}

#[derive(Serialize)]
struct HealthJson {
    noop_mutations: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    purged: Option<usize>,
}

// ── Command ────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn cmd_stats(
    output: &Output,
    temporal: bool,
    attention: bool,
    changes: bool,
    trajectory: bool,
    engagement: bool,
    drift: bool,
    health: bool,
    all: bool,
    days: i64,
    repair: bool,
    yes: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let now = Utc::now();
    let cutoff = now - chrono::Duration::days(days);
    let sig = crate::commands::signal_thresholds_from(&workspace);
    let analysis = crate::commands::analysis_thresholds_from(&workspace);

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;

    // Determine which sections to show
    let show_temporal = temporal || all;
    let show_attention = attention || all;
    let show_changes = changes || all;
    let show_trajectory = trajectory || all;
    let show_engagement = engagement || all;
    let show_drift = drift || all;
    let show_health = health || all;

    // Always show vitals
    let vitals = compute_vitals(&tensions, &all_mutations, cutoff, days, now);

    if output.is_structured() {
        let mut result = StatsJson {
            vitals: Some(vitals),
            temporal: None,
            attention: None,
            changes: None,
            trajectory: None,
            engagement: None,
            drift: None,
            health: None,
        };

        if show_temporal {
            result.temporal = Some(compute_temporal(&tensions, &store, now, &sig)?);
        }
        if show_attention {
            result.attention = Some(compute_attention(&tensions, &store, cutoff)?);
        }
        if show_changes {
            result.changes = Some(compute_changes(&tensions, &store, cutoff, now)?);
        }
        if show_trajectory {
            result.trajectory = Some(compute_trajectory(
                &tensions,
                &all_mutations,
                now,
                &analysis,
            )?);
        }
        if show_engagement {
            result.engagement = Some(compute_engagement(&tensions, &all_mutations, days, now)?);
        }
        if show_drift {
            result.drift = Some(compute_drift(&tensions, &store)?);
        }
        if show_health {
            result.health = Some(compute_health(&store, repair, yes)?);
        }

        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Text output
        let palette = output.palette();
        print_vitals(&vitals, &palette);

        if show_temporal {
            print_temporal(
                &compute_temporal(&tensions, &store, now, &sig)?,
                &tensions,
                &palette,
            );
        }
        if show_attention {
            print_attention(
                &compute_attention(&tensions, &store, cutoff)?,
                days,
                &palette,
            );
        }
        if show_changes {
            print_changes(&compute_changes(&tensions, &store, cutoff, now)?, days);
        }
        if show_trajectory {
            print_trajectory(&compute_trajectory(
                &tensions,
                &all_mutations,
                now,
                &analysis,
            )?);
        }
        if show_engagement {
            print_engagement(
                &compute_engagement(&tensions, &all_mutations, days, now)?,
                days,
                &palette,
            );
        }
        if show_drift {
            print_drift(&compute_drift(&tensions, &store)?);
        }
        if show_health {
            print_health(&compute_health(&store, repair, yes)?);
        }

        // Footer hint — point users at the most useful next gestures.
        let hint = if vitals.overdue > 0 {
            format!(
                "{} overdue — `werk list --overdue` to triage, `werk show <id>` to inspect",
                vitals.overdue
            )
        } else {
            "`werk stats --temporal` for deadlines, `werk stats --all` for everything".to_string()
        };
        crate::hints::print_hint(&palette, &hint);
    }

    Ok(())
}

// ── Vitals ─────────────────────────────────────────────────────────

fn compute_vitals(
    tensions: &[werk_core::Tension],
    mutations: &[werk_core::Mutation],
    cutoff: DateTime<Utc>,
    days: i64,
    now: DateTime<Utc>,
) -> VitalsJson {
    let active = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();
    let deadlined = tensions.iter().filter(|t| t.horizon.is_some()).count();
    let overdue = tensions
        .iter()
        .filter(|t| {
            t.status == TensionStatus::Active
                && t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false)
        })
        .count();
    let positioned = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active && t.position.is_some())
        .count();
    let held = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active && t.position.is_none())
        .count();

    let recent: Vec<&werk_core::Mutation> = mutations
        .iter()
        .filter(|m| m.timestamp() >= cutoff)
        .collect();
    let recent_count = recent.len();
    let mut touched: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for m in &recent {
        touched.insert(m.tension_id());
    }

    let avg = if days > 0 {
        recent_count as f64 / days as f64
    } else {
        0.0
    };

    VitalsJson {
        active,
        resolved,
        released,
        deadlined,
        overdue,
        positioned,
        held,
        mutations: recent_count,
        tensions_touched: touched.len(),
        avg_per_day: (avg * 10.0).round() / 10.0,
        period_days: days,
    }
}

fn print_vitals(v: &VitalsJson, palette: &Palette) {
    println!("{}", palette.bold(&palette.structure("Field")));
    // Number/role coloring: active stays at identity weight, resolved
    // is green to mirror the resolved status glyph elsewhere, released
    // is dimmed because completed-and-let-go is metadata. Overdue is
    // bold danger so it pops even in a dense vitals row.
    println!(
        "  {} active  {} resolved  {} released",
        v.active,
        palette.resolved(&v.resolved.to_string()),
        palette.chrome(&v.released.to_string()),
    );
    let overdue_display = if v.overdue > 0 {
        palette.bold(&palette.danger(&v.overdue.to_string()))
    } else {
        v.overdue.to_string()
    };
    println!(
        "  {} deadlined  {} overdue  {} positioned  {} held",
        v.deadlined, overdue_display, v.positioned, v.held,
    );
    println!(
        "  {}",
        palette.chrome(&format!(
            "Activity ({}d): {} mutations across {} tensions ({}/day)",
            v.period_days, v.mutations, v.tensions_touched, v.avg_per_day
        )),
    );
}

// ── Temporal ───────────────────────────────────────────────────────

fn compute_temporal(
    tensions: &[werk_core::Tension],
    _store: &werk_core::Store,
    now: DateTime<Utc>,
    sig: &werk_shared::SignalThresholds,
) -> Result<TemporalJson, WerkError> {
    let frame_end = now + chrono::Duration::days(sig.approaching_days);

    // Approaching
    let mut approaching: Vec<ApproachingJson> = Vec::new();
    for t in tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
    {
        if let Some(u) = compute_urgency(t, now) {
            let is_approaching = u.value > sig.approaching_urgency
                || t.horizon
                    .as_ref()
                    .map(|h| h.range_end() <= frame_end)
                    .unwrap_or(false);
            let is_overdue = t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false);
            if is_approaching || is_overdue {
                approaching.push(ApproachingJson {
                    short_code: t.short_code,
                    desired: t.desired.clone(),
                    deadline: t
                        .horizon
                        .as_ref()
                        .map(|h| h.to_string())
                        .unwrap_or_default(),
                    urgency: u.value,
                });
            }
        }
    }
    approaching.sort_by(|a, b| {
        b.urgency
            .partial_cmp(&a.urgency)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    approaching.truncate(10);

    // Critical path — collect from all roots
    let forest = Forest::from_tensions(tensions.to_vec())?;
    let root_ids: Vec<String> = tensions
        .iter()
        .filter(|t| t.parent_id.is_none() && t.status == TensionStatus::Active)
        .map(|t| t.id.clone())
        .collect();

    let tension_lookup: HashMap<String, &werk_core::Tension> =
        tensions.iter().map(|t| (t.id.clone(), t)).collect();

    let mut critical_path: Vec<CriticalPathJson> = Vec::new();
    for rid in &root_ids {
        let cps = detect_critical_path_recursive(&forest, rid, now);
        for cp in cps {
            let child = tension_lookup.get(&cp.tension_id);
            let parent = tension_lookup.get(&cp.parent_id);
            critical_path.push(CriticalPathJson {
                parent_short_code: parent.and_then(|t| t.short_code),
                child_short_code: child.and_then(|t| t.short_code),
                child_desired: child.map(|t| t.desired.clone()).unwrap_or_default(),
                slack_days: cp.slack_seconds / 86400,
            });
        }
    }

    // Sequencing pressure — collect from all parents with children
    let parent_ids: Vec<String> = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .map(|t| t.id.clone())
        .collect();

    let mut seq_pressure: Vec<PressureJson> = Vec::new();
    for pid in &parent_ids {
        let sps = detect_sequencing_pressure(&forest, pid);
        for sp in sps {
            let t = tension_lookup.get(&sp.tension_id);
            seq_pressure.push(PressureJson {
                short_code: t.and_then(|t| t.short_code),
                desired: t.map(|t| t.desired.clone()).unwrap_or_default(),
                predecessor_short_code: sp.predecessor_short_code,
            });
        }
    }

    // Containment violations
    let mut violations: Vec<ViolationJson> = Vec::new();
    for pid in &parent_ids {
        let cvs = detect_containment_violations(&forest, pid);
        for cv in cvs {
            let child = tension_lookup.get(&cv.tension_id);
            let parent = tension_lookup.get(&cv.parent_id);
            violations.push(ViolationJson {
                short_code: child.and_then(|t| t.short_code),
                desired: child.map(|t| t.desired.clone()).unwrap_or_default(),
                parent_short_code: parent.and_then(|t| t.short_code),
                excess_days: cv.excess_seconds / 86400,
            });
        }
    }

    Ok(TemporalJson {
        approaching,
        critical_path,
        sequencing_pressure: seq_pressure,
        containment_violations: violations,
    })
}

fn print_temporal(t: &TemporalJson, _tensions: &[werk_core::Tension], palette: &Palette) {
    if t.approaching.is_empty()
        && t.critical_path.is_empty()
        && t.sequencing_pressure.is_empty()
        && t.containment_violations.is_empty()
    {
        return;
    }

    println!();
    println!("{}", palette.bold(&palette.structure("Temporal situation")));

    if !t.approaching.is_empty() {
        println!("  Approaching (next 14 days)");
        for a in &t.approaching {
            let sc = format_short_code(a.short_code);
            println!(
                "    {:<6} {} [{}]  urgency {:.0}%",
                sc,
                truncate(&a.desired, 35),
                a.deadline,
                a.urgency * 100.0
            );
        }
    }

    if !t.critical_path.is_empty() {
        println!("  Critical path");
        for cp in &t.critical_path {
            let psc = format_short_code(cp.parent_short_code);
            let csc = format_short_code(cp.child_short_code);
            println!(
                "    {} \u{2192} {}  {} (slack {}d)",
                psc,
                csc,
                truncate(&cp.child_desired, 30),
                cp.slack_days
            );
        }
    }

    if !t.sequencing_pressure.is_empty() {
        println!("  Sequencing pressure");
        for sp in &t.sequencing_pressure {
            let sc = format_short_code(sp.short_code);
            let psc = format_short_code(sp.predecessor_short_code);
            println!(
                "    {}  {} — ordered after {} but due earlier",
                sc,
                truncate(&sp.desired, 30),
                psc
            );
        }
    }

    if !t.containment_violations.is_empty() {
        println!("  Containment violations");
        for v in &t.containment_violations {
            let sc = format_short_code(v.short_code);
            let psc = format_short_code(v.parent_short_code);
            println!(
                "    {}  {} exceeds parent {} by {}d",
                sc,
                truncate(&v.desired, 30),
                psc,
                v.excess_days
            );
        }
    }
}

// ── Attention ──────────────────────────────────────────────────────

fn compute_attention(
    tensions: &[werk_core::Tension],
    store: &werk_core::Store,
    cutoff: DateTime<Utc>,
) -> Result<AttentionJson, WerkError> {
    let roots: Vec<&werk_core::Tension> = tensions
        .iter()
        .filter(|t| t.parent_id.is_none() && t.status == TensionStatus::Active)
        .collect();

    let mut root_attentions = Vec::new();

    for root in &roots {
        let mut total_mutations = 0usize;
        let mut descendants_touched = 0usize;

        // Count mutations for root itself
        let root_muts = store
            .get_mutations(&root.id)
            .map_err(WerkError::StoreError)?;
        let root_recent = root_muts.iter().filter(|m| m.timestamp() >= cutoff).count();
        if root_recent > 0 {
            total_mutations += root_recent;
            descendants_touched += 1;
        }

        // Per-branch (immediate children of root)
        let children: Vec<&werk_core::Tension> = tensions
            .iter()
            .filter(|t| {
                t.parent_id.as_deref() == Some(&root.id) && t.status == TensionStatus::Active
            })
            .collect();

        let mut branches = Vec::new();

        for child in &children {
            let child_desc_ids = store
                .get_descendant_ids(&child.id)
                .map_err(WerkError::StoreError)?;

            let mut branch_mutations = 0usize;
            let mut branch_touched = 0usize;

            // Count child's own mutations
            let child_muts = store
                .get_mutations(&child.id)
                .map_err(WerkError::StoreError)?;
            let child_recent = child_muts
                .iter()
                .filter(|m| m.timestamp() >= cutoff)
                .count();
            if child_recent > 0 {
                branch_mutations += child_recent;
                branch_touched += 1;
            }

            // Count descendant mutations
            for did in &child_desc_ids {
                let d_muts = store.get_mutations(did).map_err(WerkError::StoreError)?;
                let d_recent = d_muts.iter().filter(|m| m.timestamp() >= cutoff).count();
                if d_recent > 0 {
                    branch_mutations += d_recent;
                    branch_touched += 1;
                }
            }

            total_mutations += branch_mutations;
            descendants_touched += branch_touched;

            // Count total children for untouched annotation
            let total_children = tensions
                .iter()
                .filter(|t| {
                    t.parent_id.as_deref() == Some(&child.id) && t.status == TensionStatus::Active
                })
                .count();

            let untouched = if branch_mutations == 0 && total_children > 0 {
                Some(total_children)
            } else {
                None
            };

            branches.push(BranchJson {
                short_code: child.short_code,
                desired: child.desired.clone(),
                mutations: branch_mutations,
                tensions_touched: branch_touched,
                untouched_children: untouched,
            });
        }

        branches.sort_by(|a, b| b.mutations.cmp(&a.mutations));

        root_attentions.push(RootAttentionJson {
            short_code: root.short_code,
            desired: root.desired.clone(),
            total_mutations,
            descendants_touched,
            branches,
        });
    }

    root_attentions.sort_by(|a, b| b.total_mutations.cmp(&a.total_mutations));

    Ok(AttentionJson {
        roots: root_attentions,
    })
}

fn print_attention(a: &AttentionJson, days: i64, palette: &Palette) {
    if a.roots.is_empty() {
        return;
    }

    println!();
    println!(
        "{}",
        palette.bold(&palette.structure(&format!("Attention (last {} days)", days)))
    );

    for root in &a.roots {
        let sc = format_short_code(root.short_code);
        println!(
            "  {:<6} {:<35}  {} mutations across {} descendants",
            sc,
            truncate(&root.desired, 35),
            root.total_mutations,
            root.descendants_touched
        );

        for branch in &root.branches {
            let bsc = format_short_code(branch.short_code);
            let untouched = branch
                .untouched_children
                .map(|n| format!(" \u{2190} {} children, none touched", n))
                .unwrap_or_default();
            println!(
                "    {:<6} {:<33}  {} mutations ({} tensions){}",
                bsc,
                truncate(&branch.desired, 33),
                branch.mutations,
                branch.tensions_touched,
                untouched,
            );
        }
    }
}

// ── Changes ────────────────────────────────────────────────────────

fn compute_changes(
    tensions: &[werk_core::Tension],
    store: &werk_core::Store,
    cutoff: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<ChangesJson, WerkError> {
    let tension_map: HashMap<String, &werk_core::Tension> =
        tensions.iter().map(|t| (t.id.clone(), t)).collect();

    let mutations = store
        .mutations_between(cutoff, now)
        .map_err(WerkError::StoreError)?;

    // Epochs
    let mut epochs = Vec::new();
    for t in tensions {
        let eps = store.get_epochs(&t.id).map_err(WerkError::StoreError)?;
        for ep in &eps {
            if ep.timestamp >= cutoff {
                epochs.push(ChangeEntryJson {
                    short_code: t.short_code,
                    desired: t.desired.clone(),
                });
            }
        }
    }

    // Resolutions
    let mut resolutions = Vec::new();
    let mut new_tensions = Vec::new();
    let mut reality_shifts = Vec::new();
    let mut seen_resolution: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_created: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_reality: std::collections::HashSet<String> = std::collections::HashSet::new();

    for m in &mutations {
        let t = tension_map.get(m.tension_id());
        let sc = t.and_then(|t| t.short_code);
        let desired = t.map(|t| t.desired.as_str()).unwrap_or("(deleted)");

        if m.field() == "status"
            && (m.new_value() == "Resolved" || m.new_value() == "Released")
            && seen_resolution.insert(m.tension_id().to_string())
        {
            resolutions.push(ChangeEntryJson {
                short_code: sc,
                desired: desired.to_string(),
            });
        }

        if m.field() == "created" && seen_created.insert(m.tension_id().to_string()) {
            new_tensions.push(ChangeEntryJson {
                short_code: sc,
                desired: desired.to_string(),
            });
        }

        if m.field() == "actual" && seen_reality.insert(m.tension_id().to_string()) {
            let preview = {
                let val = m.new_value();
                if val.len() > 60 {
                    let end = val.floor_char_boundary(57);
                    format!("{}...", &val[..end])
                } else {
                    val.to_string()
                }
            };
            reality_shifts.push(RealityShiftJson {
                short_code: sc,
                desired: desired.to_string(),
                preview,
            });
        }
    }

    // Limit reality shifts to 5 most recent
    reality_shifts.truncate(5);

    Ok(ChangesJson {
        epochs,
        resolutions,
        new_tensions,
        reality_shifts,
    })
}

fn print_changes(c: &ChangesJson, days: i64) {
    if c.epochs.is_empty()
        && c.resolutions.is_empty()
        && c.new_tensions.is_empty()
        && c.reality_shifts.is_empty()
    {
        return;
    }

    println!();
    println!("Structural changes (last {} days)", days);

    if !c.epochs.is_empty() {
        println!("  Epochs");
        for e in &c.epochs {
            let sc = format_short_code(e.short_code);
            println!("    {:<6} {}", sc, truncate(&e.desired, 55));
        }
    }

    if !c.resolutions.is_empty() {
        println!("  Resolutions");
        for r in &c.resolutions {
            let sc = format_short_code(r.short_code);
            println!("    {:<6} {}", sc, truncate(&r.desired, 55));
        }
    }

    if !c.new_tensions.is_empty() {
        println!("  New tensions");
        for n in &c.new_tensions {
            let sc = format_short_code(n.short_code);
            println!("    {:<6} {}", sc, truncate(&n.desired, 55));
        }
    }

    if !c.reality_shifts.is_empty() {
        println!("  Reality shifts");
        for r in &c.reality_shifts {
            let sc = format_short_code(r.short_code);
            println!("    {:<6} \"{}\"", sc, truncate(&r.preview, 55));
        }
    }
}

// ── Trajectory ─────────────────────────────────────────────────────

fn compute_trajectory(
    tensions: &[werk_core::Tension],
    mutations: &[werk_core::Mutation],
    now: DateTime<Utc>,
    analysis: &werk_shared::AnalysisThresholds,
) -> Result<TrajectoryJson, WerkError> {
    let thresholds = crate::commands::to_projection_thresholds(analysis);
    let field = project_field(tensions, mutations, &thresholds, now);

    let mut resolving = 0;
    let mut drifting = 0;
    let mut stalling = 0;
    let mut oscillating = 0;

    if let Some(buckets) = field.funnel.get(&werk_core::ProjectionHorizon::OneWeek) {
        resolving = buckets.resolving;
        drifting = buckets.drifting;
        stalling = buckets.stalling;
        oscillating = buckets.oscillating;
    }

    let tension_map: HashMap<String, &werk_core::Tension> =
        tensions.iter().map(|t| (t.id.clone(), t)).collect();

    let collisions: Vec<CollisionJson> = field
        .urgency_collisions
        .iter()
        .map(|c| {
            let scs: Vec<Option<i32>> = c
                .tension_ids
                .iter()
                .map(|id| tension_map.get(id).and_then(|t| t.short_code))
                .collect();
            CollisionJson {
                tension_short_codes: scs,
                window: format!(
                    "{} to {}",
                    c.window_start.format("%Y-%m-%d"),
                    c.window_end.format("%Y-%m-%d")
                ),
                peak_urgency: c.peak_combined_urgency,
            }
        })
        .collect();

    Ok(TrajectoryJson {
        resolving,
        drifting,
        stalling,
        oscillating,
        collisions,
    })
}

fn print_trajectory(t: &TrajectoryJson) {
    println!();
    println!("Analysis (practice layer \u{2014} instrument-originated thresholds)");
    println!(
        "  Trajectory: {} resolving  {} drifting  {} stalling  {} oscillating",
        t.resolving, t.drifting, t.stalling, t.oscillating
    );

    if !t.collisions.is_empty() {
        println!("  Urgency collisions");
        for c in &t.collisions {
            let ids: Vec<String> = c
                .tension_short_codes
                .iter()
                .map(|sc| sc.map(|c| format!("#{}", c)).unwrap_or("?".to_string()))
                .collect();
            println!(
                "    {}  [{}]  combined urgency {:.2}",
                ids.join(" + "),
                c.window,
                c.peak_urgency
            );
        }
    }
}

// ── Engagement ─────────────────────────────────────────────────────

fn compute_engagement(
    tensions: &[werk_core::Tension],
    mutations: &[werk_core::Mutation],
    days: i64,
    now: DateTime<Utc>,
) -> Result<EngagementJson, WerkError> {
    let cutoff = now - chrono::Duration::days(days);
    let recent: Vec<&werk_core::Mutation> = mutations
        .iter()
        .filter(|m| m.timestamp() >= cutoff)
        .collect();

    let field_freq = if days > 0 {
        recent.len() as f64 / days as f64
    } else {
        0.0
    };

    // Compute per-tension frequencies
    let mut per_tension: HashMap<String, usize> = HashMap::new();
    for m in &recent {
        *per_tension.entry(m.tension_id().to_string()).or_default() += 1;
    }

    let tension_map: HashMap<String, &werk_core::Tension> =
        tensions.iter().map(|t| (t.id.clone(), t)).collect();

    let most_engaged =
        per_tension
            .iter()
            .max_by_key(|(_, count)| *count)
            .and_then(|(id, count)| {
                tension_map.get(id).map(|t| EngagedTensionJson {
                    short_code: t.short_code,
                    desired: t.desired.clone(),
                    frequency: *count as f64 / days.max(1) as f64,
                    deadline: None,
                })
            });

    // Least engaged with deadline — active, has deadline, lowest mutation count
    let least_engaged = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active && t.horizon.is_some())
        .min_by_key(|t| per_tension.get(&t.id).copied().unwrap_or(0))
        .map(|t| {
            let count = per_tension.get(&t.id).copied().unwrap_or(0);
            EngagedTensionJson {
                short_code: t.short_code,
                desired: t.desired.clone(),
                frequency: count as f64 / days.max(1) as f64,
                deadline: t.horizon.as_ref().map(|h| h.to_string()),
            }
        });

    // Simple trend: compare first half vs second half of period
    let mid = cutoff + chrono::Duration::days(days / 2);
    let first_half = recent.iter().filter(|m| m.timestamp() < mid).count();
    let second_half = recent.len() - first_half;
    let trend = if second_half as f64 > first_half as f64 * 1.2 {
        "increasing".to_string()
    } else if (first_half as f64) > second_half as f64 * 1.2 {
        "decreasing".to_string()
    } else {
        "steady".to_string()
    };

    let mut activity_by_weekday = [0usize; 7];
    for m in &recent {
        let wd = m.timestamp().weekday().num_days_from_monday() as usize;
        activity_by_weekday[wd] += 1;
    }

    Ok(EngagementJson {
        field_frequency: (field_freq * 10.0).round() / 10.0,
        field_trend: trend,
        most_engaged,
        least_engaged_with_deadline: least_engaged,
        activity_by_weekday,
    })
}

fn print_engagement(e: &EngagementJson, _days: i64, palette: &Palette) {
    println!(
        "  Field frequency: {}/day ({})",
        e.field_frequency, e.field_trend
    );
    if let Some(ref me) = e.most_engaged {
        let sc = format_short_code(me.short_code);
        println!(
            "  Most engaged: {} {} ({:.1}/day)",
            sc,
            truncate(&me.desired, 35),
            me.frequency
        );
    }
    if let Some(ref le) = e.least_engaged_with_deadline {
        let sc = format_short_code(le.short_code);
        let dl = le.deadline.as_deref().unwrap_or("");
        println!(
            "  Least engaged (deadlined): {} {} ({:.1}/day, due {})",
            sc,
            truncate(&le.desired, 25),
            le.frequency,
            dl
        );
    }

    if e.activity_by_weekday.iter().any(|&c| c > 0) {
        let max = *e.activity_by_weekday.iter().max().unwrap_or(&0);
        let names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let parts: Vec<String> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                format!(
                    "{} {}",
                    palette.chrome(name),
                    bar_inline(e.activity_by_weekday[i], max, 5, palette)
                )
            })
            .collect();
        println!("  Activity by day: {}", parts.join("  "));
    }
}

fn bar_inline(count: usize, max: usize, width: usize, palette: &Palette) -> String {
    if max == 0 {
        return palette.chrome(&glyphs::BAR_EMPTY.repeat(width));
    }
    let filled = ((count as f64 / max as f64) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        palette.resolved(&glyphs::BAR_FULL.repeat(filled)),
        palette.chrome(&glyphs::BAR_EMPTY.repeat(empty)),
    )
}

// ── Drift ──────────────────────────────────────────────────────────

fn compute_drift(
    tensions: &[werk_core::Tension],
    store: &werk_core::Store,
) -> Result<Vec<DriftEntryJson>, WerkError> {
    let mut drifts = Vec::new();

    for t in tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active && t.horizon.is_some())
    {
        let mutations = store.get_mutations(&t.id).map_err(WerkError::StoreError)?;
        let drift = detect_horizon_drift(&t.id, &mutations);

        let drift_type_str = format!("{:?}", drift.drift_type);
        if drift_type_str != "Stable" && drift.change_count > 0 {
            drifts.push(DriftEntryJson {
                short_code: t.short_code,
                desired: t.desired.clone(),
                drift_type: drift_type_str,
                changes: drift.change_count,
                net_shift_days: drift.net_shift_seconds / 86400,
            });
        }
    }

    // Sort by severity: repeated postponement first
    drifts.sort_by(|a, b| b.changes.cmp(&a.changes));

    Ok(drifts)
}

fn print_drift(drifts: &[DriftEntryJson]) {
    if drifts.is_empty() {
        return;
    }

    println!("  Horizon drift");
    for d in drifts {
        let sc = format_short_code(d.short_code);
        println!(
            "    {:<6} {} \u{2014} {} ({} shifts, net {}d)",
            sc,
            truncate(&d.desired, 30),
            d.drift_type,
            d.changes,
            d.net_shift_days
        );
    }
}

// ── Health ──────────────────────────────────────────────────────────

fn compute_health(
    store: &werk_core::Store,
    repair: bool,
    yes: bool,
) -> Result<HealthJson, WerkError> {
    let noop_count = store.count_noop_mutations().map_err(WerkError::CoreError)?;

    let purged = if repair && noop_count > 0 {
        if yes || confirm_repair(noop_count) {
            let purged = store.purge_noop_mutations().map_err(WerkError::CoreError)?;
            Some(purged)
        } else {
            None
        }
    } else {
        None
    };

    Ok(HealthJson {
        noop_mutations: noop_count,
        purged,
    })
}

fn confirm_repair(count: usize) -> bool {
    use std::io::Write;
    print!("Purge {} noop mutation(s)? [y/N] ", count);
    std::io::stdout().flush().ok();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        input.trim().eq_ignore_ascii_case("y")
    } else {
        false
    }
}

fn print_health(h: &HealthJson) {
    println!();
    println!("Health");
    if h.noop_mutations == 0 {
        println!("  No issues found");
    } else {
        println!("  {} noop mutation(s)", h.noop_mutations);
        if let Some(purged) = h.purged {
            println!("  Purged {} noop mutation(s)", purged);
        }
    }
}
