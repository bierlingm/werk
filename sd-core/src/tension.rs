//! Structural tension — the primitive of structural dynamics.
//!
//! A tension represents the gap between a desired state and current reality.
//! It is the generative force in Fritz's structural dynamics model.

use crate::Horizon;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Errors that can occur in sd-core operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SdError {
    /// A required field was empty or invalid.
    #[error("validation error: {0}")]
    ValidationError(String),

    /// An invalid status transition was attempted.
    #[error("invalid status transition: cannot transition from {from} to {to}")]
    InvalidStatusTransition {
        /// The current status.
        from: TensionStatus,
        /// The attempted target status.
        to: TensionStatus,
    },

    /// A field update was rejected because the tension is not active.
    #[error("cannot update field on {0} tension")]
    UpdateOnInactiveTension(TensionStatus),
}

/// The lifecycle status of a tension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensionStatus {
    /// The tension is active — the gap between desired and actual exists.
    Active,
    /// The tension has been resolved — desired state achieved.
    Resolved,
    /// The tension has been released — no longer pursuing the desired state.
    Released,
}

impl fmt::Display for TensionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TensionStatus::Active => write!(f, "Active"),
            TensionStatus::Resolved => write!(f, "Resolved"),
            TensionStatus::Released => write!(f, "Released"),
        }
    }
}

/// A structural tension — the gap between desired state and current reality.
///
/// Tensions are the primitive of structural dynamics. They track what you
/// want (desired), what is (actual), and their lifecycle status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tension {
    /// Unique identifier (ULID).
    pub id: String,
    /// The desired state — what you want to create.
    pub desired: String,
    /// The actual state — current reality.
    pub actual: String,
    /// Optional parent tension ID for hierarchical structure.
    pub parent_id: Option<String>,
    /// When this tension was created.
    pub created_at: DateTime<Utc>,
    /// Current lifecycle status.
    pub status: TensionStatus,
    /// Optional temporal horizon — when this tension is aimed at.
    pub horizon: Option<Horizon>,
    /// Explicit ordering position among siblings. None means unpositioned.
    pub position: Option<i32>,
    /// Snapshot of parent's desired state at child creation time.
    pub parent_desired_snapshot: Option<String>,
    /// Snapshot of parent's actual state at child creation time.
    pub parent_actual_snapshot: Option<String>,
    /// JSON snapshot of parent's full descended view at child creation time.
    /// Includes children with their order, held steps, resolved steps.
    pub parent_snapshot_json: Option<String>,
    /// Workspace-local short code for human addressing. Auto-assigned, sequential.
    pub short_code: Option<i32>,
}

impl Tension {
    /// Create a new tension with the given desired and actual states.
    ///
    /// Returns an error if either `desired` or `actual` is empty.
    /// The horizon defaults to None.
    pub fn new(desired: &str, actual: &str) -> Result<Self, SdError> {
        Self::new_inner(desired, actual, None, None)
    }

    /// Create a new tension with a parent reference.
    ///
    /// Returns an error if either `desired` or `actual` is empty.
    /// The horizon defaults to None.
    pub fn new_with_parent(
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
    ) -> Result<Self, SdError> {
        Self::new_inner(desired, actual, parent_id, None)
    }

    /// Create a new tension with all optional fields.
    ///
    /// Returns an error if either `desired` or `actual` is empty.
    /// Past horizons are allowed at creation — the grammar doesn't judge.
    pub fn new_full(
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> Result<Self, SdError> {
        Self::new_inner(desired, actual, parent_id, horizon)
    }

    /// Create a new tension with all fields including parent snapshots and position.
    ///
    /// Used when creating child tensions that need to capture parent state.
    pub fn new_full_with_snapshots(
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
        position: Option<i32>,
        parent_desired_snapshot: Option<String>,
        parent_actual_snapshot: Option<String>,
        parent_snapshot_json: Option<String>,
    ) -> Result<Self, SdError> {
        let mut tension = Self::new_inner(desired, actual, parent_id, horizon)?;
        tension.position = position;
        tension.parent_desired_snapshot = parent_desired_snapshot;
        tension.parent_actual_snapshot = parent_actual_snapshot;
        tension.parent_snapshot_json = parent_snapshot_json;
        Ok(tension)
    }

    fn new_inner(
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> Result<Self, SdError> {
        if desired.is_empty() {
            return Err(SdError::ValidationError(
                "desired state cannot be empty".to_owned(),
            ));
        }
        if actual.is_empty() {
            return Err(SdError::ValidationError(
                "actual state cannot be empty".to_owned(),
            ));
        }

        Ok(Self {
            id: ulid::Ulid::new().to_string(),
            desired: desired.to_owned(),
            actual: actual.to_owned(),
            parent_id,
            created_at: Utc::now(),
            status: TensionStatus::Active,
            horizon,
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        })
    }

    /// Update the desired state.
    ///
    /// Returns an error if the new value is empty or if the tension is not active.
    pub fn update_desired(&mut self, new_desired: &str) -> Result<String, SdError> {
        if self.status != TensionStatus::Active {
            return Err(SdError::UpdateOnInactiveTension(self.status));
        }
        if new_desired.is_empty() {
            return Err(SdError::ValidationError(
                "desired state cannot be empty".to_owned(),
            ));
        }
        let old = std::mem::replace(&mut self.desired, new_desired.to_owned());
        Ok(old)
    }

    /// Update the actual state.
    ///
    /// Returns an error if the new value is empty or if the tension is not active.
    pub fn update_actual(&mut self, new_actual: &str) -> Result<String, SdError> {
        if self.status != TensionStatus::Active {
            return Err(SdError::UpdateOnInactiveTension(self.status));
        }
        if new_actual.is_empty() {
            return Err(SdError::ValidationError(
                "actual state cannot be empty".to_owned(),
            ));
        }
        let old = std::mem::replace(&mut self.actual, new_actual.to_owned());
        Ok(old)
    }

    /// Transition this tension to Resolved status.
    ///
    /// Only valid from Active status.
    pub fn resolve(&mut self) -> Result<(), SdError> {
        if self.status != TensionStatus::Active {
            return Err(SdError::InvalidStatusTransition {
                from: self.status,
                to: TensionStatus::Resolved,
            });
        }
        self.status = TensionStatus::Resolved;
        Ok(())
    }

    /// Transition this tension to Released status.
    ///
    /// Only valid from Active status.
    pub fn release(&mut self) -> Result<(), SdError> {
        if self.status != TensionStatus::Active {
            return Err(SdError::InvalidStatusTransition {
                from: self.status,
                to: TensionStatus::Released,
            });
        }
        self.status = TensionStatus::Released;
        Ok(())
    }

    /// Transition this tension back to Active status (reopen).
    ///
    /// Only valid from Resolved or Released status.
    pub fn reopen(&mut self) -> Result<(), SdError> {
        if self.status == TensionStatus::Active {
            return Err(SdError::InvalidStatusTransition {
                from: self.status,
                to: TensionStatus::Active,
            });
        }
        self.status = TensionStatus::Active;
        Ok(())
    }

    /// Update the temporal horizon.
    ///
    /// Returns an error if the tension is not active.
    /// Returns the previous horizon value on success.
    pub fn update_horizon(
        &mut self,
        new_horizon: Option<Horizon>,
    ) -> Result<Option<Horizon>, SdError> {
        if self.status != TensionStatus::Active {
            return Err(SdError::UpdateOnInactiveTension(self.status));
        }
        let old = std::mem::replace(&mut self.horizon, new_horizon);
        Ok(old)
    }

    /// Update the position for sibling ordering.
    ///
    /// Unlike other updates, position changes are allowed regardless of status.
    pub fn update_position(&mut self, new_position: Option<i32>) -> Option<i32> {
        std::mem::replace(&mut self.position, new_position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ──────────────────────────────────────────────

    #[test]
    fn test_tension_new_valid() {
        let t = Tension::new("write a novel", "have an outline").unwrap();
        assert!(!t.id.is_empty());
        assert_eq!(t.desired, "write a novel");
        assert_eq!(t.actual, "have an outline");
        assert_eq!(t.status, TensionStatus::Active);
        assert!(t.parent_id.is_none());
        // created_at should be recent (within last 5 seconds)
        let elapsed = Utc::now() - t.created_at;
        assert!(elapsed.num_seconds() < 5);
    }

    #[test]
    fn test_tension_new_empty_desired_fails() {
        let result = Tension::new("", "some reality");
        assert!(result.is_err());
        match result.unwrap_err() {
            SdError::ValidationError(msg) => assert!(msg.contains("desired")),
            other => panic!("expected ValidationError, got {other:?}"), // ubs:ignore test assertion
        }
    }

    #[test]
    fn test_tension_new_empty_actual_fails() {
        let result = Tension::new("some goal", "");
        assert!(result.is_err());
        match result.unwrap_err() {
            SdError::ValidationError(msg) => assert!(msg.contains("actual")),
            other => panic!("expected ValidationError, got {other:?}"), // ubs:ignore test assertion
        }
    }

    #[test]
    fn test_tension_new_both_empty_fails() {
        let result = Tension::new("", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_tension_ulid_uniqueness() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let t = Tension::new("desired", "actual").unwrap();
            assert!(ids.insert(t.id), "duplicate ULID detected");
        }
        assert_eq!(ids.len(), 1000);
    }

    // ── Parent assignment ─────────────────────────────────────────

    #[test]
    fn test_tension_new_with_parent() {
        let parent = Tension::new("parent goal", "parent reality").unwrap();
        let child =
            Tension::new_with_parent("child goal", "child reality", Some(parent.id.clone()))
                .unwrap();
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_tension_new_with_parent_none() {
        let t = Tension::new_with_parent("goal", "reality", None).unwrap();
        assert!(t.parent_id.is_none());
    }

    #[test]
    fn test_tension_new_with_parent_validates_desired() {
        let result = Tension::new_with_parent("", "reality", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_tension_new_with_parent_validates_actual() {
        let result = Tension::new_with_parent("goal", "", None);
        assert!(result.is_err());
    }

    // ── Field updates ─────────────────────────────────────────────

    #[test]
    fn test_update_desired_valid() {
        let mut t = Tension::new("old desire", "reality").unwrap();
        let old = t.update_desired("new desire").unwrap();
        assert_eq!(old, "old desire");
        assert_eq!(t.desired, "new desire");
    }

    #[test]
    fn test_update_desired_empty_rejected() {
        let mut t = Tension::new("desire", "reality").unwrap();
        let result = t.update_desired("");
        assert!(result.is_err());
        // Original preserved
        assert_eq!(t.desired, "desire");
    }

    #[test]
    fn test_update_actual_valid() {
        let mut t = Tension::new("desire", "old reality").unwrap();
        let old = t.update_actual("new reality").unwrap();
        assert_eq!(old, "old reality");
        assert_eq!(t.actual, "new reality");
    }

    #[test]
    fn test_update_actual_empty_rejected() {
        let mut t = Tension::new("desire", "reality").unwrap();
        let result = t.update_actual("");
        assert!(result.is_err());
        // Original preserved
        assert_eq!(t.actual, "reality");
    }

    // ── Status transitions ────────────────────────────────────────

    #[test]
    fn test_resolve_from_active() {
        let mut t = Tension::new("goal", "reality").unwrap();
        assert!(t.resolve().is_ok());
        assert_eq!(t.status, TensionStatus::Resolved);
    }

    #[test]
    fn test_release_from_active() {
        let mut t = Tension::new("goal", "reality").unwrap();
        assert!(t.release().is_ok());
        assert_eq!(t.status, TensionStatus::Released);
    }

    #[test]
    fn test_resolve_from_resolved_fails() {
        let mut t = Tension::new("goal", "reality").unwrap();
        t.resolve().unwrap();
        let result = t.resolve();
        assert!(result.is_err());
        assert_eq!(t.status, TensionStatus::Resolved);
    }

    #[test]
    fn test_resolve_from_released_fails() {
        let mut t = Tension::new("goal", "reality").unwrap();
        t.release().unwrap();
        let result = t.resolve();
        assert!(result.is_err());
        assert_eq!(t.status, TensionStatus::Released);
    }

    #[test]
    fn test_release_from_resolved_fails() {
        let mut t = Tension::new("goal", "reality").unwrap();
        t.resolve().unwrap();
        let result = t.release();
        assert!(result.is_err());
        assert_eq!(t.status, TensionStatus::Resolved);
    }

    #[test]
    fn test_release_from_released_fails() {
        let mut t = Tension::new("goal", "reality").unwrap();
        t.release().unwrap();
        let result = t.release();
        assert!(result.is_err());
        assert_eq!(t.status, TensionStatus::Released);
    }

    // ── Resolved/Released reject field updates ────────────────────

    #[test]
    fn test_update_desired_on_resolved_fails() {
        let mut t = Tension::new("desire", "reality").unwrap();
        t.resolve().unwrap();
        let result = t.update_desired("new desire");
        assert!(result.is_err());
        assert_eq!(t.desired, "desire"); // preserved
    }

    #[test]
    fn test_update_desired_on_released_fails() {
        let mut t = Tension::new("desire", "reality").unwrap();
        t.release().unwrap();
        let result = t.update_desired("new desire");
        assert!(result.is_err());
        assert_eq!(t.desired, "desire"); // preserved
    }

    #[test]
    fn test_update_actual_on_resolved_fails() {
        let mut t = Tension::new("desire", "reality").unwrap();
        t.resolve().unwrap();
        let result = t.update_actual("new reality");
        assert!(result.is_err());
        assert_eq!(t.actual, "reality"); // preserved
    }

    #[test]
    fn test_update_actual_on_released_fails() {
        let mut t = Tension::new("desire", "reality").unwrap();
        t.release().unwrap();
        let result = t.update_actual("new reality");
        assert!(result.is_err());
        assert_eq!(t.actual, "reality"); // preserved
    }

    // ── Serialization ─────────────────────────────────────────────

    #[test]
    fn test_tension_serialization_roundtrip() {
        let t = Tension::new_with_parent(
            "write a symphony",
            "have a melody",
            Some("parent123".to_owned()),
        )
        .unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let deserialized: Tension = serde_json::from_str(&json).unwrap();
        assert_eq!(t, deserialized);

        // Verify all fields are in JSON
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("id").is_some());
        assert!(value.get("desired").is_some());
        assert!(value.get("actual").is_some());
        assert!(value.get("parent_id").is_some());
        assert!(value.get("created_at").is_some());
        assert!(value.get("status").is_some());
    }

    #[test]
    fn test_tension_status_serialization_roundtrip() {
        for status in [
            TensionStatus::Active,
            TensionStatus::Resolved,
            TensionStatus::Released,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: TensionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_tension_status_serializes_to_name() {
        assert_eq!(
            serde_json::to_string(&TensionStatus::Active).unwrap(),
            "\"Active\""
        );
        assert_eq!(
            serde_json::to_string(&TensionStatus::Resolved).unwrap(),
            "\"Resolved\""
        );
        assert_eq!(
            serde_json::to_string(&TensionStatus::Released).unwrap(),
            "\"Released\""
        );
    }

    // ── Unicode and special characters ────────────────────────────

    #[test]
    fn test_tension_unicode_cjk() {
        let t = Tension::new("写一本小说", "有一个大纲").unwrap();
        assert_eq!(t.desired, "写一本小说");
        assert_eq!(t.actual, "有一个大纲");

        // Roundtrip through JSON
        let json = serde_json::to_string(&t).unwrap();
        let deserialized: Tension = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.desired, "写一本小说");
        assert_eq!(deserialized.actual, "有一个大纲");
    }

    #[test]
    fn test_tension_unicode_emoji() {
        let t = Tension::new("🎵 compose music", "🎸 learning guitar").unwrap();
        assert_eq!(t.desired, "🎵 compose music");
        assert_eq!(t.actual, "🎸 learning guitar");

        let json = serde_json::to_string(&t).unwrap();
        let deserialized: Tension = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.desired, "🎵 compose music");
        assert_eq!(deserialized.actual, "🎸 learning guitar");
    }

    #[test]
    fn test_tension_unicode_rtl_and_special() {
        let t = Tension::new("كتابة رواية", "لدي مخطط").unwrap();
        assert_eq!(t.desired, "كتابة رواية");
        assert_eq!(t.actual, "لدي مخطط");
    }

    #[test]
    fn test_tension_newlines_tabs_quotes() {
        let t = Tension::new("line1\nline2\ttab", "has \"quotes\" and 'apostrophes'").unwrap();
        assert_eq!(t.desired, "line1\nline2\ttab");
        assert_eq!(t.actual, "has \"quotes\" and 'apostrophes'");

        let json = serde_json::to_string(&t).unwrap();
        let deserialized: Tension = serde_json::from_str(&json).unwrap();
        assert_eq!(t, deserialized);
    }

    // ── Display ───────────────────────────────────────────────────

    #[test]
    fn test_tension_status_display() {
        assert_eq!(TensionStatus::Active.to_string(), "Active");
        assert_eq!(TensionStatus::Resolved.to_string(), "Resolved");
        assert_eq!(TensionStatus::Released.to_string(), "Released");
    }

    // ── Trait assertions ──────────────────────────────────────────

    #[test]
    fn test_tension_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Tension>();
        assert_send_sync::<TensionStatus>();
        assert_send_sync::<SdError>();
    }

    #[test]
    fn test_tension_is_debug_clone_partialeq() {
        let t = Tension::new("a", "b").unwrap();
        let _ = format!("{t:?}"); // Debug
        let t2 = t.clone(); // Clone
        assert_eq!(t, t2); // PartialEq
    }

    #[test]
    fn test_tension_status_enum_has_exactly_three_variants() {
        // Exhaustive match ensures exactly three variants exist at compile time
        let statuses = [
            TensionStatus::Active,
            TensionStatus::Resolved,
            TensionStatus::Released,
        ];
        assert_eq!(statuses.len(), 3);

        for status in statuses {
            match status {
                TensionStatus::Active => {}
                TensionStatus::Resolved => {}
                TensionStatus::Released => {}
            }
        }
    }

    // ── Error types ───────────────────────────────────────────────

    #[test]
    fn test_sd_error_display() {
        let e = SdError::ValidationError("test".to_owned());
        assert!(e.to_string().contains("validation error"));

        let e = SdError::InvalidStatusTransition {
            from: TensionStatus::Resolved,
            to: TensionStatus::Active,
        };
        assert!(e.to_string().contains("Resolved"));
        assert!(e.to_string().contains("Active"));

        let e = SdError::UpdateOnInactiveTension(TensionStatus::Resolved);
        assert!(e.to_string().contains("Resolved"));
    }

    // ── Horizon field ──────────────────────────────────────────────

    #[test]
    fn test_tension_horizon_defaults_to_none() {
        let t = Tension::new("goal", "reality").unwrap();
        assert!(t.horizon.is_none());
    }

    #[test]
    fn test_tension_new_with_parent_horizon_defaults_to_none() {
        let t = Tension::new_with_parent("goal", "reality", Some("parent123".to_owned())).unwrap();
        assert!(t.horizon.is_none());
    }

    #[test]
    fn test_tension_new_full_with_year_horizon() {
        use crate::Horizon;
        let h = Horizon::new_year(2026).unwrap();
        let t = Tension::new_full("goal", "reality", None, Some(h.clone())).unwrap();
        assert_eq!(t.horizon, Some(h));
    }

    #[test]
    fn test_tension_new_full_with_month_horizon() {
        use crate::Horizon;
        let h = Horizon::new_month(2026, 5).unwrap();
        let t = Tension::new_full("goal", "reality", None, Some(h.clone())).unwrap();
        assert_eq!(t.horizon, Some(h));
    }

    #[test]
    fn test_tension_new_full_with_day_horizon() {
        use crate::Horizon;
        let h = Horizon::new_day(2026, 5, 15).unwrap();
        let t = Tension::new_full("goal", "reality", None, Some(h.clone())).unwrap();
        assert_eq!(t.horizon, Some(h));
    }

    #[test]
    fn test_tension_new_full_with_datetime_horizon() {
        use crate::Horizon;
        use chrono::{TimeZone, Utc};
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 0).unwrap();
        let h = Horizon::new_datetime(dt);
        let t = Tension::new_full("goal", "reality", None, Some(h.clone())).unwrap();
        assert_eq!(t.horizon, Some(h));
    }

    #[test]
    fn test_tension_new_full_with_none_horizon() {
        let t = Tension::new_full("goal", "reality", None, None).unwrap();
        assert!(t.horizon.is_none());
    }

    #[test]
    fn test_tension_new_full_with_parent_and_horizon() {
        use crate::Horizon;
        let h = Horizon::new_month(2026, 5).unwrap();
        let t = Tension::new_full(
            "goal",
            "reality",
            Some("parent123".to_owned()),
            Some(h.clone()),
        )
        .unwrap();
        assert_eq!(t.parent_id, Some("parent123".to_owned()));
        assert_eq!(t.horizon, Some(h));
    }

    #[test]
    fn test_tension_new_full_validates_desired() {
        use crate::Horizon;
        let result = Tension::new_full("", "reality", None, Some(Horizon::new_year(2026).unwrap()));
        assert!(result.is_err());
    }

    #[test]
    fn test_tension_new_full_validates_actual() {
        use crate::Horizon;
        let result = Tension::new_full("goal", "", None, Some(Horizon::new_year(2026).unwrap()));
        assert!(result.is_err());
    }

    #[test]
    fn test_tension_new_full_with_past_horizon_succeeds() {
        // Past horizons are allowed at creation — grammar doesn't judge
        use crate::Horizon;
        let past = Horizon::new_day(2020, 1, 1).unwrap();
        let t = Tension::new_full("goal", "reality", None, Some(past.clone())).unwrap();
        assert_eq!(t.horizon, Some(past));
    }

    // ── update_horizon ──────────────────────────────────────────────

    #[test]
    fn test_update_horizon_on_active_succeeds() {
        use crate::Horizon;
        let mut t = Tension::new_full(
            "goal",
            "reality",
            None,
            Some(Horizon::new_year(2026).unwrap()),
        )
        .unwrap();
        let old = t
            .update_horizon(Some(Horizon::new_month(2026, 5).unwrap()))
            .unwrap();
        assert_eq!(old, Some(Horizon::new_year(2026).unwrap()));
        assert_eq!(t.horizon, Some(Horizon::new_month(2026, 5).unwrap()));
    }

    #[test]
    fn test_update_horizon_on_active_from_none_to_some() {
        use crate::Horizon;
        let mut t = Tension::new("goal", "reality").unwrap();
        let old = t
            .update_horizon(Some(Horizon::new_year(2026).unwrap()))
            .unwrap();
        assert!(old.is_none());
        assert_eq!(t.horizon, Some(Horizon::new_year(2026).unwrap()));
    }

    #[test]
    fn test_update_horizon_clear_to_none() {
        use crate::Horizon;
        let mut t = Tension::new_full(
            "goal",
            "reality",
            None,
            Some(Horizon::new_year(2026).unwrap()),
        )
        .unwrap();
        let old = t.update_horizon(None).unwrap();
        assert_eq!(old, Some(Horizon::new_year(2026).unwrap()));
        assert!(t.horizon.is_none());
    }

    #[test]
    fn test_update_horizon_on_resolved_fails() {
        use crate::Horizon;
        let mut t = Tension::new_full(
            "goal",
            "reality",
            None,
            Some(Horizon::new_year(2026).unwrap()),
        )
        .unwrap();
        t.resolve().unwrap();
        let result = t.update_horizon(Some(Horizon::new_month(2026, 5).unwrap()));
        assert!(result.is_err());
        match result.unwrap_err() {
            SdError::UpdateOnInactiveTension(status) => {
                assert_eq!(status, TensionStatus::Resolved);
            }
            other => panic!("expected UpdateOnInactiveTension, got {other:?}"), // ubs:ignore test assertion
        }
        // Original horizon preserved
        assert_eq!(t.horizon, Some(Horizon::new_year(2026).unwrap()));
    }

    #[test]
    fn test_update_horizon_on_released_fails() {
        use crate::Horizon;
        let mut t = Tension::new_full(
            "goal",
            "reality",
            None,
            Some(Horizon::new_year(2026).unwrap()),
        )
        .unwrap();
        t.release().unwrap();
        let result = t.update_horizon(Some(Horizon::new_month(2026, 5).unwrap()));
        assert!(result.is_err());
        match result.unwrap_err() {
            SdError::UpdateOnInactiveTension(status) => {
                assert_eq!(status, TensionStatus::Released);
            }
            other => panic!("expected UpdateOnInactiveTension, got {other:?}"), // ubs:ignore test assertion
        }
        // Original horizon preserved
        assert_eq!(t.horizon, Some(Horizon::new_year(2026).unwrap()));
    }

    // ── Horizon serialization ─────────────────────────────────────────

    #[test]
    fn test_tension_serialization_with_horizon() {
        use crate::Horizon;
        let t = Tension::new_full(
            "goal",
            "reality",
            None,
            Some(Horizon::new_month(2026, 5).unwrap()),
        )
        .unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value.get("horizon").unwrap().as_str().unwrap(), "2026-05");

        let deserialized: Tension = serde_json::from_str(&json).unwrap();
        assert_eq!(t, deserialized);
    }

    #[test]
    fn test_tension_serialization_without_horizon() {
        let t = Tension::new("goal", "reality").unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("horizon").unwrap().is_null());

        let deserialized: Tension = serde_json::from_str(&json).unwrap();
        assert_eq!(t, deserialized);
    }

    #[test]
    fn test_tension_serialization_roundtrip_all_horizon_variants() {
        use crate::Horizon;
        use chrono::{TimeZone, Utc};

        let horizons = [
            Some(Horizon::new_year(2026).unwrap()),
            Some(Horizon::new_month(2026, 5).unwrap()),
            Some(Horizon::new_day(2026, 5, 15).unwrap()),
            Some(Horizon::new_datetime(
                Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 0).unwrap(),
            )),
            None,
        ];

        for horizon in horizons {
            let t = Tension::new_full("goal", "reality", None, horizon.clone()).unwrap();
            let json = serde_json::to_string(&t).unwrap();
            let deserialized: Tension = serde_json::from_str(&json).unwrap();
            assert_eq!(t, deserialized);
        }
    }
}
