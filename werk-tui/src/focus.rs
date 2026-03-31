//! Focus graph for spatial navigation.
//!
//! Phase 2 delivered the skeleton (7 static zone nodes).
//! Phase 4 (#165) expands this: each selectable item in the frontier
//! gets a FocusNode, wired vertically. j/k navigation traverses the
//! graph instead of incrementing a flat cursor index.

use ftui::layout::Rect;
use ftui::widgets::{FocusGraph, FocusId, FocusNode, NavDirection};

use crate::deck::{AccumulatedItem, CursorTarget, Frontier};

/// Focus graph state — drives j/k navigation through the deck.
///
/// The graph is rebuilt whenever the frontier changes (after gestures,
/// expansion toggles, or data reloads). Each selectable item gets a
/// FocusNode with Up/Down edges to its neighbors.
pub struct FocusState {
    pub graph: FocusGraph,
    pub active: FocusId,
    /// Maps FocusId → CursorTarget for the current frontier layout.
    /// Ordered by display position (top to bottom).
    targets: Vec<(FocusId, CursorTarget)>,
    /// Next available FocusId for allocation.
    next_id: FocusId,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            graph: FocusGraph::new(),
            active: 0,
            targets: Vec::new(),
            next_id: 10,
        }
    }

    /// Navigate in a direction. Returns the new active FocusId.
    pub fn navigate(&mut self, dir: NavDirection) -> FocusId {
        if let Some(next) = self.graph.navigate(self.active, dir) {
            self.active = next;
        }
        self.active
    }

    /// Get the CursorTarget for the currently active focus node.
    pub fn cursor_target(&self) -> CursorTarget {
        self.target_for(self.active)
    }

    /// Get the CursorTarget for any FocusId.
    pub fn target_for(&self, id: FocusId) -> CursorTarget {
        self.targets
            .iter()
            .find(|(fid, _)| *fid == id)
            .map(|(_, target)| *target)
            .unwrap_or(CursorTarget::InputPoint)
    }

    /// Find the FocusId for a given CursorTarget.
    pub fn focus_for(&self, target: &CursorTarget) -> Option<FocusId> {
        self.targets
            .iter()
            .find(|(_, t)| t == target)
            .map(|(id, _)| *id)
    }

    /// Find the FocusId for an item by sibling index.
    pub fn focus_for_sibling(&self, sibling_idx: usize) -> Option<FocusId> {
        self.targets.iter().find(|(_, t)| {
            matches!(t,
                CursorTarget::Route(i) | CursorTarget::Overdue(i) |
                CursorTarget::Next(i) | CursorTarget::HeldItem(i) |
                CursorTarget::AccumulatedItem(i) if *i == sibling_idx
            )
        }).map(|(id, _)| *id)
    }

    /// Get the flat index of the active focus node (position in targets list).
    /// Used for compatibility during migration.
    pub fn active_index(&self) -> usize {
        self.targets
            .iter()
            .position(|(id, _)| *id == self.active)
            .unwrap_or(0)
    }

    /// Total number of selectable items in the graph.
    pub fn selectable_count(&self) -> usize {
        self.targets.len()
    }

    /// Get the default focus target (InputPoint, or first item if no InputPoint).
    pub fn default_focus(&self) -> FocusId {
        self.focus_for(&CursorTarget::InputPoint)
            .or_else(|| self.targets.first().map(|(id, _)| *id))
            .unwrap_or(0)
    }

    /// Clamp active to a valid node. Called after rebuild.
    pub fn clamp_active(&mut self) {
        if self.targets.is_empty() {
            self.active = 0;
            return;
        }
        // If active is still valid, keep it
        if self.targets.iter().any(|(id, _)| *id == self.active) {
            return;
        }
        // Otherwise reset to default
        self.active = self.default_focus();
    }

    /// Rebuild the focus graph for the current frontier.
    ///
    /// Creates a FocusNode for each selectable item in display order
    /// (desire → route → overdue → next → held → input → accumulated → reality).
    /// Wires Up/Down edges between adjacent items.
    pub fn rebuild_for_frontier(
        &mut self,
        frontier: &Frontier,
        has_desire: bool,
        has_reality: bool,
    ) {
        self.graph = FocusGraph::new();
        self.targets.clear();
        self.next_id = 10;

        let mut prev: Option<FocusId> = None;

        // Helper: allocate a node and wire it to the previous one
        let alloc = |target: CursorTarget,
                         graph: &mut FocusGraph,
                         targets: &mut Vec<(FocusId, CursorTarget)>,
                         next_id: &mut FocusId,
                         prev: &mut Option<FocusId>|
         -> FocusId {
            let id = *next_id;
            *next_id += 1;
            graph.insert(FocusNode::new(id, Rect::default()));
            if let Some(prev_id) = *prev {
                graph.connect(prev_id, NavDirection::Down, id);
                graph.connect(id, NavDirection::Up, prev_id);
            }
            *prev = Some(id);
            targets.push((id, target));
            id
        };

        // Desire anchor
        if has_desire {
            alloc(
                CursorTarget::Desire,
                &mut self.graph,
                &mut self.targets,
                &mut self.next_id,
                &mut prev,
            );
        }

        // Check for unified summary (Q28): route+held both fully compressed
        let shown_route = frontier.show_route.min(frontier.route.len());
        let route_remaining = frontier.route.len() - shown_route;
        let shown_held = frontier.show_held.min(frontier.held.len());
        let held_remaining = frontier.held.len() - shown_held;
        let unified = shown_route == 0
            && route_remaining > 0
            && shown_held == 0
            && held_remaining > 0;

        if unified {
            // Single unified summary line for route+held
            alloc(
                CursorTarget::RouteSummary,
                &mut self.graph,
                &mut self.targets,
                &mut self.next_id,
                &mut prev,
            );
        } else {
            // Route items
            for i in 0..shown_route {
                alloc(
                    CursorTarget::Route(frontier.route[i]),
                    &mut self.graph,
                    &mut self.targets,
                    &mut self.next_id,
                    &mut prev,
                );
            }
            // Route summary (remaining > 0)
            if route_remaining > 0 {
                alloc(
                    CursorTarget::RouteSummary,
                    &mut self.graph,
                    &mut self.targets,
                    &mut self.next_id,
                    &mut prev,
                );
            }

            // Overdue items
            for &idx in &frontier.overdue {
                alloc(
                    CursorTarget::Overdue(idx),
                    &mut self.graph,
                    &mut self.targets,
                    &mut self.next_id,
                    &mut prev,
                );
            }

            // Next step
            if let Some(next_idx) = frontier.next {
                alloc(
                    CursorTarget::Next(next_idx),
                    &mut self.graph,
                    &mut self.targets,
                    &mut self.next_id,
                    &mut prev,
                );
            }

            // Held items
            for i in 0..shown_held {
                alloc(
                    CursorTarget::HeldItem(frontier.held[i]),
                    &mut self.graph,
                    &mut self.targets,
                    &mut self.next_id,
                    &mut prev,
                );
            }
            // Held summary (remaining > 0)
            if held_remaining > 0 {
                alloc(
                    CursorTarget::Held,
                    &mut self.graph,
                    &mut self.targets,
                    &mut self.next_id,
                    &mut prev,
                );
            }
        }

        // Input point (always present)
        alloc(
            CursorTarget::InputPoint,
            &mut self.graph,
            &mut self.targets,
            &mut self.next_id,
            &mut prev,
        );

        // Accumulated items
        let shown_acc = frontier.show_accumulated.min(frontier.accumulated.len());
        for i in 0..shown_acc {
            let target = match &frontier.accumulated[i] {
                AccumulatedItem::Child(idx) => CursorTarget::AccumulatedItem(*idx),
                AccumulatedItem::Note { .. } => CursorTarget::NoteItem(i),
            };
            alloc(
                target,
                &mut self.graph,
                &mut self.targets,
                &mut self.next_id,
                &mut prev,
            );
        }
        // Accumulated summary (remaining > 0)
        if frontier.accumulated.len() > shown_acc {
            alloc(
                CursorTarget::Accumulated,
                &mut self.graph,
                &mut self.targets,
                &mut self.next_id,
                &mut prev,
            );
        }

        // Reality anchor
        if has_reality {
            alloc(
                CursorTarget::Reality,
                &mut self.graph,
                &mut self.targets,
                &mut self.next_id,
                &mut prev,
            );
        }

        // Clamp active to valid node
        self.clamp_active();
    }

    /// Try to preserve the current CursorTarget across a rebuild.
    /// Call this before rebuild to capture the current target,
    /// then after rebuild call restore_target().
    pub fn capture_target(&self) -> CursorTarget {
        self.cursor_target()
    }

    /// After a rebuild, try to restore focus to the same CursorTarget.
    /// Falls back to default if the target no longer exists.
    pub fn restore_target(&mut self, target: &CursorTarget) {
        if let Some(id) = self.focus_for(target) {
            self.active = id;
        } else {
            self.active = self.default_focus();
        }
    }
}
