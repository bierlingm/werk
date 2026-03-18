//! Core state types for the Operative Instrument TUI.

use sd_core::{ComputedDynamics, CreativeCyclePhase, StructuralTendency, TensionStatus};

/// A tension as displayed in the Field.
#[derive(Debug, Clone)]
pub struct FieldEntry {
    pub id: String,
    pub desired: String,
    pub actual: String,
    pub status: TensionStatus,
    pub phase: CreativeCyclePhase,
    pub tendency: StructuralTendency,
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
        computed: &Option<ComputedDynamics>,
        last_reality_update: chrono::DateTime<chrono::Utc>,
        has_children: bool,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let (phase, tendency) = match computed {
            Some(cd) => (cd.phase.phase, cd.tendency.tendency),
            None => (CreativeCyclePhase::Germination, StructuralTendency::Stagnant),
        };

        let horizon_end = tension.horizon.as_ref().map(|h| h.range_end());
        let (temporal_indicator, temporal_urgency) =
            crate::glyphs::temporal_indicator(last_reality_update, horizon_end, now);

        let now_year = chrono::Datelike::year(&now);
        let horizon_label = tension.horizon.as_ref()
            .map(|h| crate::glyphs::compact_horizon(h, now_year));

        Self {
            id: tension.id.clone(),
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            status: tension.status,
            phase,
            tendency,
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
    pub full: bool, // false = quick (desire/reality/gap), true = + dynamics + history
}

/// Data for the quick Gaze (Depth 1).
///
/// The primary gaze card: minimal information to act on.
/// Desire is not included — it's already visible in the tension line.
pub struct GazeData {
    pub id: String,
    pub actual: String,
    pub horizon: Option<String>,
    pub created_at: String,
    pub children: Vec<ChildPreview>,
    /// The most recent event for this tension, stated concisely.
    pub last_event: Option<String>,
}

/// Mini-line for children preview inside Gaze.
pub struct ChildPreview {
    pub id: String,
    pub desired: String,
    pub phase: CreativeCyclePhase,
    pub tendency: StructuralTendency,
    pub status: TensionStatus,
}

/// Data for the full Gaze (Depth 2 — dynamics + history).
pub struct FullGazeData {
    pub phase: String,
    pub tendency: String,
    pub magnitude: Option<f64>,
    pub orientation: Option<String>,
    pub conflict: Option<String>,
    pub neglect: Option<String>,
    pub oscillation: Option<String>,
    pub resolution: Option<String>,
    pub compensating_strategy: Option<String>,
    pub assimilation: Option<String>,
    pub horizon_drift: Option<String>,
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
    pub mutation_text: String,  // raw mutation YAML for display
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
