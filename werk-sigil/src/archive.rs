use chrono::{DateTime, Utc};
use std::path::PathBuf;

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

fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}
