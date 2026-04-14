//! Search across all tensions in the forest.
//!
//! Uses FrankenSearch (hash embedder) for relevance ranking when available,
//! falls back to substring matching if the search index isn't built.

use werk_core::{SearchIndex, Store};
use werk_shared::truncate;

/// A search result with parent breadcrumb.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub desired: String,
    pub parent_path: String, // "root" or "Parent › Grandparent"
    pub is_root_entry: bool, // special "(root level)" entry for move-to-root
}

/// Search state.
#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub cursor: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            cursor: 0,
        }
    }

    pub fn selected(&self) -> Option<&SearchResult> {
        self.results.get(self.cursor)
    }
}

/// Search all tensions using FrankenSearch index (preferred) or substring fallback.
pub fn search_all(query: &str, store: &Store, index: Option<&SearchIndex>) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    if let Some(idx) = index {
        search_via_index(query, idx)
    } else {
        search_via_substring(query, store)
    }
}

/// Search with a special "(root level)" entry prepended (for move-to-root).
pub fn search_all_with_root(
    query: &str,
    store: &Store,
    index: Option<&SearchIndex>,
) -> Vec<SearchResult> {
    let mut results = vec![SearchResult {
        id: String::new(),
        desired: "(root level)".to_string(),
        parent_path: String::new(),
        is_root_entry: true,
    }];
    results.extend(search_all(query, store, index));
    results
}

/// FrankenSearch-backed search. Zero database reads — all data from cache.
fn search_via_index(query: &str, index: &SearchIndex) -> Vec<SearchResult> {
    let hits = index.search(query, 20);

    hits.into_iter()
        .filter(|hit| hit.status == werk_core::TensionStatus::Active)
        .map(|hit| {
            let parent_path = index.breadcrumb(hit.parent_id.as_deref());
            SearchResult {
                id: hit.doc_id,
                desired: hit.desired,
                parent_path,
                is_root_entry: false,
            }
        })
        .collect()
}

/// Substring fallback: case-insensitive match with simple scoring.
fn search_via_substring(query: &str, store: &Store) -> Vec<SearchResult> {
    let q = query.to_lowercase();
    let tensions = store.list_tensions().unwrap_or_default();

    let mut results: Vec<SearchResult> = tensions
        .iter()
        .filter(|t| t.desired.to_lowercase().contains(&q) || t.actual.to_lowercase().contains(&q))
        .map(|t| {
            let parent_path = build_breadcrumb(t.parent_id.as_deref(), store);
            SearchResult {
                id: t.id.clone(),
                desired: t.desired.clone(),
                parent_path,
                is_root_entry: false,
            }
        })
        .collect();

    results.sort_by(|a, b| {
        let score_a = match_score(&a.desired, &q);
        let score_b = match_score(&b.desired, &q);
        score_b.cmp(&score_a)
    });

    results.truncate(20);
    results
}

fn build_breadcrumb(parent_id: Option<&str>, store: &Store) -> String {
    let mut crumbs = Vec::new();
    let mut current = parent_id.map(|s| s.to_string());
    while let Some(ref id) = current {
        if let Ok(Some(t)) = store.get_tension(id) {
            crumbs.push(truncate(&t.desired, 15).to_string());
            current = t.parent_id.clone();
        } else {
            break;
        }
    }
    if crumbs.is_empty() {
        "root".to_string()
    } else {
        crumbs.reverse();
        crumbs.join(" \u{203A} ") // ›
    }
}

fn match_score(text: &str, query: &str) -> u8 {
    let lower = text.to_lowercase();
    if lower.starts_with(query) {
        3
    } else if lower.contains(&format!(" {}", query)) {
        2
    } else {
        1
    }
}
