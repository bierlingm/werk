use std::collections::{HashMap, HashSet};

use fnx_classes::AttrMap;
use fnx_classes::digraph::DiGraph;
use fnx_runtime::CgseValue;

use crate::edge::{EDGE_CONTAINS, EDGE_MERGED_INTO, EDGE_SPLIT_FROM};
use crate::ir::{
    AttributeBuilder, AttributeContext, Attributes, Diagnostics, Ir, IrContext, IrError, IrKind,
    count_active_notes,
};
use crate::projection::{ProjectionThresholds, extract_mutation_pattern};
use crate::{Edge, Forest, Mutation, Store, Tension, compute_frontier};

#[derive(Debug, Clone)]
pub struct AttributeGraph {
    pub graph: DiGraph,
    pub attributes: HashMap<String, Attributes>,
}

impl Ir for AttributeGraph {
    fn kind(&self) -> IrKind {
        IrKind::AttributeGraph
    }
}

impl AttributeGraph {
    pub fn build(
        store: &Store,
        tensions: Vec<Tension>,
        edges: &[Edge],
        ctx: &IrContext,
    ) -> Result<Self, IrError> {
        let forest = Forest::from_tensions_and_edges(tensions, edges)?;
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

        let mut graph = forest.graph().clone();
        translate_edge_types(&mut graph, ctx.diagnostics());

        Ok(Self { graph, attributes })
    }
}

fn translate_edge_types(graph: &mut DiGraph, diagnostics: &Diagnostics) {
    let edges: Vec<(String, String, AttrMap)> = graph
        .edges_ordered_borrowed()
        .into_iter()
        .map(|(from, to, attrs)| (from.to_string(), to.to_string(), attrs.clone()))
        .collect();

    for (from, to, attrs) in edges {
        let Some(CgseValue::String(edge_type)) = attrs.get("type") else {
            continue;
        };
        let translated = translate_edge_type(edge_type, diagnostics);
        if translated == *edge_type {
            continue;
        }

        let mut new_attrs: AttrMap = attrs.clone();
        new_attrs.insert("type".to_owned(), CgseValue::String(translated));
        if let Err(err) = graph.add_edge_with_attrs(from.to_string(), to.to_string(), new_attrs) {
            diagnostics.warn(format!(
                "edge type translation failed for {from}->{to}: {err}"
            ));
        }
    }
}

fn translate_edge_type(edge_type: &str, diagnostics: &Diagnostics) -> String {
    match edge_type {
        EDGE_CONTAINS => "parent_child".to_string(),
        EDGE_MERGED_INTO => "merge_into".to_string(),
        EDGE_SPLIT_FROM => EDGE_SPLIT_FROM.to_string(),
        other => {
            diagnostics.warn(format!("unknown edge type: {other}"));
            other.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::AttributeGraph;
    use crate::ir::{AttributeValue, IrContext};
    use crate::{Edge, Store, Tension};
    use fnx_runtime::CgseValue;

    fn attribute_text(attrs: &crate::ir::Attributes, name: &str) -> String {
        match attrs.get(name) {
            Some(AttributeValue::Text(value)) => value.clone(),
            other => panic!("expected text for {name}, got {other:?}"),
        }
    }

    #[test]
    fn translates_edge_type_names() {
        let store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "parent actual").unwrap();
        let child = store
            .create_tension_with_parent("child", "child actual", Some(parent.id.clone()))
            .unwrap();
        let merged_target = store.create_tension("target", "target actual").unwrap();
        let split_child = store.create_tension("split child", "split actual").unwrap();
        let unknown = store.create_tension("unknown", "unknown actual").unwrap();

        let edges = vec![
            Edge::new(parent.id.clone(), child.id.clone(), "contains".to_string()),
            Edge::new(
                child.id.clone(),
                merged_target.id.clone(),
                "merged_into".to_string(),
            ),
            Edge::new(
                parent.id.clone(),
                split_child.id.clone(),
                "split_from".to_string(),
            ),
            Edge::new(
                child.id.clone(),
                unknown.id.clone(),
                "references".to_string(),
            ),
        ];

        let tensions: Vec<Tension> = store.list_tensions().unwrap();
        let ctx = IrContext::new(Utc::now(), "werk");
        let graph = AttributeGraph::build(&store, tensions, &edges, &ctx).unwrap();

        let edge_types: std::collections::HashMap<(String, String), String> = graph
            .graph
            .edges_ordered_borrowed()
            .into_iter()
            .filter_map(|(from, to, attrs)| match attrs.get("type") {
                Some(CgseValue::String(value)) => {
                    Some(((from.to_string(), to.to_string()), value.clone()))
                }
                _ => None,
            })
            .collect();

        assert_eq!(
            edge_types
                .get(&(parent.id.clone(), child.id.clone()))
                .map(String::as_str),
            Some("parent_child")
        );
        assert_eq!(
            edge_types
                .get(&(child.id.clone(), merged_target.id.clone()))
                .map(String::as_str),
            Some("merge_into")
        );
        assert_eq!(
            edge_types
                .get(&(parent.id.clone(), split_child.id.clone()))
                .map(String::as_str),
            Some("split_from")
        );
        assert_eq!(
            edge_types
                .get(&(child.id.clone(), unknown.id.clone()))
                .map(String::as_str),
            Some("references")
        );

        let warnings = ctx.diagnostics.warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unknown edge type"));

        let parent_attrs = graph.attributes.get(&parent.id).unwrap();
        let child_attrs = graph.attributes.get(&child.id).unwrap();
        assert_eq!(attribute_text(parent_attrs, "id"), parent.id);
        assert_eq!(attribute_text(child_attrs, "id"), child.id);
    }
}
