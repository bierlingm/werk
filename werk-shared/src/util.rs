//! Shared utility functions for werk.

use chrono::{DateTime, Utc};

/// Truncate a string to max length, adding ellipsis if needed (Unicode-safe).
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Human-readable tension identifier: short code (#N) if available, ULID prefix fallback.
pub fn display_id(short_code: Option<i32>, ulid: &str) -> String {
    match short_code {
        Some(c) => format!("#{}", c),
        None => ulid[..8.min(ulid.len())].to_string(),
    }
}

/// Format a datetime as a human-readable relative time string.
pub fn relative_time(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (now - dt).num_seconds().max(0);
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        let n = secs / 60;
        format!("{} min ago", n)
    } else if secs < 86400 {
        let n = secs / 3600;
        if n == 1 { "1 hour ago".to_string() } else { format!("{} hours ago", n) }
    } else if secs < 604800 {
        let n = secs / 86400;
        if n == 1 { "1 day ago".to_string() } else { format!("{} days ago", n) }
    } else {
        let n = secs / 604800;
        if n == 1 { "1 week ago".to_string() } else { format!("{} weeks ago", n) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    // === truncate tests ===

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world this is long", 10), "hello w...");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_truncate_unicode() {
        // Unicode characters should be counted by char, not byte
        assert_eq!(truncate("abcde", 5), "abcde");
        let result = truncate("abcdefghij", 5);
        assert_eq!(result, "ab...");
    }

    // === relative_time tests ===

    #[test]
    fn test_relative_time_just_now() {
        let now = Utc::now();
        assert_eq!(relative_time(now, now), "just now");
    }

    #[test]
    fn test_relative_time_seconds_ago() {
        let now = Utc::now();
        let dt = now - Duration::seconds(30);
        assert_eq!(relative_time(dt, now), "just now");
    }

    #[test]
    fn test_relative_time_minutes_ago() {
        let now = Utc::now();
        let dt = now - Duration::minutes(5);
        assert_eq!(relative_time(dt, now), "5 min ago");
    }

    #[test]
    fn test_relative_time_hours_ago() {
        let now = Utc::now();
        assert_eq!(relative_time(now - Duration::hours(1), now), "1 hour ago");
        assert_eq!(relative_time(now - Duration::hours(3), now), "3 hours ago");
    }

    #[test]
    fn test_relative_time_days_ago() {
        let now = Utc::now();
        assert_eq!(relative_time(now - Duration::days(1), now), "1 day ago");
        assert_eq!(relative_time(now - Duration::days(4), now), "4 days ago");
    }

    #[test]
    fn test_relative_time_weeks_ago() {
        let now = Utc::now();
        assert_eq!(relative_time(now - Duration::weeks(1), now), "1 week ago");
        assert_eq!(relative_time(now - Duration::weeks(2), now), "2 weeks ago");
    }

    #[test]
    fn test_relative_time_future_clamps_to_zero() {
        let now = Utc::now();
        let dt = now + Duration::hours(1);
        assert_eq!(relative_time(dt, now), "just now");
    }
}
