//! FrankenSearch-backed hybrid search index for tensions.
//!
//! Provides a shared `SearchIndex` that can be used by TUI, CLI, and MCP.
//! Currently uses the hash embedder (lexical overlap via FNV-1a hashing).
//! Upgrade path: enable `lexical` feature for BM25, `semantic` for quality
//! embedders — same API, richer results.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use frankensearch::HashEmbedder;

use crate::Store;

/// A search hit with document ID and relevance score.
#[derive(Debug, Clone)]
pub struct SearchHit {
    /// Tension ULID.
    pub doc_id: String,
    /// Relevance score (higher is better).
    pub score: f32,
    /// Desired text (cached at index build time — no DB read needed).
    pub desired: String,
    /// Parent tension ID, if any.
    pub parent_id: Option<String>,
    /// Tension status (active, resolved, released).
    pub status: crate::TensionStatus,
    /// Short code for display (e.g. 42 for #42).
    pub short_code: Option<i32>,
}

/// Cached metadata for a tension, populated at index build time.
struct DocMeta {
    desired: String,
    parent_id: Option<String>,
    status: crate::TensionStatus,
    short_code: Option<i32>,
}

/// FrankenSearch index over the tension corpus.
///
/// In-memory embedding cache for instant per-keystroke search. All data
/// needed for search results is cached at build time — zero database reads
/// during search.
///
/// The disk-persisted `TwoTierIndex` is written lazily via `persist()` when
/// needed for future TwoTierSearcher / Phase 2 upgrades.
pub struct SearchIndex {
    /// Cached document embeddings: (tension_id, embedding vector).
    docs: Vec<(String, Vec<f32>)>,
    /// Cached metadata for each tension (desired text, parent_id).
    meta: HashMap<String, DocMeta>,
    /// The embedder used for queries (same one that produced doc embeddings).
    embedder: HashEmbedder,
    index_path: PathBuf,
}

impl SearchIndex {
    /// Build a search index from all tensions in the store.
    ///
    /// Index is written to `.werk/search-index/` next to the store DB.
    /// Returns `None` for in-memory stores (no path to write index).
    pub fn build(store: &Store) -> Option<Self> {
        let db_path = store.path()?;
        let werk_dir = db_path.parent()?;
        let index_path = werk_dir.join("search-index");

        let tensions = store.list_tensions().ok()?;
        if tensions.is_empty() {
            return None;
        }

        let embedder = HashEmbedder::default_384();

        // Build document embeddings + metadata cache in memory.
        // Embeddings: ~11μs per doc, ~1.6ms for 150 docs. No disk I/O.
        let mut docs = Vec::with_capacity(tensions.len());
        let mut meta = HashMap::with_capacity(tensions.len());

        for t in &tensions {
            let content = format!("{} {}", t.desired, t.actual);
            let embedding = embedder.embed_sync(&content);
            docs.push((t.id.clone(), embedding));
            meta.insert(t.id.clone(), DocMeta {
                desired: t.desired.clone(),
                parent_id: t.parent_id.clone(),
                status: t.status,
                short_code: t.short_code,
            });
        }

        Some(Self {
            docs,
            meta,
            embedder,
            index_path,
        })
    }

    /// Search the index for tensions matching the query.
    ///
    /// Fully synchronous — embeds the query with `embed_sync`, computes
    /// cosine similarity against all cached doc embeddings, returns top-k.
    /// Zero database reads — all metadata is cached.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_vec = self.embedder.embed_sync(query);

        let mut scored: Vec<SearchHit> = self.docs.iter()
            .filter_map(|(id, doc_vec)| {
                let score = frankensearch::cosine_similarity(&query_vec, doc_vec);
                if score <= 0.0 {
                    return None;
                }
                let m = self.meta.get(id)?;
                Some(SearchHit {
                    doc_id: id.clone(),
                    score,
                    desired: m.desired.clone(),
                    parent_id: m.parent_id.clone(),
                    status: m.status,
                    short_code: m.short_code,
                })
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    /// Look up cached metadata for a tension by ID.
    /// Returns (desired, parent_id) without touching the database.
    pub fn get_desired(&self, id: &str) -> Option<&str> {
        self.meta.get(id).map(|m| m.desired.as_str())
    }

    /// Look up cached parent_id for a tension by ID.
    pub fn get_parent_id(&self, id: &str) -> Option<&str> {
        self.meta.get(id).and_then(|m| m.parent_id.as_deref())
    }

    /// Build a compact parent reference like "← #15 Product" from cached metadata.
    /// Returns None if the tension has no parent.
    pub fn compact_parent_ref(&self, parent_id: Option<&str>) -> Option<String> {
        let pid = parent_id?;
        let m = self.meta.get(pid)?;
        let code = m.short_code.map(|c| format!("#{}", c)).unwrap_or_default();
        let first_word = m.desired.split_whitespace().next().unwrap_or("");
        Some(format!("\u{2190} {} {}", code, first_word))
    }

    /// Build a breadcrumb path from cached metadata. Zero DB reads.
    pub fn breadcrumb(&self, parent_id: Option<&str>) -> String {
        let mut crumbs = Vec::new();
        let mut current = parent_id.map(|s| s.to_string());
        while let Some(ref id) = current {
            if let Some(m) = self.meta.get(id.as_str()) {
                let label = if m.desired.len() > 15 {
                    format!("{}…", &m.desired[..14])
                } else {
                    m.desired.clone()
                };
                crumbs.push(label);
                current = m.parent_id.clone();
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

    /// Rebuild the index from scratch. Fast enough to call on every mutation.
    pub fn rebuild(store: &Store) -> Option<Self> {
        Self::build(store)
    }

    /// Number of documents in the index.
    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }

    /// Path to the index directory.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }
}

/// Write the disk-persisted index for future TwoTierSearcher use.
///
/// This is expensive (~2s for 150 docs due to asupersync runtime + IndexBuilder).
/// Call only when the disk index is needed (e.g., before upgrading to hybrid search).
/// Not needed for the current in-memory search path.
#[allow(dead_code)]
pub fn persist_disk_index(store: &Store) -> Option<PathBuf> {
    use std::sync::Arc;
    use frankensearch::prelude::*;
    use frankensearch::{EmbedderStack, IndexBuilder};
    use frankensearch::core::traits::Embedder;

    let db_path = store.path()?;
    let werk_dir = db_path.parent()?;
    let index_path = werk_dir.join("search-index");

    let tensions = store.list_tensions().ok()?;
    if tensions.is_empty() {
        return None;
    }

    let embedder = HashEmbedder::default_384();
    let fast: Arc<dyn Embedder> = Arc::new(embedder);
    let stack = EmbedderStack::from_parts(fast, None);

    let mut builder = IndexBuilder::new(&index_path).with_embedder_stack(stack);
    for t in &tensions {
        let content = format!("{} {}", t.desired, t.actual);
        builder = builder.add_document(&t.id, &content);
    }

    let cx = Cx::for_testing();
    let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
        .build()
        .ok()?;
    let result = runtime.block_on(async { builder.build(&cx).await });
    result.ok().map(|_| index_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    #[test]
    fn build_and_search_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::init(dir.path()).expect("init store");

        store.create_tension(
            "deadline management and temporal signals",
            "basic urgency computation exists",
        ).expect("create t1");
        store.create_tension(
            "sustainable business model for revenue",
            "no revenue yet",
        ).expect("create t2");
        store.create_tension(
            "graph intelligence with centrality measures",
            "FrankenNetworkX integrated",
        ).expect("create t3");

        let idx = SearchIndex::build(&store).expect("build index");
        assert_eq!(idx.doc_count(), 3);

        // "revenue business" should rank the business tension first
        let hits = idx.search("revenue business", 3);
        assert!(!hits.is_empty(), "search should return results");
        let top = &hits[0];
        assert!(
            top.desired.contains("business") || top.desired.contains("revenue"),
            "top result should be the business/revenue tension, got: {}",
            top.desired,
        );

        // Cached metadata works
        assert!(idx.get_desired(&top.doc_id).unwrap().contains("business"));

        // Breadcrumb for root-level tension
        assert_eq!(idx.breadcrumb(top.parent_id.as_deref()), "root");

        // Empty query returns nothing
        assert!(idx.search("", 5).is_empty());
    }

    #[test]
    fn returns_none_for_empty_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::init(dir.path()).expect("init store");
        assert!(SearchIndex::build(&store).is_none());
    }

    #[test]
    fn rebuild_reflects_new_tensions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::init(dir.path()).expect("init store");

        store.create_tension("alpha tension", "first").expect("create");
        let idx = SearchIndex::build(&store).expect("build");
        assert_eq!(idx.doc_count(), 1);

        store.create_tension("beta tension", "second").expect("create");
        let idx = SearchIndex::rebuild(&store).expect("rebuild");
        assert_eq!(idx.doc_count(), 2);

        let hits = idx.search("beta", 5);
        assert!(!hits.is_empty());
    }

    #[test]
    fn breadcrumb_from_cache() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::init(dir.path()).expect("init store");

        let parent = store.create_tension("parent tension", "reality").expect("create parent");
        store.create_tension_with_parent("child tension", "child reality", Some(parent.id.clone()))
            .expect("create child");

        let idx = SearchIndex::build(&store).expect("build");

        // Child's breadcrumb should show parent
        let hits = idx.search("child", 5);
        assert!(!hits.is_empty());
        let child = &hits[0];
        let crumb = idx.breadcrumb(child.parent_id.as_deref());
        assert!(crumb.contains("parent"), "breadcrumb should contain parent, got: {crumb}");
    }
}
