//! Typed event system for structural dynamics.
//!
//! Events are deterministic: the same operations on the same initial state
//! always produce the same event sequence.
//!
//! # Event Types
//!
//! **State Change Events:**
//! - `TensionCreated` — new tension persisted
//! - `RealityConfronted` — actual field updated
//! - `DesireRevised` — desired field updated
//! - `TensionResolved` — tension marked Resolved
//! - `TensionReleased` — tension marked Released
//! - `TensionDeleted` — tension deleted
//! - `StructureChanged` — parent_id changed
//! - `HorizonChanged` — temporal horizon changed

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::temporal::HorizonDriftType;

#[cfg(test)]
use crate::tension::TensionStatus;

// ============================================================================
// Event Types
// ============================================================================

/// All possible event types emitted by the grammar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// A new tension was created.
    TensionCreated {
        tension_id: String,
        desired: String,
        actual: String,
        parent_id: Option<String>,
        horizon: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// The actual state was updated (reality confronted).
    RealityConfronted {
        tension_id: String,
        old_actual: String,
        new_actual: String,
        timestamp: DateTime<Utc>,
    },

    /// The desired state was updated.
    DesireRevised {
        tension_id: String,
        old_desired: String,
        new_desired: String,
        timestamp: DateTime<Utc>,
    },

    /// A tension was resolved.
    TensionResolved {
        tension_id: String,
        final_desired: String,
        final_actual: String,
        timestamp: DateTime<Utc>,
    },

    /// A tension was released.
    TensionReleased {
        tension_id: String,
        desired: String,
        actual: String,
        timestamp: DateTime<Utc>,
    },

    /// A tension was deleted.
    TensionDeleted {
        tension_id: String,
        desired: String,
        actual: String,
        timestamp: DateTime<Utc>,
    },

    /// Parent-child relationship changed.
    StructureChanged {
        tension_id: String,
        old_parent_id: Option<String>,
        new_parent_id: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Temporal horizon was changed.
    HorizonChanged {
        tension_id: String,
        old_horizon: Option<String>,
        new_horizon: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Urgency crossed a configured threshold (up or down).
    UrgencyThresholdCrossed {
        tension_id: String,
        old_urgency: f64,
        new_urgency: f64,
        threshold: f64,
        crossed_above: bool,
        timestamp: DateTime<Utc>,
    },

    /// Horizon drift pattern detected or changed.
    HorizonDriftDetected {
        tension_id: String,
        drift_type: HorizonDriftType,
        change_count: usize,
        timestamp: DateTime<Utc>,
    },

    /// A gesture was undone (all its mutations reversed).
    GestureUndone {
        gesture_id: String,
        undo_gesture_id: String,
        reversed_mutation_count: usize,
        timestamp: DateTime<Utc>,
    },
}

impl Event {
    /// Get the primary tension ID for this event, if any.
    pub fn tension_id(&self) -> Option<&str> {
        match self {
            Event::TensionCreated { tension_id, .. } => Some(tension_id),
            Event::RealityConfronted { tension_id, .. } => Some(tension_id),
            Event::DesireRevised { tension_id, .. } => Some(tension_id),
            Event::TensionResolved { tension_id, .. } => Some(tension_id),
            Event::TensionReleased { tension_id, .. } => Some(tension_id),
            Event::TensionDeleted { tension_id, .. } => Some(tension_id),
            Event::StructureChanged { tension_id, .. } => Some(tension_id),
            Event::HorizonChanged { tension_id, .. } => Some(tension_id),
            Event::UrgencyThresholdCrossed { tension_id, .. } => Some(tension_id),
            Event::HorizonDriftDetected { tension_id, .. } => Some(tension_id),
            Event::GestureUndone { .. } => None, // gesture-level, not tension-level
        }
    }

    /// The hook name for this event, matching the serde tag (snake_case variant name).
    ///
    /// Used by the HookBridge to derive `pre_` and `post_` hook names automatically.
    /// Adding a new Event variant here makes it hookable with zero additional wiring.
    pub fn hook_name(&self) -> &'static str {
        match self {
            Event::TensionCreated { .. } => "tension_created",
            Event::RealityConfronted { .. } => "reality_confronted",
            Event::DesireRevised { .. } => "desire_revised",
            Event::TensionResolved { .. } => "tension_resolved",
            Event::TensionReleased { .. } => "tension_released",
            Event::TensionDeleted { .. } => "tension_deleted",
            Event::StructureChanged { .. } => "structure_changed",
            Event::HorizonChanged { .. } => "horizon_changed",
            Event::UrgencyThresholdCrossed { .. } => "urgency_threshold_crossed",
            Event::HorizonDriftDetected { .. } => "horizon_drift_detected",
            Event::GestureUndone { .. } => "gesture_undone",
        }
    }

    /// Whether this event represents a commandable mutation (user-initiated).
    ///
    /// Commandable events get both `pre_` and `post_` hooks.
    /// Non-commandable events (computed signals) get only `post_` hooks.
    pub fn is_commandable(&self) -> bool {
        !matches!(
            self,
            Event::UrgencyThresholdCrossed { .. } | Event::HorizonDriftDetected { .. }
        )
    }

    /// The category for this event, used for category-level hooks.
    ///
    /// Categories are convenience abstractions: `pre_mutation` fires for any
    /// commandable mutation, `post_mutation` fires for any post-event.
    /// `post_create` and `post_status_change` are lifecycle categories.
    pub fn category(&self) -> &'static str {
        match self {
            Event::TensionCreated { .. } => "create",
            Event::TensionResolved { .. } | Event::TensionReleased { .. } => "status_change",
            Event::TensionDeleted { .. } => "delete",
            Event::GestureUndone { .. } => "undo",
            _ => "mutation",
        }
    }

    /// Get the timestamp for this event.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Event::TensionCreated { timestamp, .. } => *timestamp,
            Event::RealityConfronted { timestamp, .. } => *timestamp,
            Event::DesireRevised { timestamp, .. } => *timestamp,
            Event::TensionResolved { timestamp, .. } => *timestamp,
            Event::TensionReleased { timestamp, .. } => *timestamp,
            Event::TensionDeleted { timestamp, .. } => *timestamp,
            Event::StructureChanged { timestamp, .. } => *timestamp,
            Event::HorizonChanged { timestamp, .. } => *timestamp,
            Event::UrgencyThresholdCrossed { timestamp, .. } => *timestamp,
            Event::HorizonDriftDetected { timestamp, .. } => *timestamp,
            Event::GestureUndone { timestamp, .. } => *timestamp,
        }
    }
}

// ============================================================================
// Event Bus
// ============================================================================

/// Callback function type for event subscribers.
pub type EventCallback = Arc<dyn Fn(&Event) + Send + Sync>;

/// Unique identifier for a subscriber.
type SubscriberId = u64;

/// A handle to an event subscription.
///
/// When dropped, the subscription is automatically cancelled.
pub struct SubscriptionHandle {
    id: SubscriberId,
    bus: EventBus,
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        self.bus.unsubscribe(self.id);
    }
}

/// Inner state of the event bus.
struct EventBusInner {
    next_id: SubscriberId,
    subscribers: std::collections::HashMap<SubscriberId, EventCallback>,
    history: Vec<Event>,
}

/// A thread-safe event bus for publishing and subscribing to events.
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<Mutex<EventBusInner>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new event bus with no subscribers.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EventBusInner {
                next_id: 0,
                subscribers: std::collections::HashMap::new(),
                history: Vec::new(),
            })),
        }
    }

    /// Subscribe to all events.
    pub fn subscribe<F>(&self, callback: F) -> SubscriptionHandle
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let mut inner = self.inner.lock().unwrap(); // ubs:ignore poisoned mutex is unrecoverable
        let id = inner.next_id;
        inner.next_id += 1;
        inner.subscribers.insert(id, Arc::new(callback));
        SubscriptionHandle {
            id,
            bus: self.clone(),
        }
    }

    /// Unsubscribe by ID (called when handle is dropped).
    fn unsubscribe(&self, id: SubscriberId) {
        let mut inner = self.inner.lock().unwrap(); // ubs:ignore poisoned mutex is unrecoverable
        inner.subscribers.remove(&id);
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: &Event) {
        let mut inner = self.inner.lock().unwrap(); // ubs:ignore poisoned mutex is unrecoverable
        inner.history.push(event.clone());

        let subscribers: Vec<(SubscriberId, EventCallback)> = inner
            .subscribers
            .iter()
            .map(|(id, cb)| (*id, Arc::clone(cb)))
            .collect();

        drop(inner);

        for (id, callback) in subscribers {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(event)));
            if result.is_err() {
                let mut inner = self.inner.lock().unwrap(); // ubs:ignore poisoned mutex is unrecoverable
                inner.subscribers.remove(&id);
            }
        }
    }

    /// Get the event history (for testing).
    pub fn history(&self) -> Vec<Event> {
        let inner = self.inner.lock().unwrap(); // ubs:ignore poisoned mutex is unrecoverable
        inner.history.clone()
    }

    /// Clear the event history.
    pub fn clear_history(&self) {
        let mut inner = self.inner.lock().unwrap(); // ubs:ignore poisoned mutex is unrecoverable
        inner.history.clear();
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        let inner = self.inner.lock().unwrap(); // ubs:ignore poisoned mutex is unrecoverable
        inner.subscribers.len()
    }
}

// ============================================================================
// Event Builder Helpers
// ============================================================================

/// Helper for building events with consistent timestamps.
pub struct EventBuilder;

impl EventBuilder {
    pub fn tension_created(
        tension_id: String,
        desired: String,
        actual: String,
        parent_id: Option<String>,
        horizon: Option<String>,
    ) -> Event {
        Event::TensionCreated {
            tension_id, desired, actual, parent_id, horizon,
            timestamp: Utc::now(),
        }
    }

    pub fn reality_confronted(tension_id: String, old_actual: String, new_actual: String) -> Event {
        Event::RealityConfronted {
            tension_id, old_actual, new_actual,
            timestamp: Utc::now(),
        }
    }

    pub fn desire_revised(tension_id: String, old_desired: String, new_desired: String) -> Event {
        Event::DesireRevised {
            tension_id, old_desired, new_desired,
            timestamp: Utc::now(),
        }
    }

    pub fn tension_resolved(tension_id: String, final_desired: String, final_actual: String) -> Event {
        Event::TensionResolved {
            tension_id, final_desired, final_actual,
            timestamp: Utc::now(),
        }
    }

    pub fn tension_released(tension_id: String, desired: String, actual: String) -> Event {
        Event::TensionReleased {
            tension_id, desired, actual,
            timestamp: Utc::now(),
        }
    }

    pub fn tension_deleted(tension_id: String, desired: String, actual: String) -> Event {
        Event::TensionDeleted {
            tension_id, desired, actual,
            timestamp: Utc::now(),
        }
    }

    pub fn structure_changed(
        tension_id: String,
        old_parent_id: Option<String>,
        new_parent_id: Option<String>,
    ) -> Event {
        Event::StructureChanged {
            tension_id, old_parent_id, new_parent_id,
            timestamp: Utc::now(),
        }
    }

    pub fn horizon_changed(
        tension_id: String,
        old_horizon: Option<String>,
        new_horizon: Option<String>,
    ) -> Event {
        Event::HorizonChanged {
            tension_id, old_horizon, new_horizon,
            timestamp: Utc::now(),
        }
    }

    pub fn urgency_threshold_crossed(
        tension_id: String,
        old_urgency: f64,
        new_urgency: f64,
        threshold: f64,
        crossed_above: bool,
    ) -> Event {
        Event::UrgencyThresholdCrossed {
            tension_id, old_urgency, new_urgency, threshold, crossed_above,
            timestamp: Utc::now(),
        }
    }

    pub fn gesture_undone(
        gesture_id: String,
        undo_gesture_id: String,
        reversed_mutation_count: usize,
    ) -> Event {
        Event::GestureUndone {
            gesture_id, undo_gesture_id, reversed_mutation_count,
            timestamp: Utc::now(),
        }
    }

    pub fn horizon_drift_detected(
        tension_id: String,
        drift_type: HorizonDriftType,
        change_count: usize,
    ) -> Event {
        Event::HorizonDriftDetected {
            tension_id, drift_type, change_count,
            timestamp: Utc::now(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_event_serialization_roundtrip() {
        let events = vec![
            EventBuilder::tension_created(
                "01ABC".to_owned(), "goal".to_owned(), "reality".to_owned(), None, None,
            ),
            EventBuilder::reality_confronted(
                "01ABC".to_owned(), "old reality".to_owned(), "new reality".to_owned(),
            ),
            EventBuilder::desire_revised(
                "01ABC".to_owned(), "old goal".to_owned(), "new goal".to_owned(),
            ),
            EventBuilder::tension_resolved(
                "01ABC".to_owned(), "final goal".to_owned(), "final reality".to_owned(),
            ),
            EventBuilder::tension_released(
                "01DEF".to_owned(), "goal".to_owned(), "reality".to_owned(),
            ),
            EventBuilder::structure_changed(
                "01ABC".to_owned(), Some("parent1".to_owned()), Some("parent2".to_owned()),
            ),
            EventBuilder::horizon_changed(
                "01ABC".to_owned(), Some("2026-05".to_owned()), Some("2026-06".to_owned()),
            ),
            EventBuilder::urgency_threshold_crossed(
                "01ABC".to_owned(), 0.4, 0.6, 0.5, true,
            ),
            EventBuilder::horizon_drift_detected(
                "01ABC".to_owned(), HorizonDriftType::Postponement, 2,
            ),
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: Event = serde_json::from_str(&json).unwrap();
            assert_eq!(event, deserialized);
        }
    }

    #[test]
    fn test_event_tagged_serialization() {
        let event = EventBuilder::tension_created(
            "01ABC".to_owned(), "goal".to_owned(), "reality".to_owned(), None, None,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"tension_created\""));
    }

    #[test]
    fn test_event_tension_id() {
        let event = EventBuilder::tension_created(
            "01ABC".to_owned(), "goal".to_owned(), "reality".to_owned(), None, None,
        );
        assert_eq!(event.tension_id(), Some("01ABC"));
    }

    #[test]
    fn test_event_timestamp() {
        let before = Utc::now();
        let event = EventBuilder::tension_created(
            "01ABC".to_owned(), "goal".to_owned(), "reality".to_owned(), None, None,
        );
        let after = Utc::now();

        let ts = event.timestamp();
        assert!(ts >= before);
        assert!(ts <= after);
    }

    #[test]
    fn test_event_bus_subscribe_and_emit() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let count_clone = count.clone();
        let _handle = bus.subscribe(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = EventBuilder::tension_created(
            "01ABC".to_owned(), "goal".to_owned(), "reality".to_owned(), None, None,
        );
        bus.emit(&event);
        bus.emit(&event);

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_event_bus_unsubscribe_on_drop() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let count_clone = count.clone();
        let handle = bus.subscribe(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = EventBuilder::tension_created(
            "01ABC".to_owned(), "goal".to_owned(), "reality".to_owned(), None, None,
        );
        bus.emit(&event);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        drop(handle);
        bus.emit(&event);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_bus_history() {
        let bus = EventBus::new();

        bus.emit(&EventBuilder::tension_created(
            "01A".to_owned(), "g1".to_owned(), "r1".to_owned(), None, None,
        ));
        bus.emit(&EventBuilder::reality_confronted(
            "01A".to_owned(), "r1".to_owned(), "r2".to_owned(),
        ));

        let history = bus.history();
        assert_eq!(history.len(), 2);
    }
}
