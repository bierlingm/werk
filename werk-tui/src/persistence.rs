//! Workspace persistence — save/restore TUI state across sessions.
//!
//! Uses ftui's StateRegistry with a file backend. The workspace snapshot
//! is stored in `.werk/tui_state.json` via the registry's atomic
//! write-rename pattern.
//!
//! Domain state (parent_id, focus target, zoom, expansions, collapsed bands)
//! lives in the WorkspaceSnapshot `extensions` map as JSON strings.

use std::sync::Arc;

use ftui_runtime::state_persistence::StateRegistry;
use serde::{Deserialize, Serialize};

use crate::deck::{CursorTarget, ZoomLevel};
use crate::state::ViewOrientation;
use crate::survey::TimeBand;

const REGISTRY_KEY: &str = "workspace::instrument";
const SCHEMA_VERSION: u32 = 1;

/// Domain state persisted across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub parent_id: Option<String>,
    pub cursor_target: PersistedCursorTarget,
    pub view_orientation: PersistedOrientation,
    pub deck_zoom: PersistedZoom,
    pub route_expanded: bool,
    pub held_expanded: bool,
    pub accumulated_expanded: bool,
    pub collapsed_bands: Vec<PersistedTimeBand>,
}

/// Serializable mirror of CursorTarget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistedCursorTarget {
    Desire,
    Route(usize),
    RouteSummary,
    Overdue(usize),
    Next(usize),
    Held,
    HeldItem(usize),
    InputPoint,
    Accumulated,
    AccumulatedItem(usize),
    NoteItem(usize),
    Reality,
}

impl From<CursorTarget> for PersistedCursorTarget {
    fn from(ct: CursorTarget) -> Self {
        match ct {
            CursorTarget::Desire => Self::Desire,
            CursorTarget::Route(i) => Self::Route(i),
            CursorTarget::RouteSummary => Self::RouteSummary,
            CursorTarget::Overdue(i) => Self::Overdue(i),
            CursorTarget::Next(i) => Self::Next(i),
            CursorTarget::Held => Self::Held,
            CursorTarget::HeldItem(i) => Self::HeldItem(i),
            CursorTarget::InputPoint => Self::InputPoint,
            CursorTarget::Accumulated => Self::Accumulated,
            CursorTarget::AccumulatedItem(i) => Self::AccumulatedItem(i),
            CursorTarget::NoteItem(i) => Self::NoteItem(i),
            CursorTarget::Reality => Self::Reality,
        }
    }
}

impl From<PersistedCursorTarget> for CursorTarget {
    fn from(pct: PersistedCursorTarget) -> Self {
        match pct {
            PersistedCursorTarget::Desire => Self::Desire,
            PersistedCursorTarget::Route(i) => Self::Route(i),
            PersistedCursorTarget::RouteSummary => Self::RouteSummary,
            PersistedCursorTarget::Overdue(i) => Self::Overdue(i),
            PersistedCursorTarget::Next(i) => Self::Next(i),
            PersistedCursorTarget::Held => Self::Held,
            PersistedCursorTarget::HeldItem(i) => Self::HeldItem(i),
            PersistedCursorTarget::InputPoint => Self::InputPoint,
            PersistedCursorTarget::Accumulated => Self::Accumulated,
            PersistedCursorTarget::AccumulatedItem(i) => Self::AccumulatedItem(i),
            PersistedCursorTarget::NoteItem(i) => Self::NoteItem(i),
            PersistedCursorTarget::Reality => Self::Reality,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistedOrientation {
    Stream,
    Survey,
}

impl From<ViewOrientation> for PersistedOrientation {
    fn from(vo: ViewOrientation) -> Self {
        match vo {
            ViewOrientation::Stream => Self::Stream,
            ViewOrientation::Survey => Self::Survey,
        }
    }
}

impl From<PersistedOrientation> for ViewOrientation {
    fn from(po: PersistedOrientation) -> Self {
        match po {
            PersistedOrientation::Stream => Self::Stream,
            PersistedOrientation::Survey => Self::Survey,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistedZoom {
    Normal,
    Focus,
    Peek,
    Orient,
}

impl From<ZoomLevel> for PersistedZoom {
    fn from(zl: ZoomLevel) -> Self {
        match zl {
            ZoomLevel::Normal => Self::Normal,
            ZoomLevel::Focus => Self::Focus,
            ZoomLevel::Peek => Self::Peek,
            ZoomLevel::Orient => Self::Orient,
        }
    }
}

impl From<PersistedZoom> for ZoomLevel {
    fn from(pz: PersistedZoom) -> Self {
        match pz {
            PersistedZoom::Normal => Self::Normal,
            PersistedZoom::Focus => Self::Focus,
            PersistedZoom::Peek => Self::Peek,
            PersistedZoom::Orient => Self::Orient,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistedTimeBand {
    NoDeadline,
    Later,
    ThisMonth,
    ThisWeek,
    Overdue,
}

impl From<TimeBand> for PersistedTimeBand {
    fn from(tb: TimeBand) -> Self {
        match tb {
            TimeBand::NoDeadline => Self::NoDeadline,
            TimeBand::Later => Self::Later,
            TimeBand::ThisMonth => Self::ThisMonth,
            TimeBand::ThisWeek => Self::ThisWeek,
            TimeBand::Overdue => Self::Overdue,
        }
    }
}

impl From<PersistedTimeBand> for TimeBand {
    fn from(ptb: PersistedTimeBand) -> Self {
        match ptb {
            PersistedTimeBand::NoDeadline => Self::NoDeadline,
            PersistedTimeBand::Later => Self::Later,
            PersistedTimeBand::ThisMonth => Self::ThisMonth,
            PersistedTimeBand::ThisWeek => Self::ThisWeek,
            PersistedTimeBand::Overdue => Self::Overdue,
        }
    }
}

/// Create a file-backed StateRegistry at `.werk/tui_state.json`.
/// Returns None if the workspace directory cannot be determined.
pub fn create_registry() -> Option<Arc<StateRegistry>> {
    let werk_dir = std::env::current_dir().ok()?.join(".werk");
    if !werk_dir.exists() {
        return None;
    }
    let path = werk_dir.join("tui_state.json");
    Some(Arc::new(StateRegistry::with_file(path)))
}

/// Save workspace state to the registry.
pub fn save_workspace(registry: &StateRegistry, state: &WorkspaceState) {
    if let Ok(data) = serde_json::to_vec(state) {
        registry.set(REGISTRY_KEY, SCHEMA_VERSION, data);
    }
}

/// Load workspace state from the registry.
/// Returns None if no state exists or deserialization fails.
pub fn load_workspace(registry: &StateRegistry) -> Option<WorkspaceState> {
    let entry = registry.get(REGISTRY_KEY)?;
    if entry.version != SCHEMA_VERSION {
        return None;
    }
    serde_json::from_slice(&entry.data).ok()
}

const FEEDBACK_KEY: &str = "palette::feedback";
const FEEDBACK_VERSION: u32 = 1;

/// Save palette feedback boost map to the registry.
pub fn save_feedback(
    registry: &StateRegistry,
    feedback: &crate::feedback::FeedbackCollector,
) {
    if let Ok(json) = feedback.export_boost_map() {
        registry.set(FEEDBACK_KEY, FEEDBACK_VERSION, json.into_bytes());
    }
}

/// Load palette feedback boost map from the registry into a collector.
pub fn load_feedback(
    registry: &StateRegistry,
    feedback: &mut crate::feedback::FeedbackCollector,
) {
    if let Some(entry) = registry.get(FEEDBACK_KEY) {
        if entry.version == FEEDBACK_VERSION {
            if let Ok(json) = std::str::from_utf8(&entry.data) {
                let _ = feedback.import_boost_map(json);
            }
        }
    }
}
