//! Aggregate "field" view across registered spaces.
//!
//! Primitives for pooling per-workspace signals into a field-scoped view —
//! the aggregate command center promised by #242. Two shapes:
//!
//! - **Pure primitives** over an already-opened [`Store`]:
//!   [`compute_vitals_for_store`], [`compute_attention_for_store`]. The daemon
//!   calls these from each StoreHandle's owning thread.
//! - **CLI helpers** that open every space sequentially on the calling thread,
//!   compute, and drop: [`compute_aggregate_vitals`],
//!   [`compute_aggregate_attention`]. Suited for short-lived CLI invocations.
//!
//! Locality is enforced by the API shape: we only sum per-space counts and
//! tag attention items by space. There is no cross-space ranking, no
//! cross-space blocked inference. Cross-space addressing is #100's concern.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

use werk_core::{Store, StoreError, Tension, TensionStatus, compute_urgency};

use crate::error::Result;
use crate::registry::{self, GLOBAL_NAME, Registry};

/// Default "next up" items per space, picked so one noisy space can't dominate
/// the pooled band. Tuned for the visual bandwidth of a CLI line list.
pub const DEFAULT_NEXT_UP_PER_SPACE: usize = 5;
/// Default "held" items per space — held is informational, not urgent.
pub const DEFAULT_HELD_PER_SPACE: usize = 3;

/// A space included in the aggregate.
#[derive(Debug, Clone, Serialize)]
pub struct SpaceRef {
    pub name: String,
    pub path: PathBuf,
    pub is_global: bool,
}

/// Vitals counted for a single space.
#[derive(Debug, Clone, Serialize)]
pub struct SpaceVitals {
    pub space: SpaceRef,
    pub active: usize,
    pub resolved: usize,
    pub released: usize,
    pub deadlined: usize,
    pub overdue: usize,
    pub positioned: usize,
    pub held: usize,
    pub last_activity: Option<DateTime<Utc>>,
}

/// Sum of per-space vitals. No cross-space ratios, averages, or rankings —
/// sums are the only cross-space computation that respects locality.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VitalsTotals {
    pub active: usize,
    pub resolved: usize,
    pub released: usize,
    pub deadlined: usize,
    pub overdue: usize,
    pub positioned: usize,
    pub held: usize,
}

/// A registry entry whose workspace could not be read. Surfaced rather than
/// silently dropped so the user can see what was left out.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedSpace {
    pub name: String,
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregateVitals {
    pub computed_at: DateTime<Utc>,
    pub spaces: Vec<SpaceVitals>,
    pub totals: VitalsTotals,
    pub skipped: Vec<SkippedSpace>,
}

/// One attention-band item, tagged by space so a pooled display can
/// disambiguate `[werk:#42]` from `[global:#42]`.
#[derive(Debug, Clone, Serialize)]
pub struct AttentionItem {
    pub space_name: String,
    pub short_code: Option<i32>,
    pub desired: String,
    pub horizon: Option<String>,
    pub urgency: Option<f64>,
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregateAttention {
    pub computed_at: DateTime<Utc>,
    pub overdue: Vec<AttentionItem>,
    pub next_up: Vec<AttentionItem>,
    pub held: Vec<AttentionItem>,
    pub skipped: Vec<SkippedSpace>,
}

/// Enumerate every space the aggregate should consult.
///
/// Starts with the global space (`~/.werk/`), then every registered workspace
/// deduplicated by absolute path. Registry entries whose `.werk/` directory
/// no longer exists are reported as `skipped` — they never fail the call.
/// Mirrors [`crate::daemon_workspaces::list`]'s "skip stale, don't wedge"
/// behavior.
pub fn enumerate_spaces() -> Result<(Vec<SpaceRef>, Vec<SkippedSpace>)> {
    let mut spaces: Vec<SpaceRef> = Vec::new();
    let mut skipped: Vec<SkippedSpace> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    let global = registry::global_entry()?;
    if global.path.join(".werk").exists() {
        seen.insert(global.path.clone());
        spaces.push(SpaceRef {
            name: GLOBAL_NAME.to_string(),
            path: global.path,
            is_global: true,
        });
    } else {
        skipped.push(SkippedSpace {
            name: GLOBAL_NAME.to_string(),
            path: global.path,
            reason: "no .werk/ directory".to_string(),
        });
    }

    let reg = Registry::load()?;
    for entry in reg.list() {
        if !seen.insert(entry.path.clone()) {
            continue;
        }
        if !entry.path.join(".werk").exists() {
            skipped.push(SkippedSpace {
                name: entry.name,
                path: entry.path,
                reason: "registry entry stale (no .werk/)".to_string(),
            });
            continue;
        }
        spaces.push(SpaceRef {
            name: entry.name,
            path: entry.path,
            is_global: false,
        });
    }

    Ok((spaces, skipped))
}

/// Compute vitals for one already-opened Store. Pure over the Store.
pub fn compute_vitals_for_store(
    space: SpaceRef,
    store: &Store,
    now: DateTime<Utc>,
) -> std::result::Result<SpaceVitals, StoreError> {
    let tensions = store.list_tensions()?;

    let mut active = 0;
    let mut resolved = 0;
    let mut released = 0;
    let mut deadlined = 0;
    let mut overdue = 0;
    let mut positioned = 0;
    let mut held = 0;

    for t in &tensions {
        match t.status {
            TensionStatus::Active => active += 1,
            TensionStatus::Resolved => resolved += 1,
            TensionStatus::Released => released += 1,
        }
        if t.horizon.is_some() {
            deadlined += 1;
        }
        if t.status == TensionStatus::Active {
            if let Some(h) = &t.horizon
                && h.is_past(now)
            {
                overdue += 1;
            }
            if t.position.is_some() {
                positioned += 1;
            } else {
                held += 1;
            }
        }
    }

    // Last activity = most recent mutation across the store. all_mutations()
    // returns ASC order, so the tail holds the max. Cheap at our scale; if
    // it ever hurts, add a dedicated MAX(timestamp) query.
    let last_activity = store.all_mutations()?.last().map(|m| m.timestamp());

    Ok(SpaceVitals {
        space,
        active,
        resolved,
        released,
        deadlined,
        overdue,
        positioned,
        held,
        last_activity,
    })
}

/// Compute attention bands for one already-opened Store.
///
/// Returns `(overdue, next_up, held)`, each item pre-tagged with the space name.
/// Caps apply per-space so a single noisy space can't dominate the pooled view.
pub fn compute_attention_for_store(
    space: &SpaceRef,
    store: &Store,
    now: DateTime<Utc>,
    next_up_per_space: usize,
    held_per_space: usize,
) -> std::result::Result<
    (
        Vec<AttentionItem>,
        Vec<AttentionItem>,
        Vec<AttentionItem>,
    ),
    StoreError,
> {
    let tensions = store.list_tensions()?;

    let mut overdue: Vec<AttentionItem> = Vec::new();
    let mut next_up: Vec<AttentionItem> = Vec::new();
    let mut held: Vec<AttentionItem> = Vec::new();

    for t in &tensions {
        if t.status != TensionStatus::Active {
            continue;
        }
        let urgency = compute_urgency(t, now).map(|u| u.value);

        if let Some(h) = &t.horizon
            && h.is_past(now)
        {
            overdue.push(to_item(&space.name, t, urgency));
            continue;
        }

        if t.position.is_some() {
            next_up.push(to_item(&space.name, t, urgency));
        } else {
            held.push(to_item(&space.name, t, urgency));
        }
    }

    // Overdue is an exception — no per-space cap. Sort by urgency desc so the
    // most-past items float up.
    overdue.sort_by(urgency_desc);

    // Next up: sort by position ascending (intra-space), then cap.
    next_up.sort_by_key(|i| i.position.unwrap_or(i32::MAX));
    next_up.truncate(next_up_per_space);

    // Held: sort by urgency desc, cap.
    held.sort_by(urgency_desc);
    held.truncate(held_per_space);

    Ok((overdue, next_up, held))
}

fn to_item(space_name: &str, t: &Tension, urgency: Option<f64>) -> AttentionItem {
    AttentionItem {
        space_name: space_name.to_string(),
        short_code: t.short_code,
        desired: t.desired.clone(),
        horizon: t.horizon.as_ref().map(|h| h.to_string()),
        urgency,
        position: t.position,
    }
}

fn urgency_desc(a: &AttentionItem, b: &AttentionItem) -> std::cmp::Ordering {
    b.urgency
        .unwrap_or(0.0)
        .partial_cmp(&a.urgency.unwrap_or(0.0))
        .unwrap_or(std::cmp::Ordering::Equal)
}

/// CLI-side aggregate vitals. Opens each space sequentially on the calling
/// thread, computes, drops. Locality-safe: no cross-space thread sharing
/// required, which sidesteps `Store`'s `!Send` constraint.
pub fn compute_aggregate_vitals(now: DateTime<Utc>) -> Result<AggregateVitals> {
    let (spaces, mut skipped) = enumerate_spaces()?;
    let mut per_space: Vec<SpaceVitals> = Vec::new();

    for space in spaces {
        match open_store_for_read(&space.path) {
            Ok(store) => match compute_vitals_for_store(space.clone(), &store, now) {
                Ok(v) => per_space.push(v),
                Err(e) => skipped.push(SkippedSpace {
                    name: space.name,
                    path: space.path,
                    reason: format!("read failed: {e}"),
                }),
            },
            Err(reason) => skipped.push(SkippedSpace {
                name: space.name,
                path: space.path,
                reason,
            }),
        }
    }

    let totals = per_space.iter().fold(VitalsTotals::default(), |mut acc, v| {
        acc.active += v.active;
        acc.resolved += v.resolved;
        acc.released += v.released;
        acc.deadlined += v.deadlined;
        acc.overdue += v.overdue;
        acc.positioned += v.positioned;
        acc.held += v.held;
        acc
    });

    Ok(AggregateVitals {
        computed_at: now,
        spaces: per_space,
        totals,
        skipped,
    })
}

/// CLI-side aggregate attention. Mirrors [`compute_aggregate_vitals`].
pub fn compute_aggregate_attention(
    now: DateTime<Utc>,
    next_up_per_space: usize,
    held_per_space: usize,
) -> Result<AggregateAttention> {
    let (spaces, mut skipped) = enumerate_spaces()?;
    let mut overdue: Vec<AttentionItem> = Vec::new();
    let mut next_up: Vec<AttentionItem> = Vec::new();
    let mut held: Vec<AttentionItem> = Vec::new();

    for space in &spaces {
        match open_store_for_read(&space.path) {
            Ok(store) => match compute_attention_for_store(
                space,
                &store,
                now,
                next_up_per_space,
                held_per_space,
            ) {
                Ok((o, n, h)) => {
                    overdue.extend(o);
                    next_up.extend(n);
                    held.extend(h);
                }
                Err(e) => skipped.push(SkippedSpace {
                    name: space.name.clone(),
                    path: space.path.clone(),
                    reason: format!("read failed: {e}"),
                }),
            },
            Err(reason) => skipped.push(SkippedSpace {
                name: space.name.clone(),
                path: space.path.clone(),
                reason,
            }),
        }
    }

    // Final pooled display ordering. Pure presentation — every item's position,
    // horizon, and urgency was set inside its own space's standard. We do not
    // infer cross-space importance.
    overdue.sort_by(urgency_desc);
    next_up.sort_by(|a, b| {
        a.position
            .unwrap_or(i32::MAX)
            .cmp(&b.position.unwrap_or(i32::MAX))
            .then_with(|| a.space_name.cmp(&b.space_name))
    });
    held.sort_by(urgency_desc);

    Ok(AggregateAttention {
        computed_at: now,
        overdue,
        next_up,
        held,
        skipped,
    })
}

fn open_store_for_read(path: &std::path::Path) -> std::result::Result<Store, String> {
    // init_unlocked skips backup rotation — right for short-lived reads.
    // Schema creation is idempotent, so hitting an existing db is safe.
    Store::init_unlocked(path).map_err(|e| format!("open failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use werk_core::{Horizon, Store};

    fn in_memory_store() -> Store {
        Store::new_in_memory().expect("in-memory store")
    }

    fn make_space(name: &str) -> SpaceRef {
        SpaceRef {
            name: name.to_string(),
            path: PathBuf::from(format!("/nonexistent/{name}")),
            is_global: name == GLOBAL_NAME,
        }
    }

    #[test]
    fn vitals_empty_store_has_all_zeros() {
        let store = in_memory_store();
        let v = compute_vitals_for_store(make_space("x"), &store, Utc::now()).unwrap();
        assert_eq!(v.active, 0);
        assert_eq!(v.resolved, 0);
        assert_eq!(v.released, 0);
        assert_eq!(v.deadlined, 0);
        assert_eq!(v.overdue, 0);
        assert_eq!(v.positioned, 0);
        assert_eq!(v.held, 0);
        assert!(v.last_activity.is_none());
    }

    #[test]
    fn vitals_counts_active_held_positioned_overdue() {
        let store = in_memory_store();
        let now = Utc::now();
        let past = Horizon::parse("2020-01-01").unwrap();

        // Active, held (no position), no deadline.
        store.create_tension("do thing A", "not done").unwrap();
        // Active, positioned.
        let t2 = store
            .create_tension_full("do thing B", "not done", None, None)
            .unwrap();
        store.update_position(&t2.id, Some(1)).unwrap();
        // Active, overdue (past horizon).
        let _t3 = store
            .create_tension_full("late work", "undone", None, Some(past))
            .unwrap();

        let v = compute_vitals_for_store(make_space("x"), &store, now).unwrap();
        assert_eq!(v.active, 3);
        assert_eq!(v.overdue, 1);
        assert_eq!(v.positioned, 1);
        assert_eq!(v.held, 2);
        assert_eq!(v.deadlined, 1);
    }

    #[test]
    fn attention_tags_items_with_space_name() {
        let store = in_memory_store();
        let _t = store.create_tension("alpha", "zero").unwrap();
        let space = SpaceRef {
            name: "werk".into(),
            path: PathBuf::from("/irrelevant"),
            is_global: false,
        };
        let (_overdue, _next_up, held) =
            compute_attention_for_store(&space, &store, Utc::now(), 5, 5).unwrap();
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].space_name, "werk");
    }

    #[test]
    fn attention_next_up_cap_applies_per_space() {
        let store = in_memory_store();
        for i in 0..10 {
            let t = store
                .create_tension(&format!("t{i}"), "not yet")
                .unwrap();
            store.update_position(&t.id, Some(i as i32 + 1)).unwrap();
        }
        let space = make_space("big");
        let (_o, next_up, _h) =
            compute_attention_for_store(&space, &store, Utc::now(), 3, 3).unwrap();
        assert_eq!(next_up.len(), 3);
        // Cap preserves position-ascending order.
        assert_eq!(next_up[0].position, Some(1));
        assert_eq!(next_up[2].position, Some(3));
    }

    #[test]
    fn attention_overdue_uncapped_and_urgency_sorted() {
        let store = in_memory_store();
        let older = Horizon::parse("2019-01-01").unwrap();
        let newer = Horizon::parse("2023-01-01").unwrap();
        let _a = store
            .create_tension_full("very late", "a", None, Some(older))
            .unwrap();
        let _b = store
            .create_tension_full("less late", "b", None, Some(newer))
            .unwrap();
        let space = make_space("werk");
        let (overdue, _, _) =
            compute_attention_for_store(&space, &store, Utc::now(), 5, 5).unwrap();
        assert_eq!(overdue.len(), 2);
        // Older horizon → higher urgency → first.
        assert!(overdue[0].desired.contains("very late"));
    }
}
