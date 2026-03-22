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

/// Activity trail: ○● dots showing weekly mutation activity.
/// Each dot represents one time bucket: ● = active, ○ = quiet.
pub fn trail(activity: &[f64], max_dots: usize) -> String {
    if activity.is_empty() {
        return String::new();
    }
    activity
        .iter()
        .rev()
        .take(max_dots)
        .map(|&v| if v > 0.0 { "\u{25CF}" } else { "\u{25CB}" }) // ● or ○
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// Temporal position indicator — shows where "now" falls in the action window.
///
/// With horizon: six dots spanning from last reality update to horizon end.
///   ● marks "now", ◦ marks horizon end position.
///   Gap between ● and ◦ = remaining runway.
///
/// Without horizon: six dots showing staleness since last reality update.
///   ◎ marks how many weeks ago reality was last checked, drifting left.
///
/// Returns (indicator_string, urgency_level 0.0-1.0) for color decisions.
pub fn temporal_indicator(
    last_reality_update: chrono::DateTime<chrono::Utc>,
    horizon_end: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> (String, f64) {
    const DOTS: usize = 6;
    let empty = "\u{25CC}"; // ◌
    let now_marker = "\u{25E6}"; // ◦ (open — you are here, moving)
    let horizon_marker = "\u{25CF}"; // ● (solid — the fixed target)
    let stale_marker = "\u{25CE}"; // ◎

    if let Some(end) = horizon_end {
        // Window: last_reality_update → horizon_end
        let window = end.signed_duration_since(last_reality_update);
        let elapsed = now.signed_duration_since(last_reality_update);

        let window_secs = window.num_seconds().max(1) as f64;
        let elapsed_secs = elapsed.num_seconds().max(0) as f64;

        // Position of "now" in the window (0.0 = start, 1.0 = end)
        let now_ratio = (elapsed_secs / window_secs).clamp(0.0, 1.2); // allow slight overshoot
        let now_pos = ((now_ratio * (DOTS - 1) as f64).round() as usize).min(DOTS - 1);

        // Position of horizon end in the window (always at the ratio point)
        let horizon_pos = DOTS - 2; // second-to-last position (leaves room for overshoot)

        let urgency = now_ratio.min(1.0);

        let mut dots: Vec<&str> = vec![empty; DOTS];
        dots[horizon_pos] = horizon_marker;
        dots[now_pos] = now_marker; // now overwrites horizon if they overlap

        (dots.join(""), urgency)
    } else {
        // No horizon: staleness indicator
        let weeks_since = now.signed_duration_since(last_reality_update).num_weeks();
        let stale_pos = (weeks_since as usize).min(DOTS - 1);
        // Position from right (0 = rightmost = fresh) to left (5 = leftmost = stale)
        let display_pos = (DOTS - 1).saturating_sub(stale_pos);

        let staleness = (weeks_since as f64 / DOTS as f64).min(1.0);

        let mut dots: Vec<&str> = vec![empty; DOTS];
        dots[display_pos] = stale_marker;

        (dots.join(""), staleness)
    }
}

/// Format a horizon for compact inline display.
///
/// Adapts based on precision:
/// - Year: `2026`
/// - Month: `Mar` (or `Mar 26` if not current year)
/// - Day: `Mar 20`
/// - Week: Monday of that week, e.g. `Mar 31`
pub fn compact_horizon(horizon: &sd_core::Horizon, now_year: i32) -> String {
    use sd_core::HorizonKind;
    match horizon.kind() {
        HorizonKind::Year(y) => format!("{}", y),
        HorizonKind::Month(y, m) => {
            let month_name = match m {
                1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
                5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
                9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
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
                1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
                5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
                9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
                _ => "???",
            };
            format!("{} {}", month_name, date.day())
        }
        HorizonKind::DateTime(dt) => {
            let month_name = match dt.month() {
                1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
                5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
                9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
                _ => "???",
            };
            format!("{} {}", month_name, dt.day())
        }
    }
}

/// Format a relative time distance as compact text.
///
/// Returns e.g. "just now", "2h ago", "3d ago", "2w ago", "3mo ago"
pub fn relative_time(from: chrono::DateTime<chrono::Utc>, to: chrono::DateTime<chrono::Utc>) -> String {
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

/// Gap bar: ████░░░░
pub fn gap_bar(magnitude: f64, width: usize) -> String {
    let filled = ((magnitude * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    )
}

// Separator constants
pub const LIGHT_RULE: char = '\u{2504}'; // ┄
pub const RULE: char = '\u{2500}';       // ─
pub const HEAVY_RULE: char = '\u{2501}'; // ━
