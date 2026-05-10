use chrono::{DateTime, Utc};
use std::path::PathBuf;
use werk_core::{Store, Tension};

use crate::SigilError;

#[derive(Debug, Clone)]
pub struct CleanupReport {
    pub removed: usize,
}

pub fn archive_path(
    scope_canonical: &str,
    logic_id: &str,
    seed: u64,
    now: DateTime<Utc>,
) -> PathBuf {
    let date = now.format("%Y-%m-%d").to_string();
    let filename = format!("{}-{}-{}.svg", slugify(scope_canonical), logic_id, seed);
    let mut path = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push(".werk/sigils");
    path.push(date);
    path.push(filename);
    path
}

pub fn cache_path(
    scope_canonical: &str,
    logic_canonical: &str,
    seed: u64,
    revision: &str,
) -> PathBuf {
    let key = format!("{scope_canonical}|{logic_canonical}|{seed}|{revision}");
    let hash = blake3::hash(key.as_bytes());
    let mut path = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push(".werk/sigils/cache");
    path.push(format!("{}.svg", hash.to_hex()));
    path
}

pub fn cleanup_cache(retention_days: u32) -> Result<CleanupReport, SigilError> {
    let mut path = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push(".werk/sigils/cache");
    let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
    let mut removed = 0;
    if !path.exists() {
        return Ok(CleanupReport { removed: 0 });
    }
    for entry in std::fs::read_dir(&path).map_err(|e| SigilError::io(e.to_string()))? {
        let entry = entry.map_err(|e| SigilError::io(e.to_string()))?;
        let meta = entry.metadata().map_err(|e| SigilError::io(e.to_string()))?;
        if !meta.is_file() {
            continue;
        }
        let modified = meta
            .modified()
            .map_err(|e| SigilError::io(e.to_string()))?;
        let modified: chrono::DateTime<chrono::Utc> = modified.into();
        if modified < cutoff {
            std::fs::remove_file(entry.path())
                .map_err(|e| SigilError::io(e.to_string()))?;
            removed += 1;
        }
    }
    Ok(CleanupReport { removed })
}

pub fn werk_state_revision(store: &Store, tensions: &[Tension]) -> Result<String, SigilError> {
    if tensions.is_empty() {
        return Ok("empty".to_string());
    }
    let mut latest: Option<DateTime<Utc>> = None;
    for tension in tensions {
        let mutations = store
            .get_mutations(&tension.id)
            .map_err(|e| SigilError::io(format!("failed to read mutations: {e}")))?;
        let candidate = mutations
            .last()
            .map(|m| m.timestamp())
            .unwrap_or(tension.created_at);
        latest = Some(match latest {
            Some(prev) if prev >= candidate => prev,
            _ => candidate,
        });
    }
    Ok(latest
        .map(|ts| ts.to_rfc3339())
        .unwrap_or_else(|| "empty".to_string()))
}

fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}
