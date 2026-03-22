//! Flat search across all tensions in the forest.

use sd_core::Store;
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

/// Search all tensions for a query (case-insensitive substring match).
pub fn search_all(query: &str, store: &Store) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let q = query.to_lowercase();
    let tensions = store.list_tensions().unwrap_or_default();

    let mut results: Vec<SearchResult> = tensions
        .iter()
        .filter(|t| {
            t.desired.to_lowercase().contains(&q) || t.actual.to_lowercase().contains(&q)
        })
        .map(|t| {
            // Build parent breadcrumb
            let parent_path = build_breadcrumb(t.parent_id.as_deref(), store);
            SearchResult {
                id: t.id.clone(),
                desired: t.desired.clone(),
                parent_path,
                is_root_entry: false,
            }
        })
        .collect();

    // Score: exact prefix match > word boundary > substring
    results.sort_by(|a, b| {
        let score_a = match_score(&a.desired, &q);
        let score_b = match_score(&b.desired, &q);
        score_b.cmp(&score_a)
    });

    results.truncate(20); // max results
    results
}

/// Search with a special "(root level)" entry prepended (for move-to-root).
pub fn search_all_with_root(query: &str, store: &Store) -> Vec<SearchResult> {
    let mut results = vec![SearchResult {
        id: String::new(),
        desired: "(root level)".to_string(),
        parent_path: String::new(),
        is_root_entry: true,
    }];
    results.extend(search_all(query, store));
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
        3 // exact prefix
    } else if lower.contains(&format!(" {}", query)) {
        2 // word boundary
    } else {
        1 // substring
    }
}
