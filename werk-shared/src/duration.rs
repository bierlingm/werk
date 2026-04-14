//! Human duration parsing for CLI flags. Accepts:
//!
//! - Plain integer: `"14"` → 14 days
//! - Relative format (matches `werk recur`/`werk snooze`): `"+14d"`, `"+2w"`, `"+1m"`
//! - Named levels from the config registry: `"a week"`, `"two weeks"`, `"a month"`, etc.
//! - Bare names: `"week"`, `"month"`, `"day"`
//!
//! The parser normalizes everything to days (i64). Month = 30 days, week = 7.

/// Parse a human-or-numeric duration string to days. Returns a descriptive
/// error if the input doesn't match any accepted form.
pub fn parse_days(input: &str) -> Result<i64, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("duration cannot be empty".into());
    }

    // Plain integer.
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }

    // Relative format: +Nd / +Nw / +Nm.
    if let Some(rest) = s.strip_prefix('+') {
        if rest.len() >= 2 {
            let (num_str, unit) = rest.split_at(rest.len() - 1);
            if let Ok(n) = num_str.parse::<i64>() {
                return match unit {
                    "d" => Ok(n),
                    "w" => Ok(n * 7),
                    "m" => Ok(n * 30),
                    _ => Err(format!(
                        "unknown unit '{unit}': expected d (days), w (weeks), or m (months)"
                    )),
                };
            }
        }
    }

    // Named levels.
    match s.to_ascii_lowercase().as_str() {
        "today" | "day" => Ok(1),
        "a few days" => Ok(3),
        "a week" | "week" => Ok(7),
        "two weeks" => Ok(14),
        "a month" | "month" => Ok(30),
        "a quarter" | "quarter" => Ok(90),
        "a year" | "year" => Ok(365),
        _ => Err(format!(
            "unrecognized duration '{input}'. \
             Try a number (14), +Nd/+Nw/+Nm (+2w), or a name (a week, two weeks, a month)."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_integers() {
        assert_eq!(parse_days("14").unwrap(), 14);
        assert_eq!(parse_days("0").unwrap(), 0);
        assert_eq!(parse_days("365").unwrap(), 365);
    }

    #[test]
    fn relative_format() {
        assert_eq!(parse_days("+3d").unwrap(), 3);
        assert_eq!(parse_days("+2w").unwrap(), 14);
        assert_eq!(parse_days("+1m").unwrap(), 30);
        assert_eq!(parse_days("+12w").unwrap(), 84);
    }

    #[test]
    fn named_levels() {
        assert_eq!(parse_days("today").unwrap(), 1);
        assert_eq!(parse_days("a few days").unwrap(), 3);
        assert_eq!(parse_days("a week").unwrap(), 7);
        assert_eq!(parse_days("week").unwrap(), 7);
        assert_eq!(parse_days("two weeks").unwrap(), 14);
        assert_eq!(parse_days("a month").unwrap(), 30);
        assert_eq!(parse_days("month").unwrap(), 30);
    }

    #[test]
    fn case_insensitive_names() {
        assert_eq!(parse_days("A WEEK").unwrap(), 7);
        assert_eq!(parse_days("Two Weeks").unwrap(), 14);
    }

    #[test]
    fn invalid_returns_error() {
        assert!(parse_days("bogus").is_err());
        assert!(parse_days("+3x").is_err());
        assert!(parse_days("").is_err());
    }
}
