//! Graph intelligence — structural signals computed from the tension tree
//! using FrankenNetworkX algorithms.
//!
//! All signals are Layer 2: relational inferences from user-supplied structural
//! relationships (parent-child containment). Surfaced by exception.

use std::collections::HashMap;

use fnx_algorithms::{
    BetweennessCentralityResult, ComplexityWitness,
    betweenness_centrality_directed, dag_longest_path, descendants, topological_generations,
};
use serde::{Deserialize, Serialize};

use crate::tree::Forest;

/// Structural signals for a single tension, computed from graph topology.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuralSignals {
    /// Betweenness centrality (0.0–1.0). Higher = more shortest paths
    /// route through this node = structural hub.
    pub centrality: Option<f64>,
    /// Which topological wave this tension belongs to.
    /// Wave 0 = roots, wave 1 = children of roots, etc.
    pub wave: Option<usize>,
    /// Whether this tension lies on the DAG's longest path (deepest chain).
    pub on_longest_path: bool,
    /// Count of transitive descendants (structural reach).
    pub descendant_count: Option<usize>,
}

/// Record of what an algorithm computed — for ground mode transparency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationWitness {
    pub algorithm: String,
    pub complexity: String,
    pub nodes_touched: usize,
    pub edges_scanned: usize,
}

impl From<&ComplexityWitness> for ComputationWitness {
    fn from(w: &ComplexityWitness) -> Self {
        Self {
            algorithm: w.algorithm.clone(),
            complexity: w.complexity_claim.clone(),
            nodes_touched: w.nodes_touched,
            edges_scanned: w.edges_scanned,
        }
    }
}

/// All structural signals for the entire field, computed in one batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldStructuralSignals {
    /// Per-tension structural signals, keyed by tension ID.
    pub signals: HashMap<String, StructuralSignals>,
    /// The longest path through the DAG (tension IDs in order).
    pub longest_path: Vec<String>,
    /// Total number of topological waves (depth of the tree).
    pub wave_count: usize,
    /// Algorithm witnesses for ground mode.
    pub witnesses: Vec<ComputationWitness>,
}

/// Compute all structural signals for the entire forest in one pass.
///
/// Runs betweenness centrality, topological generations, and longest path
/// on the forest's DiGraph. Returns per-tension signals plus field-level
/// summaries. All algorithms are O(V·E) or better — microseconds for
/// typical forests (< 1000 tensions).
pub fn compute_structural_signals(forest: &Forest) -> FieldStructuralSignals {
    let graph = forest.graph();
    let mut signals: HashMap<String, StructuralSignals> = HashMap::new();
    let mut witnesses = Vec::new();

    // Initialize signals for every node.
    for node_id in graph.nodes_ordered() {
        signals.insert(node_id.to_string(), StructuralSignals::default());
    }

    // Betweenness centrality — identifies structural hubs.
    let centrality: BetweennessCentralityResult = betweenness_centrality_directed(graph);
    for score in &centrality.scores {
        if let Some(s) = signals.get_mut(&score.node) {
            s.centrality = Some(score.score);
        }
    }
    witnesses.push(ComputationWitness::from(&centrality.witness));

    // Topological generations — concurrent possibility layers.
    let mut wave_count = 0;
    if let Some(topo_gen) = topological_generations(graph) {
        wave_count = topo_gen.generations.len();
        for (wave_idx, generation) in topo_gen.generations.iter().enumerate() {
            for node_id in generation {
                if let Some(s) = signals.get_mut(node_id) {
                    s.wave = Some(wave_idx);
                }
            }
        }
        witnesses.push(ComputationWitness::from(&topo_gen.witness));
    }

    // Longest path — the deepest structural chain.
    let longest_path = dag_longest_path(graph).unwrap_or_default();
    for node_id in &longest_path {
        if let Some(s) = signals.get_mut(node_id) {
            s.on_longest_path = true;
        }
    }

    // Descendant counts — structural reach / blast radius.
    // Only compute for nodes that have children (leaf nodes always have 0).
    for node_id in graph.nodes_ordered() {
        if graph.out_degree(node_id) > 0 {
            let desc = descendants(graph, node_id);
            if let Some(s) = signals.get_mut(node_id) {
                s.descendant_count = Some(desc.len());
            }
        }
    }

    FieldStructuralSignals {
        signals,
        longest_path,
        wave_count,
        witnesses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tension::Tension;

    fn make_tension(id: &str, parent: Option<&str>) -> Tension {
        let mut t = if let Some(pid) = parent {
            Tension::new_with_parent("desired", "actual", Some(pid.to_string())).unwrap()
        } else {
            Tension::new("desired", "actual").unwrap()
        };
        t.id = id.to_string();
        t
    }

    #[test]
    fn empty_forest() {
        let forest = Forest::from_tensions(vec![]).unwrap();
        let result = compute_structural_signals(&forest);
        assert!(result.signals.is_empty());
        assert!(result.longest_path.is_empty());
        assert_eq!(result.wave_count, 0);
    }

    #[test]
    fn single_node() {
        let forest = Forest::from_tensions(vec![make_tension("a", None)]).unwrap();
        let result = compute_structural_signals(&forest);
        assert_eq!(result.signals.len(), 1);
        let s = &result.signals["a"];
        assert_eq!(s.centrality, Some(0.0));
        assert_eq!(s.wave, Some(0));
        assert_eq!(s.descendant_count, None); // leaf — not computed
        assert_eq!(result.wave_count, 1);
    }

    #[test]
    fn linear_chain() {
        // a → b → c → d
        let forest = Forest::from_tensions(vec![
            make_tension("a", None),
            make_tension("b", Some("a")),
            make_tension("c", Some("b")),
            make_tension("d", Some("c")),
        ])
        .unwrap();

        let result = compute_structural_signals(&forest);
        assert_eq!(result.wave_count, 4);
        assert_eq!(result.longest_path, vec!["a", "b", "c", "d"]);

        // All nodes on longest path.
        for id in &["a", "b", "c", "d"] {
            assert!(result.signals[*id].on_longest_path);
        }

        // Interior nodes have centrality > 0; endpoints have 0.
        assert_eq!(result.signals["a"].centrality, Some(0.0));
        assert!(result.signals["b"].centrality.unwrap() > 0.0);
        assert!(result.signals["c"].centrality.unwrap() > 0.0);
        assert_eq!(result.signals["d"].centrality, Some(0.0));

        // Descendant counts.
        assert_eq!(result.signals["a"].descendant_count, Some(3));
        assert_eq!(result.signals["b"].descendant_count, Some(2));
        assert_eq!(result.signals["c"].descendant_count, Some(1));
        assert_eq!(result.signals["d"].descendant_count, None); // leaf
    }

    #[test]
    fn wide_fan() {
        // root → {c1, c2, c3, c4}
        let forest = Forest::from_tensions(vec![
            make_tension("root", None),
            make_tension("c1", Some("root")),
            make_tension("c2", Some("root")),
            make_tension("c3", Some("root")),
            make_tension("c4", Some("root")),
        ])
        .unwrap();

        let result = compute_structural_signals(&forest);
        assert_eq!(result.wave_count, 2);
        assert_eq!(result.signals["root"].descendant_count, Some(4));
        assert_eq!(result.signals["root"].wave, Some(0));

        // All children are wave 1.
        for id in &["c1", "c2", "c3", "c4"] {
            assert_eq!(result.signals[*id].wave, Some(1));
        }
    }

    #[test]
    fn diamond_structure() {
        // root → {left, right}, left → bottom, right → bottom
        // This is not a valid tree (bottom has two parents), but Forest
        // only keeps the first parent_id. So it's really root → {left, right},
        // left → bottom OR right → bottom depending on insertion order.
        // Test that we don't panic.
        let mut bottom = Tension::new_with_parent("desired", "actual", Some("left".to_string())).unwrap();
        bottom.id = "bottom".to_string();

        let forest = Forest::from_tensions(vec![
            make_tension("root", None),
            make_tension("left", Some("root")),
            make_tension("right", Some("root")),
            bottom,
        ])
        .unwrap();

        let result = compute_structural_signals(&forest);
        assert_eq!(result.signals.len(), 4);
        assert!(result.wave_count >= 2);
    }

    #[test]
    fn witnesses_recorded() {
        let forest = Forest::from_tensions(vec![
            make_tension("a", None),
            make_tension("b", Some("a")),
        ])
        .unwrap();

        let result = compute_structural_signals(&forest);
        // At least centrality + topo generations witnesses.
        assert!(result.witnesses.len() >= 2);
        assert!(result.witnesses.iter().any(|w| w.algorithm.contains("betweenness")));
    }
}
