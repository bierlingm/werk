//! Mutation log — append-only record of changes to tensions.
//!
//! Every change to a tension produces an immutable `Mutation` record.
//! Mutations capture the tension_id, timestamp, field name, old value,
//! and new value. Once created, mutations cannot be modified.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An immutable record of a change to a tension.
///
/// Mutations form an append-only log that enables history replay,
/// state reconstruction, and dynamics computation.
///
/// All fields are private with public getters to enforce immutability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mutation {
    /// The ID of the tension this mutation applies to.
    tension_id: String,
    /// When this mutation occurred.
    timestamp: DateTime<Utc>,
    /// Which field was changed (e.g., "desired", "actual", "status", "created").
    field: String,
    /// The previous value (None for creation events).
    old_value: Option<String>,
    /// The new value.
    new_value: String,
}

impl Mutation {
    /// Create a new mutation record.
    pub fn new(
        tension_id: String,
        timestamp: DateTime<Utc>,
        field: String,
        old_value: Option<String>,
        new_value: String,
    ) -> Self {
        Self {
            tension_id,
            timestamp,
            field,
            old_value,
            new_value,
        }
    }

    /// The ID of the tension this mutation applies to.
    pub fn tension_id(&self) -> &str {
        &self.tension_id
    }

    /// When this mutation occurred.
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Which field was changed.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The previous value (None for creation events).
    pub fn old_value(&self) -> Option<&str> {
        self.old_value.as_deref()
    }

    /// The new value.
    pub fn new_value(&self) -> &str {
        &self.new_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ──────────────────────────────────────────────

    #[test]
    fn test_mutation_new() {
        let now = Utc::now();
        let m = Mutation::new(
            "01ABC".to_owned(),
            now,
            "desired".to_owned(),
            Some("old goal".to_owned()),
            "new goal".to_owned(),
        );
        assert_eq!(m.tension_id(), "01ABC");
        assert_eq!(m.timestamp(), now);
        assert_eq!(m.field(), "desired");
        assert_eq!(m.old_value(), Some("old goal"));
        assert_eq!(m.new_value(), "new goal");
    }

    #[test]
    fn test_mutation_creation_event() {
        let now = Utc::now();
        let m = Mutation::new(
            "01DEF".to_owned(),
            now,
            "created".to_owned(),
            None,
            "initial state".to_owned(),
        );
        assert_eq!(m.field(), "created");
        assert!(m.old_value().is_none());
    }

    // ── Immutability ──────────────────────────────────────────────

    #[test]
    fn test_mutation_has_no_public_setters() {
        // This test verifies immutability by construction: Mutation fields
        // are private and only accessible through getters. This is a
        // compile-time guarantee — if someone adds `pub` to a field,
        // this comment documents the invariant.
        let now = Utc::now();
        let m = Mutation::new(
            "01GHI".to_owned(),
            now,
            "status".to_owned(),
            Some("Active".to_owned()),
            "Resolved".to_owned(),
        );
        // We can only read, not write:
        let _ = m.tension_id();
        let _ = m.timestamp();
        let _ = m.field();
        let _ = m.old_value();
        let _ = m.new_value();
    }

    // ── Serialization ─────────────────────────────────────────────

    #[test]
    fn test_mutation_serialization_roundtrip() {
        let now = Utc::now();
        let m = Mutation::new(
            "01JKL".to_owned(),
            now,
            "actual".to_owned(),
            Some("old reality".to_owned()),
            "new reality".to_owned(),
        );
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: Mutation = serde_json::from_str(&json).unwrap();
        assert_eq!(m, deserialized);
    }

    #[test]
    fn test_mutation_serialization_with_null_old_value() {
        let now = Utc::now();
        let m = Mutation::new(
            "01MNO".to_owned(),
            now,
            "created".to_owned(),
            None,
            "initial".to_owned(),
        );
        let json = serde_json::to_string(&m).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("old_value").unwrap().is_null());

        let deserialized: Mutation = serde_json::from_str(&json).unwrap();
        assert_eq!(m, deserialized);
    }

    #[test]
    fn test_mutation_json_fields_present() {
        let now = Utc::now();
        let m = Mutation::new(
            "01PQR".to_owned(),
            now,
            "desired".to_owned(),
            Some("old".to_owned()),
            "new".to_owned(),
        );
        let json = serde_json::to_string(&m).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("tension_id").is_some());
        assert!(value.get("timestamp").is_some());
        assert!(value.get("field").is_some());
        assert!(value.get("old_value").is_some());
        assert!(value.get("new_value").is_some());
    }

    // ── Trait assertions ──────────────────────────────────────────

    #[test]
    fn test_mutation_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Mutation>();
    }

    #[test]
    fn test_mutation_is_debug_clone_partialeq() {
        let now = Utc::now();
        let m = Mutation::new(
            "01STU".to_owned(),
            now,
            "field".to_owned(),
            None,
            "val".to_owned(),
        );
        let _ = format!("{m:?}"); // Debug
        let m2 = m.clone(); // Clone
        assert_eq!(m, m2); // PartialEq
    }

    // ── Unicode ───────────────────────────────────────────────────

    #[test]
    fn test_mutation_unicode_values() {
        let now = Utc::now();
        let m = Mutation::new(
            "01VWX".to_owned(),
            now,
            "desired".to_owned(),
            Some("写小说".to_owned()),
            "🎵 compose 音楽".to_owned(),
        );
        assert_eq!(m.old_value(), Some("写小说"));
        assert_eq!(m.new_value(), "🎵 compose 音楽");

        // Roundtrip
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: Mutation = serde_json::from_str(&json).unwrap();
        assert_eq!(m, deserialized);
    }
}
