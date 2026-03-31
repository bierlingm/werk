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
    /// Number of children (for →N indicator in deck).
    pub child_count: usize,
    pub parent_id: Option<String>,
    /// Explicit ordering position among siblings. None means unpositioned.
    pub position: Option<i32>,
    /// Compact horizon label (e.g. "Mar", "Mar 20", "2026"). None if no horizon.
    pub horizon_label: Option<String>,
    /// Urgency level: 0.0 = fresh, 1.0 = at deadline, >1.0 = overdue.
    pub temporal_urgency: f64,
    /// Short numeric code for display (e.g. #3). None for ULIDs without a short code.
    pub short_code: Option<i32>,
    /// Compact age string (e.g. "2d", "3w"). Computed from created_at.
    pub created_age: String,
    /// Timestamp of the last status change (resolve/release). For epoch filtering.
    pub last_status_change: chrono::DateTime<chrono::Utc>,
}

impl FieldEntry {
    pub fn from_tension(
        tension: &sd_core::Tension,
        last_reality_update: chrono::DateTime<chrono::Utc>,
        child_count: usize,
        last_status_change: chrono::DateTime<chrono::Utc>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let has_children = child_count > 0;
        let horizon_end = tension.horizon.as_ref().map(|h| h.range_end());
        let temporal_urgency =
            crate::glyphs::temporal_urgency(last_reality_update, horizon_end, now);

        let now_year = chrono::Datelike::year(&now);
        let horizon_label = tension
            .horizon
            .as_ref()
            .map(|h| crate::glyphs::compact_horizon(h, now_year));

        let created_age = crate::glyphs::compact_age(tension.created_at, now);

        Self {
            id: tension.id.clone(),
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            status: tension.status,
            has_children,
            child_count,
            parent_id: tension.parent_id.clone(),
            position: tension.position,
            horizon_label,
            temporal_urgency,
            short_code: tension.short_code,
            created_age,
            last_status_change,
        }
    }
}

/// Top-level view orientation — which axis dominates the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewOrientation {
    /// Structure-first: one tension's theory of closure through time.
    Stream,
    /// Time-first: all active tensions organized by temporal urgency.
    Survey,
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
    Help,
    Pathway,
}

#[derive(Debug, Clone)]
pub enum AddStep {
    Desire,
    Reality { desire: String },
    Horizon { desire: String, reality: String },
}

#[derive(Debug, Clone)]
pub enum EditField {
    Desire,
    Reality,
    Horizon,
}

impl EditField {
    pub fn label(&self) -> &'static str {
        match self {
            EditField::Desire => "desire",
            EditField::Reality => "reality",
            EditField::Horizon => "horizon",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConfirmKind {
    Resolve { tension_id: String, desired: String },
    Release { tension_id: String, desired: String },
}

/// Active pathway palette state — held separately from InputMode because
/// PaletteContext doesn't derive Clone/Debug.
pub struct PathwayState {
    pub palette: werk_shared::palette::Palette,
    pub context: werk_shared::palette::PaletteContext,
    /// 0-based cursor into palette.options.
    pub cursor: usize,
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
