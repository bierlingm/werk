//! Diff command handler — shows what changed in a time window.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use serde::Serialize;
use std::collections::BTreeMap;
use werk_shared::{display_id, relative_time, truncate};

/// Parse a human-friendly `--since` value into a `DateTime<Utc>`.
///
/// Supported formats:
///   - "today"             -> start of today (midnight UTC)
///   - "yesterday"         -> start of yesterday
///   - "N days ago"        -> N days before now at midnight UTC
///   - "2026-03-10"        -> ISO date at midnight UTC
///   - "monday" … "sunday" -> most recent occurrence of that weekday
fn parse_since(value: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, WerkError> {
    let v = value.trim().to_lowercase();

    // "today"
    if v == "today" {
        return Ok(start_of_day(now));
    }

    // "yesterday"
    if v == "yesterday" {
        let yesterday = now - chrono::Duration::days(1);
        return Ok(start_of_day(yesterday));
    }

    // "N days ago"
    if let Some(rest) = v.strip_suffix(" days ago") {
        let n: i64 = rest
            .trim()
            .parse()
            .map_err(|_| WerkError::InvalidInput(format!("invalid number in '{}'", value)))?;
        let past = now - chrono::Duration::days(n);
        return Ok(start_of_day(past));
    }

    // "1 day ago"
    if v == "1 day ago" {
        let past = now - chrono::Duration::days(1);
        return Ok(start_of_day(past));
    }

    // Weekday names
    if let Some(target_weekday) = parse_weekday(&v) {
        let today_weekday = now.weekday();
        let days_back = days_since_weekday(today_weekday, target_weekday);
        let target_date = now - chrono::Duration::days(days_back as i64);
        return Ok(start_of_day(target_date));
    }

    // ISO date "YYYY-MM-DD"
    if let Ok(date) = NaiveDate::parse_from_str(&v, "%Y-%m-%d") {
        let dt = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| WerkError::InvalidInput(format!("invalid date: {}", value)))?;
        return Ok(dt.and_utc());
    }

    Err(WerkError::InvalidInput(format!(
        "unrecognized --since value: '{}'. Try 'today', 'yesterday', '3 days ago', '2026-03-10', or a weekday name.",
        value
    )))
}

fn start_of_day(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|naive| naive.and_utc())
        .unwrap_or(dt)
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s {
        "monday" | "mon" => Some(Weekday::Mon),
        "tuesday" | "tue" => Some(Weekday::Tue),
        "wednesday" | "wed" => Some(Weekday::Wed),
        "thursday" | "thu" => Some(Weekday::Thu),
        "friday" | "fri" => Some(Weekday::Fri),
        "saturday" | "sat" => Some(Weekday::Sat),
        "sunday" | "sun" => Some(Weekday::Sun),
        _ => None,
    }
}

/// How many days back from `from` to the most recent `target` weekday.
/// If today IS the target weekday, returns 0 (i.e., "since start of today").
fn days_since_weekday(from: Weekday, target: Weekday) -> u32 {
    let from_num = from.num_days_from_monday();
    let target_num = target.num_days_from_monday();
    if from_num >= target_num {
        from_num - target_num
    } else {
        7 - (target_num - from_num)
    }
}

// ── JSON output structures ──────────────────────────────────────

#[derive(Serialize)]
struct DiffOutput {
    since: String,
    changes: Vec<TensionChanges>,
    summary: DiffSummary,
}

#[derive(Serialize)]
struct TensionChanges {
    tension_id: String,
    tension_desired: String,
    mutations: Vec<MutationInfo>,
}

#[derive(Serialize)]
struct MutationInfo {
    timestamp: String,
    field: String,
    old_value: Option<String>,
    new_value: String,
}

#[derive(Serialize)]
struct DiffSummary {
    updated: usize,
    created: usize,
    resolved: usize,
}

// ── Command implementation ──────────────────────────────────────

pub fn cmd_diff(output: &Output, since: String) -> Result<(), WerkError> {
    let now = Utc::now();
    let since_dt = parse_since(&since, now)?;

    // Discover workspace and get store
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get mutations in the time window
    let mutations = store
        .mutations_between(since_dt, now)
        .map_err(WerkError::StoreError)?;

    if mutations.is_empty() {
        if output.is_structured() {
            let result = DiffOutput {
                since: since_dt.to_rfc3339(),
                changes: vec![],
                summary: DiffSummary {
                    updated: 0,
                    created: 0,
                    resolved: 0,
                },
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            let since_label = format_since_label(since_dt);
            println!("Changes since {}:", since_label);
            println!();
            println!("  (no changes)");
            println!();
            println!("Summary: 0 tensions updated, 0 created, 0 resolved");
        }
        return Ok(());
    }

    // Group mutations by tension_id (preserving order with BTreeMap)
    let mut grouped: BTreeMap<String, Vec<&sd_core::Mutation>> = BTreeMap::new();
    for m in &mutations {
        grouped
            .entry(m.tension_id().to_owned())
            .or_default()
            .push(m);
    }

    // Get all tensions for label lookup
    let all_tensions = store
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    let tension_map: std::collections::HashMap<String, &sd_core::Tension> = all_tensions
        .iter()
        .map(|t| (t.id.clone(), t))
        .collect();

    // Build output structures
    let mut changes: Vec<TensionChanges> = Vec::new();
    let mut created_count = 0usize;
    let mut resolved_count = 0usize;
    let mut updated_count = 0usize;

    for (tid, muts) in &grouped {
        let desired_label = tension_map
            .get(tid)
            .map(|t| t.desired.clone())
            .unwrap_or_else(|| "(deleted)".to_string());

        let mut is_created = false;
        let mut is_resolved = false;

        let mutation_infos: Vec<MutationInfo> = muts
            .iter()
            .map(|m| {
                if m.field() == "created" {
                    is_created = true;
                }
                if m.field() == "status" && m.new_value() == "Resolved" {
                    is_resolved = true;
                }
                MutationInfo {
                    timestamp: m.timestamp().to_rfc3339(),
                    field: m.field().to_owned(),
                    old_value: m.old_value().map(|s| s.to_owned()),
                    new_value: m.new_value().to_owned(),
                }
            })
            .collect();

        if is_created {
            created_count += 1;
        } else if is_resolved {
            resolved_count += 1;
        } else {
            updated_count += 1;
        }

        changes.push(TensionChanges {
            tension_id: tid.clone(),
            tension_desired: desired_label,
            mutations: mutation_infos,
        });
    }

    let summary = DiffSummary {
        updated: updated_count,
        created: created_count,
        resolved: resolved_count,
    };

    if output.is_structured() {
        let result = DiffOutput {
            since: since_dt.to_rfc3339(),
            changes,
            summary,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        let since_label = format_since_label(since_dt);
        println!("Changes since {}:", since_label);
        println!();

        for change in &changes {
            let tension_sc = tension_map.get(&change.tension_id).and_then(|t| t.short_code);
            let id_display = display_id(tension_sc, &change.tension_id);
            println!("  {} {}", id_display, truncate(&change.tension_desired, 60));

            // Collapse consecutive position/hold mutations into summary lines
            let mut i = 0;
            while i < change.mutations.len() {
                let m = &change.mutations[i];
                let is_position = m.field == "position";

                if is_position {
                    // Count consecutive position mutations
                    let run_start = i;
                    while i < change.mutations.len() && change.mutations[i].field == "position" {
                        i += 1;
                    }
                    let run_len = i - run_start;

                    if run_len > 2 {
                        // Collapse: show first, summary, last
                        let first = &change.mutations[run_start];
                        let last = &change.mutations[i - 1];
                        let first_ts = chrono::DateTime::parse_from_rfc3339(&first.timestamp)
                            .map(|dt| relative_time(dt.with_timezone(&Utc), now))
                            .unwrap_or_else(|_| first.timestamp[..19].replace('T', " "));
                        let last_summary = super::show::format_mutation_summary(
                            &last.field, last.old_value.as_deref(), &last.new_value
                        );
                        println!("    {:>12}  {} ({} position changes)", first_ts, last_summary, run_len);
                    } else {
                        // 1-2 position mutations: show normally
                        for j in run_start..i {
                            let pm = &change.mutations[j];
                            let ts = chrono::DateTime::parse_from_rfc3339(&pm.timestamp)
                                .map(|dt| relative_time(dt.with_timezone(&Utc), now))
                                .unwrap_or_else(|_| pm.timestamp[..19].replace('T', " "));
                            let summary = super::show::format_mutation_summary(
                                &pm.field, pm.old_value.as_deref(), &pm.new_value
                            );
                            println!("    {:>12}  {}", ts, summary);
                        }
                    }
                } else {
                    let ts = chrono::DateTime::parse_from_rfc3339(&m.timestamp)
                        .map(|dt| relative_time(dt.with_timezone(&Utc), now))
                        .unwrap_or_else(|_| m.timestamp[..19].replace('T', " "));
                    let summary = super::show::format_mutation_summary(
                        &m.field, m.old_value.as_deref(), &m.new_value
                    );
                    println!("    {:>12}  {}", ts, summary);
                    i += 1;
                }
            }
            println!();
        }

        println!(
            "Summary: {} tensions updated, {} created, {} resolved",
            summary.updated, summary.created, summary.resolved
        );
    }

    Ok(())
}

/// Format the since datetime as a human-friendly label like "Mar 13, 2026".
fn format_since_label(dt: DateTime<Utc>) -> String {
    dt.format("%b %d, %Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_since_today() {
        let now = Utc::now();
        let result = parse_since("today", now).unwrap();
        assert_eq!(result, start_of_day(now));
    }

    #[test]
    fn test_parse_since_yesterday() {
        let now = Utc::now();
        let result = parse_since("yesterday", now).unwrap();
        let expected = start_of_day(now - chrono::Duration::days(1));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_since_n_days_ago() {
        let now = Utc::now();
        let result = parse_since("3 days ago", now).unwrap();
        let expected = start_of_day(now - chrono::Duration::days(3));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_since_iso_date() {
        let now = Utc::now();
        let result = parse_since("2026-03-10", now).unwrap();
        let expected = NaiveDate::from_ymd_opt(2026, 3, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .map(|naive| naive.and_utc())
            .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_since_weekday() {
        let now = Utc::now();
        let result = parse_since("monday", now);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_since_invalid() {
        let now = Utc::now();
        let result = parse_since("not-a-date", now);
        assert!(result.is_err());
    }

    #[test]
    fn test_days_since_weekday_same_day() {
        assert_eq!(days_since_weekday(Weekday::Mon, Weekday::Mon), 0);
    }

    #[test]
    fn test_days_since_weekday_yesterday() {
        assert_eq!(days_since_weekday(Weekday::Tue, Weekday::Mon), 1);
    }

    #[test]
    fn test_days_since_weekday_wrap() {
        // From Monday, last Saturday was 2 days ago
        assert_eq!(days_since_weekday(Weekday::Mon, Weekday::Sat), 2);
    }
}
