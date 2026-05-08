use std::collections::{HashMap, HashSet};

use crate::ctx::Ctx;
use crate::error::SigilError;
use crate::ir::IrKind;
use crate::scope::ResolvedScope;
use werk_core::ir::{
    AttributeBuilder, AttributeGraph, AttributeValue, EpochSeries, Ir, IrContext, TensionListEntry,
    TensionTree,
};

pub enum Featurized {
    TensionList(Vec<TensionListEntry>),
    TensionTree(TensionTree),
    AttributeGraph(AttributeGraph),
    EpochSeries(EpochSeries),
}

impl Featurized {
    pub fn kind(&self) -> IrKind {
        match self {
            Featurized::TensionList(_) => IrKind::TensionList,
            Featurized::TensionTree(ir) => ir.kind(),
            Featurized::AttributeGraph(ir) => ir.kind(),
            Featurized::EpochSeries(ir) => ir.kind(),
        }
    }
}

pub trait Featurizer {
    fn ir_kind(&self) -> IrKind;
    fn featurize(&self, scope: &ResolvedScope, ctx: &mut Ctx<'_>)
    -> Result<Featurized, SigilError>;
}

#[derive(Debug, Clone)]
pub struct TensionTreeFeaturizer {
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TensionListFeaturizer {
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AttributeGraphFeaturizer {
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EpochSeriesFeaturizer;

impl Featurizer for TensionTreeFeaturizer {
    fn ir_kind(&self) -> IrKind {
        IrKind::TensionTree
    }

    fn featurize(
        &self,
        scope: &ResolvedScope,
        ctx: &mut Ctx<'_>,
    ) -> Result<Featurized, SigilError> {
        let ir_ctx = IrContext::new(ctx.now, ctx.workspace_name.clone());
        let tree = TensionTree::build(ctx.store, scope.forest.clone(), &ir_ctx)
            .map_err(|e| SigilError::render(e.to_string()))?;
        let filtered = filter_tree_attributes(tree, &self.attributes)?;
        Ok(Featurized::TensionTree(filtered))
    }
}

impl Featurizer for TensionListFeaturizer {
    fn ir_kind(&self) -> IrKind {
        IrKind::TensionList
    }

    fn featurize(
        &self,
        scope: &ResolvedScope,
        ctx: &mut Ctx<'_>,
    ) -> Result<Featurized, SigilError> {
        let ir_ctx = IrContext::new(ctx.now, ctx.workspace_name.clone());
        let list = werk_core::ir::TensionList::build(ctx.store, scope.tensions.clone(), &ir_ctx)
            .map_err(|e| SigilError::render(e.to_string()))?;
        let filtered = filter_list_attributes(list, &self.attributes)?;
        Ok(Featurized::TensionList(filtered))
    }
}

impl Featurizer for AttributeGraphFeaturizer {
    fn ir_kind(&self) -> IrKind {
        IrKind::AttributeGraph
    }

    fn featurize(
        &self,
        scope: &ResolvedScope,
        ctx: &mut Ctx<'_>,
    ) -> Result<Featurized, SigilError> {
        let edges = ctx
            .store
            .get_all_edges()
            .map_err(|e| SigilError::render(e.to_string()))?;
        let ir_ctx = IrContext::new(ctx.now, ctx.workspace_name.clone());
        let graph = AttributeGraph::build(ctx.store, scope.tensions.clone(), &edges, &ir_ctx)
            .map_err(|e| SigilError::render(e.to_string()))?;
        let filtered = filter_graph_attributes(graph, &self.attributes)?;
        Ok(Featurized::AttributeGraph(filtered))
    }
}

impl Featurizer for EpochSeriesFeaturizer {
    fn ir_kind(&self) -> IrKind {
        IrKind::EpochSeries
    }

    fn featurize(
        &self,
        scope: &ResolvedScope,
        ctx: &mut Ctx<'_>,
    ) -> Result<Featurized, SigilError> {
        let root = scope
            .scope
            .root
            .clone()
            .ok_or_else(|| SigilError::render("missing tension id for epoch series"))?;
        let series = EpochSeries::for_tension(ctx.store, &root)
            .map_err(|e| SigilError::render(e.to_string()))?;
        Ok(Featurized::EpochSeries(series))
    }
}

fn split_requested(requested: &[String]) -> (Vec<String>, Vec<String>) {
    let allowed: HashSet<&str> = AttributeBuilder::registry_attribute_names()
        .iter()
        .copied()
        .collect();
    let mut registry = Vec::new();
    let mut custom = Vec::new();
    for name in requested {
        if allowed.contains(name.as_str()) {
            registry.push(name.clone());
        } else {
            custom.push(name.clone());
        }
    }
    (registry, custom)
}

fn filter_tree_attributes(
    tree: TensionTree,
    requested: &[String],
) -> Result<TensionTree, SigilError> {
    let (registry, custom) = split_requested(requested);
    let builder =
        AttributeBuilder::new(&registry).map_err(|e| SigilError::render(e.to_string()))?;
    let mut filtered = HashMap::new();
    for (id, attrs) in tree.attributes.iter() {
        let mut selected = werk_core::ir::Attributes::new();
        for name in builder.requested() {
            if let Some(value) = attrs.get(name) {
                selected.insert(name.clone(), value.clone());
            }
        }
        for name in &custom {
            selected.insert(name.clone(), AttributeValue::Unknown);
        }
        filtered.insert(id.clone(), selected);
    }
    Ok(TensionTree {
        forest: tree.forest,
        attributes: filtered,
    })
}

fn filter_list_attributes(
    list: Vec<TensionListEntry>,
    requested: &[String],
) -> Result<Vec<TensionListEntry>, SigilError> {
    let (registry, custom) = split_requested(requested);
    let builder =
        AttributeBuilder::new(&registry).map_err(|e| SigilError::render(e.to_string()))?;
    let mut entries = Vec::new();
    for entry in list {
        let mut selected = werk_core::ir::Attributes::new();
        for name in builder.requested() {
            if let Some(value) = entry.attributes.get(name) {
                selected.insert(name.clone(), value.clone());
            }
        }
        for name in &custom {
            selected.insert(name.clone(), AttributeValue::Unknown);
        }
        entries.push(werk_core::ir::TensionListEntry {
            tension_id: entry.tension_id,
            attributes: selected,
        });
    }
    Ok(entries)
}

fn filter_graph_attributes(
    graph: AttributeGraph,
    requested: &[String],
) -> Result<AttributeGraph, SigilError> {
    let (registry, custom) = split_requested(requested);
    let builder =
        AttributeBuilder::new(&registry).map_err(|e| SigilError::render(e.to_string()))?;
    let mut filtered = HashMap::new();
    for (id, attrs) in graph.attributes.iter() {
        let mut selected = werk_core::ir::Attributes::new();
        for name in builder.requested() {
            if let Some(value) = attrs.get(name) {
                selected.insert(name.clone(), value.clone());
            }
        }
        for name in &custom {
            selected.insert(name.clone(), AttributeValue::Unknown);
        }
        filtered.insert(id.clone(), selected);
    }
    Ok(AttributeGraph {
        graph: graph.graph,
        attributes: filtered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use werk_core::store::Store;
    use werk_core::tree::Forest;

    #[test]
    fn respects_requested_attributes() {
        let store = Store::new_in_memory().unwrap();
        let root = store.create_tension("root", "root actual").unwrap();
        let ctx = IrContext::new(Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(), "werk");
        let forest = Forest::from_tensions(vec![root.clone()]).unwrap();
        let tree = TensionTree::build(&store, forest, &ctx).unwrap();
        let filtered = filter_tree_attributes(tree, &["status".into(), "custom".into()]).unwrap();
        let attrs = filtered.attributes.get(&root.id).unwrap();
        assert!(attrs.get("status").is_some());
        assert!(matches!(attrs.get("custom"), Some(AttributeValue::Unknown)));
        assert!(attrs.get("urgency").is_none());
    }
}
