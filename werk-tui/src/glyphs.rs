//! Symbol vocabulary for the Operative Instrument.

use chrono::Datelike;
use sd_core::TensionStatus;

/// Status glyph: ✦ resolved, · released, ◇ active.
pub fn status_glyph(status: TensionStatus) -> &'static str {
    match status {
        TensionStatus::Resolved => "\u{2726}", // ✦
        TensionStatus::Released => "\u{00B7}", // ·
        TensionStatus::Active => "\u{25C7}",   // ◇
    }
}

/// Card border character — thin vertical line for card edges.
pub const CARD_EDGE: char = '\u{2502}'; // │

/// Temporal urgency — how far "now" has progressed through the action window.
///
/// With horizon: ratio of elapsed time to total window (last reality update → horizon end).
///   0.0 = just started, 1.0 = at deadline, >1.0 = overdue.
///
/// Without horizon: staleness as fraction of 6 weeks since last reality update.
pub fn temporal_urgency(
    last_reality_update: chrono::DateTime<chrono::Utc>,
    horizon_end: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> f64 {
    if let Some(end) = horizon_end {
        let window_secs = end.signed_duration_since(last_reality_update).num_seconds().max(1) as f64;
        let elapsed_secs = now.signed_duration_since(last_reality_update).num_seconds().max(0) as f64;
        elapsed_secs / window_secs
    } else {
        let weeks_since = now.signed_duration_since(last_reality_update).num_weeks();
        (weeks_since as f64 / 6.0).min(1.0)
    }
}

/// Format a horizon for compact inline display.
///
/// Adapts based on precision:
/// - Year: `2026`
/// - Month: `Mar` (or `Mar 26` if not current year)
/// - Day: `Mar 20`
/// - Week: Monday of that week, e.g. `Mar 31`
pub fn compact_horizon(horizon: &werk_core::Horizon, now_year: i32) -> String {
    use werk_core::HorizonKind;
    match horizon.kind() {
        HorizonKind::Year(y) => format!("{}", y),
        HorizonKind::Month(y, m) => {
            let month_name = match m {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "???",
            };
            if y == now_year {
                month_name.to_string()
            } else {
                format!("{} {}", month_name, y % 100)
            }
        }
        HorizonKind::Day(date) => {
            let month_name = match date.month() {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "???",
            };
            format!("{} {}", month_name, date.day())
        }
        HorizonKind::DateTime(dt) => {
            let month_name = match dt.month() {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "???",
            };
            format!("{} {}", month_name, dt.day())
        }
    }
}

/// Format a relative time distance as compact text.
///
/// Returns e.g. "just now", "2h ago", "3d ago", "2w ago", "3mo ago"
pub fn relative_time(
    from: chrono::DateTime<chrono::Utc>,
    to: chrono::DateTime<chrono::Utc>,
) -> String {
    let delta = to.signed_duration_since(from);
    let mins = delta.num_minutes();
    let hours = delta.num_hours();
    let days = delta.num_days();
    let weeks = delta.num_weeks();

    if mins < 1 {
        "just now".to_string()
    } else if hours < 1 {
        format!("{}m ago", mins)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else if days < 14 {
        format!("{}d ago", days)
    } else if weeks < 9 {
        format!("{}w ago", weeks)
    } else {
        format!("{}mo ago", days / 30)
    }
}

/// Compact age string for the deck right column.
/// Returns e.g. "2d", "3w", "1mo" — no "ago" suffix, minimal width.
pub fn compact_age(from: chrono::DateTime<chrono::Utc>, now: chrono::DateTime<chrono::Utc>) -> String {
    let delta = now.signed_duration_since(from);
    let hours = delta.num_hours();
    let days = delta.num_days();
    let weeks = delta.num_weeks();

    if hours < 24 {
        format!("{}h", hours.max(0))
    } else if days < 14 {
        format!("{}d", days)
    } else if weeks < 9 {
        format!("{}w", weeks)
    } else {
        format!("{}mo", days / 30)
    }
}

// Separator constants
pub const LIGHT_RULE: char = '\u{2504}'; // ┄
pub const RULE: char = '\u{2500}';       // ─
pub const HEAVY_RULE: char = '\u{2501}'; // ━
