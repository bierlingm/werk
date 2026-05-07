use std::collections::{HashMap, HashSet};

use crate::ir::{
    AttributeBuilder, AttributeContext, Attributes, Ir, IrContext, IrError, IrKind,
    count_active_notes,
};
use crate::projection::{ProjectionThresholds, extract_mutation_pattern};
use crate::{Forest, Mutation, Store, Tension, compute_frontier};

#[derive(Debug, Clone)]
pub struct TensionTree {
    pub forest: Forest,
    pub attributes: HashMap<String, Attributes>,
}

impl Ir for TensionTree {
    fn kind(&self) -> IrKind {
        IrKind::TensionTree
    }
}

impl TensionTree {
    pub fn build(store: &Store, forest: Forest, ctx: &IrContext) -> Result<Self, IrError> {
        let mut tensions = Vec::new();
        forest.traverse_dfs_pre(|node| tensions.push(node.tension.clone()));

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
        for tension in &tensions {
            let children = forest.children(&tension.id).unwrap_or_default();
            if children.is_empty() {
                continue;
            }

            let mut child_mutations = Vec::with_capacity(children.len());
            for child in &children {
                let muts = if let Some(muts) = mutations_by_id.get(child.id()) {
                    muts.clone()
                } else {
                    let fetched = store.get_mutations(child.id())?;
                    mutations_by_id.insert(child.id().to_string(), fetched.clone());
                    fetched
                };
                child_mutations.push((child.id().to_string(), muts));
            }

            let epochs = store.get_epochs(&tension.id)?;
            let frontier =
                compute_frontier(&forest, &tension.id, ctx.now, &epochs, &child_mutations);
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

        let mut attributes = HashMap::with_capacity(tensions.len());
        for tension in tensions {
            let attrs = builder.build_for(&tension, &attribute_ctx)?;
            attributes.insert(tension.id.clone(), attrs);
        }

        Ok(Self { forest, attributes })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::TensionTree;
    use crate::ir::{AttributeValue, IrContext};
    use crate::{Forest, Horizon, Store};

    fn attribute_number(attrs: &crate::ir::Attributes, name: &str) -> f64 {
        match attrs.get(name) {
            Some(AttributeValue::Number(value)) => *value,
            other => panic!("expected number for {name}, got {other:?}"),
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
    fn depths_and_attributes_match_forest() {
        let store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "parent actual").unwrap();
        let child = store
            .create_tension_with_parent("child", "child actual", Some(parent.id.clone()))
            .unwrap();
        let grandchild = store
            .create_tension_with_parent("grandchild", "grandchild actual", Some(child.id.clone()))
            .unwrap();

        let tensions = store.list_tensions().unwrap();
        let forest = Forest::from_tensions(tensions).unwrap();
        let ctx = IrContext::new(parent.created_at + Duration::days(1), "werk");

        let tree = TensionTree::build(&store, forest, &ctx).unwrap();

        let parent_attrs = tree.attributes.get(&parent.id).unwrap();
        let child_attrs = tree.attributes.get(&child.id).unwrap();
        let grandchild_attrs = tree.attributes.get(&grandchild.id).unwrap();

        assert_eq!(attribute_number(parent_attrs, "depth"), 0.0);
        assert_eq!(attribute_number(child_attrs, "depth"), 1.0);
        assert_eq!(attribute_number(grandchild_attrs, "depth"), 2.0);

        assert_eq!(
            tree.forest.depth(&parent.id).unwrap() as f64,
            attribute_number(parent_attrs, "depth")
        );
        assert_eq!(
            tree.forest.depth(&child.id).unwrap() as f64,
            attribute_number(child_attrs, "depth")
        );
        assert_eq!(
            tree.forest.depth(&grandchild.id).unwrap() as f64,
            attribute_number(grandchild_attrs, "depth")
        );
    }

    #[test]
    fn derives_is_held_for_unstarted_horizon() {
        let store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "parent actual").unwrap();
        let child = store
            .create_tension_with_parent("held child", "held child actual", Some(parent.id.clone()))
            .unwrap();
        let horizon = Horizon::new_datetime(child.created_at + Duration::days(30));
        store.update_horizon(&child.id, Some(horizon)).unwrap();

        let tensions = store.list_tensions().unwrap();
        let forest = Forest::from_tensions(tensions).unwrap();
        let ctx = IrContext::new(child.created_at + Duration::days(1), "werk");

        let tree = TensionTree::build(&store, forest, &ctx).unwrap();
        let attrs = tree.attributes.get(&child.id).unwrap();

        assert!(attribute_bool(attrs, "is_held"));
        assert_eq!(attribute_categorical(attrs, "status"), "held".to_string());
    }

    #[test]
    fn urgency_clamped_and_raw_exposed() {
        let store = Store::new_in_memory().unwrap();
        let tension = store
            .create_tension("past horizon", "past horizon actual")
            .unwrap();
        let horizon = Horizon::new_datetime(tension.created_at - Duration::days(30));
        store.update_horizon(&tension.id, Some(horizon)).unwrap();

        let tensions = store.list_tensions().unwrap();
        let forest = Forest::from_tensions(tensions).unwrap();
        let ctx = IrContext::new(tension.created_at + Duration::days(60), "werk");

        let tree = TensionTree::build(&store, forest, &ctx).unwrap();
        let attrs = tree.attributes.get(&tension.id).unwrap();

        let urgency = attribute_number(attrs, "urgency");
        let urgency_raw = attribute_number(attrs, "urgency_raw");

        assert_eq!(urgency, 1.0);
        assert!(urgency_raw > 1.0);
    }
}
