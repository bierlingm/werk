//! Gesture undo/redo — gesture-ID stack backed by Engine::undo_gesture.
//!
//! Stores gesture IDs only. Ctrl+Z calls Engine::undo_gesture(gesture_id)
//! which appends reversal mutations to the database. Redo undoes the undo gesture.
//!
//! `GestureId` and `UndoGestureId` are distinct newtypes so that `push` and
//! `push_redo` cannot be accidentally swapped — a compile-time guard on the
//! two-stack protocol.

/// Maximum number of gesture IDs to retain.
const MAX_HISTORY: usize = 50;

/// A gesture ID (ULID of a completed user-visible gesture).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GestureId(String);

impl GestureId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// An undo-gesture ID (ULID of the reversal gesture created by Engine::undo_gesture).
///
/// Distinct from `GestureId` so we cannot push an ordinary gesture onto the
/// redo stack.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UndoGestureId(String);

impl UndoGestureId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// Undo/redo stacks storing gesture IDs.
pub struct UndoStack {
    /// Gesture IDs eligible for undo (most recent at end).
    undo: Vec<GestureId>,
    /// Undo-gesture IDs eligible for redo (most recent at end).
    redo: Vec<UndoGestureId>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Record a completed gesture. Clears the redo stack (branching).
    pub fn push(&mut self, gesture_id: String) {
        self.redo.clear();
        self.undo.push(GestureId::new(gesture_id));
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
    }

    /// Pop the most recent gesture ID for undo.
    pub fn pop_undo(&mut self) -> Option<String> {
        self.undo.pop().map(GestureId::into_string)
    }

    /// Push an undo-gesture ID onto the redo stack.
    pub fn push_redo(&mut self, undo_gesture_id: String) {
        self.redo.push(UndoGestureId::new(undo_gesture_id));
    }

    /// Pop the most recent undo-gesture ID for redo.
    pub fn pop_redo(&mut self) -> Option<String> {
        self.redo.pop().map(UndoGestureId::into_string)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_then_pop_undo_roundtrips() {
        let mut s = UndoStack::new();
        s.push("g1".to_owned());
        s.push("g2".to_owned());
        assert_eq!(s.pop_undo(), Some("g2".to_owned()));
        assert_eq!(s.pop_undo(), Some("g1".to_owned()));
        assert_eq!(s.pop_undo(), None);
    }

    #[test]
    fn test_push_clears_redo() {
        let mut s = UndoStack::new();
        s.push("g1".to_owned());
        s.push_redo("u1".to_owned());
        assert!(s.can_redo());
        s.push("g2".to_owned());
        assert!(!s.can_redo());
    }

    #[test]
    fn test_redo_stack_independent() {
        let mut s = UndoStack::new();
        s.push_redo("u1".to_owned());
        assert_eq!(s.pop_redo(), Some("u1".to_owned()));
    }

    #[test]
    fn test_max_history_trims_oldest() {
        let mut s = UndoStack::new();
        for i in 0..(MAX_HISTORY + 5) {
            s.push(format!("g{}", i));
        }
        // First MAX_HISTORY entries dropped; top of stack is the newest.
        assert_eq!(s.pop_undo(), Some(format!("g{}", MAX_HISTORY + 4)));
    }

    #[test]
    fn test_gesture_id_newtype_roundtrip() {
        let id = GestureId::new("01JX".to_owned());
        assert_eq!(id.as_str(), "01JX");
        assert_eq!(id.into_string(), "01JX");
    }
}
