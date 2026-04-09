//! Store wrapper with convenience methods.
//!
//! Provides a thin layer over Store that handles parent snapshot capture
//! during tension creation.

use crate::temporal::compute_urgency;
use crate::events::{EventBus};
use crate::horizon::Horizon;
use crate::store::Store;
use crate::tension::Tension;

/// Store wrapper with event bus and convenience methods.
pub struct Engine {
    store: Store,
    bus: EventBus,
}

impl Engine {
    /// Create a new engine with an in-memory store.
    pub fn new_in_memory() -> Result<Self, crate::store::StoreError> {
        let mut store = Store::new_in_memory()?;
        let bus = EventBus::new();
        store.set_event_bus(bus.clone());
        Ok(Self { store, bus })
    }

    /// Create an engine with an existing store.
    pub fn with_store(mut store: Store) -> Self {
        let bus = EventBus::new();
        store.set_event_bus(bus.clone());
        Self { store, bus }
    }

    /// Set the event bus (also updates the store's event bus).
    pub fn set_event_bus(&mut self, bus: EventBus) {
        self.store.set_event_bus(bus.clone());
        self.bus = bus;
    }

    /// Get a reference to the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.bus
    }

    /// Get a mutable reference to the store.
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    /// Get a reference to the store.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Create a tension and emit TensionCreated event.
    pub fn create_tension(
        &mut self,
        desired: &str,
        actual: &str,
    ) -> Result<Tension, crate::tension::SdError> {
        self.store.create_tension(desired, actual)
    }

    /// Capture parent's full state for snapshot storage.
    /// Returns (desired_text, actual_text, full_json_snapshot).
    fn parent_snapshots(&self, parent_id: &Option<String>) -> (Option<String>, Option<String>, Option<String>) {
        if let Some(pid) = parent_id {
            match self.store.get_tension(pid) {
                Ok(Some(parent)) => {
                    let desired = parent.desired.clone();
                    let actual = parent.actual.clone();
                    let json = self.build_parent_snapshot_json(pid, &parent);
                    (Some(desired), Some(actual), json)
                }
                _ => (None, None, None),
            }
        } else {
            (None, None, None)
        }
    }

    /// Build a JSON snapshot of a parent's full descended view state.
    fn build_parent_snapshot_json(&self, parent_id: &str, parent: &Tension) -> Option<String> {
        let children = self.store.get_children(parent_id).ok()?;
        let children_json: Vec<serde_json::Value> = children.iter().map(|c| {
            serde_json::json!({
                "id": c.id,
                "desired": c.desired,
                "actual": c.actual,
                "status": c.status.to_string(),
                "position": c.position,
                "horizon": c.horizon.as_ref().map(|h| h.to_string()),
            })
        }).collect();

        let snapshot = serde_json::json!({
            "desired": parent.desired,
            "actual": parent.actual,
            "status": parent.status.to_string(),
            "horizon": parent.horizon.as_ref().map(|h| h.to_string()),
            "children": children_json,
        });

        serde_json::to_string(&snapshot).ok()
    }

    /// Create a tension with parent, capturing parent snapshots.
    pub fn create_tension_with_parent(
        &mut self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
    ) -> Result<Tension, crate::tension::SdError> {
        let (parent_desired_snapshot, parent_actual_snapshot, parent_snapshot_json) =
            self.parent_snapshots(&parent_id);

        self.store.create_tension_full_with_snapshots(
            desired,
            actual,
            parent_id,
            None,
            None,
            parent_desired_snapshot,
            parent_actual_snapshot,
            parent_snapshot_json,
        )
    }

    /// Update actual.
    pub fn update_actual(
        &mut self,
        id: &str,
        new_actual: &str,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_actual(id, new_actual)
    }

    /// Update desired.
    pub fn update_desired(
        &mut self,
        id: &str,
        new_desired: &str,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_desired(id, new_desired)
    }

    /// Update the position of a tension for sibling ordering.
    /// Returns true if position actually changed, false if already at target value.
    pub fn update_position(
        &mut self,
        id: &str,
        new_position: Option<i32>,
    ) -> Result<bool, crate::tension::SdError> {
        self.store.update_position(id, new_position)
    }

    /// Reorder siblings by assigning positions to all children of a parent.
    pub fn reorder_siblings(
        &mut self,
        ordered_ids: &[String],
    ) -> Result<(), crate::tension::SdError> {
        self.store.reorder_siblings(ordered_ids)
    }

    /// Update parent.
    pub fn update_parent(
        &mut self,
        id: &str,
        new_parent_id: Option<&str>,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_parent(id, new_parent_id)
    }

    /// Resolve a tension.
    pub fn resolve(&mut self, id: &str) -> Result<(), crate::tension::SdError> {
        self.store
            .update_status(id, crate::tension::TensionStatus::Resolved)
    }

    /// Release a tension.
    pub fn release(&mut self, id: &str) -> Result<(), crate::tension::SdError> {
        self.store
            .update_status(id, crate::tension::TensionStatus::Released)
    }

    /// Create a tension with all optional fields including horizon.
    /// Captures parent snapshots automatically.
    pub fn create_tension_full(
        &mut self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> Result<Tension, crate::tension::SdError> {
        let (parent_desired_snapshot, parent_actual_snapshot, parent_snapshot_json) =
            self.parent_snapshots(&parent_id);

        self.store.create_tension_full_with_snapshots(
            desired,
            actual,
            parent_id,
            horizon,
            None,
            parent_desired_snapshot,
            parent_actual_snapshot,
            parent_snapshot_json,
        )
    }

    // ── Gesture lifecycle ──────────────────────────────────────────

    /// Begin a gesture within a session (for TUI instances).
    pub fn begin_gesture_in_session(
        &mut self,
        session_id: &str,
        description: Option<&str>,
    ) -> Result<String, crate::store::StoreError> {
        self.store.begin_gesture_in_session(session_id, description)
    }

    /// Begin a sessionless gesture (for CLI commands).
    pub fn begin_gesture(
        &mut self,
        description: Option<&str>,
    ) -> Result<String, crate::store::StoreError> {
        self.store.begin_gesture(description)
    }

    /// End the current gesture, returning its ID.
    pub fn end_gesture(&mut self) -> Option<String> {
        self.store.end_gesture()
    }

    /// Get the currently active gesture ID, if any.
    pub fn active_gesture(&self) -> Option<&str> {
        self.store.active_gesture()
    }

    /// Update the temporal horizon of a tension.
    pub fn update_horizon(
        &mut self,
        id: &str,
        new_horizon: Option<Horizon>,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_horizon(id, new_horizon)
    }

    /// Get urgency for a tension (convenience method).
    pub fn compute_urgency(&self, tension: &Tension) -> Option<crate::temporal::Urgency> {
        compute_urgency(tension, chrono::Utc::now())
    }

    // ── Gesture undo ──────────────────────────────────────────────

    /// Undo a gesture by appending reversal mutations.
    ///
    /// Returns the undo gesture ID on success.
    pub fn undo_gesture(
        &mut self,
        gesture_id: &str,
    ) -> Result<String, crate::tension::SdError> {
        let undo_id = self.store.undo_gesture(gesture_id)?;

        self.bus.emit(&crate::events::Event::GestureUndone {
            gesture_id: gesture_id.to_owned(),
            undo_gesture_id: undo_id.clone(),
            reversed_mutation_count: self
                .store
                .get_gesture_mutations(gesture_id)
                .map(|m| m.len())
                .unwrap_or(0),
            timestamp: chrono::Utc::now(),
        });

        Ok(undo_id)
    }

    /// Get the most recent gesture ID.
    pub fn get_last_gesture_id(&self) -> Result<Option<String>, crate::store::StoreError> {
        self.store.get_last_gesture_id()
    }

    // ── Edge operations ───────────────────────────────────────────

    /// Create a typed edge between two tensions.
    pub fn create_edge(
        &mut self,
        from_id: &str,
        to_id: &str,
        edge_type: &str,
    ) -> Result<crate::edge::Edge, crate::store::StoreError> {
        self.store.create_edge(from_id, to_id, edge_type)
    }

    /// Remove an edge by from_id, to_id, and type.
    pub fn remove_edge(
        &mut self,
        from_id: &str,
        to_id: &str,
        edge_type: &str,
    ) -> Result<bool, crate::store::StoreError> {
        self.store.remove_edge(from_id, to_id, edge_type)
    }

    /// Get all edges for a tension.
    pub fn get_edges_for_tension(
        &self,
        tension_id: &str,
    ) -> Result<Vec<crate::edge::Edge>, crate::store::StoreError> {
        self.store.get_edges_for_tension(tension_id)
    }

    /// Get all edges (for Forest construction).
    pub fn get_all_edges(&self) -> Result<Vec<crate::edge::Edge>, crate::store::StoreError> {
        self.store.get_all_edges()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creates_tension() {
        let mut engine = Engine::new_in_memory().unwrap();
        let t = engine.create_tension("goal", "reality").unwrap();
        assert_eq!(t.desired, "goal");
        assert_eq!(t.actual, "reality");
    }

    #[test]
    fn test_engine_creates_tension_with_parent() {
        let mut engine = Engine::new_in_memory().unwrap();
        let parent = engine.create_tension("big goal", "starting point").unwrap();
        let child = engine
            .create_tension_with_parent("sub goal", "sub reality", Some(parent.id.clone()))
            .unwrap();
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_engine_creates_tension_full_with_horizon() {
        let mut engine = Engine::new_in_memory().unwrap();
        let h = Horizon::new_month(2026, 6).unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(h))
            .unwrap();
        assert!(t.horizon.is_some());
    }

    #[test]
    fn test_engine_update_horizon() {
        let mut engine = Engine::new_in_memory().unwrap();
        let t = engine.create_tension("goal", "reality").unwrap();
        let h = Horizon::new_month(2026, 6).unwrap();
        engine.update_horizon(&t.id, Some(h)).unwrap();
        let updated = engine.store().get_tension(&t.id).unwrap().unwrap();
        assert!(updated.horizon.is_some());
    }

    #[test]
    fn test_engine_store_access() {
        let mut engine = Engine::new_in_memory().unwrap();
        let t = engine.create_tension("goal", "reality").unwrap();
        let found = engine.store().get_tension(&t.id).unwrap();
        assert!(found.is_some());

        engine.store_mut().update_actual(&t.id, "new reality").unwrap();
        let updated = engine.store().get_tension(&t.id).unwrap().unwrap();
        assert_eq!(updated.actual, "new reality");
    }
}
