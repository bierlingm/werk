use std::time::{Duration, Instant};
use ftui::PackedRgba;
use crate::theme::{CLR_LIGHT_GRAY, CLR_YELLOW, CLR_RED};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UrgencyTier {
    Urgent,
    Active,
    Neglected,
    Resolved,
}

#[derive(Debug, Clone)]
pub struct TensionRow {
    pub id: String,
    pub short_id: String,
    pub desired: String,
    pub actual: String,
    pub status: String,
    pub phase: String,
    pub movement: String,
    pub urgency: Option<f64>,
    pub magnitude: Option<f64>,
    pub neglected: bool,
    pub horizon_display: String,
    pub tier: UrgencyTier,
    pub activity: Vec<f64>,  // 7 buckets, one per day, most recent last
    pub trajectory: Option<sd_core::Trajectory>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Active,
    All,
    Resolved,
    Released,
}

impl Filter {
    pub fn next(self) -> Self {
        match self {
            Filter::Active => Filter::All,
            Filter::All => Filter::Resolved,
            Filter::Resolved => Filter::Released,
            Filter::Released => Filter::Active,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Filter::Active => "Active",
            Filter::All => "All",
            Filter::Resolved => "Resolved",
            Filter::Released => "Released",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastSeverity {
    Info,
    Warning,
    Alert,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub severity: ToastSeverity,
    pub created_at: Instant,
}

impl Toast {
    pub fn new(message: String, severity: ToastSeverity) -> Self {
        Self {
            message,
            severity,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > Duration::from_secs(5)
    }

    pub fn color(&self) -> PackedRgba {
        match self.severity {
            ToastSeverity::Info => CLR_LIGHT_GRAY,
            ToastSeverity::Warning => CLR_YELLOW,
            ToastSeverity::Alert => CLR_RED,
        }
    }
}

pub const MAX_VISIBLE_TOASTS: usize = 3;
pub const URGENCY_ALERT_THRESHOLD: f64 = 0.75;

#[derive(Debug, Clone)]
pub struct TreeItem {
    pub tension_id: String,
    pub short_id: String,
    pub desired: String,
    pub phase: String,
    pub movement: String,
    pub horizon_display: String,
    pub urgency: Option<f64>,
    pub depth: usize,
    pub connector: String,
    pub tier: UrgencyTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationKind {
    Created,
    FieldUpdate,
    StatusChange,
    ParentChange,
    HorizonChange,
    Note,
}

#[derive(Debug, Clone)]
pub struct MutationDisplay {
    pub relative_time: String,
    pub field: String,
    pub kind: MutationKind,
    pub old_value: Option<String>,
    pub new_value: String,
    pub resolved_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DetailDynamics {
    pub phase: String,
    pub movement: String,
    pub magnitude: Option<f64>,
    pub urgency: Option<f64>,
    pub neglect: Option<String>,
    pub conflict: Option<String>,
    pub oscillation: Option<String>,
    pub resolution: Option<String>,
    pub orientation: Option<String>,
    pub compensating_strategy: Option<String>,
    pub assimilation_depth: Option<String>,
    pub horizon_drift: Option<String>,
    pub forecast_line: Option<(String, PackedRgba)>,
}
