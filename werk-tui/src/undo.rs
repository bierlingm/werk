//! Gesture undo/redo — TUI-state snapshot approach.
//!
//! Before each gesture, the app captures a `StateSnapshot` (siblings, cursor, parent, etc.).
//! On undo, the snapshot is restored and the DB-level mutation is reversed via old_value.
//! This is NOT using ftui's HistoryManager (which is widget-level, requires Send+Sync callbacks).
//!
//! The undo stack respects gesture integrity: a gesture groups multiple mutations
//! under one gesture_id. Undoing a gesture undoes the entire group.

use crate::state::FieldEntry;
use crate::deck;

/// Maximum number of gesture snapshots to retain.
const MAX_HISTORY: usize = 50;

/// Captured TUI state before a gesture — enough to restore the view.
#[derive(Clone)]
pub struct StateSnapshot {
    pub parent_id: Option<String>,
    pub siblings: Vec<FieldEntry>,
    pub deck_cursor_index: usize,
    pub deck_zoom: deck::ZoomLevel,
    pub route_expanded: bool,
    pub held_expanded: bool,
    pub accumulated_expanded: bool,
}

/// A completed gesture with its before-state snapshot.
struct GestureRecord {
    description: String,
    snapshot: StateSnapshot,
}

/// Undo/redo history for gesture-level operations.
pub struct GestureHistory {
    /// Undo stack — most recent at end.
    undo_stack: Vec<GestureRecord>,
    /// Redo stack — most recent at end. Cleared on any new gesture.
    redo_stack: Vec<GestureRecord>,
}

impl GestureHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Record a completed gesture with its before-state snapshot.
    /// Clears the redo stack (branching).
    pub fn push(&mut self, description: String, snapshot: StateSnapshot) {
        self.redo_stack.clear();
        self.undo_stack.push(GestureRecord { description, snapshot });
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }
    }

    /// Pop the most recent gesture for undo. Returns (description, snapshot).
    /// The caller should capture the current state and push it to redo.
    pub fn undo(&mut self, current_snapshot: StateSnapshot) -> Option<(String, StateSnapshot)> {
        let record = self.undo_stack.pop()?;
        let desc = record.description.clone();
        // Push current state to redo so we can re-apply
        self.redo_stack.push(GestureRecord {
            description: desc.clone(),
            snapshot: current_snapshot,
        });
        Some((desc, record.snapshot))
    }

    /// Pop the most recent redo. Returns (description, snapshot).
    pub fn redo(&mut self, current_snapshot: StateSnapshot) -> Option<(String, StateSnapshot)> {
        let record = self.redo_stack.pop()?;
        let desc = record.description.clone();
        self.undo_stack.push(GestureRecord {
            description: desc.clone(),
            snapshot: current_snapshot,
        });
        Some((desc, record.snapshot))
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
