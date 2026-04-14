//! Natural language horizon parsing.
//!
//! Wraps `werk_core::Horizon::parse()` with additional support for relative dates
//! and human-friendly expressions like "tomorrow", "in 3 days", "eom", etc.

use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use werk_core::Horizon;

/// Parse a horizon string, trying ISO formats first, then natural language.
///
/// ISO formats (delegated to `Horizon::parse`): "2026", "2026-03", "2026-03-15"
///
/// Natural language formats:
/// - Relative words: `today`, `tomorrow`, `next week`, `next month`, `next friday`
/// - Relative durations: `in 3 days`, `in 2 weeks`, `in 1 month`, `3 days`, `2 weeks`, `3d`, `2w`, `1m`, `1y`
/// - Plus-prefixed relative: `+3d`, `+2w`, `+3m`, `+1y`
/// - End-of-period: `eow`, `friday`, `eom`, `end of month`, `eoq`, `end of quarter`, `eoy`, `end of year`
pub fn parse_horizon(input: &str) -> Result<Horizon, String> {
    let trimmed = input.trim();

    // Try ISO format first.
    if let Ok(h) = Horizon::parse(trimmed) {
        return Ok(h);
    }

    let lower = trimmed.to_lowercase();
    let now = Utc::now();
    let today = now.date_naive();

    // Try hour/minute expressions first (these need DateTime precision)
    if let Some(dt) = parse_time_duration(&lower, now) {
        return Ok(Horizon::new_datetime(dt));
    }

    parse_natural(&lower, today)
}

/// Parse hour/minute duration expressions: "2h", "30min", "1h30m", "in 2 hours", etc.
fn parse_time_duration(input: &str, now: chrono::DateTime<Utc>) -> Option<chrono::DateTime<Utc>> {
    let input = input.strip_prefix("in ").unwrap_or(input).trim();
    let input = input.strip_prefix('+').unwrap_or(input);

    // Compact: "2h", "30min", "90m"
    if input.ends_with("min") {
        let num = input.strip_suffix("min")?.parse::<i64>().ok()?;
        return Some(now + Duration::minutes(num));
    }
    if input.ends_with('h') && !input.ends_with("th") {
        let num = input.strip_suffix('h')?.parse::<i64>().ok()?;
        return Some(now + Duration::hours(num));
    }

    // Word form: "2 hours", "30 minutes"
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() == 2 {
        if let Ok(n) = parts[0].parse::<i64>() {
            let unit = parts[1].trim_end_matches('s');
            match unit {
                "hour" => return Some(now + Duration::hours(n)),
                "minute" | "min" => return Some(now + Duration::minutes(n)),
                _ => {}
            }
        }
    }

    None
}

fn parse_natural(input: &str, today: NaiveDate) -> Result<Horizon, String> {
    // Simple relative words
    match input {
        "today" => return horizon_day(today),
        "tomorrow" => return horizon_day(today + Duration::days(1)),
        "next week" => {
            // Next Monday
            let days_until_monday = (Weekday::Mon.num_days_from_monday() as i64
                + 7
                - today.weekday().num_days_from_monday() as i64)
                % 7;
            let days = if days_until_monday == 0 { 7 } else { days_until_monday };
            return horizon_day(today + Duration::days(days));
        }
        "next month" => {
            let (y, m) = next_month(today.year(), today.month());
            return Horizon::new_month(y, m).map_err(|e| e.to_string());
        }
        "eow" | "friday" => {
            // End of this week = Friday (today if already Friday)
            let today_wd = today.weekday().num_days_from_monday(); // Mon=0 .. Sun=6
            let friday_wd = Weekday::Fri.num_days_from_monday(); // 4
            let days_ahead = (friday_wd as i64 - today_wd as i64 + 7) % 7;
            // If today is already Friday, days_ahead == 0 means today
            return horizon_day(today + Duration::days(days_ahead));
        }
        "eom" | "end of month" => {
            return horizon_end_of_month(today);
        }
        "eoq" | "end of quarter" => {
            return horizon_end_of_quarter(today);
        }
        "eoy" | "end of year" => {
            return Horizon::new_year(today.year()).map_err(|e| e.to_string());
        }
        _ => {}
    }

    // Month names: "may", "january", "oct", etc. → month precision in current/next year
    if let Some(month_num) = parse_month_name(input) {
        let year = if month_num < today.month() {
            today.year() + 1 // month already passed → next year
        } else {
            today.year()
        };
        return Horizon::new_month(year, month_num).map_err(|e| e.to_string());
    }

    // "next <weekday>"
    if let Some(rest) = input.strip_prefix("next ") {
        if let Some(wd) = parse_weekday(rest.trim()) {
            let target = next_occurrence(today, wd);
            return horizon_day(target);
        }
        // "next <month>"
        if let Some(month_num) = parse_month_name(rest.trim()) {
            let year = if month_num <= today.month() {
                today.year() + 1
            } else {
                today.year()
            };
            return Horizon::new_month(year, month_num).map_err(|e| e.to_string());
        }
    }

    // "+Nd", "+Nw", "+Nm", "+Ny" compact relative format
    if let Some(rest) = input.strip_prefix('+') {
        if let Some(date) = parse_duration_expr(rest, today) {
            return horizon_from_duration(date, rest);
        }
    }

    // "in N days/weeks/months" or "N days/weeks/months" or compact "3d", "2w", "1m"
    if let Some(rest) = input.strip_prefix("in ") {
        if let Some(date) = parse_duration_expr(rest.trim(), today) {
            return horizon_from_duration(date, rest.trim());
        }
    }

    // Without "in" prefix
    if let Some(date) = parse_duration_expr(input, today) {
        return horizon_from_duration(date, input);
    }

    Err(format!("unrecognized horizon format: '{input}'"))
}

/// Parse expressions like "3 days", "2 weeks", "1 month", "3d", "2w", "1m".
fn parse_duration_expr(input: &str, today: NaiveDate) -> Option<NaiveDate> {
    // Try compact form: "3d", "2w", "1m"
    if input.len() >= 2 {
        let (num_part, unit_char) = input.split_at(input.len() - 1);
        if let Ok(n) = num_part.parse::<i64>() {
            match unit_char {
                "d" => return Some(today + Duration::days(n)),
                "w" => return Some(today + Duration::weeks(n)),
                "m" => return Some(add_months(today, n as u32)),
                "y" => return Some(add_months(today, n as u32 * 12)),
                _ => {}
            }
        }
    }

    // Try "N days/weeks/months" form
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() == 2 {
        if let Ok(n) = parts[0].parse::<i64>() {
            let unit = parts[1].trim_end_matches('s'); // normalize plural
            match unit {
                "day" => return Some(today + Duration::days(n)),
                "week" => return Some(today + Duration::weeks(n)),
                "month" => return Some(add_months(today, n as u32)),
                _ => {}
            }
        }
    }

    None
}

/// Choose Day, Month, or Year precision depending on the unit used.
fn horizon_from_duration(date: NaiveDate, expr: &str) -> Result<Horizon, String> {
    let unit = expr.split_whitespace().last().unwrap_or(expr);

    // Year precision for year-based durations
    {
        let is_compact_year = unit.len() >= 2
            && unit.ends_with('y')
            && unit[..unit.len() - 1].parse::<i64>().is_ok();
        if unit == "year" || unit == "years" || is_compact_year {
            return Horizon::new_year(date.year()).map_err(|e| e.to_string());
        }
    }

    // Month precision for month-based durations
    if unit == "month" || unit == "months" || unit.ends_with('m') && !unit.ends_with("dm") {
        // Only use month precision if the compact form is just digits + 'm'
        let is_compact_month = unit.len() >= 2
            && unit.ends_with('m')
            && unit[..unit.len() - 1].parse::<i64>().is_ok();
        if unit == "month" || unit == "months" || is_compact_month {
            return Horizon::new_month(date.year(), date.month()).map_err(|e| e.to_string());
        }
    }
    horizon_day(date)
}

fn horizon_day(date: NaiveDate) -> Result<Horizon, String> {
    Horizon::new_day(date.year(), date.month(), date.day()).map_err(|e| e.to_string())
}

fn horizon_end_of_month(today: NaiveDate) -> Result<Horizon, String> {
    // Return month-precision for the current month.
    Horizon::new_month(today.year(), today.month()).map_err(|e| e.to_string())
}

fn horizon_end_of_quarter(today: NaiveDate) -> Result<Horizon, String> {
    // Quarter end months: 3, 6, 9, 12
    let quarter_end_month = ((today.month() - 1) / 3 + 1) * 3;
    Horizon::new_month(today.year(), quarter_end_month).map_err(|e| e.to_string())
}

fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    let total_months = date.year() * 12 + date.month() as i32 - 1 + months as i32;
    let new_year = total_months / 12;
    let new_month = (total_months % 12 + 1) as u32;
    // Clamp day to valid range for the target month
    let max_day = NaiveDate::from_ymd_opt(new_year, new_month + 1, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(new_year + 1, 1, 1).unwrap()) // ubs:ignore year+1 is valid for any realistic date
        .pred_opt()
        .unwrap() // ubs:ignore pred of a valid date is always valid (not MIN)
        .day();
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap() // ubs:ignore day clamped to valid range above
}

fn next_occurrence(today: NaiveDate, target: Weekday) -> NaiveDate {
    let today_wd = today.weekday().num_days_from_monday();
    let target_wd = target.num_days_from_monday();
    let days_ahead = (target_wd as i64 - today_wd as i64 + 7) % 7;
    // If it's the same weekday, go to next week
    let days = if days_ahead == 0 { 7 } else { days_ahead };
    today + Duration::days(days)
}

fn parse_month_name(s: &str) -> Option<u32> {
    match s {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s {
        "monday" | "mon" => Some(Weekday::Mon),
        "tuesday" | "tue" | "tues" => Some(Weekday::Tue),
        "wednesday" | "wed" => Some(Weekday::Wed),
        "thursday" | "thu" | "thur" | "thurs" => Some(Weekday::Thu),
        "friday" | "fri" => Some(Weekday::Fri),
        "saturday" | "sat" => Some(Weekday::Sat),
        "sunday" | "sun" => Some(Weekday::Sun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_passthrough() {
        assert!(parse_horizon("2026").is_ok());
        assert!(parse_horizon("2026-03").is_ok());
        assert!(parse_horizon("2026-03-15").is_ok());
    }

    #[test]
    fn relative_words() {
        // These should all succeed (we can't assert exact dates since they depend on now)
        assert!(parse_horizon("today").is_ok());
        assert!(parse_horizon("tomorrow").is_ok());
        assert!(parse_horizon("next week").is_ok());
        assert!(parse_horizon("next month").is_ok());
    }

    #[test]
    fn weekday_names() {
        assert!(parse_horizon("next monday").is_ok());
        assert!(parse_horizon("next Friday").is_ok());
        assert!(parse_horizon("next sun").is_ok());
    }

    #[test]
    fn duration_expressions() {
        assert!(parse_horizon("in 3 days").is_ok());
        assert!(parse_horizon("in 2 weeks").is_ok());
        assert!(parse_horizon("in 1 month").is_ok());
        assert!(parse_horizon("3 days").is_ok());
        assert!(parse_horizon("2 weeks").is_ok());
        assert!(parse_horizon("3d").is_ok());
        assert!(parse_horizon("2w").is_ok());
        assert!(parse_horizon("1m").is_ok());
    }

    #[test]
    fn end_of_period() {
        assert!(parse_horizon("eom").is_ok());
        assert!(parse_horizon("end of month").is_ok());
        assert!(parse_horizon("eoq").is_ok());
        assert!(parse_horizon("end of quarter").is_ok());
        assert!(parse_horizon("eoy").is_ok());
        assert!(parse_horizon("end of year").is_ok());
    }

    #[test]
    fn case_insensitive() {
        assert!(parse_horizon("Tomorrow").is_ok());
        assert!(parse_horizon("NEXT WEEK").is_ok());
        assert!(parse_horizon("EOM").is_ok());
        assert!(parse_horizon("In 3 Days").is_ok());
    }

    #[test]
    fn plus_prefixed_relative() {
        assert!(parse_horizon("+1d").is_ok());
        assert!(parse_horizon("+3d").is_ok());
        assert!(parse_horizon("+14d").is_ok());
        assert!(parse_horizon("+2w").is_ok());
        assert!(parse_horizon("+3m").is_ok());
        assert!(parse_horizon("+6m").is_ok());
        assert!(parse_horizon("+1y").is_ok());
    }

    #[test]
    fn named_shortcuts() {
        assert!(parse_horizon("eow").is_ok());
        assert!(parse_horizon("friday").is_ok());
        assert!(parse_horizon("eom").is_ok());
        assert!(parse_horizon("eoq").is_ok());
        assert!(parse_horizon("eoy").is_ok());
    }

    #[test]
    fn zero_offset_edge_cases() {
        assert!(parse_horizon("+0d").is_ok());
        assert!(parse_horizon("+0w").is_ok());
    }

    #[test]
    fn plus_prefixed_relative_with_known_date() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 14).unwrap(); // Saturday

        // +1d => 2026-03-15
        let h = parse_natural("+1d", today).unwrap();
        assert_eq!(h.to_string(), "2026-03-15");

        // +2w => 2026-03-28
        let h = parse_natural("+2w", today).unwrap();
        assert_eq!(h.to_string(), "2026-03-28");

        // +3m => 2026-06 (month precision)
        let h = parse_natural("+3m", today).unwrap();
        assert_eq!(h.to_string(), "2026-06");

        // +1y => 2027 (year precision)
        let h = parse_natural("+1y", today).unwrap();
        assert_eq!(h.to_string(), "2027");

        // +0d => today
        let h = parse_natural("+0d", today).unwrap();
        assert_eq!(h.to_string(), "2026-03-14");

        // +0w => today
        let h = parse_natural("+0w", today).unwrap();
        assert_eq!(h.to_string(), "2026-03-14");
    }

    #[test]
    fn eow_on_different_weekdays() {
        // On a Monday, eow => that Friday
        let monday = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        let h = parse_natural("eow", monday).unwrap();
        assert_eq!(h.to_string(), "2026-03-13"); // Friday

        // On a Friday, eow => today (that Friday)
        let friday = NaiveDate::from_ymd_opt(2026, 3, 13).unwrap();
        let h = parse_natural("eow", friday).unwrap();
        assert_eq!(h.to_string(), "2026-03-13");

        // On a Saturday, eow => next Friday
        let saturday = NaiveDate::from_ymd_opt(2026, 3, 14).unwrap();
        let h = parse_natural("eow", saturday).unwrap();
        assert_eq!(h.to_string(), "2026-03-20");

        // "friday" should behave the same as "eow"
        let h = parse_natural("friday", monday).unwrap();
        assert_eq!(h.to_string(), "2026-03-13");
    }

    #[test]
    fn eoq_quarters() {
        // Q1: Jan => end of March
        let jan = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let h = parse_natural("eoq", jan).unwrap();
        assert_eq!(h.to_string(), "2026-03");

        // Q2: Apr => end of June
        let apr = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let h = parse_natural("eoq", apr).unwrap();
        assert_eq!(h.to_string(), "2026-06");

        // Q4: Dec => end of December
        let dec = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        let h = parse_natural("eoq", dec).unwrap();
        assert_eq!(h.to_string(), "2026-12");
    }

    #[test]
    fn month_names() {
        assert!(parse_horizon("May").is_ok());
        assert!(parse_horizon("january").is_ok());
        assert!(parse_horizon("oct").is_ok());
        assert!(parse_horizon("December").is_ok());
        assert!(parse_horizon("next may").is_ok());
        assert!(parse_horizon("next January").is_ok());
        // Month name gives month precision
        let h = parse_natural("may", NaiveDate::from_ymd_opt(2026, 3, 25).unwrap()).unwrap();
        assert_eq!(h.to_string(), "2026-05");
        // Past month → next year
        let h = parse_natural("jan", NaiveDate::from_ymd_opt(2026, 3, 25).unwrap()).unwrap();
        assert_eq!(h.to_string(), "2027-01");
        // Current month → this year
        let h = parse_natural("mar", NaiveDate::from_ymd_opt(2026, 3, 25).unwrap()).unwrap();
        assert_eq!(h.to_string(), "2026-03");
    }

    #[test]
    fn invalid_input() {
        assert!(parse_horizon("gibberish").is_err());
        assert!(parse_horizon("").is_err());
    }
}
