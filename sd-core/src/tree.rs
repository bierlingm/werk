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

    /// Get the children of a node, sorted by structural sequence.
    ///
    /// The display order mirrors the tension chart: vision at top, reality at bottom.
    /// Positioned children appear first, sorted by position DESC (highest position =
    /// closest to vision = top of display, position 1 = closest to reality = bottom
    /// of positioned group). Unpositioned children follow, sorted by deadline.
    ///
    /// Full ordering:
    /// 1. Positioned before unpositioned
    /// 2. Within positioned: `position DESC` (highest first = closest to vision)
    /// 3. Within unpositioned: earliest `range_end` first (soonest deadline)
    /// 4. Ties broken by precision (narrower first: DateTime < Day < Month < Year)
    /// 5. Nodes without horizons sort after those with horizons
    /// 6. Final tiebreaker: `created_at ASC`
    ///
    /// Returns an empty vector if the parent node doesn't exist or has no children.
    pub fn children_by_horizon(&self, parent_id: &str) -> Vec<&Node> {
        let Some(parent) = self.nodes.get(parent_id) else {
            return Vec::new();
        };

        let mut children: Vec<&Node> = parent
            .children
            .iter()
            .filter_map(|cid| self.nodes.get(cid))
            .collect();

        children.sort_by(|a, b| {
            let a_pos = a.tension.position;
            let b_pos = b.tension.position;

            // Positioned before unpositioned
            match (a_pos, b_pos) {
                // DESC within positioned: higher position = closer to vision = top
                (Some(pa), Some(pb)) => return pb.cmp(&pa),
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                (None, None) => {} // fall through to horizon ordering
            }

            // Unpositioned: sort by range_end (earliest deadline first)
            match (&a.tension.horizon, &b.tension.horizon) {
                (Some(ha), Some(hb)) => {
                    let end_order = ha.range_end().cmp(&hb.range_end());
                    if end_order != std::cmp::Ordering::Equal {
                        return end_order;
                    }
                    // Tie-break by precision (narrower first)
                    let prec_order = ha.precision_level().cmp(&hb.precision_level());
                    if prec_order != std::cmp::Ordering::Equal {
                        return prec_order;
                    }
                }
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                (None, None) => {}
            }

            // Final tiebreaker: creation time
            a.tension.created_at.cmp(&b.tension.created_at)
        });

        children
    }

    /// Get all active tensions whose horizon window has fully elapsed.
    ///
    /// Returns tensions where:
    /// - Status is Active
    /// - Horizon exists and `is_past(now)` is true
    ///
    /// Excludes:
    /// - Resolved/Released tensions
    /// - Tensions without horizons
    pub fn tensions_past_horizon(&self, now: chrono::DateTime<chrono::Utc>) -> Vec<&Node> {
        self.nodes
            .values()
            .filter(|node| {
                // Must be Active
                if node.tension.status != crate::tension::TensionStatus::Active {
                    return false;
                }
                // Must have a horizon
                let horizon = match &node.tension.horizon {
                    Some(h) => h,
                    None => return false,
                };
                // Horizon must be past
                horizon.is_past(now)
            })
            .collect()
    }

    /// Get all active tensions whose horizon ends within the given duration.
    ///
    /// Returns tensions where:
    /// - Status is Active
    /// - Horizon exists and ends within `now + within`
    /// - Horizon is NOT already past
    ///
    /// Excludes:
    /// - Resolved/Released tensions
    /// - Tensions without horizons
    /// - Tensions where horizon is already past
    ///
    /// Zero duration returns an empty vector.
    pub fn tensions_approaching_horizon(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        within: chrono::Duration,
    ) -> Vec<&Node> {
        // Zero duration is degenerate - nothing can be "approaching" in zero time
        if within.is_zero() {
            return Vec::new();
        }

        let deadline = now + within;

        self.nodes
            .values()
            .filter(|node| {
                // Must be Active
                if node.tension.status != crate::tension::TensionStatus::Active {
                    return false;
                }
                // Must have a horizon
                let horizon = match &node.tension.horizon {
                    Some(h) => h,
                    None => return false,
                };
                // Must not be past
                if horizon.is_past(now) {
                    return false;
                }
                // Horizon ends within the duration (range_end <= deadline)
                horizon.range_end() <= deadline
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Horizon;
    use crate::tension::Tension;
    use chrono::{TimeZone, Utc};

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

    fn make_tension_with_horizon(
        id: &str,
        desired: &str,
        actual: &str,
        horizon: Option<Horizon>,
    ) -> Tension {
        let mut t = Tension::new_full(desired, actual, None, horizon).unwrap();
        t.id = id.to_string();
        t
    }

    fn make_tension_with_parent_and_horizon(
        id: &str,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> Tension {
        let mut t = Tension::new_full(desired, actual, parent_id, horizon).unwrap();
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

    // ── children_by_horizon ─────────────────────────────────────────

    #[test]
    fn test_children_by_horizon_sorted_earliest_first() {
        // Parent with three children at different horizons
        let parent = make_tension_with_id("parent", "p", "p");
        // March (earliest)
        let child_march = make_tension_with_parent_and_horizon(
            "child_march",
            "march",
            "march",
            Some("parent".to_string()),
            Some(Horizon::new_month(2026, 3).unwrap()),
        );
        // May (middle)
        let child_may = make_tension_with_parent_and_horizon(
            "child_may",
            "may",
            "may",
            Some("parent".to_string()),
            Some(Horizon::new_month(2026, 5).unwrap()),
        );
        // August (latest)
        let child_aug = make_tension_with_parent_and_horizon(
            "child_aug",
            "aug",
            "aug",
            Some("parent".to_string()),
            Some(Horizon::new_month(2026, 8).unwrap()),
        );

        let forest =
            Forest::from_tensions(vec![parent, child_march, child_may, child_aug]).unwrap();

        let children = forest.children_by_horizon("parent");
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id(), "child_march");
        assert_eq!(children[1].id(), "child_may");
        assert_eq!(children[2].id(), "child_aug");
    }

    #[test]
    fn test_children_by_horizon_none_last() {
        // Parent with children including one without horizon
        let parent = make_tension_with_id("parent", "p", "p");
        let child_jan = make_tension_with_parent_and_horizon(
            "child_jan",
            "jan",
            "jan",
            Some("parent".to_string()),
            Some(Horizon::new_month(2026, 1).unwrap()),
        );
        let child_none = make_tension_with_parent_and_horizon(
            "child_none",
            "none",
            "none",
            Some("parent".to_string()),
            None,
        );
        let child_dec = make_tension_with_parent_and_horizon(
            "child_dec",
            "dec",
            "dec",
            Some("parent".to_string()),
            Some(Horizon::new_month(2026, 12).unwrap()),
        );

        let forest = Forest::from_tensions(vec![parent, child_jan, child_none, child_dec]).unwrap();

        let children = forest.children_by_horizon("parent");
        assert_eq!(children.len(), 3);
        // Jan first, Dec second, None last
        assert_eq!(children[0].id(), "child_jan");
        assert_eq!(children[1].id(), "child_dec");
        assert_eq!(children[2].id(), "child_none");
    }

    #[test]
    fn test_children_by_horizon_precision_ties() {
        // All children have same range_start (2026-01-01)
        // Should sort by precision: DateTime < Day < Month < Year
        let parent = make_tension_with_id("parent", "p", "p");
        let child_year = make_tension_with_parent_and_horizon(
            "child_year",
            "year",
            "year",
            Some("parent".to_string()),
            Some(Horizon::new_year(2026).unwrap()),
        );
        let child_month = make_tension_with_parent_and_horizon(
            "child_month",
            "month",
            "month",
            Some("parent".to_string()),
            Some(Horizon::new_month(2026, 1).unwrap()),
        );
        let child_day = make_tension_with_parent_and_horizon(
            "child_day",
            "day",
            "day",
            Some("parent".to_string()),
            Some(Horizon::new_day(2026, 1, 1).unwrap()),
        );
        let child_dt = make_tension_with_parent_and_horizon(
            "child_dt",
            "dt",
            "dt",
            Some("parent".to_string()),
            Some(Horizon::new_datetime(
                Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            )),
        );

        let forest =
            Forest::from_tensions(vec![parent, child_year, child_month, child_day, child_dt])
                .unwrap();

        let children = forest.children_by_horizon("parent");
        assert_eq!(children.len(), 4);
        // DateTime (most precise) first
        assert_eq!(children[0].id(), "child_dt");
        // Day second
        assert_eq!(children[1].id(), "child_day");
        // Month third
        assert_eq!(children[2].id(), "child_month");
        // Year (least precise) last
        assert_eq!(children[3].id(), "child_year");
    }

    #[test]
    fn test_children_by_horizon_all_none() {
        // All children without horizon - should return in stable order
        let parent = make_tension_with_id("parent", "p", "p");
        let child_a = make_tension_with_parent("child_a", "a", "a", Some("parent".to_string()));
        let child_b = make_tension_with_parent("child_b", "b", "b", Some("parent".to_string()));
        let child_c = make_tension_with_parent("child_c", "c", "c", Some("parent".to_string()));

        let forest = Forest::from_tensions(vec![parent, child_a, child_b, child_c]).unwrap();

        let children = forest.children_by_horizon("parent");
        assert_eq!(children.len(), 3);
        // All should be present
        let ids: Vec<&str> = children.iter().map(|n| n.id()).collect();
        assert!(ids.contains(&"child_a"));
        assert!(ids.contains(&"child_b"));
        assert!(ids.contains(&"child_c"));
    }

    #[test]
    fn test_children_by_horizon_nonexistent_parent() {
        let forest = Forest::from_tensions(vec![]).unwrap();
        // Returns empty vec for nonexistent parent (contract specifies Vec<&Node>)
        assert!(forest.children_by_horizon("nonexistent").is_empty());
    }

    #[test]
    fn test_children_by_horizon_leaf_node() {
        let leaf = make_tension_with_id("leaf", "l", "l");
        let forest = Forest::from_tensions(vec![leaf]).unwrap();

        let children = forest.children_by_horizon("leaf");
        assert!(children.is_empty());
    }

    // ── tensions_past_horizon ──────────────────────────────────────────

    #[test]
    fn test_tensions_past_horizon_active_only() {
        // Now is 2026-06-01
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();

        // Past horizon (May 2026) - should be included
        let past_active = make_tension_with_horizon(
            "past_active",
            "past",
            "past",
            Some(Horizon::new_month(2026, 5).unwrap()),
        );

        // Past horizon but resolved - should be excluded
        let mut past_resolved = make_tension_with_horizon(
            "past_resolved",
            "resolved",
            "resolved",
            Some(Horizon::new_month(2026, 4).unwrap()),
        );
        past_resolved.resolve().unwrap();

        // Past horizon but released - should be excluded
        let mut past_released = make_tension_with_horizon(
            "past_released",
            "released",
            "released",
            Some(Horizon::new_month(2026, 3).unwrap()),
        );
        past_released.release().unwrap();

        // Future horizon - should be excluded
        let future_active = make_tension_with_horizon(
            "future_active",
            "future",
            "future",
            Some(Horizon::new_month(2026, 12).unwrap()),
        );

        // No horizon - should be excluded
        let no_horizon = make_tension_with_horizon("no_horizon", "none", "none", None);

        let forest = Forest::from_tensions(vec![
            past_active,
            past_resolved,
            past_released,
            future_active,
            no_horizon,
        ])
        .unwrap();

        let past = forest.tensions_past_horizon(now);
        assert_eq!(past.len(), 1);
        assert_eq!(past[0].id(), "past_active");
    }

    #[test]
    fn test_tensions_past_horizon_empty_when_none() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();

        // All tensions have no horizon
        let t1 = make_tension_with_id("t1", "a", "a");
        let t2 = make_tension_with_id("t2", "b", "b");

        let forest = Forest::from_tensions(vec![t1, t2]).unwrap();

        let past = forest.tensions_past_horizon(now);
        assert!(past.is_empty());
    }

    #[test]
    fn test_tensions_past_horizon_empty_when_future() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();

        // All horizons are in the future
        let t1 =
            make_tension_with_horizon("t1", "a", "a", Some(Horizon::new_month(2026, 12).unwrap()));
        let t2 = make_tension_with_horizon("t2", "b", "b", Some(Horizon::new_year(2027).unwrap()));

        let forest = Forest::from_tensions(vec![t1, t2]).unwrap();

        let past = forest.tensions_past_horizon(now);
        assert!(past.is_empty());
    }

    #[test]
    fn test_tensions_past_horizon_datetime_past() {
        // DateTime horizons should work correctly
        let dt_past = Utc.with_ymd_and_hms(2026, 5, 15, 14, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 5, 15, 15, 0, 0).unwrap();

        let t = make_tension_with_horizon("t", "a", "a", Some(Horizon::new_datetime(dt_past)));

        let forest = Forest::from_tensions(vec![t]).unwrap();

        let past = forest.tensions_past_horizon(now);
        assert_eq!(past.len(), 1);
    }

    #[test]
    fn test_tensions_past_horizon_at_boundary() {
        // At the exact end of the horizon, is_past should be false
        let h = Horizon::new_month(2026, 5).unwrap();
        let end = h.range_end(); // 2026-05-31 23:59:59

        let t = make_tension_with_horizon("t", "a", "a", Some(h));

        let forest = Forest::from_tensions(vec![t]).unwrap();

        // At the boundary, not past yet
        let past = forest.tensions_past_horizon(end);
        assert!(past.is_empty());

        // One second after boundary, is past
        let past = forest.tensions_past_horizon(end + chrono::Duration::seconds(1));
        assert_eq!(past.len(), 1);
    }

    // ── tensions_approaching_horizon ────────────────────────────────────

    #[test]
    fn test_tensions_approaching_horizon_within_duration() {
        // Now is 2026-05-28
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();
        // Within 5 days = May 28 + 5 days = June 2
        let within = chrono::Duration::days(5);

        // Ends May 31 - within 5 days, should be included
        let approaching = make_tension_with_horizon(
            "approaching",
            "approaching",
            "approaching",
            Some(Horizon::new_month(2026, 5).unwrap()),
        );

        // Ends June 10 - NOT within 5 days, should be excluded
        let not_yet = make_tension_with_horizon(
            "not_yet",
            "not_yet",
            "not_yet",
            Some(Horizon::new_month(2026, 6).unwrap()),
        );

        // Already past (April) - should be excluded
        let already_past = make_tension_with_horizon(
            "already_past",
            "past",
            "past",
            Some(Horizon::new_month(2026, 4).unwrap()),
        );

        // No horizon - should be excluded
        let no_horizon = make_tension_with_horizon("no_horizon", "none", "none", None);

        let forest =
            Forest::from_tensions(vec![approaching, not_yet, already_past, no_horizon]).unwrap();

        let approaching_list = forest.tensions_approaching_horizon(now, within);
        assert_eq!(approaching_list.len(), 1);
        assert_eq!(approaching_list[0].id(), "approaching");
    }

    #[test]
    fn test_tensions_approaching_horizon_zero_duration() {
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();

        // Horizon ending very soon
        let t =
            make_tension_with_horizon("t", "a", "a", Some(Horizon::new_day(2026, 5, 28).unwrap()));

        let forest = Forest::from_tensions(vec![t]).unwrap();

        // Zero duration should return empty
        let approaching = forest.tensions_approaching_horizon(now, chrono::Duration::zero());
        assert!(approaching.is_empty());
    }

    #[test]
    fn test_tensions_approaching_horizon_resolved_excluded() {
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();
        let within = chrono::Duration::days(5);

        // Resolved tension approaching horizon
        let mut resolved = make_tension_with_horizon(
            "resolved",
            "resolved",
            "resolved",
            Some(Horizon::new_month(2026, 5).unwrap()),
        );
        resolved.resolve().unwrap();

        // Active tension approaching horizon
        let active = make_tension_with_horizon(
            "active",
            "active",
            "active",
            Some(Horizon::new_month(2026, 5).unwrap()),
        );

        let forest = Forest::from_tensions(vec![resolved, active]).unwrap();

        let approaching = forest.tensions_approaching_horizon(now, within);
        assert_eq!(approaching.len(), 1);
        assert_eq!(approaching[0].id(), "active");
    }

    #[test]
    fn test_tensions_approaching_horizon_released_excluded() {
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();
        let within = chrono::Duration::days(5);

        // Released tension approaching horizon
        let mut released = make_tension_with_horizon(
            "released",
            "released",
            "released",
            Some(Horizon::new_month(2026, 5).unwrap()),
        );
        released.release().unwrap();

        // Active tension approaching horizon
        let active = make_tension_with_horizon(
            "active",
            "active",
            "active",
            Some(Horizon::new_month(2026, 5).unwrap()),
        );

        let forest = Forest::from_tensions(vec![released, active]).unwrap();

        let approaching = forest.tensions_approaching_horizon(now, within);
        assert_eq!(approaching.len(), 1);
        assert_eq!(approaching[0].id(), "active");
    }

    #[test]
    fn test_tensions_approaching_horizon_no_horizon_empty() {
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();
        let within = chrono::Duration::days(5);

        // All tensions without horizon
        let t1 = make_tension_with_id("t1", "a", "a");
        let t2 = make_tension_with_id("t2", "b", "b");

        let forest = Forest::from_tensions(vec![t1, t2]).unwrap();

        let approaching = forest.tensions_approaching_horizon(now, within);
        assert!(approaching.is_empty());
    }

    #[test]
    fn test_tensions_approaching_horizon_empty_when_all_past() {
        let now = Utc.with_ymd_and_hms(2026, 6, 15, 12, 0, 0).unwrap();
        let within = chrono::Duration::days(5);

        // All horizons are in the past
        let t1 =
            make_tension_with_horizon("t1", "a", "a", Some(Horizon::new_month(2026, 5).unwrap()));
        let t2 =
            make_tension_with_horizon("t2", "b", "b", Some(Horizon::new_month(2026, 4).unwrap()));

        let forest = Forest::from_tensions(vec![t1, t2]).unwrap();

        let approaching = forest.tensions_approaching_horizon(now, within);
        assert!(approaching.is_empty());
    }

    #[test]
    fn test_tensions_approaching_horizon_empty_when_all_future() {
        let now = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
        let within = chrono::Duration::days(5);

        // All horizons are far in the future
        let t1 =
            make_tension_with_horizon("t1", "a", "a", Some(Horizon::new_month(2026, 12).unwrap()));
        let t2 = make_tension_with_horizon("t2", "b", "b", Some(Horizon::new_year(2027).unwrap()));

        let forest = Forest::from_tensions(vec![t1, t2]).unwrap();

        let approaching = forest.tensions_approaching_horizon(now, within);
        assert!(approaching.is_empty());
    }

    #[test]
    fn test_tensions_approaching_horizon_day_precision() {
        // Day precision should work correctly
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();
        let within = chrono::Duration::hours(36); // 1.5 days

        // Day horizon ending within 36 hours
        let approaching = make_tension_with_horizon(
            "approaching",
            "a",
            "a",
            Some(Horizon::new_day(2026, 5, 29).unwrap()),
        );

        // Day horizon ending after 36 hours
        let not_yet = make_tension_with_horizon(
            "not_yet",
            "b",
            "b",
            Some(Horizon::new_day(2026, 5, 30).unwrap()),
        );

        let forest = Forest::from_tensions(vec![approaching, not_yet]).unwrap();

        let approaching_list = forest.tensions_approaching_horizon(now, within);
        assert_eq!(approaching_list.len(), 1);
        assert_eq!(approaching_list[0].id(), "approaching");
    }

    #[test]
    fn test_tensions_approaching_horizon_datetime_precision() {
        // DateTime precision should work correctly
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();
        let within = chrono::Duration::hours(2);

        // DateTime horizon ending within 2 hours
        let approaching = make_tension_with_horizon(
            "approaching",
            "a",
            "a",
            Some(Horizon::new_datetime(
                Utc.with_ymd_and_hms(2026, 5, 28, 13, 30, 0).unwrap(),
            )),
        );

        // DateTime horizon ending after 2 hours
        let not_yet = make_tension_with_horizon(
            "not_yet",
            "b",
            "b",
            Some(Horizon::new_datetime(
                Utc.with_ymd_and_hms(2026, 5, 28, 15, 0, 0).unwrap(),
            )),
        );

        let forest = Forest::from_tensions(vec![approaching, not_yet]).unwrap();

        let approaching_list = forest.tensions_approaching_horizon(now, within);
        assert_eq!(approaching_list.len(), 1);
        assert_eq!(approaching_list[0].id(), "approaching");
    }

    #[test]
    fn test_tensions_approaching_horizon_at_boundary() {
        // Horizon that ends exactly at (now + within)
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 12, 0, 0).unwrap();
        let within = chrono::Duration::hours(24);

        // Horizon ends exactly at now + 24h
        let boundary = make_tension_with_horizon(
            "boundary",
            "a",
            "a",
            Some(Horizon::new_datetime(
                Utc.with_ymd_and_hms(2026, 5, 29, 12, 0, 0).unwrap(),
            )),
        );

        let forest = Forest::from_tensions(vec![boundary]).unwrap();

        let approaching = forest.tensions_approaching_horizon(now, within);
        // Should be included (ends within the duration, inclusive)
        assert_eq!(approaching.len(), 1);
    }
}
