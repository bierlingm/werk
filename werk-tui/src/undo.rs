//! Gesture undo/redo — gesture-ID stack backed by Engine::undo_gesture.
//!
//! Stores gesture IDs only. Ctrl+Z calls Engine::undo_gesture(gesture_id)
//! which appends reversal mutations to the database. Redo undoes the undo gesture.

/// Maximum number of gesture IDs to retain.
const MAX_HISTORY: usize = 50;

/// Undo/redo stacks storing gesture IDs.
pub struct UndoStack {
    /// Gesture IDs eligible for undo (most recent at end).
    undo: Vec<String>,
    /// Undo-gesture IDs eligible for redo (most recent at end).
    redo: Vec<String>,
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
        self.undo.push(gesture_id);
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
    }

    /// Pop the most recent gesture ID for undo.
    pub fn pop_undo(&mut self) -> Option<String> {
        self.undo.pop()
    }

    /// Push an undo-gesture ID onto the redo stack.
    pub fn push_redo(&mut self, undo_gesture_id: String) {
        self.redo.push(undo_gesture_id);
    }

    /// Pop the most recent undo-gesture ID for redo.
    pub fn pop_redo(&mut self) -> Option<String> {
        self.redo.pop()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
