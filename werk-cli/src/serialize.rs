//! JSON serialization types for CLI output.
//!
//! Provides serializable types for tension info, mutation info,
//! and horizon data used by show, context, and run commands.

use chrono::{DateTime, Utc};
use serde::Serialize;

use sd_core::{compute_urgency, Mutation, Tension};

// ============================================================================
// JSON Serialization Types
// ============================================================================

#[derive(Serialize, Clone, Debug)]
pub struct HorizonRangeJson {
    pub start: String,
    pub end: String,
}

/// Tension information with horizon data (used by context and run commands).
#[derive(Serialize, Clone, Debug)]
pub struct TensionInfo {
    pub id: String,
    pub short_code: Option<i32>,
    pub desired: String,
    pub actual: String,
    pub status: String,
    pub created_at: String,
    pub parent_id: Option<String>,
    pub horizon: Option<String>,
    pub horizon_range: Option<HorizonRangeJson>,
    pub urgency: Option<f64>,
    pub staleness_ratio: Option<f64>,
}

/// Mutation information for display.
#[derive(Serialize, Clone, Debug)]
pub struct MutationInfo {
    pub tension_id: String,
    pub timestamp: String,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: String,
}

// ============================================================================
// Computation Helpers
// ============================================================================

/// Compute tension info with horizon data for a forest node.
pub fn node_to_tension_info(node: &sd_core::Node, now: DateTime<Utc>) -> TensionInfo {
    let horizon = node.tension.horizon.as_ref().map(|h| h.to_string());
    let horizon_range = node.tension.horizon.as_ref().map(|h| HorizonRangeJson {
        start: h.range_start().to_rfc3339(),
        end: h.range_end().to_rfc3339(),
    });
    let urgency = compute_urgency(&node.tension, now).map(|u| u.value);

    TensionInfo {
        id: node.id().to_string(),
        short_code: node.tension.short_code,
        desired: node.tension.desired.clone(),
        actual: node.tension.actual.clone(),
        status: node.tension.status.to_string(),
        created_at: node.tension.created_at.to_rfc3339(),
        parent_id: node.tension.parent_id.clone(),
        horizon,
        horizon_range,
        urgency,
        staleness_ratio: None,
    }
}

/// Build a TensionInfo for the primary tension (includes staleness_ratio).
pub fn tension_to_info(
    tension: &Tension,
    mutations: &[Mutation],
    now: DateTime<Utc>,
) -> TensionInfo {
    let horizon = tension.horizon.as_ref().map(|h| h.to_string());
    let horizon_range = tension.horizon.as_ref().map(|h| HorizonRangeJson {
        start: h.range_start().to_rfc3339(),
        end: h.range_end().to_rfc3339(),
    });
    let urgency = compute_urgency(tension, now).map(|u| u.value);

    let last_mutation_time = mutations.last().map(|m| m.timestamp());
    let staleness_ratio = match (&tension.horizon, last_mutation_time) {
        (Some(h), Some(last_time)) => Some(h.staleness(last_time, now)),
        _ => None,
    };

    TensionInfo {
        id: tension.id.clone(),
        short_code: tension.short_code,
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        created_at: tension.created_at.to_rfc3339(),
        parent_id: tension.parent_id.clone(),
        horizon,
        horizon_range,
        urgency,
        staleness_ratio,
    }
}

/// Build a MutationInfo from a Mutation.
pub fn mutation_to_info(m: &Mutation) -> MutationInfo {
    MutationInfo {
        tension_id: m.tension_id().to_owned(),
        timestamp: m.timestamp().to_rfc3339(),
        field: m.field().to_owned(),
        old_value: m.old_value().map(|s| s.to_owned()),
        new_value: m.new_value().to_owned(),
    }
}
