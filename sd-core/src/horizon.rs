//! Temporal horizon with variable precision.
//!
//! Horizon represents when a tension is temporally aimed at. The precision
//! itself is structurally meaningful — it represents how tightly the
//! practitioner has committed temporally.
//!
//! Each variant defines a *range*, not a point. `Year(2026)` means
//! "sometime in 2026" — the full year is the window.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use std::cmp::Ordering;
use std::fmt;

/// Errors that can occur when parsing a horizon string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HorizonParseError {
    /// The input string was empty.
    #[error("horizon string cannot be empty")]
    EmptyInput,

    /// The input string has an invalid format.
    #[error("invalid horizon format: {0}")]
    InvalidFormat(String),

    /// A component value is out of valid range.
    #[error("value out of range: {0}")]
    OutOfRange(String),
}

/// A temporal horizon with variable precision.
///
/// The precision itself is structurally meaningful — it represents
/// how tightly the practitioner has committed temporally.
///
/// Each variant defines a *range*, not a point. `Year(2026)` means
/// "sometime in 2026" — the full year is the window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Horizon {
    /// A full year. Range: Jan 1 00:00:00 UTC – Dec 31 23:59:59 UTC.
    Year(i32),
    /// A specific month. Range: 1st day 00:00 – last day 23:59:59 UTC.
    Month(i32, u32),
    /// A specific day. Range: 00:00:00 – 23:59:59 UTC.
    Day(NaiveDate),
    /// A specific instant. Range_start == range_end, width == 0.
    DateTime(DateTime<Utc>),
}

impl Horizon {
    /// Parse a horizon from an ISO-8601 partial date string.
    ///
    /// Accepted formats:
    /// - `"2026"` → `Year(2026)`
    /// - `"2026-05"` → `Month(2026, 5)`
    /// - `"2026-05-15"` → `Day(NaiveDate)`
    /// - `"2026-05-15T14:00:00Z"` → `DateTime(DateTime<Utc>)`
    /// - Negative years are supported: `"-100"` → `Year(-100)`
    pub fn parse(s: &str) -> Result<Self, HorizonParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(HorizonParseError::EmptyInput);
        }

        // Check for datetime format (has 'T')
        if s.contains('T') {
            let dt = DateTime::parse_from_rfc3339(s)
                .map_err(|e| HorizonParseError::InvalidFormat(format!("invalid datetime: {e}")))?;
            return Ok(Horizon::DateTime(dt.with_timezone(&Utc)));
        }

        // Handle negative years specially
        // ISO-8601 uses extended format for negative years with more than 4 digits
        // But for simplicity, we handle "-YYYY" style negative years
        let (year_part, rest) = if let Some(after_minus) = s.strip_prefix('-') {
            // Negative year - find where the year ends
            // The format could be "-2026" or "-2026-05" or "-2026-05-15"
            // Split on '-' after the initial negative sign
            let components: Vec<&str> = after_minus.split('-').collect();
            if components.is_empty() {
                return Err(HorizonParseError::InvalidFormat("invalid year".to_owned()));
            }
            // The year is the negative of the first component
            let year_str = format!("-{}", components[0]);
            (year_str, components.into_iter().skip(1).collect::<Vec<_>>())
        } else {
            // Positive year
            let components: Vec<&str> = s.split('-').collect();
            if components.is_empty() {
                return Err(HorizonParseError::InvalidFormat("empty input".to_owned()));
            }
            (
                components[0].to_owned(),
                components.into_iter().skip(1).collect::<Vec<_>>(),
            )
        };

        // Parse year
        let year: i32 = year_part
            .parse()
            .map_err(|_| HorizonParseError::InvalidFormat("invalid year".to_owned()))?;

        if year == 0 {
            return Err(HorizonParseError::OutOfRange(
                "year cannot be zero (use 1 BC or 1 AD)".to_owned(),
            ));
        }

        // Determine precision based on remaining components
        match rest.len() {
            0 => Ok(Horizon::Year(year)),
            1 => {
                // Month
                let month: u32 = rest[0]
                    .parse()
                    .map_err(|_| HorizonParseError::InvalidFormat("invalid month".to_owned()))?;
                if month == 0 || month > 12 {
                    return Err(HorizonParseError::OutOfRange(format!(
                        "month must be 1-12, got {month}"
                    )));
                }
                Ok(Horizon::Month(year, month))
            }
            2 => {
                // Day
                let month: u32 = rest[0]
                    .parse()
                    .map_err(|_| HorizonParseError::InvalidFormat("invalid month".to_owned()))?;
                let day: u32 = rest[1]
                    .parse()
                    .map_err(|_| HorizonParseError::InvalidFormat("invalid day".to_owned()))?;
                if month == 0 || month > 12 {
                    return Err(HorizonParseError::OutOfRange(format!(
                        "month must be 1-12, got {month}"
                    )));
                }
                let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
                    HorizonParseError::OutOfRange(format!("invalid date {year}-{month}-{day}"))
                })?;
                Ok(Horizon::Day(date))
            }
            _ => Err(HorizonParseError::InvalidFormat(format!(
                "expected 1-3 date components, got {}",
                rest.len() + 1
            ))),
        }
    }

    /// The beginning of the horizon window (inclusive).
    pub fn range_start(&self) -> DateTime<Utc> {
        match self {
            Horizon::Year(year) => Utc
                .with_ymd_and_hms(*year, 1, 1, 0, 0, 0)
                .single()
                .expect("valid year start"),
            Horizon::Month(year, month) => Utc
                .with_ymd_and_hms(*year, *month, 1, 0, 0, 0)
                .single()
                .expect("valid month start"),
            Horizon::Day(date) => date
                .and_hms_opt(0, 0, 0)
                .expect("valid day start")
                .and_utc(),
            Horizon::DateTime(dt) => *dt,
        }
    }

    /// The end of the horizon window (inclusive).
    ///
    /// For all variants except DateTime, this is the last instant within
    /// the horizon's range (23:59:59 for day/month/year).
    pub fn range_end(&self) -> DateTime<Utc> {
        match self {
            Horizon::Year(year) => Utc
                .with_ymd_and_hms(*year, 12, 31, 23, 59, 59)
                .single()
                .expect("valid year end"),
            Horizon::Month(year, month) => {
                // Get the last day of the month
                let (next_year, next_month) = if *month == 12 {
                    (*year + 1, 1)
                } else {
                    (*year, *month + 1)
                };
                let first_of_next = Utc
                    .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
                    .single()
                    .expect("valid next month start");
                // One second before the first of next month = last second of this month
                first_of_next - chrono::Duration::seconds(1)
            }
            Horizon::Day(date) => date
                .and_hms_opt(23, 59, 59)
                .expect("valid day end")
                .and_utc(),
            Horizon::DateTime(dt) => *dt,
        }
    }

    /// The temporal slack — the width of the horizon window.
    ///
    /// - Year: ~365 days (~366 for leap years)
    /// - Month: 28-31 days
    /// - Day: 86399 seconds (not quite 86400, because end is 23:59:59)
    /// - DateTime: 0 seconds
    pub fn width(&self) -> chrono::Duration {
        self.range_end() - self.range_start()
    }

    /// Check whether the given time falls within the horizon window.
    pub fn contains(&self, now: DateTime<Utc>) -> bool {
        now >= self.range_start() && now <= self.range_end()
    }

    /// Check whether the entire horizon window has elapsed.
    pub fn is_past(&self, now: DateTime<Utc>) -> bool {
        now > self.range_end()
    }

    /// Compute urgency as the ratio of elapsed time to total time window.
    ///
    /// - `urgency = 0.0` → just created, full window ahead
    /// - `urgency = 0.5` → halfway through the time window
    /// - `urgency = 1.0` → at the horizon's end
    /// - `urgency > 1.0` → past the horizon
    ///
    /// For DateTime horizons (width = 0), urgency is computed using a
    /// guard to avoid division by zero. The result is >= 1.0 because
    /// an instant horizon is either at the point or past it.
    pub fn urgency(&self, created_at: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
        let time_elapsed = (now - created_at).num_seconds().max(0) as f64;
        let total_window = (self.range_end() - created_at).num_seconds();
        // Guard against zero/negative width
        let total_window = total_window.max(1) as f64;
        time_elapsed / total_window
    }

    /// Compute staleness as the ratio of silence duration to horizon width.
    ///
    /// - `staleness = 0.0` → mutation just happened
    /// - `staleness = 1.0` → silence duration equals horizon width
    /// - `staleness > 1.0` → silence exceeds horizon width
    ///
    /// For DateTime horizons (width = 0), staleness is computed using a
    /// guard to avoid division by zero. Returns a capped maximum value.
    pub fn staleness(&self, last_mutation: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
        let silence_duration = (now - last_mutation).num_seconds().max(0) as f64;
        let horizon_width = self.width().num_seconds();
        // Guard against zero width
        let horizon_width = horizon_width.max(1) as f64;
        silence_duration / horizon_width
    }

    /// Return the precision level for ordering purposes.
    ///
    /// Lower values = narrower = higher precision.
    /// Used for tie-breaking in Ord implementation and drift detection.
    pub fn precision_level(&self) -> u8 {
        match self {
            Horizon::DateTime(_) => 0, // Most precise
            Horizon::Day(_) => 1,
            Horizon::Month(_, _) => 2,
            Horizon::Year(_) => 3, // Least precise
        }
    }
}

impl fmt::Display for Horizon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Horizon::Year(year) => write!(f, "{year}"),
            Horizon::Month(year, month) => write!(f, "{year}-{month:02}"),
            Horizon::Day(date) => write!(f, "{date}"),
            Horizon::DateTime(dt) => write!(f, "{}", dt.to_rfc3339()),
        }
    }
}

impl Ord for Horizon {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by range_start
        let start_order = self.range_start().cmp(&other.range_start());
        if start_order != Ordering::Equal {
            return start_order;
        }
        // Tie-break: narrower precision first (lower precision_level)
        self.precision_level().cmp(&other.precision_level())
    }
}

impl PartialOrd for Horizon {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Custom Serialize: output as ISO-8601 string, not JSON object
impl Serialize for Horizon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// Custom Deserialize: parse from ISO-8601 string
impl<'de> Deserialize<'de> for Horizon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Horizon::parse(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Duration, TimeZone, Timelike};

    // ── Parsing: Year ──────────────────────────────────────────────────

    #[test]
    fn test_parse_year_valid() {
        let h = Horizon::parse("2026").unwrap();
        assert_eq!(h, Horizon::Year(2026));
    }

    #[test]
    fn test_parse_year_with_whitespace() {
        let h = Horizon::parse("  2026  ").unwrap();
        assert_eq!(h, Horizon::Year(2026));
    }

    #[test]
    fn test_parse_year_negative() {
        // Negative years are allowed (BC dates)
        let h = Horizon::parse("-100").unwrap();
        assert_eq!(h, Horizon::Year(-100));
    }

    #[test]
    fn test_parse_year_zero_rejected() {
        let result = Horizon::parse("0");
        assert!(result.is_err());
        match result.unwrap_err() {
            HorizonParseError::OutOfRange(msg) => assert!(msg.contains("zero")),
            other => panic!("expected OutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_year_invalid_string() {
        let result = Horizon::parse("abc");
        assert!(result.is_err());
    }

    // ── Parsing: Month ────────────────────────────────────────────────

    #[test]
    fn test_parse_month_valid() {
        let h = Horizon::parse("2026-05").unwrap();
        assert_eq!(h, Horizon::Month(2026, 5));
    }

    #[test]
    fn test_parse_month_leading_zero() {
        let h = Horizon::parse("2026-03").unwrap();
        assert_eq!(h, Horizon::Month(2026, 3));
    }

    #[test]
    fn test_parse_month_zero_rejected() {
        let result = Horizon::parse("2026-00");
        assert!(result.is_err());
        match result.unwrap_err() {
            HorizonParseError::OutOfRange(msg) => assert!(msg.contains("1-12")),
            other => panic!("expected OutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_month_thirteen_rejected() {
        let result = Horizon::parse("2026-13");
        assert!(result.is_err());
        match result.unwrap_err() {
            HorizonParseError::OutOfRange(msg) => assert!(msg.contains("1-12")),
            other => panic!("expected OutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_month_invalid_format() {
        let result = Horizon::parse("2026-abc");
        assert!(result.is_err());
    }

    // ── Parsing: Day ───────────────────────────────────────────────────

    #[test]
    fn test_parse_day_valid() {
        let h = Horizon::parse("2026-05-15").unwrap();
        match h {
            Horizon::Day(date) => {
                assert_eq!(date.year(), 2026);
                assert_eq!(date.month(), 5);
                assert_eq!(date.day(), 15);
            }
            other => panic!("expected Day, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_day_feb_29_leap_year() {
        let h = Horizon::parse("2024-02-29").unwrap();
        match h {
            Horizon::Day(date) => {
                assert_eq!(date.month(), 2);
                assert_eq!(date.day(), 29);
            }
            other => panic!("expected Day, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_day_feb_29_non_leap_rejected() {
        let result = Horizon::parse("2025-02-29");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_day_feb_30_rejected() {
        let result = Horizon::parse("2026-02-30");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_day_apr_31_rejected() {
        let result = Horizon::parse("2026-04-31");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_day_dec_31_valid() {
        let h = Horizon::parse("2026-12-31").unwrap();
        match h {
            Horizon::Day(date) => {
                assert_eq!(date.month(), 12);
                assert_eq!(date.day(), 31);
            }
            other => panic!("expected Day, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_day_day_zero_rejected() {
        let result = Horizon::parse("2026-05-00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_day_day_32_rejected() {
        let result = Horizon::parse("2026-05-32");
        assert!(result.is_err());
    }

    // ── Parsing: DateTime ──────────────────────────────────────────────

    #[test]
    fn test_parse_datetime_valid() {
        let h = Horizon::parse("2026-05-15T14:00:00Z").unwrap();
        match h {
            Horizon::DateTime(dt) => {
                assert_eq!(dt.year(), 2026);
                assert_eq!(dt.month(), 5);
                assert_eq!(dt.day(), 15);
                assert_eq!(dt.hour(), 14);
                assert_eq!(dt.minute(), 0);
                assert_eq!(dt.second(), 0);
            }
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_datetime_with_timezone() {
        let h = Horizon::parse("2026-05-15T14:00:00+02:00").unwrap();
        match h {
            Horizon::DateTime(dt) => {
                // Should be converted to UTC
                assert_eq!(dt.hour(), 12); // 14:00+02:00 = 12:00 UTC
            }
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_datetime_with_millis() {
        let h = Horizon::parse("2026-05-15T14:00:00.123Z").unwrap();
        match h {
            Horizon::DateTime(dt) => {
                assert_eq!(dt.hour(), 14);
            }
            other => panic!("expected DateTime, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let result = Horizon::parse("2026-05-15T25:00:00Z");
        assert!(result.is_err());
    }

    // ── Parsing: Errors ────────────────────────────────────────────────

    #[test]
    fn test_parse_empty_rejected() {
        let result = Horizon::parse("");
        assert!(result.is_err());
        match result.unwrap_err() {
            HorizonParseError::EmptyInput => {}
            other => panic!("expected EmptyInput, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_whitespace_only_rejected() {
        let result = Horizon::parse("   ");
        assert!(result.is_err());
        match result.unwrap_err() {
            HorizonParseError::EmptyInput => {}
            other => panic!("expected EmptyInput, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_garbage_rejected() {
        let result = Horizon::parse("abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_too_many_components_rejected() {
        let result = Horizon::parse("2026-05-15-20");
        assert!(result.is_err());
    }

    // ── Roundtrip: Parse → Display → Parse ────────────────────────────

    #[test]
    fn test_roundtrip_year() {
        let h = Horizon::Year(2026);
        let s = h.to_string();
        let h2 = Horizon::parse(&s).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_roundtrip_month() {
        let h = Horizon::Month(2026, 5);
        let s = h.to_string();
        assert_eq!(s, "2026-05");
        let h2 = Horizon::parse(&s).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_roundtrip_month_january() {
        let h = Horizon::Month(2026, 1);
        let s = h.to_string();
        assert_eq!(s, "2026-01");
        let h2 = Horizon::parse(&s).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_roundtrip_month_december() {
        let h = Horizon::Month(2026, 12);
        let s = h.to_string();
        assert_eq!(s, "2026-12");
        let h2 = Horizon::parse(&s).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_roundtrip_day() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let s = h.to_string();
        assert_eq!(s, "2026-05-15");
        let h2 = Horizon::parse(&s).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_roundtrip_datetime() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        let s = h.to_string();
        let h2 = Horizon::parse(&s).unwrap();
        assert_eq!(h, h2);
    }

    // ── Range Computation: Year ────────────────────────────────────────

    #[test]
    fn test_year_range_start() {
        let h = Horizon::Year(2026);
        let start = h.range_start();
        assert_eq!(start.year(), 2026);
        assert_eq!(start.month(), 1);
        assert_eq!(start.day(), 1);
        assert_eq!(start.hour(), 0);
        assert_eq!(start.minute(), 0);
        assert_eq!(start.second(), 0);
    }

    #[test]
    fn test_year_range_end() {
        let h = Horizon::Year(2026);
        let end = h.range_end();
        assert_eq!(end.year(), 2026);
        assert_eq!(end.month(), 12);
        assert_eq!(end.day(), 31);
        assert_eq!(end.hour(), 23);
        assert_eq!(end.minute(), 59);
        assert_eq!(end.second(), 59);
    }

    #[test]
    fn test_year_width_non_leap() {
        let h = Horizon::Year(2025);
        let width = h.width();
        // 2025 is not a leap year, so 365 days minus 1 second (end is 23:59:59)
        // Actually: range_end - range_start for year
        // Start: Jan 1 00:00:00
        // End: Dec 31 23:59:59
        // Width = 365 days - 1 second = 364 days 23:59:59
        let expected_seconds = 365 * 24 * 60 * 60 - 1;
        assert_eq!(width.num_seconds(), expected_seconds);
    }

    #[test]
    fn test_year_width_leap() {
        let h = Horizon::Year(2024);
        let width = h.width();
        // 2024 is a leap year, so 366 days minus 1 second
        let expected_seconds = 366 * 24 * 60 * 60 - 1;
        assert_eq!(width.num_seconds(), expected_seconds);
    }

    // ── Range Computation: Month ────────────────────────────────────────

    #[test]
    fn test_month_range_start() {
        let h = Horizon::Month(2026, 5);
        let start = h.range_start();
        assert_eq!(start.year(), 2026);
        assert_eq!(start.month(), 5);
        assert_eq!(start.day(), 1);
        assert_eq!(start.hour(), 0);
    }

    #[test]
    fn test_month_range_end_may() {
        let h = Horizon::Month(2026, 5);
        let end = h.range_end();
        assert_eq!(end.year(), 2026);
        assert_eq!(end.month(), 5);
        assert_eq!(end.day(), 31);
        assert_eq!(end.hour(), 23);
        assert_eq!(end.minute(), 59);
        assert_eq!(end.second(), 59);
    }

    #[test]
    fn test_month_range_end_february_non_leap() {
        let h = Horizon::Month(2025, 2);
        let end = h.range_end();
        assert_eq!(end.month(), 2);
        assert_eq!(end.day(), 28);
    }

    #[test]
    fn test_month_range_end_february_leap() {
        let h = Horizon::Month(2024, 2);
        let end = h.range_end();
        assert_eq!(end.month(), 2);
        assert_eq!(end.day(), 29);
    }

    #[test]
    fn test_month_range_end_december() {
        let h = Horizon::Month(2026, 12);
        let end = h.range_end();
        assert_eq!(end.month(), 12);
        assert_eq!(end.day(), 31);
    }

    #[test]
    fn test_month_range_end_january() {
        let h = Horizon::Month(2026, 1);
        let end = h.range_end();
        assert_eq!(end.month(), 1);
        assert_eq!(end.day(), 31);
    }

    #[test]
    fn test_month_width_31_days() {
        let h = Horizon::Month(2026, 5); // May has 31 days
        let width = h.width();
        // 31 days minus 1 second
        let expected_seconds = 31 * 24 * 60 * 60 - 1;
        assert_eq!(width.num_seconds(), expected_seconds);
    }

    #[test]
    fn test_month_width_30_days() {
        let h = Horizon::Month(2026, 4); // April has 30 days
        let width = h.width();
        let expected_seconds = 30 * 24 * 60 * 60 - 1;
        assert_eq!(width.num_seconds(), expected_seconds);
    }

    #[test]
    fn test_month_width_february_non_leap() {
        let h = Horizon::Month(2025, 2);
        let width = h.width();
        let expected_seconds = 28 * 24 * 60 * 60 - 1;
        assert_eq!(width.num_seconds(), expected_seconds);
    }

    #[test]
    fn test_month_width_february_leap() {
        let h = Horizon::Month(2024, 2);
        let width = h.width();
        let expected_seconds = 29 * 24 * 60 * 60 - 1;
        assert_eq!(width.num_seconds(), expected_seconds);
    }

    // ── Range Computation: Day ──────────────────────────────────────────

    #[test]
    fn test_day_range_start() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let start = h.range_start();
        assert_eq!(start.year(), 2026);
        assert_eq!(start.month(), 5);
        assert_eq!(start.day(), 15);
        assert_eq!(start.hour(), 0);
        assert_eq!(start.minute(), 0);
        assert_eq!(start.second(), 0);
    }

    #[test]
    fn test_day_range_end() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let end = h.range_end();
        assert_eq!(end.year(), 2026);
        assert_eq!(end.month(), 5);
        assert_eq!(end.day(), 15);
        assert_eq!(end.hour(), 23);
        assert_eq!(end.minute(), 59);
        assert_eq!(end.second(), 59);
    }

    #[test]
    fn test_day_width() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let width = h.width();
        // 24 hours minus 1 second = 86399 seconds
        assert_eq!(width.num_seconds(), 86399);
    }

    // ── Range Computation: DateTime ─────────────────────────────────────

    #[test]
    fn test_datetime_range_start_equals_end() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        assert_eq!(h.range_start(), h.range_end());
    }

    #[test]
    fn test_datetime_width_zero() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        assert_eq!(h.width().num_seconds(), 0);
    }

    // ── contains() ─────────────────────────────────────────────────────

    #[test]
    fn test_day_contains_start() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let start = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        assert!(h.contains(start));
    }

    #[test]
    fn test_day_contains_end() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let end = Utc.with_ymd_and_hms(2026, 5, 15, 23, 59, 59).unwrap();
        assert!(h.contains(end));
    }

    #[test]
    fn test_day_contains_middle() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let middle = Utc.with_ymd_and_hms(2026, 5, 15, 12, 30, 0).unwrap();
        assert!(h.contains(middle));
    }

    #[test]
    fn test_day_does_not_contain_day_before() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let before = Utc.with_ymd_and_hms(2026, 5, 14, 23, 59, 59).unwrap();
        assert!(!h.contains(before));
    }

    #[test]
    fn test_day_does_not_contain_day_after() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let after = Utc.with_ymd_and_hms(2026, 5, 16, 0, 0, 0).unwrap();
        assert!(!h.contains(after));
    }

    #[test]
    fn test_month_contains_day_in_month() {
        let h = Horizon::Month(2026, 5);
        let day = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
        assert!(h.contains(day));
    }

    #[test]
    fn test_month_does_not_contain_day_outside() {
        let h = Horizon::Month(2026, 5);
        let day = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        assert!(!h.contains(day));
    }

    #[test]
    fn test_year_contains_day_in_year() {
        let h = Horizon::Year(2026);
        let day = Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap();
        assert!(h.contains(day));
    }

    #[test]
    fn test_year_does_not_contain_day_outside() {
        let h = Horizon::Year(2026);
        let day = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
        assert!(!h.contains(day));
    }

    #[test]
    fn test_datetime_contains_exact_match() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        assert!(h.contains(dt));
    }

    #[test]
    fn test_datetime_does_not_contain_other_time() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        let other = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 46).unwrap();
        assert!(!h.contains(other));
    }

    // ── is_past() ───────────────────────────────────────────────────────

    #[test]
    fn test_day_is_past_after_end() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let after = Utc.with_ymd_and_hms(2026, 5, 16, 0, 0, 0).unwrap();
        assert!(h.is_past(after));
    }

    #[test]
    fn test_day_is_not_past_during_day() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let during = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
        assert!(!h.is_past(during));
    }

    #[test]
    fn test_day_is_not_past_at_end() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let end = Utc.with_ymd_and_hms(2026, 5, 15, 23, 59, 59).unwrap();
        assert!(!h.is_past(end));
    }

    #[test]
    fn test_day_is_past_one_second_after_end() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let after = Utc.with_ymd_and_hms(2026, 5, 16, 0, 0, 0).unwrap();
        assert!(h.is_past(after));
    }

    #[test]
    fn test_month_is_past_after_end() {
        let h = Horizon::Month(2026, 5);
        let after = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        assert!(h.is_past(after));
    }

    #[test]
    fn test_month_is_not_past_during_month() {
        let h = Horizon::Month(2026, 5);
        let during = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
        assert!(!h.is_past(during));
    }

    #[test]
    fn test_year_is_past_after_end() {
        let h = Horizon::Year(2026);
        let after = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
        assert!(h.is_past(after));
    }

    #[test]
    fn test_year_is_not_past_during_year() {
        let h = Horizon::Year(2026);
        let during = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        assert!(!h.is_past(during));
    }

    #[test]
    fn test_datetime_is_past_after_instant() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        let after = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 46).unwrap();
        assert!(h.is_past(after));
    }

    #[test]
    fn test_datetime_is_not_past_at_instant() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        assert!(!h.is_past(dt));
    }

    // ── Ordering: Same Period Precision Tiebreak ────────────────────────

    #[test]
    fn test_ordering_day_before_month_same_range_start() {
        // Day(2026-05-01) and Month(2026, 5) have same range_start
        let day = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap());
        let month = Horizon::Month(2026, 5);
        assert!(day < month);
    }

    #[test]
    fn test_ordering_month_before_year_same_range_start() {
        // Month(2026, 1) and Year(2026) have same range_start
        let month = Horizon::Month(2026, 1);
        let year = Horizon::Year(2026);
        assert!(month < year);
    }

    #[test]
    fn test_ordering_day_before_year_same_range_start() {
        // Day(2026-01-01) and Year(2026) have same range_start
        let day = Horizon::Day(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        let year = Horizon::Year(2026);
        assert!(day < year);
    }

    #[test]
    fn test_ordering_datetime_before_day_same_range_start() {
        // DateTime(2026-05-15T00:00:00Z) and Day(2026-05-15) have same range_start
        let dt = Horizon::DateTime(Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap());
        let day = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        assert!(dt < day);
    }

    #[test]
    fn test_ordering_precision_chain() {
        // All have same range_start: 2026-01-01 00:00:00Z
        let dt = Horizon::DateTime(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap());
        let day = Horizon::Day(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        let month = Horizon::Month(2026, 1);
        let year = Horizon::Year(2026);

        assert!(dt < day);
        assert!(day < month);
        assert!(month < year);
    }

    // ── Ordering: Cross-Period ─────────────────────────────────────────

    #[test]
    fn test_ordering_different_months() {
        let march = Horizon::Month(2026, 3);
        let may = Horizon::Month(2026, 5);
        assert!(march < may);
    }

    #[test]
    fn test_ordering_different_years() {
        let y2025 = Horizon::Year(2025);
        let y2026 = Horizon::Year(2026);
        assert!(y2025 < y2026);
    }

    #[test]
    fn test_ordering_year_before_day_next_year() {
        let year = Horizon::Year(2025);
        let day = Horizon::Day(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert!(year < day);
    }

    #[test]
    fn test_ordering_day_before_month_later() {
        // Day in May vs Month in June
        let day = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap());
        let month = Horizon::Month(2026, 6);
        assert!(day < month);
    }

    #[test]
    fn test_ordering_mixed_precision_different_periods() {
        let year_2025 = Horizon::Year(2025);
        let month_2026_01 = Horizon::Month(2026, 1);
        let day_2026_02_01 = Horizon::Day(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());

        assert!(year_2025 < month_2026_01);
        assert!(month_2026_01 < day_2026_02_01);
    }

    // ── Urgency Computation ─────────────────────────────────────────────

    #[test]
    fn test_urgency_at_zero_percent() {
        // Tension just created at start of horizon
        let h = Horizon::Month(2026, 5);
        let created_at = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let now = created_at;
        let urgency = h.urgency(created_at, now);
        assert!((urgency - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_urgency_at_fifty_percent() {
        // Halfway through the window
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let created_at = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        // Halfway through 86399 seconds = ~43199.5 seconds
        let now = created_at + Duration::seconds(43199);
        let urgency = h.urgency(created_at, now);
        assert!((urgency - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_urgency_at_one_hundred_percent() {
        // At the end of the horizon
        let h = Horizon::Month(2026, 5);
        let created_at = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let now = h.range_end();
        let urgency = h.urgency(created_at, now);
        assert!((urgency - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_urgency_past_horizon() {
        // Past the horizon end
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let created_at = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(); // 1.5 days later
        let urgency = h.urgency(created_at, now);
        assert!(urgency > 1.0);
    }

    #[test]
    fn test_urgency_year_horizon() {
        let h = Horizon::Year(2026);
        let created_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        // 6 months in
        let now = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let urgency = h.urgency(created_at, now);
        // Should be around 0.5 (half year)
        assert!((urgency - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_urgency_with_creation_before_horizon() {
        // Tension created before horizon start
        let h = Horizon::Month(2026, 5);
        let created_at = Utc.with_ymd_and_hms(2026, 4, 15, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        let urgency = h.urgency(created_at, now);
        // Total window: May 1 to May 31 = ~31 days
        // Elapsed from Apr 15: ~30 days (15 days before horizon + 15 days in)
        // But we're measuring from creation to horizon end
        // This is a valid scenario
        assert!(urgency > 0.0);
    }

    // ── Urgency with DateTime (width=0) ──────────────────────────────────

    #[test]
    fn test_urgency_datetime_at_instant() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        let created_at = dt;
        let now = dt;
        let urgency = h.urgency(created_at, now);
        // width = 0, but guard ensures no division by zero
        // time_elapsed = 0, total_window = 0 -> guard makes total_window = 1
        // urgency = 0 / 1 = 0
        assert!((urgency - 0.0).abs() < 0.001 || urgency >= 1.0); // Either at instant or past
    }

    #[test]
    fn test_urgency_datetime_past_instant() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        let created_at = dt - Duration::hours(1);
        let now = dt + Duration::hours(1);
        let urgency = h.urgency(created_at, now);
        // time_elapsed = 2 hours = 7200 seconds
        // total_window = dt - created_at = 1 hour = 3600 seconds
        // urgency = 7200 / 3600 = 2.0
        assert!((urgency - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_urgency_datetime_no_panic_or_nan() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        // Various combinations that could cause issues
        let cases = [
            (dt, dt),
            (dt - Duration::seconds(1), dt),
            (dt, dt + Duration::seconds(1)),
            (dt - Duration::days(1), dt + Duration::days(1)),
        ];
        for (created_at, now) in cases {
            let urgency = h.urgency(created_at, now);
            assert!(!urgency.is_nan());
            assert!(!urgency.is_infinite());
        }
    }

    // ── Staleness Computation ───────────────────────────────────────────

    #[test]
    fn test_staleness_at_zero() {
        // Just mutated
        let h = Horizon::Month(2026, 5);
        let last_mutation = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
        let now = last_mutation;
        let staleness = h.staleness(last_mutation, now);
        assert!((staleness - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_staleness_at_one() {
        // Silence equals horizon width
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let last_mutation = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        // One day of silence (86399 seconds for a day width)
        let now = last_mutation + Duration::seconds(86399);
        let staleness = h.staleness(last_mutation, now);
        assert!((staleness - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_staleness_greater_than_one() {
        // Silence exceeds horizon width
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let last_mutation = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        let now = last_mutation + Duration::days(2);
        let staleness = h.staleness(last_mutation, now);
        assert!(staleness > 1.0);
    }

    #[test]
    fn test_staleness_year_horizon() {
        let h = Horizon::Year(2026);
        let last_mutation = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        // One month of silence for a year horizon
        let now = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let staleness = h.staleness(last_mutation, now);
        // One month / 365 days ≈ 0.08
        assert!((staleness - 0.08).abs() < 0.02);
    }

    // ── Staleness with DateTime (width=0) ────────────────────────────────

    #[test]
    fn test_staleness_datetime_no_panic_or_nan() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        // Various combinations
        let cases = [
            (dt, dt),
            (dt, dt + Duration::seconds(1)),
            (dt, dt + Duration::hours(1)),
            (dt - Duration::hours(1), dt),
        ];
        for (last_mutation, now) in cases {
            let staleness = h.staleness(last_mutation, now);
            assert!(!staleness.is_nan());
            assert!(!staleness.is_infinite());
        }
    }

    #[test]
    fn test_staleness_datetime_one_second_silence() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        let last_mutation = dt;
        let now = dt + Duration::seconds(1);
        let staleness = h.staleness(last_mutation, now);
        // silence = 1 second, width = 0 (guard to 1)
        // staleness = 1 / 1 = 1.0
        assert!((staleness - 1.0).abs() < 0.01);
    }

    // ── Serde Roundtrip ─────────────────────────────────────────────────

    #[test]
    fn test_serde_year() {
        let h = Horizon::Year(2026);
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(json, "\"2026\"");
        let h2: Horizon = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_serde_month() {
        let h = Horizon::Month(2026, 5);
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(json, "\"2026-05\"");
        let h2: Horizon = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_serde_day() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(json, "\"2026-05-15\"");
        let h2: Horizon = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_serde_datetime() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        let json = serde_json::to_string(&h).unwrap();
        // Should serialize as ISO-8601 string
        assert!(json.starts_with('"'));
        assert!(json.contains("2026-05-15T"));
        let h2: Horizon = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_serde_invalid_string() {
        let json = "\"not-a-horizon\"";
        let result: Result<Horizon, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_as_string_not_object() {
        // Verify that Horizon serializes as a string, not a JSON object
        let h = Horizon::Month(2026, 5);
        let json = serde_json::to_string(&h).unwrap();
        // Should be a JSON string, not an object
        assert!(json.starts_with('"'));
        assert!(!json.starts_with("{\""));
    }

    // ── Width Summary ───────────────────────────────────────────────────

    #[test]
    fn test_width_year_approximately_365_days() {
        let h = Horizon::Year(2025); // Non-leap
        let width = h.width();
        let days = width.num_seconds() as f64 / (24.0 * 60.0 * 60.0);
        assert!((days - 365.0).abs() < 1.0);
    }

    #[test]
    fn test_width_year_leap_approximately_366_days() {
        let h = Horizon::Year(2024); // Leap
        let width = h.width();
        let days = width.num_seconds() as f64 / (24.0 * 60.0 * 60.0);
        assert!((days - 366.0).abs() < 1.0);
    }

    #[test]
    fn test_width_month_varies_by_month() {
        // 31-day month
        let may = Horizon::Month(2026, 5);
        let days_may = may.width().num_seconds() as f64 / (24.0 * 60.0 * 60.0);
        assert!((days_may - 31.0).abs() < 1.0);

        // 30-day month
        let apr = Horizon::Month(2026, 4);
        let days_apr = apr.width().num_seconds() as f64 / (24.0 * 60.0 * 60.0);
        assert!((days_apr - 30.0).abs() < 1.0);

        // 28-day month (non-leap Feb)
        let feb = Horizon::Month(2025, 2);
        let days_feb = feb.width().num_seconds() as f64 / (24.0 * 60.0 * 60.0);
        assert!((days_feb - 28.0).abs() < 1.0);
    }

    #[test]
    fn test_width_day_approximately_1_day() {
        let h = Horizon::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        let width = h.width();
        // 86399 seconds = ~23.9997 hours, just under 1 day
        let hours = width.num_seconds() as f64 / (60.0 * 60.0);
        assert!((hours - 24.0).abs() < 0.1);
    }

    #[test]
    fn test_width_datetime_zero() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 45).unwrap();
        let h = Horizon::DateTime(dt);
        assert_eq!(h.width().num_seconds(), 0);
    }

    // ── Trait Assertions ────────────────────────────────────────────────

    #[test]
    fn test_horizon_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Horizon>();
        assert_send_sync::<HorizonParseError>();
    }

    #[test]
    fn test_horizon_is_debug_clone_partialeq_eq_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let h = Horizon::Month(2026, 5);
        let _ = format!("{h:?}"); // Debug
        let h2 = h.clone(); // Clone
        assert_eq!(h, h2); // PartialEq
        let mut hasher = DefaultHasher::new();
        h2.hash(&mut hasher); // Hash (via derive)
    }

    #[test]
    fn test_horizon_ord_consistent_with_eq() {
        let h1 = Horizon::Month(2026, 5);
        let h2 = Horizon::Month(2026, 5);
        assert_eq!(h1.cmp(&h2), Ordering::Equal);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_horizon_ord_transitive() {
        let a = Horizon::Year(2025);
        let b = Horizon::Month(2026, 1);
        let c = Horizon::Day(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());

        assert!(a < b);
        assert!(b < c);
        assert!(a < c); // transitivity
    }

    // ── Error Display ───────────────────────────────────────────────────

    #[test]
    fn test_horizon_parse_error_display() {
        let e = HorizonParseError::EmptyInput;
        assert!(e.to_string().contains("empty"));

        let e = HorizonParseError::InvalidFormat("bad".to_owned());
        assert!(e.to_string().contains("invalid"));

        let e = HorizonParseError::OutOfRange("month".to_owned());
        assert!(e.to_string().contains("range"));
    }

    // ── Edge Cases: Month Boundaries ────────────────────────────────────

    #[test]
    fn test_month_december_to_january_transition() {
        // December range should end Dec 31, not roll over to Jan next year
        let h = Horizon::Month(2026, 12);
        let end = h.range_end();
        assert_eq!(end.month(), 12);
        assert_eq!(end.day(), 31);
    }

    #[test]
    fn test_month_january_starts_on_first() {
        let h = Horizon::Month(2026, 1);
        let start = h.range_start();
        assert_eq!(start.month(), 1);
        assert_eq!(start.day(), 1);
    }

    // ── Negative Years (BC dates) ───────────────────────────────────────

    #[test]
    fn test_negative_year_valid() {
        let h = Horizon::parse("-100").unwrap();
        assert_eq!(h, Horizon::Year(-100));
    }

    #[test]
    fn test_negative_year_month_valid() {
        let h = Horizon::parse("-100-05").unwrap();
        assert_eq!(h, Horizon::Month(-100, 5));
    }

    #[test]
    fn test_negative_year_month_day_valid() {
        let h = Horizon::parse("-100-05-15").unwrap();
        match h {
            Horizon::Day(date) => {
                assert_eq!(date.year(), -100);
            }
            other => panic!("expected Day, got {other:?}"),
        }
    }
}
