//! Mutation log — append-only record of changes to tensions.
//!
//! Every change to a tension produces an immutable `Mutation` record.
//! Mutations capture the tension_id, timestamp, field name, old value,
//! and new value. Once created, mutations cannot be modified.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::horizon::Horizon;
use crate::tension::{Tension, TensionStatus};

/// The kind of a mutation field, strongly typed.
///
/// The wire format (SQLite `mutations.field` TEXT column, JSON `"field":"..."`)
/// is preserved — this enum is a parsed *view* over the underlying string
/// obtained via `Mutation::field_kind()`. Readers that used to compare strings
/// like `m.field() == "status"` can use `matches!(m.field_kind(), MutationFieldKind::Status)`
/// for a compile-time-checked comparison.
///
/// The `Other(String)` variant preserves forward compatibility with new field
/// kinds that might appear in stored mutations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MutationFieldKind {
    /// The initial creation event for a tension.
    Created,
    /// A change to `Tension::desired`.
    Desired,
    /// A change to `Tension::actual`.
    Actual,
    /// A change to `Tension::parent_id`. Empty string new_value means cleared.
    ParentId,
    /// A lifecycle status transition (Active ↔ Resolved ↔ Released).
    Status,
    /// A change to `Tension::horizon`. Empty string new_value means cleared.
    Horizon,
    /// A change to `Tension::position`.
    Position,
    /// A note was appended. Narrative, doesn't mutate tension state.
    Note,
    /// A note was retracted. Narrative, doesn't mutate tension state.
    NoteRetracted,
    /// A soft-delete marker.
    Deleted,
    /// A split gesture produced this tension (or from this tension).
    Split,
    /// A merge gesture absorbed/produced this tension.
    Merge,
    /// A release reason was recorded.
    ReleaseReason,
    /// A reopen reason was recorded.
    ReopenReason,
    /// A recurrence pattern was set.
    Recurrence,
    /// A recurrence pattern was cleared.
    RecurrenceCleared,
    /// A snooze deadline was set.
    SnoozedUntil,
    /// A snooze was cleared.
    SnoozeCleared,
    /// A generic "cleared" marker.
    Cleared,
    /// A literal "none" field name (used in some edge/no-op mutations).
    None,
    /// An unrecognized field kind. Carries the raw string so the wire format
    /// roundtrips even when the stored value predates or post-dates this enum.
    Other(String),
}

impl MutationFieldKind {
    /// The canonical string representation, identical to the stored value.
    pub fn as_str(&self) -> &str {
        match self {
            MutationFieldKind::Created => "created",
            MutationFieldKind::Desired => "desired",
            MutationFieldKind::Actual => "actual",
            MutationFieldKind::ParentId => "parent_id",
            MutationFieldKind::Status => "status",
            MutationFieldKind::Horizon => "horizon",
            MutationFieldKind::Position => "position",
            MutationFieldKind::Note => "note",
            MutationFieldKind::NoteRetracted => "note_retracted",
            MutationFieldKind::Deleted => "deleted",
            MutationFieldKind::Split => "split",
            MutationFieldKind::Merge => "merge",
            MutationFieldKind::ReleaseReason => "release_reason",
            MutationFieldKind::ReopenReason => "reopen_reason",
            MutationFieldKind::Recurrence => "recurrence",
            MutationFieldKind::RecurrenceCleared => "recurrence_cleared",
            MutationFieldKind::SnoozedUntil => "snoozed_until",
            MutationFieldKind::SnoozeCleared => "snooze_cleared",
            MutationFieldKind::Cleared => "cleared",
            MutationFieldKind::None => "none",
            MutationFieldKind::Other(s) => s.as_str(),
        }
    }

    /// Parse a raw field string into its typed kind.
    ///
    /// Unknown strings are preserved in `Other` — never fails.
    pub fn from_raw(field: &str) -> Self {
        match field {
            "created" => MutationFieldKind::Created,
            "desired" => MutationFieldKind::Desired,
            "actual" => MutationFieldKind::Actual,
            "parent_id" => MutationFieldKind::ParentId,
            "status" => MutationFieldKind::Status,
            "horizon" => MutationFieldKind::Horizon,
            "position" => MutationFieldKind::Position,
            "note" => MutationFieldKind::Note,
            "note_retracted" => MutationFieldKind::NoteRetracted,
            "deleted" => MutationFieldKind::Deleted,
            "split" => MutationFieldKind::Split,
            "merge" => MutationFieldKind::Merge,
            "release_reason" => MutationFieldKind::ReleaseReason,
            "reopen_reason" => MutationFieldKind::ReopenReason,
            "recurrence" => MutationFieldKind::Recurrence,
            "recurrence_cleared" => MutationFieldKind::RecurrenceCleared,
            "snoozed_until" => MutationFieldKind::SnoozedUntil,
            "snooze_cleared" => MutationFieldKind::SnoozeCleared,
            "cleared" => MutationFieldKind::Cleared,
            "none" => MutationFieldKind::None,
            other => MutationFieldKind::Other(other.to_owned()),
        }
    }
}

impl std::fmt::Display for MutationFieldKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Reconstructed tension state from mutation replay.
///
/// This struct contains the tension field values that can be reconstructed
/// from mutation history. Note that `id` and `created_at` are taken from
/// the initial creation mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconstructedTension {
    /// Unique identifier (ULID).
    pub id: String,
    /// The desired state.
    pub desired: String,
    /// The actual state.
    pub actual: String,
    /// Optional parent tension ID.
    pub parent_id: Option<String>,
    /// When this tension was created.
    pub created_at: DateTime<Utc>,
    /// Current lifecycle status.
    pub status: TensionStatus,
    /// Optional temporal horizon.
    pub horizon: Option<Horizon>,
}

impl ReconstructedTension {
    /// Convert to a Tension struct.
    pub fn to_tension(&self) -> Tension {
        Tension {
            id: self.id.clone(),
            desired: self.desired.clone(),
            actual: self.actual.clone(),
            parent_id: self.parent_id.clone(),
            created_at: self.created_at,
            status: self.status,
            horizon: self.horizon.clone(),
            position: None,
            parent_desired_snapshot: None,
            parent_actual_snapshot: None,
            parent_snapshot_json: None,
            short_code: None,
        }
    }
}

/// Replay a sequence of mutations to reconstruct the tension state.
///
/// Given mutations ordered chronologically (oldest first), this function
/// reconstructs the final tension field values. The first mutation must
/// be a "created" mutation containing the initial state.
///
/// # Arguments
///
/// * `mutations` - Chronologically ordered mutations for a single tension
///
/// # Returns
///
/// The reconstructed tension state, or an error if mutations are invalid
/// or empty.
///
/// # Example
///
/// ```
/// # use werk_core::mutation::{Mutation, replay_mutations};
/// # use werk_core::store::Store;
/// let store = Store::new_in_memory().unwrap();
/// let t = store.create_tension("goal", "reality").unwrap();
/// store.update_desired(&t.id, "new goal").unwrap();
///
/// let mutations = store.get_mutations(&t.id).unwrap();
/// let reconstructed = replay_mutations(&mutations).unwrap();
/// assert_eq!(reconstructed.desired, "new goal");
/// ```
pub fn replay_mutations(mutations: &[Mutation]) -> Result<ReconstructedTension, ReplayError> {
    if mutations.is_empty() {
        return Err(ReplayError::EmptyMutations);
    }

    // First mutation must be "created"
    let first = &mutations[0];
    if first.field() != "created" {
        return Err(ReplayError::MissingCreation);
    }

    // Parse the initial state from the creation mutation's new_value
    // Format: "desired='...';actual='...'" or "desired='...';actual='...';horizon='...'"
    let initial_state = parse_creation_value(first.new_value())?;

    let mut reconstructed = ReconstructedTension {
        id: first.tension_id().to_owned(),
        desired: initial_state.desired,
        actual: initial_state.actual,
        parent_id: None, // Parent is set via separate mutation if needed
        created_at: first.timestamp(),
        status: TensionStatus::Active,
        horizon: initial_state.horizon,
    };

    // Replay subsequent mutations
    for mutation in &mutations[1..] {
        apply_mutation(&mut reconstructed, mutation)?;
    }

    Ok(reconstructed)
}

/// Parsed initial state from a creation mutation.
struct InitialState {
    desired: String,
    actual: String,
    horizon: Option<Horizon>,
}

/// Parse the creation mutation's new_value format.
///
/// Format: "desired='...';actual='...'" or "desired='...';actual='...';horizon='...'"
fn parse_creation_value(value: &str) -> Result<InitialState, ReplayError> {
    // We need to extract the values, handling potential edge cases

    let desired = extract_field_value(value, "desired")
        .ok_or_else(|| ReplayError::InvalidCreationFormat(value.to_owned()))?;
    let actual = extract_field_value(value, "actual")
        .ok_or_else(|| ReplayError::InvalidCreationFormat(value.to_owned()))?;

    // Horizon is optional in the creation format
    let horizon =
        match extract_field_value(value, "horizon") {
            Some(h) if !h.is_empty() => Some(Horizon::parse(&h).map_err(|_| {
                ReplayError::InvalidCreationFormat(format!("invalid horizon: {}", h))
            })?),
            _ => None,
        };

    Ok(InitialState {
        desired,
        actual,
        horizon,
    })
}

/// Extract a field value from the creation format.
fn extract_field_value(format: &str, field_name: &str) -> Option<String> {
    let prefix = format!("{}='", field_name);
    let start = format.find(&prefix)?;
    let value_start = start + prefix.len();

    // Find the closing quote
    let remaining = &format[value_start..];
    let end = remaining.find("'")?;
    Some(remaining[..end].to_owned())
}

/// Apply a single mutation to the reconstructed tension.
fn apply_mutation(
    tension: &mut ReconstructedTension,
    mutation: &Mutation,
) -> Result<(), ReplayError> {
    use MutationFieldKind as F;
    match mutation.field_kind() {
        F::Desired => {
            tension.desired = mutation.new_value().to_owned();
        }
        F::Actual => {
            tension.actual = mutation.new_value().to_owned();
        }
        F::ParentId => {
            // Empty string represents null
            tension.parent_id = if mutation.new_value().is_empty() {
                None
            } else {
                Some(mutation.new_value().to_owned())
            };
        }
        F::Status => {
            tension.status = match mutation.new_value() {
                "Active" => TensionStatus::Active,
                "Resolved" => TensionStatus::Resolved,
                "Released" => TensionStatus::Released,
                _ => return Err(ReplayError::InvalidStatus(mutation.new_value().to_owned())),
            };
        }
        F::Horizon => {
            // Empty string represents None; otherwise parse as Horizon
            tension.horizon =
                if mutation.new_value().is_empty() {
                    None
                } else {
                    Some(Horizon::parse(mutation.new_value()).map_err(|_| {
                        ReplayError::InvalidHorizon(mutation.new_value().to_owned())
                    })?)
                };
        }
        F::Created => {
            // Creation should only appear as the first mutation
            return Err(ReplayError::UnexpectedCreation);
        }
        // Narrative and structural-event mutations don't change reconstructed
        // tension field values. Explicitly listed here so the exhaustiveness
        // check catches newly introduced field kinds at compile time.
        F::Note
        | F::NoteRetracted
        | F::Position
        | F::Deleted
        | F::Split
        | F::Merge
        | F::ReleaseReason
        | F::ReopenReason
        | F::Recurrence
        | F::RecurrenceCleared
        | F::SnoozedUntil
        | F::SnoozeCleared
        | F::Cleared
        | F::None => {}
        F::Other(field) => {
            return Err(ReplayError::UnknownField(field));
        }
    }
    Ok(())
}

/// Errors that can occur during mutation replay.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ReplayError {
    /// No mutations were provided.
    #[error("cannot replay empty mutations")]
    EmptyMutations,

    /// The first mutation is not a creation mutation.
    #[error("expected creation mutation as first, got different field")]
    MissingCreation,

    /// The creation mutation format is invalid.
    #[error("invalid creation format: {0}")]
    InvalidCreationFormat(String),

    /// An invalid status value was encountered.
    #[error("invalid status value: {0}")]
    InvalidStatus(String),

    /// A creation mutation appeared in the middle of the sequence.
    #[error("unexpected creation mutation in middle of sequence")]
    UnexpectedCreation,

    /// An unknown field was encountered.
    #[error("unknown field: {0}")]
    UnknownField(String),

    /// An invalid horizon value was encountered.
    #[error("invalid horizon value: {0}")]
    InvalidHorizon(String),
}

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
    /// When this mutation occurred (reported resolution point).
    timestamp: DateTime<Utc>,
    /// Which field was changed (e.g., "desired", "actual", "status", "created", "note").
    field: String,
    /// The previous value (None for creation events).
    old_value: Option<String>,
    /// The new value.
    new_value: String,
    /// The gesture this mutation belongs to (unit of meaningful change).
    gesture_id: Option<String>,
    /// When this actually happened in reality (if different from timestamp).
    /// Supports "I did this yesterday" — the gap between actual_at and timestamp
    /// is engagement pattern data per the Calculus of Time.
    actual_at: Option<DateTime<Utc>>,
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
            gesture_id: None,
            actual_at: None,
        }
    }

    /// Create a new mutation record with gesture and actual_at.
    pub fn new_with_gesture(
        tension_id: String,
        timestamp: DateTime<Utc>,
        field: String,
        old_value: Option<String>,
        new_value: String,
        gesture_id: Option<String>,
        actual_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            tension_id,
            timestamp,
            field,
            old_value,
            new_value,
            gesture_id,
            actual_at,
        }
    }

    /// The ID of the tension this mutation applies to.
    pub fn tension_id(&self) -> &str {
        &self.tension_id
    }

    /// When this mutation occurred (reported resolution point).
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Which field was changed, as a raw string (wire format).
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Which field was changed, as a strongly-typed enum.
    ///
    /// Prefer this over string-matching on `field()`: the enum is exhaustive
    /// and unknown values are preserved in `MutationFieldKind::Other`.
    pub fn field_kind(&self) -> MutationFieldKind {
        MutationFieldKind::from_raw(&self.field)
    }

    /// The previous value (None for creation events).
    pub fn old_value(&self) -> Option<&str> {
        self.old_value.as_deref()
    }

    /// The new value.
    pub fn new_value(&self) -> &str {
        &self.new_value
    }

    /// The gesture this mutation belongs to.
    pub fn gesture_id(&self) -> Option<&str> {
        self.gesture_id.as_deref()
    }

    /// When this actually happened (if different from reported timestamp).
    pub fn actual_at(&self) -> Option<DateTime<Utc>> {
        self.actual_at
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

    // ── VAL-MUTATION-011: Mutation replay ─────────────────────────

    #[test]
    fn test_replay_empty_mutations_fails() {
        let result = super::replay_mutations(&[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            super::ReplayError::EmptyMutations => {}
            other => panic!("expected EmptyMutations, got {other:?}"), // ubs:ignore test assertion
        }
    }

    #[test]
    fn test_replay_missing_creation_fails() {
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "desired".to_owned(),
            Some("old".to_owned()),
            "new".to_owned(),
        )];
        let result = super::replay_mutations(&mutations);
        assert!(result.is_err());
        match result.unwrap_err() {
            super::ReplayError::MissingCreation => {}
            other => panic!("expected MissingCreation, got {other:?}"), // ubs:ignore test assertion
        }
    }

    #[test]
    fn test_replay_creation_only() {
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='my goal';actual='my reality'".to_owned(),
        )];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.id, "01ABC");
        assert_eq!(result.desired, "my goal");
        assert_eq!(result.actual, "my reality");
        assert!(result.parent_id.is_none());
        assert_eq!(result.status, TensionStatus::Active);
    }

    #[test]
    fn test_replay_with_desired_update() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='old goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "desired".to_owned(),
                Some("old goal".to_owned()),
                "new goal".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.desired, "new goal");
        assert_eq!(result.actual, "reality");
    }

    #[test]
    fn test_replay_with_actual_update() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='old reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "actual".to_owned(),
                Some("old reality".to_owned()),
                "new reality".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.desired, "goal");
        assert_eq!(result.actual, "new reality");
    }

    #[test]
    fn test_replay_with_parent_id_update() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "parent_id".to_owned(),
                None,
                "parent123".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.parent_id, Some("parent123".to_owned()));
    }

    #[test]
    fn test_replay_with_parent_id_set_to_null() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "parent_id".to_owned(),
                Some("parent123".to_owned()),
                "".to_owned(), // Empty means null
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert!(result.parent_id.is_none());
    }

    #[test]
    fn test_replay_with_status_update() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "status".to_owned(),
                Some("Active".to_owned()),
                "Resolved".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.status, TensionStatus::Resolved);
    }

    #[test]
    fn test_replay_multiple_updates() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='initial goal';actual='initial reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "desired".to_owned(),
                Some("initial goal".to_owned()),
                "second goal".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(2),
                "actual".to_owned(),
                Some("initial reality".to_owned()),
                "second reality".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(3),
                "desired".to_owned(),
                Some("second goal".to_owned()),
                "final goal".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.desired, "final goal");
        assert_eq!(result.actual, "second reality");
    }

    #[test]
    fn test_replay_invalid_status_fails() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "status".to_owned(),
                Some("Active".to_owned()),
                "InvalidStatus".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations);
        assert!(result.is_err());
        match result.unwrap_err() {
            super::ReplayError::InvalidStatus(s) => assert_eq!(s, "InvalidStatus"),
            other => panic!("expected InvalidStatus, got {other:?}"), // ubs:ignore test assertion
        }
    }

    #[test]
    fn test_replay_invalid_creation_format_fails() {
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "invalid format".to_owned(),
        )];
        let result = super::replay_mutations(&mutations);
        assert!(result.is_err());
    }

    // ── MutationFieldKind ─────────────────────────────────────────

    #[test]
    fn test_mutation_field_kind_roundtrip() {
        let cases = [
            "created",
            "desired",
            "actual",
            "parent_id",
            "status",
            "horizon",
            "position",
            "note",
            "note_retracted",
            "deleted",
            "split",
            "merge",
            "release_reason",
            "reopen_reason",
            "recurrence",
            "recurrence_cleared",
            "snoozed_until",
            "snooze_cleared",
            "cleared",
            "none",
        ];
        for case in cases {
            let kind = MutationFieldKind::from_raw(case);
            assert_eq!(kind.as_str(), case, "roundtrip failed for {}", case);
            assert!(
                !matches!(kind, MutationFieldKind::Other(_)),
                "expected known variant for {}",
                case
            );
        }
    }

    #[test]
    fn test_mutation_field_kind_other_preserves_unknown() {
        let kind = MutationFieldKind::from_raw("some_future_field");
        assert_eq!(kind, MutationFieldKind::Other("some_future_field".to_owned()));
        assert_eq!(kind.as_str(), "some_future_field");
    }

    #[test]
    fn test_mutation_field_kind_wire_format_unchanged() {
        let now = Utc::now();
        let m = Mutation::new(
            "01ABC".to_owned(),
            now,
            "status".to_owned(),
            Some("Active".to_owned()),
            "Resolved".to_owned(),
        );
        // field() returns the raw string (unchanged).
        assert_eq!(m.field(), "status");
        // field_kind() returns the strongly typed enum.
        assert_eq!(m.field_kind(), MutationFieldKind::Status);
        // JSON wire format preserves "field":"status".
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"field\":\"status\""));
    }

    #[test]
    fn test_replay_noop_fields_do_not_error() {
        // Fields that are valid mutations but don't change reconstructed state
        // must not trigger ReplayError::UnknownField. This was a bug prior to
        // the MutationFieldKind enum — "split", "merge", "release_reason",
        // "reopen_reason", "recurrence", "recurrence_cleared", "snoozed_until",
        // "snooze_cleared", "cleared", "none" were all rejected.
        let now = Utc::now();
        let noop_fields = [
            "split",
            "merge",
            "release_reason",
            "reopen_reason",
            "recurrence",
            "recurrence_cleared",
            "snoozed_until",
            "snooze_cleared",
            "cleared",
            "none",
        ];
        for field in noop_fields {
            let mutations = vec![
                Mutation::new(
                    "01ABC".to_owned(),
                    now,
                    "created".to_owned(),
                    None,
                    "desired='g';actual='r'".to_owned(),
                ),
                Mutation::new(
                    "01ABC".to_owned(),
                    now + chrono::Duration::seconds(1),
                    field.to_owned(),
                    None,
                    "some value".to_owned(),
                ),
            ];
            let result = super::replay_mutations(&mutations);
            assert!(result.is_ok(), "noop field {} should not error, got {:?}", field, result.err());
        }
    }

    #[test]
    fn test_replay_unknown_field_fails() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "unknown_field".to_owned(),
                None,
                "value".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations);
        assert!(result.is_err());
    }

    #[test]
    fn test_replay_to_tension() {
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='goal';actual='reality'".to_owned(),
        )];
        let reconstructed = super::replay_mutations(&mutations).unwrap();
        let tension = reconstructed.to_tension();
        assert_eq!(tension.id, "01ABC");
        assert_eq!(tension.desired, "goal");
        assert_eq!(tension.actual, "reality");
    }

    // ── VAL-HMUT-001: Horizon recognized as mutation field ─────────────

    #[test]
    fn test_replay_with_horizon_update_year() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "horizon".to_owned(),
                None,
                "2026".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.horizon, Some(Horizon::new_year(2026).unwrap()));
    }

    #[test]
    fn test_replay_with_horizon_update_month() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "horizon".to_owned(),
                None,
                "2026-05".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.horizon, Some(Horizon::new_month(2026, 5).unwrap()));
    }

    #[test]
    fn test_replay_with_horizon_update_day() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "horizon".to_owned(),
                None,
                "2026-05-15".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.horizon, Some(Horizon::new_day(2026, 5, 15).unwrap()));
    }

    #[test]
    fn test_replay_with_horizon_clear_to_none() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "horizon".to_owned(),
                None,
                "2026".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(2),
                "horizon".to_owned(),
                Some("2026".to_owned()),
                "".to_owned(), // Empty string means None
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert!(result.horizon.is_none());
    }

    #[test]
    fn test_replay_with_horizon_invalid_format_fails() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "horizon".to_owned(),
                None,
                "invalid-horizon".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations);
        assert!(result.is_err());
        match result.unwrap_err() {
            super::ReplayError::InvalidHorizon(s) => assert_eq!(s, "invalid-horizon"),
            other => panic!("expected InvalidHorizon, got {other:?}"), // ubs:ignore test assertion
        }
    }

    // ── VAL-HMUT-002: Replay creation with horizon ──────────────────────

    #[test]
    fn test_replay_creation_with_horizon_year() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='goal';actual='reality';horizon='2026'".to_owned(),
        )];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.desired, "goal");
        assert_eq!(result.actual, "reality");
        assert_eq!(result.horizon, Some(Horizon::new_year(2026).unwrap()));
    }

    #[test]
    fn test_replay_creation_with_horizon_month() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='goal';actual='reality';horizon='2026-05'".to_owned(),
        )];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.horizon, Some(Horizon::new_month(2026, 5).unwrap()));
    }

    #[test]
    fn test_replay_creation_with_horizon_day() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='goal';actual='reality';horizon='2026-05-15'".to_owned(),
        )];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.horizon, Some(Horizon::new_day(2026, 5, 15).unwrap()));
    }

    // ── VAL-HMUT-003: Replay creation without horizon (backward compat) ─

    #[test]
    fn test_replay_creation_without_horizon_backward_compat() {
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='goal';actual='reality'".to_owned(),
        )];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.desired, "goal");
        assert_eq!(result.actual, "reality");
        assert!(result.horizon.is_none());
    }

    #[test]
    fn test_replay_creation_with_parent_without_horizon_backward_compat() {
        // Existing format with parent_id as a separate field (not in creation value)
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "parent_id".to_owned(),
                None,
                "parent123".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.parent_id, Some("parent123".to_owned()));
        assert!(result.horizon.is_none());
    }

    // ── VAL-HMUT-004: Horizon set-update-clear sequence ─────────────────

    #[test]
    fn test_replay_horizon_set_update_clear_sequence() {
        let now = Utc::now();
        let mutations = vec![
            Mutation::new(
                "01ABC".to_owned(),
                now,
                "created".to_owned(),
                None,
                "desired='goal';actual='reality'".to_owned(),
            ),
            // Set to Year(2026)
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(1),
                "horizon".to_owned(),
                None,
                "2026".to_owned(),
            ),
            // Update to Month(2026, 5)
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(2),
                "horizon".to_owned(),
                Some("2026".to_owned()),
                "2026-05".to_owned(),
            ),
            // Clear to None
            Mutation::new(
                "01ABC".to_owned(),
                now + chrono::Duration::seconds(3),
                "horizon".to_owned(),
                Some("2026-05".to_owned()),
                "".to_owned(),
            ),
        ];
        let result = super::replay_mutations(&mutations).unwrap();
        assert!(result.horizon.is_none());
    }

    // ── VAL-HMUT-005: ReconstructedTension includes horizon ─────────────

    #[test]
    fn test_reconstructed_tension_horizon_field() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='goal';actual='reality';horizon='2026-05'".to_owned(),
        )];
        let reconstructed = super::replay_mutations(&mutations).unwrap();
        assert_eq!(
            reconstructed.horizon,
            Some(Horizon::new_month(2026, 5).unwrap())
        );

        // to_tension() preserves horizon
        let tension = reconstructed.to_tension();
        assert_eq!(tension.horizon, Some(Horizon::new_month(2026, 5).unwrap()));
    }

    // ── Unicode in creation value with horizon ──────────────────────────

    #[test]
    fn test_replay_creation_with_horizon_unicode() {
        use crate::Horizon;
        let now = Utc::now();
        let mutations = vec![Mutation::new(
            "01ABC".to_owned(),
            now,
            "created".to_owned(),
            None,
            "desired='写小说';actual='有大纲';horizon='2026-05'".to_owned(),
        )];
        let result = super::replay_mutations(&mutations).unwrap();
        assert_eq!(result.desired, "写小说");
        assert_eq!(result.actual, "有大纲");
        assert_eq!(result.horizon, Some(Horizon::new_month(2026, 5).unwrap()));
    }
}
