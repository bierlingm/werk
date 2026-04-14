//! Typed edges — the universal relationship substrate.
//!
//! All structural relationships between tensions are edges in a directed graph.
//! Edge types distinguish the nature of the relationship:
//!
//! - `contains` — parent→child containment (replaces the old parent_id column)
//! - `split_from` — provenance: this tension was split from another
//! - `merged_into` — provenance: this tension was merged into another
//!
//! Future user-configurable edge types will extend this vocabulary.
//! The edges table is the source of truth for all relationships.
//! The FrankenNetworkX DiGraph is built from edges and serves as the
//! structural substrate for graph algorithms and queries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Known edge types used by the instrument's core gestures.
/// User-configurable types will be arbitrary strings validated at the application layer.
pub const EDGE_CONTAINS: &str = "contains";
pub const EDGE_SPLIT_FROM: &str = "split_from";
pub const EDGE_MERGED_INTO: &str = "merged_into";

/// A typed directed edge between two tensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Unique identifier (ULID).
    pub id: String,
    /// Source tension ID (edge goes from → to).
    pub from_id: String,
    /// Target tension ID.
    pub to_id: String,
    /// Edge type (e.g., "contains", "split_from", "merged_into").
    pub edge_type: String,
    /// When this edge was created.
    pub created_at: DateTime<Utc>,
    /// The gesture that created this edge, if any.
    pub gesture_id: Option<String>,
}

impl Edge {
    /// Create a new edge.
    pub fn new(from_id: String, to_id: String, edge_type: String) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            from_id,
            to_id,
            edge_type,
            created_at: Utc::now(),
            gesture_id: None,
        }
    }

    /// Create a new edge with a gesture ID.
    pub fn with_gesture(mut self, gesture_id: String) -> Self {
        self.gesture_id = Some(gesture_id);
        self
    }

    /// Is this a containment (parent→child) edge?
    pub fn is_contains(&self) -> bool {
        self.edge_type == EDGE_CONTAINS
    }

    /// Is this a provenance edge (split_from or merged_into)?
    pub fn is_provenance(&self) -> bool {
        self.edge_type == EDGE_SPLIT_FROM || self.edge_type == EDGE_MERGED_INTO
    }
}
