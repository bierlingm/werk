//! Forest construction and tree operations for tensions.
//!
//! A forest is a collection of trees (multiple roots allowed). Each tension
//! can have an optional parent, forming a hierarchical structure. Loose
//! tensions (no parent, no children) appear as isolated roots.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::tension::Tension;

/// Errors that can occur during forest construction or tree operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TreeError {
    /// A tension references itself as its parent.
    #[error("self-reference detected: tension {0} references itself as parent")]
    SelfReference(String),

    /// A circular parent chain was detected.
    #[error("circular reference detected in chain: {0}")]
    CircularReference(String),
}

/// A node in the forest tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// The tension at this node.
    pub tension: Tension,
    /// Child node IDs (stored separately for efficient lookups).
    #[serde(skip)]
    pub children: Vec<String>,
}

impl Node {
    /// Create a new node from a tension.
    pub fn new(tension: Tension) -> Self {
        Self {
            tension,
            children: Vec::new(),
        }
    }

    /// Get the ID of this node's tension.
    pub fn id(&self) -> &str {
        &self.tension.id
    }

    /// Get the parent ID of this node's tension.
    pub fn parent_id(&self) -> Option<&str> {
        self.tension.parent_id.as_deref()
    }
}

/// A forest of tensions — multiple trees with potentially multiple roots.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Forest {
    /// All nodes indexed by tension ID for O(1) lookup.
    nodes: HashMap<String, Node>,
    /// Root node IDs (tensions with no parent or orphaned).
    roots: Vec<String>,
}

impl Forest {
    /// Create an empty forest.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
        }
    }

    /// Build a forest from a flat list of tensions.
    ///
    /// Validates for self-references and circular references.
    /// Orphans (parent_id pointing to non-existent tension) become roots.
    ///
    /// # Errors
    ///
    /// Returns `TreeError::SelfReference` if any tension references itself.
    /// Returns `TreeError::CircularReference` if a circular parent chain is detected.
    pub fn from_tensions(tensions: Vec<Tension>) -> Result<Self, TreeError> {
        let mut forest = Forest::new();

        if tensions.is_empty() {
            return Ok(forest);
        }

        // First pass: create all nodes and check for self-references
        for tension in tensions.iter() {
            // Check for self-reference
            if let Some(ref parent_id) = tension.parent_id
                && parent_id == &tension.id
            {
                return Err(TreeError::SelfReference(tension.id.clone()));
            }

            let node = Node::new(tension.clone());
            forest.nodes.insert(tension.id.clone(), node);
        }

        // Second pass: establish parent-child relationships and identify roots/orphans
        let tension_ids: HashSet<&str> = tensions.iter().map(|t| t.id.as_str()).collect();

        for tension in tensions.iter() {
            if let Some(ref parent_id) = tension.parent_id {
                if tension_ids.contains(parent_id.as_str()) {
                    // Add child to parent
                    if let Some(parent_node) = forest.nodes.get_mut(parent_id) {
                        parent_node.children.push(tension.id.clone());
                    }
                } else {
                    // Orphan: parent doesn't exist, treat as root
                    forest.roots.push(tension.id.clone());
                }
            } else {
                // No parent = root
                forest.roots.push(tension.id.clone());
            }
        }

        // Third pass: detect cycles using DFS
        // This is O(n + e) instead of O(n * depth)
        if let Some(cycle_node) = forest.detect_cycle() {
            return Err(TreeError::CircularReference(cycle_node));
        }

        // Roots and children are naturally ordered by creation time since we process
        // tensions in the order they were created. No explicit sorting needed.

        Ok(forest)
    }

    /// Detect cycles using DFS. Returns the cycle description if found.
    fn detect_cycle(&self) -> Option<String> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum VisitState {
            Unvisited,
            Visiting, // In current DFS path
            Visited,  // Fully processed
        }

        let mut state: HashMap<&str, VisitState> = self
            .nodes
            .keys()
            .map(|k| k.as_str())
            .map(|k| (k, VisitState::Unvisited))
            .collect();

        fn dfs<'a>(
            forest: &'a Forest,
            node_id: &'a str,
            state: &mut HashMap<&'a str, VisitState>,
            path: &mut Vec<&'a str>,
        ) -> Option<String> {
            match state.get(node_id).copied() {
                Some(VisitState::Visiting) => {
                    // Found a cycle - node is in current path
                    // Find where this node appears in the path
                    if let Some(pos) = path.iter().position(|&id| id == node_id) {
                        let cycle: Vec<&str> = path[pos..].to_vec();
                        return Some(format!("{} -> {}", cycle.join(" -> "), node_id));
                    }
                    return Some(format!("cycle at {}", node_id));
                }
                Some(VisitState::Visited) => return None,
                Some(VisitState::Unvisited) | None => {}
            }

            state.insert(node_id, VisitState::Visiting);
            path.push(node_id);

            if let Some(node) = forest.nodes.get(node_id) {
                for child_id in &node.children {
                    if let Some(cycle) = dfs(forest, child_id.as_str(), state, path) {
                        return Some(cycle);
                    }
                }
            }

            path.pop();
            state.insert(node_id, VisitState::Visited);
            None
        }

        for root_id in &self.roots {
            let mut path = Vec::new();
            if let Some(cycle) = dfs(self, root_id.as_str(), &mut state, &mut path) {
                return Some(cycle);
            }
        }

        // Also check for cycles in non-root nodes (in case of orphan or disconnected components)
        for node_id in self.nodes.keys() {
            if state.get(node_id.as_str()) == Some(&VisitState::Unvisited) {
                let mut path = Vec::new();
                if let Some(cycle) = dfs(self, node_id.as_str(), &mut state, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// Get the number of nodes in the forest.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the forest is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the number of root nodes.
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Get all root node IDs.
    pub fn root_ids(&self) -> &[String] {
        &self.roots
    }

    /// Find a node by tension ID.
    pub fn find(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Get the children of a node.
    pub fn children(&self, id: &str) -> Option<Vec<&Node>> {
        self.nodes.get(id).map(|node| {
            node.children
                .iter()
                .filter_map(|cid| self.nodes.get(cid))
                .collect()
        })
    }

    /// Get the siblings of a node (other children of the same parent).
    pub fn siblings(&self, id: &str) -> Option<Vec<&Node>> {
        let node = self.nodes.get(id)?;

        match &node.tension.parent_id {
            Some(parent_id) => self.children(parent_id).map(|children| {
                children
                    .into_iter()
                    .filter(|c| c.tension.id != id)
                    .collect()
            }),
            None => {
                // Root node: siblings are other roots
                Some(
                    self.roots
                        .iter()
                        .filter(|rid| *rid != id)
                        .filter_map(|rid| self.nodes.get(rid))
                        .collect(),
                )
            }
        }
    }

    /// Get the ancestor chain for a node, root-first.
    pub fn ancestors(&self, id: &str) -> Option<Vec<&Node>> {
        let node = self.nodes.get(id)?;

        let mut ancestors = Vec::new();
        let mut current = node.tension.parent_id.clone();

        while let Some(parent_id) = current {
            if let Some(parent_node) = self.nodes.get(&parent_id) {
                ancestors.push(parent_node);
                current = parent_node.tension.parent_id.clone();
            } else {
                break; // Parent doesn't exist (orphan case)
            }
        }

        // Reverse to get root-first order
        ancestors.reverse();
        Some(ancestors)
    }

    /// Get all descendants of a node.
    pub fn descendants(&self, id: &str) -> Option<Vec<&Node>> {
        let _node = self.nodes.get(id)?;

        let mut descendants = Vec::new();
        let mut queue: Vec<&str> = vec![id];

        while let Some(current_id) = queue.pop() {
            if let Some(node) = self.nodes.get(current_id) {
                for child_id in &node.children {
                    if let Some(child_node) = self.nodes.get(child_id) {
                        descendants.push(child_node);
                        queue.push(child_id);
                    }
                }
            }
        }

        Some(descendants)
    }

    /// Extract a subtree rooted at the given node.
    pub fn subtree(&self, id: &str) -> Option<Forest> {
        let _root_node = self.nodes.get(id)?;

        let mut subtree = Forest::new();

        // Collect all nodes in the subtree using DFS
        let mut stack = vec![id];
        while let Some(current_id) = stack.pop() {
            if let Some(node) = self.nodes.get(current_id) {
                let mut subtree_node = node.clone();
                subtree_node.tension.parent_id = if current_id == id {
                    // Root of subtree has no parent
                    None
                } else {
                    // Keep the parent reference for structure
                    node.tension.parent_id.clone()
                };

                // Add children to stack for processing
                for child_id in &node.children {
                    stack.push(child_id.as_str());
                }

                subtree.nodes.insert(current_id.to_string(), subtree_node);
            }
        }

        // Set the root
        subtree.roots.push(id.to_string());

        Some(subtree)
    }

    /// Traverse the forest depth-first, pre-order (node before children).
    pub fn traverse_dfs_pre<F>(&self, mut visitor: F)
    where
        F: FnMut(&Node),
    {
        for root_id in &self.roots {
            self.dfs_pre_recursive(root_id, &mut visitor);
        }
    }

    fn dfs_pre_recursive<F>(&self, node_id: &str, visitor: &mut F)
    where
        F: FnMut(&Node),
    {
        if let Some(node) = self.nodes.get(node_id) {
            visitor(node);
            for child_id in &node.children {
                self.dfs_pre_recursive(child_id, visitor);
            }
        }
    }

    /// Traverse the forest depth-first, post-order (children before node).
    pub fn traverse_dfs_post<F>(&self, mut visitor: F)
    where
        F: FnMut(&Node),
    {
        for root_id in &self.roots {
            self.dfs_post_recursive(root_id, &mut visitor);
        }
    }

    fn dfs_post_recursive<F>(&self, node_id: &str, visitor: &mut F)
    where
        F: FnMut(&Node),
    {
        if let Some(node) = self.nodes.get(node_id) {
            for child_id in &node.children {
                self.dfs_post_recursive(child_id, visitor);
            }
            visitor(node);
        }
    }

    /// Traverse the forest breadth-first across all roots.
    pub fn traverse_bfs<F>(&self, mut visitor: F)
    where
        F: FnMut(&Node),
    {
        use std::collections::VecDeque;

        let mut queue: VecDeque<&str> = self.roots.iter().map(|s| s.as_str()).collect();

        while let Some(node_id) = queue.pop_front() {
            if let Some(node) = self.nodes.get(node_id) {
                visitor(node);
                for child_id in &node.children {
                    queue.push_back(child_id);
                }
            }
        }
    }

    /// Get the depth of a node (distance from root).
    pub fn depth(&self, id: &str) -> Option<usize> {
        let node = self.nodes.get(id)?;

        let mut depth = 0;
        let mut current = node.tension.parent_id.clone();

        while let Some(parent_id) = current {
            depth += 1;
            if let Some(parent_node) = self.nodes.get(&parent_id) {
                current = parent_node.tension.parent_id.clone();
            } else {
                break;
            }
        }

        Some(depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tension::Tension;

    // Helper to create tensions with specific IDs for testing
    fn make_tension_with_id(id: &str, desired: &str, actual: &str) -> Tension {
        let mut t = Tension::new(desired, actual).unwrap();
        t.id = id.to_string();
        t
    }

    fn make_tension_with_parent(
        id: &str,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
    ) -> Tension {
        let mut t = Tension::new_with_parent(desired, actual, parent_id).unwrap();
        t.id = id.to_string();
        t
    }

    // ── Empty Forest ─────────────────────────────────────────────

    #[test]
    fn test_empty_forest() {
        let forest = Forest::from_tensions(vec![]).unwrap();
        assert!(forest.is_empty());
        assert_eq!(forest.len(), 0);
        assert_eq!(forest.root_count(), 0);
    }

    // ── Single Root ───────────────────────────────────────────────

    #[test]
    fn test_single_root() {
        let t1 = Tension::new("goal", "reality").unwrap();
        let id = t1.id.clone();

        let forest = Forest::from_tensions(vec![t1]).unwrap();

        assert!(!forest.is_empty());
        assert_eq!(forest.len(), 1);
        assert_eq!(forest.root_count(), 1);
        assert!(forest.find(&id).is_some());
    }

    // ── Multiple Roots ─────────────────────────────────────────────

    #[test]
    fn test_multiple_roots() {
        let t1 = Tension::new("goal1", "reality1").unwrap();
        let t2 = Tension::new("goal2", "reality2").unwrap();
        let t3 = Tension::new("goal3", "reality3").unwrap();

        let id1 = t1.id.clone();
        let id2 = t2.id.clone();
        let id3 = t3.id.clone();

        let forest = Forest::from_tensions(vec![t1, t2, t3]).unwrap();

        assert_eq!(forest.len(), 3);
        assert_eq!(forest.root_count(), 3);

        // All should be roots (no parent)
        assert!(forest.find(&id1).unwrap().parent_id().is_none());
        assert!(forest.find(&id2).unwrap().parent_id().is_none());
        assert!(forest.find(&id3).unwrap().parent_id().is_none());
    }

    // ── Parent-Child Relationships ────────────────────────────────

    #[test]
    fn test_parent_child_relationship() {
        let parent = Tension::new("parent goal", "parent reality").unwrap();
        let parent_id = parent.id.clone();

        let child =
            Tension::new_with_parent("child goal", "child reality", Some(parent_id.clone()))
                .unwrap();
        let child_id = child.id.clone();

        let forest = Forest::from_tensions(vec![parent, child]).unwrap();

        assert_eq!(forest.len(), 2);
        assert_eq!(forest.root_count(), 1);

        // Parent should have one child
        let parent_node = forest.find(&parent_id).unwrap();
        assert_eq!(parent_node.children.len(), 1);
        assert_eq!(parent_node.children[0], child_id);

        // Child should have parent
        let child_node = forest.find(&child_id).unwrap();
        assert_eq!(child_node.parent_id(), Some(parent_id.as_str()));
    }

    // ── Deep Hierarchy ────────────────────────────────────────────

    #[test]
    fn test_deep_hierarchy() {
        // Create chain: A -> B -> C -> D
        let a = make_tension_with_id("A", "goal a", "reality a");
        let b = make_tension_with_parent("B", "goal b", "reality b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "goal c", "reality c", Some("B".to_string()));
        let d = make_tension_with_parent("D", "goal d", "reality d", Some("C".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        assert_eq!(forest.len(), 4);
        assert_eq!(forest.root_count(), 1);
        assert_eq!(forest.root_ids()[0], "A");

        // Check depths
        assert_eq!(forest.depth("A").unwrap(), 0);
        assert_eq!(forest.depth("B").unwrap(), 1);
        assert_eq!(forest.depth("C").unwrap(), 2);
        assert_eq!(forest.depth("D").unwrap(), 3);
    }

    // ── Loose Tensions ────────────────────────────────────────────

    #[test]
    fn test_loose_tensions() {
        // Create one root with children, and one loose tension
        let parent = Tension::new("parent", "reality").unwrap();
        let parent_id = parent.id.clone();

        let child = Tension::new_with_parent("child", "reality", Some(parent_id.clone())).unwrap();

        let loose = Tension::new("loose", "reality").unwrap();
        let loose_id = loose.id.clone();

        let forest = Forest::from_tensions(vec![parent, child, loose]).unwrap();

        assert_eq!(forest.len(), 3);
        assert_eq!(forest.root_count(), 2); // parent and loose are both roots

        // Loose tension has no parent and no children
        let loose_node = forest.find(&loose_id).unwrap();
        assert!(loose_node.parent_id().is_none());
        assert!(loose_node.children.is_empty());
    }

    // ── Orphan Handling ────────────────────────────────────────────

    #[test]
    fn test_orphan_handling() {
        // Child references non-existent parent
        let orphan =
            make_tension_with_parent("orphan", "goal", "reality", Some("nonexistent".to_string()));

        let forest = Forest::from_tensions(vec![orphan]).unwrap();

        assert_eq!(forest.len(), 1);
        assert_eq!(forest.root_count(), 1); // Orphan becomes root
        assert_eq!(forest.root_ids()[0], "orphan");
    }

    // ── Self-Reference Rejection ──────────────────────────────────

    #[test]
    fn test_self_reference_rejected() {
        let self_ref =
            make_tension_with_parent("self", "goal", "reality", Some("self".to_string()));

        let result = Forest::from_tensions(vec![self_ref]);

        assert!(result.is_err());
        match result.unwrap_err() {
            TreeError::SelfReference(id) => assert_eq!(id, "self"),
            _ => panic!("expected SelfReference error"),
        }
    }

    // ── Circular Reference Rejection ──────────────────────────────

    #[test]
    fn test_circular_reference_rejected() {
        // A -> B -> C -> A (cycle)
        let a = make_tension_with_parent("A", "goal a", "reality a", Some("C".to_string()));
        let b = make_tension_with_parent("B", "goal b", "reality b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "goal c", "reality c", Some("B".to_string()));

        let result = Forest::from_tensions(vec![a, b, c]);

        assert!(result.is_err());
        match result.unwrap_err() {
            TreeError::CircularReference(msg) => {
                assert!(msg.contains("->"));
            }
            _ => panic!("expected CircularReference error"),
        }
    }

    #[test]
    fn test_circular_reference_longer_chain() {
        // A -> B -> C -> D -> A (cycle back to A)
        let a = make_tension_with_parent("A", "goal a", "reality a", Some("D".to_string()));
        let b = make_tension_with_parent("B", "goal b", "reality b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "goal c", "reality c", Some("B".to_string()));
        let d = make_tension_with_parent("D", "goal d", "reality d", Some("C".to_string()));

        let result = Forest::from_tensions(vec![a, b, c, d]);
        assert!(result.is_err());
    }

    // ── DFS Pre-Order Traversal ───────────────────────────────────

    #[test]
    fn test_dfs_pre_order_single_root() {
        //     A
        //    / \
        //   B   C
        //  /
        // D
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("A".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("B".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        let mut visited = Vec::new();
        forest.traverse_dfs_pre(|node| visited.push(node.id().to_string()));

        // Pre-order: A, B, D, C
        assert_eq!(visited, vec!["A", "B", "D", "C"]);
    }

    #[test]
    fn test_dfs_pre_order_multiple_roots() {
        // A    X
        // |    |
        // B    Y
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let x = make_tension_with_id("X", "x", "x");
        let y = make_tension_with_parent("Y", "y", "y", Some("X".to_string()));

        let forest = Forest::from_tensions(vec![a, b, x, y]).unwrap();

        let mut visited = Vec::new();
        forest.traverse_dfs_pre(|node| visited.push(node.id().to_string()));

        // Pre-order across all roots: A, B, X, Y
        assert_eq!(visited, vec!["A", "B", "X", "Y"]);
    }

    // ── DFS Post-Order Traversal ──────────────────────────────────

    #[test]
    fn test_dfs_post_order_single_root() {
        //     A
        //    / \
        //   B   C
        //  /
        // D
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("A".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("B".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        let mut visited = Vec::new();
        forest.traverse_dfs_post(|node| visited.push(node.id().to_string()));

        // Post-order: D, B, C, A
        assert_eq!(visited, vec!["D", "B", "C", "A"]);
    }

    // ── BFS Traversal ──────────────────────────────────────────────

    #[test]
    fn test_bfs_single_root() {
        //     A        (level 0)
        //    / \
        //   B   C      (level 1)
        //  /
        // D            (level 2)
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("A".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("B".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        let mut visited = Vec::new();
        forest.traverse_bfs(|node| visited.push(node.id().to_string()));

        // BFS: A, B, C, D
        assert_eq!(visited, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_bfs_multiple_roots() {
        // A    X       (level 0)
        // |    |
        // B    Y       (level 1)
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let x = make_tension_with_id("X", "x", "x");
        let y = make_tension_with_parent("Y", "y", "y", Some("X".to_string()));

        let forest = Forest::from_tensions(vec![a, b, x, y]).unwrap();

        let mut visited = Vec::new();
        forest.traverse_bfs(|node| visited.push(node.id().to_string()));

        // BFS across all roots: A, X, B, Y
        assert_eq!(visited, vec!["A", "X", "B", "Y"]);
    }

    // ── Subtree Extraction ────────────────────────────────────────

    #[test]
    fn test_subtree_root() {
        //     A
        //    / \
        //   B   C
        //  /
        // D
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("A".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("B".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        let subtree = forest.subtree("A").unwrap();

        assert_eq!(subtree.len(), 4);
        assert_eq!(subtree.root_count(), 1);
        assert_eq!(subtree.root_ids()[0], "A");
    }

    #[test]
    fn test_subtree_internal_node() {
        //     A
        //    / \
        //   B   C
        //  /
        // D
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("A".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("B".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        let subtree = forest.subtree("B").unwrap();

        assert_eq!(subtree.len(), 2); // B and D
        assert_eq!(subtree.root_count(), 1);
        assert_eq!(subtree.root_ids()[0], "B");
        assert!(subtree.find("D").is_some());
        assert!(subtree.find("A").is_none()); // Parent not included
        assert!(subtree.find("C").is_none()); // Sibling not included
    }

    #[test]
    fn test_subtree_leaf() {
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));

        let forest = Forest::from_tensions(vec![a, b]).unwrap();

        let subtree = forest.subtree("B").unwrap();

        assert_eq!(subtree.len(), 1);
        assert_eq!(subtree.root_ids()[0], "B");
    }

    #[test]
    fn test_subtree_unknown_node() {
        let forest = Forest::from_tensions(vec![]).unwrap();
        assert!(forest.subtree("unknown").is_none());
    }

    // ── Ancestor Query ────────────────────────────────────────────

    #[test]
    fn test_ancestors_deep_chain() {
        // A -> B -> C -> D
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("B".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("C".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        let ancestors = forest.ancestors("D").unwrap();

        // Root-first: A, B, C
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0].id(), "A");
        assert_eq!(ancestors[1].id(), "B");
        assert_eq!(ancestors[2].id(), "C");
    }

    #[test]
    fn test_ancestors_root() {
        let a = make_tension_with_id("A", "a", "a");
        let forest = Forest::from_tensions(vec![a]).unwrap();

        let ancestors = forest.ancestors("A").unwrap();

        assert!(ancestors.is_empty()); // Root has no ancestors
    }

    #[test]
    fn test_ancestors_unknown_node() {
        let forest = Forest::from_tensions(vec![]).unwrap();
        assert!(forest.ancestors("unknown").is_none());
    }

    // ── Sibling Query ─────────────────────────────────────────────

    #[test]
    fn test_siblings_middle_child() {
        //     A
        //    /|\
        //   B C D
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("A".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("A".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d]).unwrap();

        let siblings = forest.siblings("C").unwrap();

        assert_eq!(siblings.len(), 2);
        let sibling_ids: Vec<&str> = siblings.iter().map(|n| n.id()).collect();
        assert!(sibling_ids.contains(&"B"));
        assert!(sibling_ids.contains(&"D"));
        assert!(!sibling_ids.contains(&"C")); // Not itself
    }

    #[test]
    fn test_siblings_only_child() {
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));

        let forest = Forest::from_tensions(vec![a, b]).unwrap();

        let siblings = forest.siblings("B").unwrap();
        assert!(siblings.is_empty());
    }

    #[test]
    fn test_siblings_root() {
        // Multiple roots
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_id("B", "b", "b");
        let c = make_tension_with_id("C", "c", "c");

        let forest = Forest::from_tensions(vec![a, b, c]).unwrap();

        let siblings = forest.siblings("B").unwrap();
        assert_eq!(siblings.len(), 2); // A and C
    }

    // ── Descendant Query ──────────────────────────────────────────

    #[test]
    fn test_descendants() {
        //     A
        //    /|\
        //   B C D
        //     |
        //     E
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));
        let c = make_tension_with_parent("C", "c", "c", Some("A".to_string()));
        let d = make_tension_with_parent("D", "d", "d", Some("A".to_string()));
        let e = make_tension_with_parent("E", "e", "e", Some("C".to_string()));

        let forest = Forest::from_tensions(vec![a, b, c, d, e]).unwrap();

        let descendants = forest.descendants("A").unwrap();
        assert_eq!(descendants.len(), 4);

        let descendant_ids: HashSet<&str> = descendants.iter().map(|n| n.id()).collect();
        assert!(descendant_ids.contains("B"));
        assert!(descendant_ids.contains("C"));
        assert!(descendant_ids.contains("D"));
        assert!(descendant_ids.contains("E"));
    }

    #[test]
    fn test_descendants_leaf() {
        let a = make_tension_with_id("A", "a", "a");
        let b = make_tension_with_parent("B", "b", "b", Some("A".to_string()));

        let forest = Forest::from_tensions(vec![a, b]).unwrap();

        let descendants = forest.descendants("B").unwrap();
        assert!(descendants.is_empty());
    }

    // ── Find by ID ────────────────────────────────────────────────

    #[test]
    fn test_find_existing() {
        let t = Tension::new("goal", "reality").unwrap();
        let id = t.id.clone();

        let forest = Forest::from_tensions(vec![t]).unwrap();

        let found = forest.find(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id(), id);
    }

    #[test]
    fn test_find_nonexistent() {
        let forest = Forest::from_tensions(vec![]).unwrap();
        assert!(forest.find("nonexistent").is_none());
    }

    // ── Performance ───────────────────────────────────────────────

    #[test]
    fn test_performance_1000_tensions() {
        use std::time::Instant;

        // Create 1000 tensions in a single deep chain + some extra trees
        let mut tensions = Vec::new();

        // Deep chain of 500
        let first = Tension::new("root", "reality").unwrap();
        let first_id = first.id.clone();
        tensions.push(first);

        let mut prev_id = first_id;
        for i in 1..500 {
            let t = Tension::new_with_parent(
                &format!("goal {}", i),
                &format!("reality {}", i),
                Some(prev_id.clone()),
            )
            .unwrap();
            prev_id = t.id.clone();
            tensions.push(t);
        }

        // Add 500 more roots
        for i in 0..500 {
            let t = Tension::new(
                &format!("extra goal {}", i),
                &format!("extra reality {}", i),
            )
            .unwrap();
            tensions.push(t);
        }

        let start = Instant::now();
        let forest = Forest::from_tensions(tensions).unwrap();
        let build_time = start.elapsed();

        // Traverse
        let start = Instant::now();
        let mut count = 0;
        forest.traverse_dfs_pre(|_| count += 1);
        let traverse_time = start.elapsed();

        // 100 lookups
        let start = Instant::now();
        for id in forest.root_ids().iter().take(100) {
            let _ = forest.find(id);
        }
        let lookup_time = start.elapsed();

        let total = build_time + traverse_time + lookup_time;

        println!("Build: {:?}", build_time);
        println!("Traverse: {:?}", traverse_time);
        println!("100 lookups: {:?}", lookup_time);
        println!("Total: {:?}", total);

        assert!(
            total.as_millis() < 100,
            "Performance target not met: {:?}",
            total
        );
        assert_eq!(count, 1000);
        assert_eq!(forest.len(), 1000);
    }

    // ── Empty Traversals ──────────────────────────────────────────

    #[test]
    fn test_traverse_empty_forest() {
        let forest = Forest::from_tensions(vec![]).unwrap();

        let mut visited_dfs_pre = Vec::new();
        forest.traverse_dfs_pre(|n| visited_dfs_pre.push(n.id().to_string()));
        assert!(visited_dfs_pre.is_empty());

        let mut visited_dfs_post = Vec::new();
        forest.traverse_dfs_post(|n| visited_dfs_post.push(n.id().to_string()));
        assert!(visited_dfs_post.is_empty());

        let mut visited_bfs = Vec::new();
        forest.traverse_bfs(|n| visited_bfs.push(n.id().to_string()));
        assert!(visited_bfs.is_empty());
    }

    // ── Node Trait Implementations ────────────────────────────────

    #[test]
    fn test_node_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Node>();
        assert_send_sync::<Forest>();
        assert_send_sync::<TreeError>();
    }

    #[test]
    fn test_node_debug_clone() {
        let t = Tension::new("goal", "reality").unwrap();
        let node = Node::new(t);
        let _ = format!("{:?}", node);
        let node2 = node.clone();
        assert_eq!(node, node2);
    }

    // ── TreeError Display ────────────────────────────────────────

    #[test]
    fn test_tree_error_display() {
        let e = TreeError::SelfReference("test".to_string());
        assert!(e.to_string().contains("self-reference"));

        let e = TreeError::CircularReference("A -> B".to_string());
        assert!(e.to_string().contains("circular"));
    }
}
