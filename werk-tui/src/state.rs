//! Core state types for the Operative Instrument TUI.

use sd_core::TensionStatus;

/// A tension as displayed in the Field.
#[derive(Debug, Clone)]
pub struct FieldEntry {
    pub id: String,
    pub desired: String,
    pub actual: String,
    pub status: TensionStatus,
    pub has_children: bool,
    pub parent_id: Option<String>,
    /// Explicit ordering position among siblings. None means unpositioned.
    pub position: Option<i32>,
    /// Compact horizon label (e.g. "Mar", "Mar 20", "2026"). None if no horizon.
    pub horizon_label: Option<String>,
    /// Temporal indicator string (six dots showing window position or staleness).
    pub temporal_indicator: String,
    /// Urgency level 0.0-1.0 for coloring the temporal indicator.
    pub temporal_urgency: f64,
}

impl FieldEntry {
    pub fn from_tension(
        tension: &sd_core::Tension,
        last_reality_update: chrono::DateTime<chrono::Utc>,
        has_children: bool,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let horizon_end = tension.horizon.as_ref().map(|h| h.range_end());
        let (temporal_indicator, temporal_urgency) =
            crate::glyphs::temporal_indicator(last_reality_update, horizon_end, now);

        let now_year = chrono::Datelike::year(&now);
        let horizon_label = tension
            .horizon
            .as_ref()
            .map(|h| crate::glyphs::compact_horizon(h, now_year));

        Self {
            id: tension.id.clone(),
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            status: tension.status,
            has_children,
            parent_id: tension.parent_id.clone(),
            position: tension.position,
            horizon_label,
            temporal_indicator,
            temporal_urgency,
        }
    }
}

/// Which sibling is gazed and how deep.
#[derive(Debug, Clone)]
pub struct GazeState {
    pub index: usize,
    pub full: bool,
}

/// Data for the quick Gaze (Depth 1).
pub struct GazeData {
    pub id: String,
    pub actual: String,
    pub horizon: Option<String>,
    pub created_at: String,
    pub children: Vec<ChildPreview>,
    pub last_event: Option<String>,
}

/// Mini-line for children preview inside Gaze.
pub struct ChildPreview {
    pub id: String,
    pub desired: String,
    pub status: TensionStatus,
    /// Explicit ordering position. None means unpositioned (backlog).
    pub position: Option<i32>,
}

/// Data for the full Gaze (Depth 2 — facts + history).
pub struct FullGazeData {
    pub urgency: Option<String>,
    pub horizon_drift: Option<String>,
    pub closure: Option<String>,
    pub history: Vec<HistoryEntry>,
}

/// A single mutation in the history.
pub struct HistoryEntry {
    pub relative_time: String,
    pub description: String,
}

/// Input mode — what the user is currently doing.
#[derive(Debug, Clone)]
pub enum InputMode {
    Normal,
    Adding(AddStep),
    Editing {
        tension_id: String,
        field: EditField,
    },
    Annotating {
        tension_id: String,
    },
    Confirming(ConfirmKind),
    Moving {
        tension_id: String,
    },
    Reordering {
        tension_id: String,
    },
    Searching,
    AgentPrompt {
        tension_id: String,
    },
    ReviewingMutations,
    ReviewingInsights,
    Help,
}

/// A pending watch insight loaded from disk for TUI review.
pub struct InsightData {
    pub file_path: std::path::PathBuf,
    pub tension_id: String,
    pub tension_desired: String,
    pub trigger: String,
    pub response: String,
    pub mutation_count: usize,
    pub mutation_text: String,
    pub timestamp: String,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub enum AddStep {
    Name,
    Desire { name: String },
    Reality { name: String, desire: String },
    Horizon { name: String, desire: String, reality: String },
}

#[derive(Debug, Clone)]
pub enum EditField {
    Desire,
    Reality,
    Horizon,
}

#[derive(Debug, Clone)]
pub enum ConfirmKind {
    Resolve { tension_id: String, desired: String },
    Release { tension_id: String, desired: String },
}

/// An alert computed from current tension state.
#[derive(Debug, Clone)]
pub struct Alert {
    pub kind: AlertKind,
    pub message: String,
    pub action_hint: String,
}

#[derive(Debug, Clone)]
pub enum AlertKind {
    Neglect { weeks: i64 },
    HorizonPast { days: i64 },
    MultipleRoots { count: usize },
}

/// Transient message shown in the Lever, auto-expires.
#[derive(Debug, Clone)]
pub struct TransientMessage {
    pub text: String,
    pub expires: std::time::Instant,
}

impl TransientMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            expires: std::time::Instant::now() + std::time::Duration::from_secs(3),
        }
    }

    pub fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires
    }
}
