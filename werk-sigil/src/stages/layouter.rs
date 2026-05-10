use std::collections::HashMap;
use std::f64::consts::PI;

use crate::ctx::Ctx;
use crate::error::SigilError;
use crate::ir::IrKind;
use crate::stages::featurizer::Featurized;
use crate::stages::{Layout, MarkSpec, PlacedMark, StructuralMark};
use rand::RngCore;
use werk_core::ir::{AttributeValue, TensionTree};

pub trait Layouter {
    fn expected_ir(&self) -> IrKind;
    fn layout(
        &self,
        ir: &Featurized,
        marks: Vec<MarkSpec>,
        ctx: &mut Ctx<'_>,
    ) -> Result<Layout, SigilError>;
}

#[derive(Debug, Clone)]
pub struct RadialMandala {
    pub ring_step: f64,
    pub inner_padding: f64,
    pub root_radius: f64,
    pub center: (f64, f64),
    pub parent_child_curves: bool,
    pub ring_guides: bool,
    pub respect_status: bool,
}

#[derive(Debug, Clone)]
pub struct FractalBranch {
    pub center: (f64, f64),
    pub branch_step: f64,
}

#[derive(Debug, Clone)]
pub struct Constellation {
    pub center: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub center: (f64, f64),
    pub columns: usize,
    pub rows: usize,
    pub cell_size: f64,
}

impl Layouter for RadialMandala {
    fn expected_ir(&self) -> IrKind {
        IrKind::TensionTree
    }

    fn layout(
        &self,
        ir: &Featurized,
        marks: Vec<MarkSpec>,
        _ctx: &mut Ctx<'_>,
    ) -> Result<Layout, SigilError> {
        let tree = match ir {
            Featurized::TensionTree(tree) => tree,
            _ => return Err(SigilError::internal("radial_mandala expects TensionTree")),
        };

        let mut positions: HashMap<String, (f64, f64)> = HashMap::new();
        let mut angles: HashMap<String, (f64, f64)> = HashMap::new();

        let root_id = tree
            .forest
            .root_ids()
            .first()
            .cloned()
            .unwrap_or_else(|| marks.first().map(|m| m.id.clone()).unwrap_or_default());
        positions.insert(root_id.clone(), self.center);
        angles.insert(root_id.clone(), (0.0, 2.0 * PI));

        let mut queue = vec![root_id.clone()];
        while let Some(id) = queue.pop() {
            let (start, end) = angles.get(&id).copied().unwrap_or((0.0, 2.0 * PI));
            let children = tree.forest.children(&id).unwrap_or_default();
            if children.is_empty() {
                continue;
            }
            let weights: Vec<f64> = children
                .iter()
                .map(|child| {
                    let descendant =
                        number_attr(tree, child.id(), "descendant_count").unwrap_or(1.0);
                    let status = categorical_attr(tree, child.id(), "status").unwrap_or("active");
                    let mut weight = descendant.max(1.0);
                    if self.respect_status && (status == "held" || status == "released") {
                        weight *= 0.6;
                    }
                    weight
                })
                .collect();
            let total: f64 = weights.iter().sum();
            let mut cursor = start;
            for (child, weight) in children.iter().zip(weights.iter()) {
                let span = (end - start - self.inner_padding * children.len() as f64).max(0.0)
                    * (*weight / total.max(1.0));
                let angle = cursor + span / 2.0;
                let depth = number_attr(tree, child.id(), "depth").unwrap_or(1.0);
                let radius = self.ring_step * depth;
                let x = self.center.0 + radius * angle.cos();
                let y = self.center.1 + radius * angle.sin();
                positions.insert(child.id().to_string(), (x, y));
                angles.insert(child.id().to_string(), (cursor, cursor + span));
                cursor += span + self.inner_padding;
                queue.push(child.id().to_string());
            }
        }

        let placed = marks
            .into_iter()
            .map(|mark| {
                let (x, y) = positions.get(&mark.id).copied().unwrap_or(self.center);
                PlacedMark {
                    mark,
                    cx: x,
                    cy: y,
                    rotation: 0.0,
                    scale: 1.0,
                }
            })
            .collect::<Vec<_>>();

        let mut structural = Vec::new();
        if self.parent_child_curves {
            tree.forest.traverse_dfs_pre(|node| {
                if let Some(children) = tree.forest.children(node.id()) {
                    for child in children {
                        let from = node.id();
                        let to = child.id();
                        if let (Some((sx, sy)), Some((tx, ty))) =
                            (positions.get(from), positions.get(to))
                        {
                            let path = format!("M{sx:.2} {sy:.2} Q{sx:.2} {ty:.2} {tx:.2} {ty:.2}");
                            structural.push(StructuralMark {
                                path,
                                stroke_width: 0.4,
                                opacity: 0.5,
                            });
                        }
                    }
                }
            });
        }
        if self.ring_guides {
            let max_depth = tree
                .attributes
                .values()
                .filter_map(|attrs| match attrs.get("depth") {
                    Some(AttributeValue::Number(value)) => Some(*value as usize),
                    _ => None,
                })
                .max()
                .unwrap_or(0);
            for depth in 1..=max_depth {
                let r = self.ring_step * depth as f64;
                let path = format!(
                    "M{cx} {cy} m-{r} 0 a{r} {r} 0 1 0 {d} 0 a{r} {r} 0 1 0 -{d} 0",
                    cx = self.center.0,
                    cy = self.center.1,
                    r = r,
                    d = r * 2.0
                );
                structural.push(StructuralMark {
                    path,
                    stroke_width: 0.3,
                    opacity: 0.18,
                });
            }
        }

        Ok(Layout {
            marks: placed,
            structural,
        })
    }
}

impl Layouter for FractalBranch {
    fn expected_ir(&self) -> IrKind {
        IrKind::TensionTree
    }

    fn layout(
        &self,
        ir: &Featurized,
        marks: Vec<MarkSpec>,
        _ctx: &mut Ctx<'_>,
    ) -> Result<Layout, SigilError> {
        let tree = match ir {
            Featurized::TensionTree(tree) => tree,
            _ => return Err(SigilError::internal("fractal_branch expects TensionTree")),
        };
        let root_id = tree
            .forest
            .root_ids()
            .first()
            .cloned()
            .unwrap_or_else(|| marks.first().map(|m| m.id.clone()).unwrap_or_default());
        let mut positions = HashMap::new();
        positions.insert(root_id.clone(), self.center);
        let mut stack = vec![(root_id.clone(), 0usize, 0.0)];
        while let Some((id, depth, angle)) = stack.pop() {
            let children = tree.forest.children(&id).unwrap_or_default();
            if children.is_empty() {
                continue;
            }
            let spread = PI / 3.0;
            let step = if children.len() > 1 {
                spread / (children.len() - 1) as f64
            } else {
                0.0
            };
            for (idx, child) in children.iter().enumerate() {
                let a = angle - spread / 2.0 + step * idx as f64;
                let radius = self.branch_step / (depth as f64 + 1.0);
                let parent = positions.get(&id).copied().unwrap_or(self.center);
                let x = parent.0 + radius * a.cos();
                let y = parent.1 + radius * a.sin();
                positions.insert(child.id().to_string(), (x, y));
                stack.push((child.id().to_string(), depth + 1, a));
            }
        }
        let placed = marks
            .into_iter()
            .map(|mark| {
                let (x, y) = positions.get(&mark.id).copied().unwrap_or(self.center);
                PlacedMark {
                    mark,
                    cx: x,
                    cy: y,
                    rotation: 0.0,
                    scale: 1.0,
                }
            })
            .collect();
        Ok(Layout {
            marks: placed,
            structural: Vec::new(),
        })
    }
}

impl Layouter for Constellation {
    fn expected_ir(&self) -> IrKind {
        IrKind::AttributeGraph
    }

    fn layout(
        &self,
        ir: &Featurized,
        marks: Vec<MarkSpec>,
        ctx: &mut Ctx<'_>,
    ) -> Result<Layout, SigilError> {
        let graph = match ir {
            Featurized::AttributeGraph(graph) => graph,
            _ => return Err(SigilError::internal("constellation expects AttributeGraph")),
        };

        let mut positions: HashMap<String, (f64, f64)> = HashMap::new();
        for mark in &marks {
            let jitter = (ctx.rng.next_u64() % 100) as f64 / 100.0;
            positions.insert(
                mark.id.clone(),
                (
                    self.center.0 + (jitter - 0.5) * 200.0,
                    self.center.1 + (0.5 - jitter) * 200.0,
                ),
            );
        }

        for _ in 0..6 {
            for node_id in graph.graph.nodes_ordered() {
                for (from, to) in graph.graph.out_edges(node_id) {
                    let strength = 0.1;
                    if let (Some((fx, fy)), Some((tx, ty))) =
                        (positions.get(from), positions.get(to))
                    {
                        let nx = fx + (tx - fx) * strength;
                        let ny = fy + (ty - fy) * strength;
                        positions.insert(from.to_string(), (nx, ny));
                    }
                }
            }
        }

        let placed = marks
            .into_iter()
            .map(|mark| {
                let (x, y) = positions.get(&mark.id).copied().unwrap_or(self.center);
                PlacedMark {
                    mark,
                    cx: x,
                    cy: y,
                    rotation: 0.0,
                    scale: 1.0,
                }
            })
            .collect();
        Ok(Layout {
            marks: placed,
            structural: Vec::new(),
        })
    }
}

impl Layouter for Grid {
    fn expected_ir(&self) -> IrKind {
        IrKind::TensionTree
    }

    fn layout(
        &self,
        _ir: &Featurized,
        marks: Vec<MarkSpec>,
        _ctx: &mut Ctx<'_>,
    ) -> Result<Layout, SigilError> {
        let mut marks = marks;
        marks.sort_by(|a, b| {
            let a_key = mark_short_code(a).unwrap_or(f64::MAX);
            let b_key = mark_short_code(b).unwrap_or(f64::MAX);
            a_key
                .partial_cmp(&b_key)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut placed = Vec::new();
        let start_x = self.center.0 - (self.columns as f64 - 1.0) * self.cell_size / 2.0;
        let start_y = self.center.1 - (self.rows as f64 - 1.0) * self.cell_size / 2.0;
        for (idx, mark) in marks.into_iter().enumerate() {
            let col = idx % self.columns;
            let row = idx / self.columns;
            let x = start_x + col as f64 * self.cell_size;
            let y = start_y + row as f64 * self.cell_size;
            placed.push(PlacedMark {
                mark,
                cx: x,
                cy: y,
                rotation: 0.0,
                scale: 1.0,
            });
        }
        Ok(Layout {
            marks: placed,
            structural: Vec::new(),
        })
    }
}

fn mark_short_code(mark: &MarkSpec) -> Option<f64> {
    match mark.channels.get("short_code") {
        Some(crate::stages::ChannelValue::Number(value)) => Some(*value),
        _ => None,
    }
}

fn number_attr(tree: &TensionTree, id: &str, key: &str) -> Option<f64> {
    tree.attributes
        .get(id)
        .and_then(|attrs| match attrs.get(key) {
            Some(AttributeValue::Number(value)) => Some(*value),
            Some(AttributeValue::Bool(value)) => Some(if *value { 1.0 } else { 0.0 }),
            _ => None,
        })
}

fn categorical_attr<'a>(tree: &'a TensionTree, id: &str, key: &str) -> Option<&'a str> {
    tree.attributes
        .get(id)
        .and_then(|attrs| match attrs.get(key) {
            Some(AttributeValue::Categorical(value)) => Some(value.as_str()),
            Some(AttributeValue::Text(value)) => Some(value.as_str()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Primitive;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;
    use werk_core::ir::AttributeGraph;
    use werk_core::store::Store;

    fn simple_tree() -> (TensionTree, Vec<MarkSpec>) {
        let store = Store::new_in_memory().unwrap();
        let root = store.create_tension("root", "root").unwrap();
        let child = store
            .create_tension_with_parent("child", "child", Some(root.id.clone()))
            .unwrap();
        let forest =
            werk_core::tree::Forest::from_tensions(vec![root.clone(), child.clone()]).unwrap();
        let mut attrs = HashMap::new();
        let mut root_attrs = werk_core::ir::Attributes::new();
        root_attrs.insert("depth", AttributeValue::Number(0.0));
        root_attrs.insert("descendant_count", AttributeValue::Number(1.0));
        root_attrs.insert("status", AttributeValue::Categorical("active".into()));
        attrs.insert(root.id.clone(), root_attrs);
        let mut child_attrs = werk_core::ir::Attributes::new();
        child_attrs.insert("depth", AttributeValue::Number(1.0));
        child_attrs.insert("descendant_count", AttributeValue::Number(0.0));
        child_attrs.insert("status", AttributeValue::Categorical("active".into()));
        attrs.insert(child.id.clone(), child_attrs);
        let tree = TensionTree {
            forest,
            attributes: attrs,
        };
        let marks = vec![
            MarkSpec {
                id: root.id.clone(),
                primitive: Primitive::Circle,
                channels: HashMap::new(),
            },
            MarkSpec {
                id: child.id.clone(),
                primitive: Primitive::Circle,
                channels: HashMap::new(),
            },
        ];
        (tree, marks)
    }

    #[test]
    fn places_root_and_rings() {
        let (tree, marks) = simple_tree();
        let root_id = tree.forest.root_ids().first().cloned().unwrap_or_default();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(
            Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            &store,
            "werk",
            0,
        );
        let layouter = RadialMandala {
            ring_step: 80.0,
            inner_padding: 0.08,
            root_radius: 12.0,
            center: (300.0, 300.0),
            parent_child_curves: true,
            ring_guides: true,
            respect_status: true,
        };
        let layout = layouter.layout(&ir, marks, &mut ctx).unwrap();
        let root = layout.marks.iter().find(|m| m.mark.id == root_id).unwrap();
        assert_eq!(root.cx, 300.0);
        assert_eq!(root.cy, 300.0);
        let child = layout
            .marks
            .iter()
            .find(|m| m.mark.id != root.mark.id)
            .unwrap();
        let distance = ((child.cx - 300.0).powi(2) + (child.cy - 300.0).powi(2)).sqrt();
        assert!((distance - 80.0).abs() < 1.0);
        assert!(!layout.structural.is_empty());
    }

    #[test]
    fn recursive_placement() {
        let (tree, marks) = simple_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 0);
        let layouter = FractalBranch {
            center: (300.0, 300.0),
            branch_step: 120.0,
        };
        let layout = layouter.layout(&ir, marks, &mut ctx).unwrap();
        assert_eq!(layout.marks.len(), 2);
    }

    #[test]
    fn deterministic_force_layout() {
        let store = Store::new_in_memory().unwrap();
        let a = store.create_tension("a", "a").unwrap();
        let b = store.create_tension("b", "b").unwrap();
        store
            .create_edge(&a.id, &b.id, werk_core::edge::EDGE_CONTAINS)
            .unwrap();
        let edges = store.get_all_edges().unwrap();
        let ir_ctx = werk_core::ir::IrContext::new(Utc::now(), "werk");
        let graph =
            AttributeGraph::build(&store, vec![a.clone(), b.clone()], &edges, &ir_ctx).unwrap();
        let marks = vec![
            MarkSpec {
                id: a.id.clone(),
                primitive: Primitive::Circle,
                channels: HashMap::new(),
            },
            MarkSpec {
                id: b.id.clone(),
                primitive: Primitive::Circle,
                channels: HashMap::new(),
            },
        ];
        let ir = Featurized::AttributeGraph(graph);
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 7);
        let layouter = Constellation {
            center: (300.0, 300.0),
        };
        let layout1 = layouter.layout(&ir, marks.clone(), &mut ctx).unwrap();
        let mut ctx2 = Ctx::new(Utc::now(), &store, "werk", 7);
        let layout2 = layouter.layout(&ir, marks, &mut ctx2).unwrap();
        assert_eq!(layout1.marks[0].cx, layout2.marks[0].cx);
    }

    #[test]
    fn tiles_2x2() {
        let (tree, marks) = simple_tree();
        let ir = Featurized::TensionTree(tree);
        let store = Store::new_in_memory().unwrap();
        let mut ctx = Ctx::new(Utc::now(), &store, "werk", 0);
        let layouter = Grid {
            center: (300.0, 300.0),
            columns: 2,
            rows: 2,
            cell_size: 200.0,
        };
        let layout = layouter.layout(&ir, marks, &mut ctx).unwrap();
        assert_eq!(layout.marks.len(), 2);
        assert!(layout.marks.iter().any(|m| m.cx != 300.0));
    }
}
