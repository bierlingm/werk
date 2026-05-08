use std::collections::HashSet;

use crate::ctx::Ctx;
use crate::error::SigilError;
use crate::scope::{ResolvedScope, Scope, ScopeKind};
use werk_core::tree::Forest;

pub trait Selector {
    fn select(&self, scope: Scope, ctx: &mut Ctx<'_>) -> Result<ResolvedScope, SigilError>;
}

#[derive(Debug, Clone)]
pub struct SubtreeSelector {
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct SpaceSelector;

#[derive(Debug, Clone)]
pub struct QuerySelector;

#[derive(Debug, Clone)]
pub struct UnionSelector;

impl Selector for SubtreeSelector {
    fn select(&self, scope: Scope, ctx: &mut Ctx<'_>) -> Result<ResolvedScope, SigilError> {
        let depth = scope.depth.unwrap_or(self.depth);
        select_scope(scope, ctx, depth)
    }
}

impl Selector for SpaceSelector {
    fn select(&self, scope: Scope, ctx: &mut Ctx<'_>) -> Result<ResolvedScope, SigilError> {
        select_scope(scope, ctx, usize::MAX)
    }
}

impl Selector for QuerySelector {
    fn select(&self, scope: Scope, ctx: &mut Ctx<'_>) -> Result<ResolvedScope, SigilError> {
        select_scope(scope, ctx, usize::MAX)
    }
}

impl Selector for UnionSelector {
    fn select(&self, scope: Scope, ctx: &mut Ctx<'_>) -> Result<ResolvedScope, SigilError> {
        select_scope(scope, ctx, usize::MAX)
    }
}

fn select_scope(
    scope: Scope,
    ctx: &mut Ctx<'_>,
    depth_limit: usize,
) -> Result<ResolvedScope, SigilError> {
    if scope.at.is_some() {
        return Err(SigilError::unsupported("historical scope"));
    }

    let tensions = ctx
        .store
        .list_tensions()
        .map_err(|e| SigilError::render(e.to_string()))?;
    if tensions.is_empty() {
        return Ok(ResolvedScope {
            scope,
            tensions: Vec::new(),
            forest: Forest::new(),
        });
    }

    let forest =
        Forest::from_tensions(tensions.clone()).map_err(|e| SigilError::render(e.to_string()))?;

    let selected = match scope.kind {
        ScopeKind::Tension | ScopeKind::Subtree => {
            let root = scope
                .root
                .clone()
                .ok_or_else(|| SigilError::render("missing root"))?;
            let mut ids = HashSet::new();
            ids.insert(root.clone());
            let mut queue: Vec<(String, usize)> = vec![(root, 0)];
            while let Some((id, depth)) = queue.pop() {
                if depth >= depth_limit {
                    continue;
                }
                if let Some(children) = forest.children(&id) {
                    for child in children {
                        let child_id = child.id().to_string();
                        if ids.insert(child_id.clone()) {
                            queue.push((child_id, depth + 1));
                        }
                    }
                }
            }
            tensions
                .into_iter()
                .filter(|t| ids.contains(&t.id))
                .collect()
        }
        ScopeKind::Space => {
            let name = scope.name.clone().unwrap_or_else(|| "active".into());
            let status = name.to_lowercase();
            tensions
                .into_iter()
                .filter(|t| t.status.to_string().to_lowercase() == status)
                .collect()
        }
        ScopeKind::Query => {
            let status = scope.status.clone().unwrap_or_else(|| "active".into());
            tensions
                .into_iter()
                .filter(|t| t.status.to_string().to_lowercase() == status)
                .collect()
        }
        ScopeKind::Union => {
            let mut ids = HashSet::new();
            let mut selected = Vec::new();
            for member in scope.members.iter() {
                let member_scope = Scope {
                    kind: member.kind.clone(),
                    root: member.root.clone(),
                    depth: member.depth,
                    name: member.name.clone(),
                    status: member.status.clone(),
                    members: member.members.clone(),
                    at: member.at,
                };
                let resolved = select_scope(member_scope, ctx, depth_limit)?;
                for tension in resolved.tensions {
                    if ids.insert(tension.id.clone()) {
                        selected.push(tension);
                    }
                }
            }
            selected
        }
    };

    let forest =
        Forest::from_tensions(selected.clone()).map_err(|e| SigilError::render(e.to_string()))?;

    Ok(ResolvedScope {
        scope,
        tensions: selected,
        forest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashSet;
    use werk_core::store::Store;

    #[test]
    fn respects_depth_limit() {
        let store = Store::new_in_memory().unwrap();
        let root = store.create_tension("root", "root actual").unwrap();
        let child = store
            .create_tension_with_parent("child", "child actual", Some(root.id.clone()))
            .unwrap();
        let grandchild = store
            .create_tension_with_parent("grand", "grand actual", Some(child.id.clone()))
            .unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );
        let scope = Scope {
            kind: ScopeKind::Subtree,
            root: Some(root.id.clone()),
            depth: Some(1),
            name: None,
            status: None,
            members: Vec::new(),
            at: None,
        };
        let resolved = SubtreeSelector { depth: 4 }
            .select(scope, &mut ctx)
            .unwrap();
        let ids: HashSet<String> = resolved.tensions.into_iter().map(|t| t.id).collect();
        assert!(ids.contains(&root.id));
        assert!(ids.contains(&child.id));
        assert!(!ids.contains(&grandchild.id));
    }

    #[test]
    fn space_scope_filters_by_name() {
        let store = Store::new_in_memory().unwrap();
        let active = store.create_tension("active", "active actual").unwrap();
        let resolved = store.create_tension("resolved", "resolved actual").unwrap();
        let released = store.create_tension("released", "released actual").unwrap();
        store
            .update_status(&resolved.id, werk_core::tension::TensionStatus::Resolved)
            .unwrap();
        store
            .update_status(&released.id, werk_core::tension::TensionStatus::Released)
            .unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );

        let active_scope = Scope {
            kind: ScopeKind::Space,
            root: None,
            depth: None,
            name: Some("active".into()),
            status: None,
            members: Vec::new(),
            at: None,
        };
        let resolved_scope = Scope {
            kind: ScopeKind::Space,
            root: None,
            depth: None,
            name: Some("resolved".into()),
            status: None,
            members: Vec::new(),
            at: None,
        };
        let released_scope = Scope {
            kind: ScopeKind::Space,
            root: None,
            depth: None,
            name: Some("released".into()),
            status: None,
            members: Vec::new(),
            at: None,
        };

        let active_ids: HashSet<String> = SpaceSelector
            .select(active_scope, &mut ctx)
            .unwrap()
            .tensions
            .into_iter()
            .map(|t| t.id)
            .collect();
        let resolved_ids: HashSet<String> = SpaceSelector
            .select(resolved_scope, &mut ctx)
            .unwrap()
            .tensions
            .into_iter()
            .map(|t| t.id)
            .collect();
        let released_ids: HashSet<String> = SpaceSelector
            .select(released_scope, &mut ctx)
            .unwrap()
            .tensions
            .into_iter()
            .map(|t| t.id)
            .collect();

        assert!(active_ids.contains(&active.id));
        assert!(!active_ids.contains(&resolved.id));
        assert!(!active_ids.contains(&released.id));

        assert!(resolved_ids.contains(&resolved.id));
        assert!(!resolved_ids.contains(&active.id));
        assert!(!resolved_ids.contains(&released.id));

        assert!(released_ids.contains(&released.id));
        assert!(!released_ids.contains(&active.id));
        assert!(!released_ids.contains(&resolved.id));
    }

    #[test]
    fn space_scope_defaults_to_active() {
        let store = Store::new_in_memory().unwrap();
        let active = store.create_tension("active", "active actual").unwrap();
        let resolved = store.create_tension("resolved", "resolved actual").unwrap();
        store
            .update_status(&resolved.id, werk_core::tension::TensionStatus::Resolved)
            .unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );

        let scope = Scope {
            kind: ScopeKind::Space,
            root: None,
            depth: None,
            name: None,
            status: None,
            members: Vec::new(),
            at: None,
        };
        let ids: HashSet<String> = SpaceSelector
            .select(scope, &mut ctx)
            .unwrap()
            .tensions
            .into_iter()
            .map(|t| t.id)
            .collect();

        assert!(ids.contains(&active.id));
        assert!(!ids.contains(&resolved.id));
    }
}
