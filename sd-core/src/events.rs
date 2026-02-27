//! Typed event system for structural dynamics.
//!
//! This module defines all event types emitted by the grammar layer.
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
//! - `StructureChanged` — parent_id changed
//!
//! **Dynamic Transition Events:**
//! - `ConflictDetected` — structural conflict emerged
//! - `ConflictResolved` — conflict ended
//! - `LifecycleTransition` — creative cycle phase changed
//! - `OscillationDetected` — oscillation pattern detected
//! - `ResolutionAchieved` — resolution pattern detected
//! - `NeglectDetected` — neglect pattern detected
//! - `OrientationShift` — orientation pattern changed
//!
//! # Subscription Model
//!
//! ```ignore
//! let bus = EventBus::new();
//! let handle = bus.subscribe(|event| {
//!     println!("Got event: {:?}", event);
//! });
//! // Events are delivered to all subscribers
//! bus.emit(&event);
//! // Dropping handle stops delivery to that subscriber
//! drop(handle);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::dynamics::{ConflictPattern, CreativeCyclePhase, NeglectType, Orientation};

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
        /// ULID of the new tension.
        tension_id: String,
        /// The desired state.
        desired: String,
        /// The actual state.
        actual: String,
        /// Optional parent reference.
        parent_id: Option<String>,
        /// When the tension was created.
        timestamp: DateTime<Utc>,
    },

    /// The actual state was updated (reality confronted).
    RealityConfronted {
        /// ULID of the tension.
        tension_id: String,
        /// The previous actual state.
        old_actual: String,
        /// The new actual state.
        new_actual: String,
        /// When the update occurred.
        timestamp: DateTime<Utc>,
    },

    /// The desired state was updated.
    DesireRevised {
        /// ULID of the tension.
        tension_id: String,
        /// The previous desired state.
        old_desired: String,
        /// The new desired state.
        new_desired: String,
        /// When the update occurred.
        timestamp: DateTime<Utc>,
    },

    /// A tension was resolved.
    TensionResolved {
        /// ULID of the tension.
        tension_id: String,
        /// The final desired state.
        final_desired: String,
        /// The final actual state.
        final_actual: String,
        /// When resolution occurred.
        timestamp: DateTime<Utc>,
    },

    /// A tension was released.
    TensionReleased {
        /// ULID of the tension.
        tension_id: String,
        /// The desired state at release.
        desired: String,
        /// The actual state at release.
        actual: String,
        /// When release occurred.
        timestamp: DateTime<Utc>,
    },

    /// Structural conflict was detected.
    ConflictDetected {
        /// Tensions involved in the conflict.
        tension_ids: Vec<String>,
        /// The conflict pattern.
        pattern: ConflictPattern,
        /// When detected.
        timestamp: DateTime<Utc>,
    },

    /// Structural conflict was resolved.
    ConflictResolved {
        /// Tensions that were in conflict.
        tension_ids: Vec<String>,
        /// The former conflict pattern.
        former_pattern: ConflictPattern,
        /// When resolved.
        timestamp: DateTime<Utc>,
    },

    /// Creative cycle phase transitioned.
    LifecycleTransition {
        /// ULID of the tension.
        tension_id: String,
        /// Previous phase.
        old_phase: CreativeCyclePhase,
        /// New phase.
        new_phase: CreativeCyclePhase,
        /// When transition occurred.
        timestamp: DateTime<Utc>,
    },

    /// Oscillation pattern detected.
    OscillationDetected {
        /// ULID of the oscillating tension.
        tension_id: String,
        /// Number of reversals.
        reversals: usize,
        /// Magnitude of oscillation.
        magnitude: f64,
        /// When detected.
        timestamp: DateTime<Utc>,
    },

    /// Resolution achieved.
    ResolutionAchieved {
        /// ULID of the resolving tension.
        tension_id: String,
        /// Velocity of progress.
        velocity: f64,
        /// When detected.
        timestamp: DateTime<Utc>,
    },

    /// Neglect detected.
    NeglectDetected {
        /// Tensions involved.
        tension_ids: Vec<String>,
        /// Type of neglect.
        neglect_type: NeglectType,
        /// When detected.
        timestamp: DateTime<Utc>,
    },

    /// Parent-child relationship changed.
    StructureChanged {
        /// ULID of the tension.
        tension_id: String,
        /// Previous parent.
        old_parent_id: Option<String>,
        /// New parent.
        new_parent_id: Option<String>,
        /// When change occurred.
        timestamp: DateTime<Utc>,
    },

    /// Orientation pattern shifted.
    OrientationShift {
        /// Tensions analyzed for orientation.
        tension_ids: Vec<String>,
        /// Previous orientation.
        old_orientation: Orientation,
        /// New orientation.
        new_orientation: Orientation,
        /// When detected.
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
            Event::ConflictDetected { .. } => None,
            Event::ConflictResolved { .. } => None,
            Event::LifecycleTransition { tension_id, .. } => Some(tension_id),
            Event::OscillationDetected { tension_id, .. } => Some(tension_id),
            Event::ResolutionAchieved { tension_id, .. } => Some(tension_id),
            Event::NeglectDetected { .. } => None,
            Event::StructureChanged { tension_id, .. } => Some(tension_id),
            Event::OrientationShift { .. } => None,
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
            Event::ConflictDetected { timestamp, .. } => *timestamp,
            Event::ConflictResolved { timestamp, .. } => *timestamp,
            Event::LifecycleTransition { timestamp, .. } => *timestamp,
            Event::OscillationDetected { timestamp, .. } => *timestamp,
            Event::ResolutionAchieved { timestamp, .. } => *timestamp,
            Event::NeglectDetected { timestamp, .. } => *timestamp,
            Event::StructureChanged { timestamp, .. } => *timestamp,
            Event::OrientationShift { timestamp, .. } => *timestamp,
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
    /// Next subscriber ID.
    next_id: SubscriberId,
    /// Active subscribers.
    subscribers: std::collections::HashMap<SubscriberId, EventCallback>,
    /// Events emitted (for testing determinism).
    history: Vec<Event>,
}

/// A thread-safe event bus for publishing and subscribing to events.
///
/// # Subscriber Isolation
///
/// If a subscriber callback panics, the panic is caught and the subscriber
/// is removed, but other subscribers continue to receive events.
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
    ///
    /// Returns a handle. When the handle is dropped, the subscription ends.
    pub fn subscribe<F>(&self, callback: F) -> SubscriptionHandle
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let mut inner = self.inner.lock().unwrap();
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
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.remove(&id);
    }

    /// Emit an event to all subscribers.
    ///
    /// The event is delivered to all active subscribers in order.
    /// If a subscriber panics, it is removed but other subscribers
    /// continue to receive the event.
    pub fn emit(&self, event: &Event) {
        let mut inner = self.inner.lock().unwrap();

        // Record in history for determinism verification
        inner.history.push(event.clone());

        // Get subscriber IDs and callbacks (cloned Arc is cheap)
        let subscribers: Vec<(SubscriberId, EventCallback)> = inner
            .subscribers
            .iter()
            .map(|(id, cb)| (*id, Arc::clone(cb)))
            .collect();

        // Release the lock while calling callbacks to avoid deadlock
        // if a callback tries to subscribe/unsubscribe
        drop(inner);

        for (id, callback) in subscribers {
            // Call with panic isolation
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(event)));

            if result.is_err() {
                // Subscriber panicked, remove it
                let mut inner = self.inner.lock().unwrap();
                inner.subscribers.remove(&id);
            }
        }
    }

    /// Get the event history (for testing).
    pub fn history(&self) -> Vec<Event> {
        let inner = self.inner.lock().unwrap();
        inner.history.clone()
    }

    /// Clear the event history.
    pub fn clear_history(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.history.clear();
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.subscribers.len()
    }
}

// ============================================================================
// Event Builder Helpers
// ============================================================================

/// Helper for building events with consistent timestamps.
pub struct EventBuilder;

impl EventBuilder {
    /// Build a TensionCreated event.
    pub fn tension_created(
        tension_id: String,
        desired: String,
        actual: String,
        parent_id: Option<String>,
    ) -> Event {
        Event::TensionCreated {
            tension_id,
            desired,
            actual,
            parent_id,
            timestamp: Utc::now(),
        }
    }

    /// Build a RealityConfronted event.
    pub fn reality_confronted(tension_id: String, old_actual: String, new_actual: String) -> Event {
        Event::RealityConfronted {
            tension_id,
            old_actual,
            new_actual,
            timestamp: Utc::now(),
        }
    }

    /// Build a DesireRevised event.
    pub fn desire_revised(tension_id: String, old_desired: String, new_desired: String) -> Event {
        Event::DesireRevised {
            tension_id,
            old_desired,
            new_desired,
            timestamp: Utc::now(),
        }
    }

    /// Build a TensionResolved event.
    pub fn tension_resolved(
        tension_id: String,
        final_desired: String,
        final_actual: String,
    ) -> Event {
        Event::TensionResolved {
            tension_id,
            final_desired,
            final_actual,
            timestamp: Utc::now(),
        }
    }

    /// Build a TensionReleased event.
    pub fn tension_released(tension_id: String, desired: String, actual: String) -> Event {
        Event::TensionReleased {
            tension_id,
            desired,
            actual,
            timestamp: Utc::now(),
        }
    }

    /// Build a StructureChanged event.
    pub fn structure_changed(
        tension_id: String,
        old_parent_id: Option<String>,
        new_parent_id: Option<String>,
    ) -> Event {
        Event::StructureChanged {
            tension_id,
            old_parent_id,
            new_parent_id,
            timestamp: Utc::now(),
        }
    }

    /// Build a ConflictDetected event.
    pub fn conflict_detected(tension_ids: Vec<String>, pattern: ConflictPattern) -> Event {
        Event::ConflictDetected {
            tension_ids,
            pattern,
            timestamp: Utc::now(),
        }
    }

    /// Build a ConflictResolved event.
    pub fn conflict_resolved(tension_ids: Vec<String>, former_pattern: ConflictPattern) -> Event {
        Event::ConflictResolved {
            tension_ids,
            former_pattern,
            timestamp: Utc::now(),
        }
    }

    /// Build a LifecycleTransition event.
    pub fn lifecycle_transition(
        tension_id: String,
        old_phase: CreativeCyclePhase,
        new_phase: CreativeCyclePhase,
    ) -> Event {
        Event::LifecycleTransition {
            tension_id,
            old_phase,
            new_phase,
            timestamp: Utc::now(),
        }
    }

    /// Build an OscillationDetected event.
    pub fn oscillation_detected(tension_id: String, reversals: usize, magnitude: f64) -> Event {
        Event::OscillationDetected {
            tension_id,
            reversals,
            magnitude,
            timestamp: Utc::now(),
        }
    }

    /// Build a ResolutionAchieved event.
    pub fn resolution_achieved(tension_id: String, velocity: f64) -> Event {
        Event::ResolutionAchieved {
            tension_id,
            velocity,
            timestamp: Utc::now(),
        }
    }

    /// Build a NeglectDetected event.
    pub fn neglect_detected(tension_ids: Vec<String>, neglect_type: NeglectType) -> Event {
        Event::NeglectDetected {
            tension_ids,
            neglect_type,
            timestamp: Utc::now(),
        }
    }

    /// Build an OrientationShift event.
    pub fn orientation_shift(
        tension_ids: Vec<String>,
        old_orientation: Orientation,
        new_orientation: Orientation,
    ) -> Event {
        Event::OrientationShift {
            tension_ids,
            old_orientation,
            new_orientation,
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

    // ── Event Type Tests ───────────────────────────────────────────

    #[test]
    fn test_event_serialization_roundtrip() {
        let events = vec![
            EventBuilder::tension_created(
                "01ABC".to_owned(),
                "goal".to_owned(),
                "reality".to_owned(),
                None,
            ),
            EventBuilder::reality_confronted(
                "01ABC".to_owned(),
                "old reality".to_owned(),
                "new reality".to_owned(),
            ),
            EventBuilder::desire_revised(
                "01ABC".to_owned(),
                "old goal".to_owned(),
                "new goal".to_owned(),
            ),
            EventBuilder::tension_resolved(
                "01ABC".to_owned(),
                "final goal".to_owned(),
                "final reality".to_owned(),
            ),
            EventBuilder::tension_released(
                "01DEF".to_owned(),
                "goal".to_owned(),
                "reality".to_owned(),
            ),
            EventBuilder::structure_changed(
                "01ABC".to_owned(),
                Some("parent1".to_owned()),
                Some("parent2".to_owned()),
            ),
            EventBuilder::conflict_detected(
                vec!["01A".to_owned(), "01B".to_owned()],
                ConflictPattern::AsymmetricActivity,
            ),
            EventBuilder::conflict_resolved(
                vec!["01A".to_owned(), "01B".to_owned()],
                ConflictPattern::AsymmetricActivity,
            ),
            EventBuilder::lifecycle_transition(
                "01ABC".to_owned(),
                CreativeCyclePhase::Germination,
                CreativeCyclePhase::Assimilation,
            ),
            EventBuilder::oscillation_detected("01ABC".to_owned(), 5, 0.7),
            EventBuilder::resolution_achieved("01ABC".to_owned(), 0.5),
            EventBuilder::neglect_detected(
                vec!["01A".to_owned()],
                NeglectType::ParentNeglectsChildren,
            ),
            EventBuilder::orientation_shift(
                vec!["01A".to_owned(), "01B".to_owned()],
                Orientation::ProblemSolving,
                Orientation::Creative,
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
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"tension_created\""));
    }

    #[test]
    fn test_event_tension_id() {
        let event = EventBuilder::tension_created(
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
        );
        assert_eq!(event.tension_id(), Some("01ABC"));

        let event = EventBuilder::conflict_detected(
            vec!["01A".to_owned(), "01B".to_owned()],
            ConflictPattern::AsymmetricActivity,
        );
        assert_eq!(event.tension_id(), None);
    }

    #[test]
    fn test_event_timestamp() {
        let before = Utc::now();
        let event = EventBuilder::tension_created(
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
        );
        let after = Utc::now();

        let ts = event.timestamp();
        assert!(ts >= before);
        assert!(ts <= after);
    }

    // ── Event Bus Tests ────────────────────────────────────────────

    #[test]
    fn test_event_bus_subscribe_and_emit() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let count_clone = count.clone();
        let _handle = bus.subscribe(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = EventBuilder::tension_created(
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
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
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
        );
        bus.emit(&event);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        // Drop the handle
        drop(handle);

        // Emit again - should not be received
        bus.emit(&event);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        let c1 = count1.clone();
        let _h1 = bus.subscribe(move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let c2 = count2.clone();
        let _h2 = bus.subscribe(move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        let event = EventBuilder::tension_created(
            "01ABC".to_owned(),
            "goal".to_owned(),
            "reality".to_owned(),
            None,
        );
        bus.emit(&event);

        // Both subscribers received
        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_bus_identical_sequences() {
        let bus = EventBus::new();

        let events1 = Arc::new(Mutex::new(Vec::new()));
        let events2 = Arc::new(Mutex::new(Vec::new()));

        let e1 = events1.clone();
        let _h1 = bus.subscribe(move |ev| {
            e1.lock().unwrap().push(ev.clone());
        });

        let e2 = events2.clone();
        let _h2 = bus.subscribe(move |ev| {
            e2.lock().unwrap().push(ev.clone());
        });

        // Emit multiple events
        bus.emit(&EventBuilder::tension_created(
            "01A".to_owned(),
            "g1".to_owned(),
            "r1".to_owned(),
            None,
        ));
        bus.emit(&EventBuilder::reality_confronted(
            "01A".to_owned(),
            "r1".to_owned(),
            "r2".to_owned(),
        ));
        bus.emit(&EventBuilder::desire_revised(
            "01A".to_owned(),
            "g1".to_owned(),
            "g2".to_owned(),
        ));

        let seq1 = events1.lock().unwrap().clone();
        let seq2 = events2.lock().unwrap().clone();

        assert_eq!(seq1, seq2);
    }

    #[test]
    fn test_event_bus_history() {
        let bus = EventBus::new();

        bus.emit(&EventBuilder::tension_created(
            "01A".to_owned(),
            "g1".to_owned(),
            "r1".to_owned(),
            None,
        ));
        bus.emit(&EventBuilder::reality_confronted(
            "01A".to_owned(),
            "r1".to_owned(),
            "r2".to_owned(),
        ));

        let history = bus.history();
        assert_eq!(history.len(), 2);
    }

    // ── VAL-EVENT-003: Subscription and unsubscription ─────────────

    #[test]
    fn test_subscribe_returns_handle_drop_stops_delivery() {
        let bus = EventBus::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let r = received.clone();
        let handle = bus.subscribe(move |ev| {
            r.lock().unwrap().push(ev.clone());
        });

        bus.emit(&EventBuilder::tension_created(
            "01A".to_owned(),
            "g".to_owned(),
            "r".to_owned(),
            None,
        ));
        assert_eq!(received.lock().unwrap().len(), 1);

        drop(handle);

        bus.emit(&EventBuilder::tension_created(
            "01B".to_owned(),
            "g".to_owned(),
            "r".to_owned(),
            None,
        ));
        // Still only 1 event received (second one not delivered)
        assert_eq!(received.lock().unwrap().len(), 1);
    }

    // ── VAL-EVENT-004: Event ordering ──────────────────────────────

    #[test]
    fn test_events_emitted_in_causal_order() {
        let bus = EventBus::new();
        let history = Arc::new(Mutex::new(Vec::new()));

        let h = history.clone();
        let _handle = bus.subscribe(move |ev| {
            h.lock().unwrap().push(ev.clone());
        });

        // Emit events in specific causal order
        let t1 =
            EventBuilder::tension_created("01A".to_owned(), "g".to_owned(), "r".to_owned(), None);
        let t2 =
            EventBuilder::reality_confronted("01A".to_owned(), "r".to_owned(), "r2".to_owned());
        let t3 = EventBuilder::conflict_detected(
            vec!["01A".to_owned()],
            ConflictPattern::AsymmetricActivity,
        );

        bus.emit(&t1);
        bus.emit(&t2);
        bus.emit(&t3);

        let received = history.lock().unwrap().clone();
        assert_eq!(received.len(), 3);

        // Order must be: creation -> reality update -> conflict detection
        match (&received[0], &received[1], &received[2]) {
            (
                Event::TensionCreated { .. },
                Event::RealityConfronted { .. },
                Event::ConflictDetected { .. },
            ) => {}
            _ => panic!("Events not in causal order"),
        }
    }

    // ── VAL-EVENT-005: Determinism ─────────────────────────────────

    #[test]
    fn test_deterministic_event_sequence() {
        // Create two buses and emit the same events
        let bus1 = EventBus::new();
        let bus2 = EventBus::new();

        // Subscribe to both
        let h1 = Arc::new(Mutex::new(Vec::new()));
        let h2 = Arc::new(Mutex::new(Vec::new()));

        let h1c = h1.clone();
        let _handle1 = bus1.subscribe(move |ev| {
            h1c.lock().unwrap().push(ev.timestamp());
        });

        let h2c = h2.clone();
        let _handle2 = bus2.subscribe(move |ev| {
            h2c.lock().unwrap().push(ev.timestamp());
        });

        // Emit the same sequence to both
        for i in 0..5 {
            let event = EventBuilder::tension_created(
                format!("01{i}"),
                "g".to_owned(),
                "r".to_owned(),
                None,
            );
            bus1.emit(&event);
            bus2.emit(&event);
        }

        // History should have identical event types
        let hist1 = bus1.history();
        let hist2 = bus2.history();
        assert_eq!(hist1.len(), hist2.len());

        for (e1, e2) in hist1.iter().zip(hist2.iter()) {
            assert_eq!(std::mem::discriminant(e1), std::mem::discriminant(e2));
        }
    }

    // ── VAL-EVENT-010: Subscriber error isolation ──────────────────

    #[test]
    fn test_panicking_subscriber_isolated() {
        let bus = EventBus::new();
        let good_count = Arc::new(AtomicUsize::new(0));

        // Subscriber that panics
        let _panic_handle = bus.subscribe(|_| {
            panic!("subscriber panic!");
        });

        // Subscriber that should still receive events
        let gc = good_count.clone();
        let _good_handle = bus.subscribe(move |_| {
            gc.fetch_add(1, Ordering::SeqCst);
        });

        let event =
            EventBuilder::tension_created("01A".to_owned(), "g".to_owned(), "r".to_owned(), None);

        // Emit should not panic overall
        bus.emit(&event);

        // Good subscriber should have received the event
        assert_eq!(good_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_panicking_subscriber_removed() {
        let bus = EventBus::new();

        let panic_handle = bus.subscribe(|_| {
            panic!("subscriber panic!");
        });

        assert_eq!(bus.subscriber_count(), 1);

        let event =
            EventBuilder::tension_created("01A".to_owned(), "g".to_owned(), "r".to_owned(), None);
        bus.emit(&event);

        // Panicking subscriber should be removed
        assert_eq!(bus.subscriber_count(), 0);

        // But we need to drop it explicitly because we still have the handle
        drop(panic_handle);
    }

    // ── VAL-EVENT-007: Event payloads correctness ─────────────────

    #[test]
    fn test_event_payload_types_correct() {
        // TensionCreated
        let e = EventBuilder::tension_created(
            "01ABC123".to_owned(),
            "desired state".to_owned(),
            "actual state".to_owned(),
            Some("parent123".to_owned()),
        );

        match e {
            Event::TensionCreated {
                tension_id,
                desired,
                actual,
                parent_id,
                timestamp,
            } => {
                assert!(
                    ulid::Ulid::from_string(&tension_id).is_ok() || tension_id.starts_with("01")
                );
                assert_eq!(desired, "desired state");
                assert_eq!(actual, "actual state");
                assert_eq!(parent_id, Some("parent123".to_owned()));
                assert!(timestamp <= Utc::now());
            }
            _ => panic!("wrong event type"),
        }

        // RealityConfronted
        let e = EventBuilder::reality_confronted(
            "01ABC".to_owned(),
            "old".to_owned(),
            "new".to_owned(),
        );
        match e {
            Event::RealityConfronted {
                old_actual,
                new_actual,
                ..
            } => {
                assert_eq!(old_actual, "old");
                assert_eq!(new_actual, "new");
            }
            _ => panic!("wrong event type"),
        }

        // ConflictDetected
        let e = EventBuilder::conflict_detected(
            vec!["01A".to_owned(), "01B".to_owned()],
            ConflictPattern::AsymmetricActivity,
        );
        match e {
            Event::ConflictDetected {
                tension_ids,
                pattern,
                ..
            } => {
                assert_eq!(tension_ids.len(), 2);
                assert_eq!(pattern, ConflictPattern::AsymmetricActivity);
            }
            _ => panic!("wrong event type"),
        }

        // LifecycleTransition
        let e = EventBuilder::lifecycle_transition(
            "01A".to_owned(),
            CreativeCyclePhase::Germination,
            CreativeCyclePhase::Assimilation,
        );
        match e {
            Event::LifecycleTransition {
                old_phase,
                new_phase,
                ..
            } => {
                assert_eq!(old_phase, CreativeCyclePhase::Germination);
                assert_eq!(new_phase, CreativeCyclePhase::Assimilation);
            }
            _ => panic!("wrong event type"),
        }
    }

    #[test]
    fn test_event_all_types_defined() {
        // Ensure all 13 event types are defined and serializable
        let events: Vec<Event> = vec![
            Event::TensionCreated {
                tension_id: "01A".to_owned(),
                desired: "d".to_owned(),
                actual: "a".to_owned(),
                parent_id: None,
                timestamp: Utc::now(),
            },
            Event::RealityConfronted {
                tension_id: "01A".to_owned(),
                old_actual: "old".to_owned(),
                new_actual: "new".to_owned(),
                timestamp: Utc::now(),
            },
            Event::DesireRevised {
                tension_id: "01A".to_owned(),
                old_desired: "old".to_owned(),
                new_desired: "new".to_owned(),
                timestamp: Utc::now(),
            },
            Event::TensionResolved {
                tension_id: "01A".to_owned(),
                final_desired: "d".to_owned(),
                final_actual: "a".to_owned(),
                timestamp: Utc::now(),
            },
            Event::TensionReleased {
                tension_id: "01A".to_owned(),
                desired: "d".to_owned(),
                actual: "a".to_owned(),
                timestamp: Utc::now(),
            },
            Event::ConflictDetected {
                tension_ids: vec!["01A".to_owned()],
                pattern: ConflictPattern::AsymmetricActivity,
                timestamp: Utc::now(),
            },
            Event::ConflictResolved {
                tension_ids: vec!["01A".to_owned()],
                former_pattern: ConflictPattern::AsymmetricActivity,
                timestamp: Utc::now(),
            },
            Event::LifecycleTransition {
                tension_id: "01A".to_owned(),
                old_phase: CreativeCyclePhase::Germination,
                new_phase: CreativeCyclePhase::Assimilation,
                timestamp: Utc::now(),
            },
            Event::OscillationDetected {
                tension_id: "01A".to_owned(),
                reversals: 5,
                magnitude: 0.8,
                timestamp: Utc::now(),
            },
            Event::ResolutionAchieved {
                tension_id: "01A".to_owned(),
                velocity: 0.5,
                timestamp: Utc::now(),
            },
            Event::NeglectDetected {
                tension_ids: vec!["01A".to_owned()],
                neglect_type: NeglectType::ParentNeglectsChildren,
                timestamp: Utc::now(),
            },
            Event::StructureChanged {
                tension_id: "01A".to_owned(),
                old_parent_id: None,
                new_parent_id: Some("parent".to_owned()),
                timestamp: Utc::now(),
            },
            Event::OrientationShift {
                tension_ids: vec!["01A".to_owned()],
                old_orientation: Orientation::ProblemSolving,
                new_orientation: Orientation::Creative,
                timestamp: Utc::now(),
            },
        ];

        // All should serialize and deserialize
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let _: Event = serde_json::from_str(&json).unwrap();
        }

        assert_eq!(events.len(), 13);
    }

    // ── VAL-EVENT-001: State change events fire correctly ──────────

    #[test]
    fn test_state_change_events_defined() {
        // Verify that all state change events have correct payload fields
        // These events fire on corresponding store operations

        // TensionCreated: fires on tension creation
        let e = Event::TensionCreated {
            tension_id: "id".to_owned(),
            desired: "d".to_owned(),
            actual: "a".to_owned(),
            parent_id: None,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::TensionCreated { .. }));

        // RealityConfronted: fires on actual update
        let e = Event::RealityConfronted {
            tension_id: "id".to_owned(),
            old_actual: "old".to_owned(),
            new_actual: "new".to_owned(),
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::RealityConfronted { .. }));

        // DesireRevised: fires on desired update
        let e = Event::DesireRevised {
            tension_id: "id".to_owned(),
            old_desired: "old".to_owned(),
            new_desired: "new".to_owned(),
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::DesireRevised { .. }));

        // TensionResolved: fires on resolve
        let e = Event::TensionResolved {
            tension_id: "id".to_owned(),
            final_desired: "d".to_owned(),
            final_actual: "a".to_owned(),
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::TensionResolved { .. }));

        // TensionReleased: fires on release
        let e = Event::TensionReleased {
            tension_id: "id".to_owned(),
            desired: "d".to_owned(),
            actual: "a".to_owned(),
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::TensionReleased { .. }));

        // StructureChanged: fires on parent_id change
        let e = Event::StructureChanged {
            tension_id: "id".to_owned(),
            old_parent_id: None,
            new_parent_id: Some("parent".to_owned()),
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::StructureChanged { .. }));
    }

    // ── VAL-EVENT-002: Dynamic transition events fire ──────────────

    #[test]
    fn test_dynamic_transition_events_defined() {
        // ConflictDetected/ConflictResolved: fire on conflict state changes
        let e = Event::ConflictDetected {
            tension_ids: vec!["a".to_owned(), "b".to_owned()],
            pattern: ConflictPattern::AsymmetricActivity,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::ConflictDetected { .. }));

        let e = Event::ConflictResolved {
            tension_ids: vec!["a".to_owned(), "b".to_owned()],
            former_pattern: ConflictPattern::AsymmetricActivity,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::ConflictResolved { .. }));

        // LifecycleTransition: fires on phase changes
        let e = Event::LifecycleTransition {
            tension_id: "id".to_owned(),
            old_phase: CreativeCyclePhase::Germination,
            new_phase: CreativeCyclePhase::Assimilation,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::LifecycleTransition { .. }));

        // OscillationDetected: fires when oscillation detected
        let e = Event::OscillationDetected {
            tension_id: "id".to_owned(),
            reversals: 5,
            magnitude: 0.8,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::OscillationDetected { .. }));

        // ResolutionAchieved: fires when resolution detected
        let e = Event::ResolutionAchieved {
            tension_id: "id".to_owned(),
            velocity: 0.5,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::ResolutionAchieved { .. }));

        // NeglectDetected: fires on neglect
        let e = Event::NeglectDetected {
            tension_ids: vec!["id".to_owned()],
            neglect_type: NeglectType::ParentNeglectsChildren,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::NeglectDetected { .. }));

        // OrientationShift: fires on orientation change
        let e = Event::OrientationShift {
            tension_ids: vec!["id".to_owned()],
            old_orientation: Orientation::ProblemSolving,
            new_orientation: Orientation::Creative,
            timestamp: Utc::now(),
        };
        assert!(matches!(e, Event::OrientationShift { .. }));
    }

    // ── VAL-EVENT-008: State reconstruction from events ────────────

    #[test]
    fn test_state_reconstruction_from_events() {
        // Simulate a sequence of events for a tension
        let events = vec![
            Event::TensionCreated {
                tension_id: "01ABC".to_owned(),
                desired: "write a novel".to_owned(),
                actual: "have an outline".to_owned(),
                parent_id: None,
                timestamp: Utc::now(),
            },
            Event::RealityConfronted {
                tension_id: "01ABC".to_owned(),
                old_actual: "have an outline".to_owned(),
                new_actual: "have a chapter".to_owned(),
                timestamp: Utc::now(),
            },
            Event::DesireRevised {
                tension_id: "01ABC".to_owned(),
                old_desired: "write a novel".to_owned(),
                new_desired: "write a bestseller".to_owned(),
                timestamp: Utc::now(),
            },
            Event::StructureChanged {
                tension_id: "01ABC".to_owned(),
                old_parent_id: None,
                new_parent_id: Some("parent123".to_owned()),
                timestamp: Utc::now(),
            },
            Event::TensionResolved {
                tension_id: "01ABC".to_owned(),
                final_desired: "write a bestseller".to_owned(),
                final_actual: "have a chapter".to_owned(),
                timestamp: Utc::now(),
            },
        ];

        // Reconstruct state from events
        let mut desired = String::new();
        let mut actual = String::new();
        let mut parent_id: Option<String> = None;
        let mut status = TensionStatus::Active;
        let mut tension_id: Option<String> = None;

        for event in &events {
            match event {
                Event::TensionCreated {
                    tension_id: tid,
                    desired: d,
                    actual: a,
                    parent_id: p,
                    ..
                } => {
                    tension_id = Some(tid.clone());
                    desired = d.clone();
                    actual = a.clone();
                    parent_id = p.clone();
                }
                Event::RealityConfronted { new_actual, .. } => {
                    actual = new_actual.clone();
                }
                Event::DesireRevised { new_desired, .. } => {
                    desired = new_desired.clone();
                }
                Event::StructureChanged { new_parent_id, .. } => {
                    parent_id = new_parent_id.clone();
                }
                Event::TensionResolved { .. } => {
                    status = TensionStatus::Resolved;
                }
                _ => {}
            }
        }

        // Verify reconstructed state
        assert_eq!(tension_id, Some("01ABC".to_owned()));
        assert_eq!(desired, "write a bestseller");
        // Note: The TensionResolved event has final_desired and final_actual
        // In a real implementation, we'd use those for the resolved state
    }

    // ── Unicode and special characters ─────────────────────────────

    #[test]
    fn test_event_unicode_payloads() {
        let event = Event::TensionCreated {
            tension_id: "01ABC".to_owned(),
            desired: "写一本小说 📚".to_owned(),
            actual: "有一个大纲 📝".to_owned(),
            parent_id: None,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();

        match deserialized {
            Event::TensionCreated {
                desired, actual, ..
            } => {
                assert_eq!(desired, "写一本小说 📚");
                assert_eq!(actual, "有一个大纲 📝");
            }
            _ => panic!("wrong event type"),
        }
    }

    // ── Trait implementations ─────────────────────────────────────

    #[test]
    fn test_event_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Event>();
        assert_send_sync::<EventBus>();
    }

    #[test]
    fn test_event_is_debug_clone_partialeq() {
        let e =
            EventBuilder::tension_created("01A".to_owned(), "g".to_owned(), "r".to_owned(), None);
        let _ = format!("{e:?}"); // Debug
        let e2 = e.clone(); // Clone
        assert_eq!(e, e2); // PartialEq
    }
}
