use std::collections::{HashMap, HashSet};

use crate::ir::{
    AttributeBuilder, AttributeContext, AttributeValue, Attributes, Ir, IrContext, IrError, IrKind,
    count_active_notes,
};
use crate::projection::{ProjectionThresholds, extract_mutation_pattern};
use crate::temporal::{compute_urgency, gap_magnitude};
use crate::{Forest, Mutation, Store, Tension, TensionStatus, compute_frontier};

#[derive(Debug, Clone)]
pub struct TensionListEntry {
    pub tension_id: String,
    pub attributes: Attributes,
}

#[derive(Debug, Clone)]
pub struct TensionList;

impl Ir for TensionList {
    fn kind(&self) -> IrKind {
        IrKind::TensionList
    }
}

impl AttributeBuilder {
    pub(crate) fn build_for(
        &self,
        tension: &Tension,
        ctx: &AttributeContext<'_>,
    ) -> Result<Attributes, IrError> {
        let mut attrs = Attributes::new();
        let updated_at = ctx.last_mutation(&tension.id, tension.created_at);
        let last_pulse_at = updated_at;
        let urgency_raw = compute_urgency(tension, ctx.now).map(|u| u.value);
        let urgency = urgency_raw.map(|value| value.clamp(0.0, 1.0));
        let staleness = match (&tension.horizon, ctx.last_mutations.get(&tension.id)) {
            (Some(horizon), Some(last_mutation)) => {
                Some(horizon.staleness(*last_mutation, ctx.now).clamp(0.0, 1.0))
            }
            _ => None,
        };
        let gap = gap_magnitude(&tension.desired, &tension.actual);
        let pattern = ctx.pattern(&tension.id);
        let depth = ctx.forest.depth(&tension.id).unwrap_or(0) as f64;
        let child_count = ctx
            .forest
            .children(&tension.id)
            .map(|children| children.len())
            .unwrap_or(0) as f64;
        let descendant_count = ctx
            .forest
            .descendants(&tension.id)
            .map(|descendants| descendants.len())
            .unwrap_or(0) as f64;
        let has_children = child_count > 0.0;
        let parent_short_code = ctx
            .parent_short_codes
            .get(&tension.id)
            .and_then(|value| *value);
        let is_held = tension.status == TensionStatus::Active && ctx.held_ids.contains(&tension.id);
        let status = if tension.status == TensionStatus::Resolved {
            "resolved"
        } else if tension.status == TensionStatus::Released {
            "released"
        } else if is_held {
            "held"
        } else {
            "active"
        };

        let deadline = tension.horizon.as_ref().map(|h| h.range_end());
        let time_to_deadline = deadline.map(|d| (d - ctx.now).num_seconds() as f64);
        let age_seconds = (ctx.now - tension.created_at).num_seconds().max(0) as f64;

        for name in self.requested() {
            match name.as_str() {
                "id" => attrs.insert(name, AttributeValue::Text(tension.id.clone())),
                "short_code" => attrs.insert(
                    name,
                    tension
                        .short_code
                        .map(|value| AttributeValue::Number(value as f64))
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "space" => attrs.insert(name, AttributeValue::Text(ctx.workspace_name.to_string())),
                "desire" => attrs.insert(name, AttributeValue::Text(tension.desired.clone())),
                "actual" => attrs.insert(name, AttributeValue::Text(tension.actual.clone())),
                "status" => attrs.insert(name, AttributeValue::Categorical(status.to_string())),
                "is_held" => attrs.insert(name, AttributeValue::Bool(is_held)),
                "is_resolved" => attrs.insert(
                    name,
                    AttributeValue::Bool(tension.status == TensionStatus::Resolved),
                ),
                "is_released" => attrs.insert(
                    name,
                    AttributeValue::Bool(tension.status == TensionStatus::Released),
                ),
                "created_at" => {
                    attrs.insert(name, AttributeValue::Text(tension.created_at.to_rfc3339()))
                }
                "updated_at" => attrs.insert(name, AttributeValue::Text(updated_at.to_rfc3339())),
                "deadline" => attrs.insert(
                    name,
                    deadline
                        .map(|d| AttributeValue::Text(d.to_rfc3339()))
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "last_pulse_at" => {
                    attrs.insert(name, AttributeValue::Text(last_pulse_at.to_rfc3339()))
                }
                "age_seconds" => attrs.insert(name, AttributeValue::Number(age_seconds)),
                "time_to_deadline_seconds" => attrs.insert(
                    name,
                    time_to_deadline
                        .map(AttributeValue::Number)
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "urgency" => attrs.insert(
                    name,
                    urgency
                        .map(AttributeValue::Number)
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "urgency_raw" => attrs.insert(
                    name,
                    urgency_raw
                        .map(AttributeValue::Number)
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "staleness" => attrs.insert(
                    name,
                    staleness
                        .map(AttributeValue::Number)
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "gap_magnitude" => attrs.insert(name, AttributeValue::Number(gap)),
                "frequency_per_day" => {
                    attrs.insert(name, AttributeValue::Number(pattern.frequency_per_day));
                }
                "frequency_trend" => {
                    attrs.insert(name, AttributeValue::Number(pattern.frequency_trend));
                }
                "gap_trend" => attrs.insert(name, AttributeValue::Number(pattern.gap_trend)),
                "mutation_count" => {
                    attrs.insert(name, AttributeValue::Number(pattern.mutation_count as f64));
                }
                "is_projectable" => {
                    attrs.insert(name, AttributeValue::Bool(pattern.is_projectable));
                }
                "depth" => attrs.insert(name, AttributeValue::Number(depth)),
                "child_count" => attrs.insert(name, AttributeValue::Number(child_count)),
                "descendant_count" => {
                    attrs.insert(name, AttributeValue::Number(descendant_count));
                }
                "parent_id" => attrs.insert(
                    name,
                    tension
                        .parent_id
                        .clone()
                        .map(AttributeValue::Text)
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "parent_short_code" => attrs.insert(
                    name,
                    parent_short_code
                        .map(|value| AttributeValue::Number(value as f64))
                        .unwrap_or(AttributeValue::Unknown),
                ),
                "note_count" => attrs.insert(
                    name,
                    AttributeValue::Number(ctx.note_count(&tension.id) as f64),
                ),
                "has_children" => attrs.insert(name, AttributeValue::Bool(has_children)),
                _ => return Err(IrError::unknown_attribute(name)),
            }
        }

        Ok(attrs)
    }
}

impl TensionList {
    pub fn build(
        store: &Store,
        tensions: Vec<Tension>,
        ctx: &IrContext,
    ) -> Result<Vec<TensionListEntry>, IrError> {
        let forest = Forest::from_tensions(tensions.clone())?;
        let tension_map: HashMap<String, Tension> = tensions
            .iter()
            .cloned()
            .map(|t| (t.id.clone(), t))
            .collect();
        let thresholds = ProjectionThresholds::default();

        let mut patterns = HashMap::new();
        let mut note_counts = HashMap::new();
        let mut last_mutations = HashMap::new();
        let mut parent_short_codes: HashMap<String, Option<i32>> = HashMap::new();
        let mut mutations_by_id: HashMap<String, Vec<Mutation>> = HashMap::new();

        for tension in &tensions {
            let tension_mutations = store.get_mutations(&tension.id)?;
            mutations_by_id.insert(tension.id.clone(), tension_mutations.clone());
            let last_mutation = tension_mutations
                .last()
                .map(|m| m.timestamp())
                .unwrap_or(tension.created_at);
            last_mutations.insert(tension.id.clone(), last_mutation);

            let pattern = extract_mutation_pattern(
                tension,
                &tension_mutations,
                thresholds.pattern_window_seconds,
                ctx.now,
            );
            patterns.insert(tension.id.clone(), pattern);

            note_counts.insert(tension.id.clone(), count_active_notes(&tension_mutations));

            let parent_short_code = tension.parent_id.as_ref().and_then(|parent_id| {
                tension_map
                    .get(parent_id)
                    .and_then(|parent| parent.short_code)
                    .or_else(|| store.get_tension(parent_id).ok().flatten()?.short_code)
            });
            parent_short_codes.insert(tension.id.clone(), parent_short_code);
        }

        let mut held_ids = HashSet::new();
        let mut computed_parents = HashSet::new();
        for tension in &tensions {
            let Some(parent_id) = tension.parent_id.as_ref() else {
                continue;
            };
            if !computed_parents.insert(parent_id.clone()) {
                continue;
            }

            let Some(parent) = store.get_tension(parent_id)? else {
                continue;
            };
            let children = store.get_children(parent_id)?;
            if children.is_empty() {
                continue;
            }

            let mut family = Vec::with_capacity(children.len() + 1);
            family.push(parent);
            family.extend(children.clone());

            let parent_forest = Forest::from_tensions(family)?;
            let mut child_mutations = Vec::with_capacity(children.len());
            for child in &children {
                let muts = if let Some(muts) = mutations_by_id.get(&child.id) {
                    muts.clone()
                } else {
                    let fetched = store.get_mutations(&child.id)?;
                    mutations_by_id.insert(child.id.clone(), fetched.clone());
                    fetched
                };
                child_mutations.push((child.id.clone(), muts));
            }

            let epochs = store.get_epochs(parent_id)?;
            let frontier = compute_frontier(
                &parent_forest,
                parent_id,
                ctx.now,
                &epochs,
                &child_mutations,
            );
            for step in frontier.held {
                held_ids.insert(step.tension_id);
            }
        }

        let attribute_ctx = AttributeContext {
            now: ctx.now,
            workspace_name: ctx.workspace_name(),
            forest: &forest,
            patterns,
            note_counts,
            last_mutations,
            parent_short_codes,
            held_ids,
        };
        let builder = AttributeBuilder::new(AttributeBuilder::registry_attribute_names())?;

        let mut entries = Vec::with_capacity(tensions.len());
        for tension in tensions {
            let attributes = builder.build_for(&tension, &attribute_ctx)?;
            entries.push(TensionListEntry {
                tension_id: tension.id.clone(),
                attributes,
            });
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use crate::ir::{AttributeBuilder, AttributeValue, IrContext, TensionList};
    use crate::{Horizon, Store};

    fn attribute_number(attrs: &crate::ir::Attributes, name: &str) -> f64 {
        match attrs.get(name) {
            Some(AttributeValue::Number(value)) => *value,
            other => panic!("expected number for {name}, got {other:?}"),
        }
    }

    fn attribute_text(attrs: &crate::ir::Attributes, name: &str) -> String {
        match attrs.get(name) {
            Some(AttributeValue::Text(value)) => value.clone(),
            other => panic!("expected text for {name}, got {other:?}"),
        }
    }

    fn attribute_bool(attrs: &crate::ir::Attributes, name: &str) -> bool {
        match attrs.get(name) {
            Some(AttributeValue::Bool(value)) => *value,
            other => panic!("expected bool for {name}, got {other:?}"),
        }
    }

    fn attribute_categorical(attrs: &crate::ir::Attributes, name: &str) -> String {
        match attrs.get(name) {
            Some(AttributeValue::Categorical(value)) => value.clone(),
            other => panic!("expected categorical for {name}, got {other:?}"),
        }
    }

    #[test]
    fn builds_full_attribute_set() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let _child = store
            .create_tension_with_parent("child goal", "child reality", Some(parent.id.clone()))
            .unwrap();
        let _other = store.create_tension("other goal", "other reality").unwrap();
        let _root = store.create_tension("root goal", "root reality").unwrap();
        let _leaf = store.create_tension("leaf goal", "leaf reality").unwrap();

        let tensions = store.list_tensions().unwrap();
        let ctx = IrContext::new(Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap(), "werk");
        let entries = TensionList::build(&store, tensions, &ctx).unwrap();

        let expected: std::collections::HashSet<&str> =
            AttributeBuilder::registry_attribute_names()
                .iter()
                .copied()
                .collect();
        for entry in entries {
            let keys: std::collections::HashSet<&str> =
                entry.attributes.keys().map(|k| k.as_str()).collect();
            assert_eq!(keys, expected);
        }
    }

    #[test]
    fn gap_magnitude_is_binary() {
        let store = Store::new_in_memory().unwrap();
        let same = store.create_tension("same", "same").unwrap();
        let diff = store.create_tension("goal", "reality").unwrap();

        let ctx = IrContext::new(Utc::now(), "werk");
        let entries = TensionList::build(&store, vec![same.clone(), diff.clone()], &ctx).unwrap();

        let mut entry_by_id = std::collections::HashMap::new();
        for entry in entries {
            entry_by_id.insert(entry.tension_id.clone(), entry);
        }

        let same_attrs = &entry_by_id[&same.id].attributes;
        let diff_attrs = &entry_by_id[&diff.id].attributes;

        assert_eq!(attribute_number(same_attrs, "gap_magnitude"), 0.0);
        assert_eq!(attribute_number(diff_attrs, "gap_magnitude"), 1.0);
    }

    #[test]
    fn space_attribute_from_ctx() {
        let store = Store::new_in_memory().unwrap();
        let tension = store.create_tension("goal", "reality").unwrap();
        let ctx = IrContext::new(Utc::now(), "journal");

        let entries = TensionList::build(&store, vec![tension.clone()], &ctx).unwrap();
        let attrs = &entries[0].attributes;

        assert_eq!(attribute_text(attrs, "space"), "journal".to_string());
    }

    #[test]
    fn derived_time_and_relationship_attributes() {
        let store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "parent reality").unwrap();
        let horizon = Horizon::new_month(2026, 6).unwrap();
        let child = store
            .create_tension_full(
                "child",
                "child reality",
                Some(parent.id.clone()),
                Some(horizon),
            )
            .unwrap();

        store.update_actual(&child.id, "updated reality").unwrap();
        store.record_note(&child.id, "observation").unwrap();

        let child_mutations = store.get_mutations(&child.id).unwrap();
        let updated_at = child_mutations.last().map(|m| m.timestamp()).unwrap();

        let ctx = IrContext::new(updated_at + Duration::seconds(3600), "werk");
        let entries =
            TensionList::build(&store, vec![parent.clone(), child.clone()], &ctx).unwrap();
        let child_entry = entries
            .iter()
            .find(|entry| entry.tension_id == child.id)
            .unwrap();
        let attrs = &child_entry.attributes;

        assert_eq!(attribute_text(attrs, "updated_at"), updated_at.to_rfc3339());
        assert_eq!(
            attribute_text(attrs, "last_pulse_at"),
            updated_at.to_rfc3339()
        );
        assert_eq!(
            attribute_number(attrs, "age_seconds"),
            (ctx.now - child.created_at).num_seconds() as f64
        );
        assert_eq!(
            attribute_number(attrs, "time_to_deadline_seconds"),
            (child.horizon.as_ref().unwrap().range_end() - ctx.now).num_seconds() as f64
        );
        assert_eq!(
            attribute_number(attrs, "parent_short_code"),
            parent.short_code.unwrap() as f64
        );
        assert_eq!(attribute_number(attrs, "note_count"), 1.0);
        assert!(!attribute_bool(attrs, "has_children"));
    }

    #[test]
    fn unknown_requested_attribute_fails_loudly() {
        let err = AttributeBuilder::new(&["urgency", "not_real"]).unwrap_err();
        assert!(err.to_string().contains("not_real"));
    }

    #[test]
    fn root_tension_is_not_marked_held() {
        let store = Store::new_in_memory().unwrap();
        let root = store.create_tension("root goal", "root reality").unwrap();
        let ctx = IrContext::new(Utc::now(), "werk");

        let entries = TensionList::build(&store, vec![root.clone()], &ctx).unwrap();
        let attrs = &entries[0].attributes;

        assert!(!attribute_bool(attrs, "is_held"));
        assert_eq!(attribute_categorical(attrs, "status"), "active".to_string());
    }

    #[test]
    fn subset_child_held_derived_from_parent_context() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let child = store
            .create_tension_with_parent("child goal", "child reality", Some(parent.id.clone()))
            .unwrap();

        let ctx = IrContext::new(Utc::now(), "werk");
        let entries = TensionList::build(&store, vec![child.clone()], &ctx).unwrap();
        let attrs = &entries[0].attributes;

        assert!(attribute_bool(attrs, "is_held"));
        assert_eq!(attribute_categorical(attrs, "status"), "held".to_string());
    }

    #[test]
    fn subset_child_held_uses_parent_context_with_sibling_positions() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let held_child = store
            .create_tension_with_parent("held goal", "held reality", Some(parent.id.clone()))
            .unwrap();
        let positioned_child = store
            .create_tension_with_parent(
                "positioned goal",
                "positioned reality",
                Some(parent.id.clone()),
            )
            .unwrap();
        store
            .update_position(&positioned_child.id, Some(1))
            .unwrap();

        let ctx = IrContext::new(Utc::now(), "werk");
        let entries = TensionList::build(&store, vec![held_child.clone()], &ctx).unwrap();
        let attrs = &entries[0].attributes;

        assert!(attribute_bool(attrs, "is_held"));
        assert_eq!(attribute_categorical(attrs, "status"), "held".to_string());
    }
}
