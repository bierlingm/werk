//! Shared data transfer objects (DTOs) used across the werk surfaces.
//!
//! These types are the wire format for JSON responses from the CLI
//! `--json` output, the Web REST API, and the Tauri desktop IPC layer.
//! Centralizing them keeps every surface in sync: add a field once here
//! and every consumer updates together.
//!
//! # Design
//!
//! - Fields are `pub` so surface code can construct them field-by-field
//!   when needed (e.g. tree.rs in the CLI augments with signal flags).
//! - Each DTO is built from a `werk_core::Tension` (plus context) via a
//!   constructor (`TensionDto::from_tension`). Constructors stringify
//!   enums and convert optional types to their wire representation.
//! - DTOs are `Serialize` only (not `Deserialize`) unless a surface
//!   needs to parse them back (request bodies); request DTOs carry both.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use werk_core::{Tension, TensionStatus};

/// A JSON-serialisable tension for the REST API / Tauri IPC.
///
/// Same shape whether served by axum, Tauri, or the CLI's tree command.
#[derive(Debug, Clone, Serialize)]
pub struct TensionDto {
    pub id: String,
    pub short_code: Option<i32>,
    pub desired: String,
    pub actual: String,
    pub status: String,
    pub parent_id: Option<String>,
    pub horizon: Option<String>,
    pub position: Option<i32>,
    pub created_at: String,
    pub overdue: bool,
}

impl TensionDto {
    /// Build a DTO from a core `Tension`.
    ///
    /// `overdue` is computed as `horizon.is_past(now) && status == Active`.
    pub fn from_tension(t: &Tension) -> Self {
        let overdue = match (&t.horizon, &t.status) {
            (Some(h), TensionStatus::Active) => h.is_past(Utc::now()),
            _ => false,
        };
        Self {
            id: t.id.clone(),
            short_code: t.short_code,
            desired: t.desired.clone(),
            actual: t.actual.clone(),
            status: t.status.to_string(),
            parent_id: t.parent_id.clone(),
            horizon: t.horizon.as_ref().map(|h| h.to_string()),
            position: t.position,
            created_at: t.created_at.to_rfc3339(),
            overdue,
        }
    }
}

/// Aggregate tension counts by status.
#[derive(Debug, Clone, Serialize)]
pub struct SummaryDto {
    pub active: usize,
    pub resolved: usize,
    pub released: usize,
    pub total: usize,
}

impl SummaryDto {
    /// Compute a summary from a slice of tensions.
    pub fn from_tensions(tensions: &[Tension]) -> Self {
        let mut active = 0;
        let mut resolved = 0;
        let mut released = 0;
        for t in tensions {
            match t.status {
                TensionStatus::Active => active += 1,
                TensionStatus::Resolved => resolved += 1,
                TensionStatus::Released => released += 1,
            }
        }
        Self {
            active,
            resolved,
            released,
            total: tensions.len(),
        }
    }
}

/// Request body for creating a tension.
///
/// Shared between the Web POST /api/tensions body and the Tauri
/// `create_tension` command args.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTensionRequest {
    pub desired: String,
    pub actual: Option<String>,
    pub parent_id: Option<String>,
    pub horizon: Option<String>,
}

/// Request body for updating a single field (desired or actual).
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFieldRequest {
    pub value: String,
}

/// Error envelope for REST API error responses.
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub error: String,
}

impl ApiError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}
