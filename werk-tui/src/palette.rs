//! CommandPalette integration — unified search/command/navigation surface.
//!
//! Opens on Ctrl+K (or `/`). Shows both command actions and FrankenSearch
//! tension results in a single surface. Typing filters both — the palette's
//! Bayesian scorer handles fuzzy matching across all items.
//!
//! **Learning**: A FeedbackCollector tracks which actions the practitioner
//! selects. Boosts decay exponentially so recent usage matters more.
//! The boost map persists across sessions via StateRegistry.

use ftui::widgets::command_palette::{ActionItem, CommandPalette};

use crate::feedback::{FeedbackCollector, FeedbackConfig};

/// Palette action IDs in default display order.
/// Reordered at startup by feedback boosts — most-used actions float to top.
const ACTION_IDS: &[&str] = &[
    "add", "resolve", "release", "descend", "ascend",
    "edit_desire", "edit_reality", "edit_horizon", "note", "move",
    "undo", "redo", "help", "survey", "logbase", "hold", "quit",
];

/// Build a CommandPalette with actions ordered by feedback boosts.
/// Pass None for a fresh palette (no learning history).
pub fn build_palette(feedback: Option<&FeedbackCollector>) -> CommandPalette {
    // Determine action registration order: boosted actions first.
    let ordered_ids = match feedback {
        Some(fc) => {
            let mut ids_with_boost: Vec<(&str, f64)> = ACTION_IDS
                .iter()
                .map(|id| (*id, fc.get_boost(id)))
                .collect();
            // Sort by descending boost (highest usage first).
            ids_with_boost.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            ids_with_boost.iter().map(|(id, _)| *id).collect::<Vec<_>>()
        }
        None => ACTION_IDS.to_vec(),
    };

    let mut palette = CommandPalette::new()
        .with_max_visible(12);

    for id in ordered_ids {
        if let Some(action) = make_action(id) {
            palette.register_action(action);
        }
    }

    palette
}

/// Create a FeedbackCollector configured for palette action learning.
/// Short decay (24h half-life) since palette actions are high-frequency.
pub fn create_feedback_collector() -> FeedbackCollector {
    FeedbackCollector::new(FeedbackConfig {
        decay_halflife_hours: 24.0,
        max_boost: 3.0,
        min_boost: 0.5,
        select_weight: 3.0,
    })
}

/// Record that a palette action was selected by the practitioner.
pub fn record_action_selected(feedback: &mut FeedbackCollector, action_id: &str) {
    feedback.record_select(action_id);
}

/// Prefix for tension IDs in the palette — distinguishes them from action IDs.
pub const TENSION_ID_PREFIX: &str = "t:";

/// Prefix for epoch address actions in the palette.
pub const EPOCH_ADDR_PREFIX: &str = "epoch:";

/// Build a combined list of action + tension ActionItems for the palette.
///
/// Actions come first (feedback-ordered), then tension results from FrankenSearch.
/// Called on every query change to keep results current.
pub fn build_combined_items(
    query: &str,
    feedback: Option<&FeedbackCollector>,
    search_index: Option<&sd_core::SearchIndex>,
    store: &sd_core::Store,
) -> Vec<ActionItem> {
    let mut items = Vec::new();

    // Actions — always included (palette's Bayesian scorer filters by query)
    let ordered_ids = match feedback {
        Some(fc) => {
            let mut ids_with_boost: Vec<(&str, f64)> = ACTION_IDS
                .iter()
                .map(|id| (*id, fc.get_boost(id)))
                .collect();
            ids_with_boost.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            ids_with_boost.iter().map(|(id, _)| *id).collect::<Vec<_>>()
        }
        None => ACTION_IDS.to_vec(),
    };
    for id in ordered_ids {
        if let Some(action) = make_action(id) {
            items.push(action);
        }
    }

    // Address resolution — detect deep addresses like #42~e3, #42.n3
    let mut address_resolved = false;
    if !query.is_empty() {
        if let Ok(addr) = sd_core::address::parse_address(query) {
            match addr {
                sd_core::address::Address::Epoch { tension, epoch_num } => {
                    // Find tension by short code and offer logbase navigation
                    if let Ok(tensions) = store.list_tensions() {
                        if let Some(t) = tensions.iter().find(|t| t.short_code == Some(tension)) {
                            let action_id = format!("{}{}~e{}", EPOCH_ADDR_PREFIX, t.id, epoch_num);
                            // Title includes the address syntax so the fuzzy matcher can find it
                            let title = format!("#{}~e{} — epoch {} — {}", tension, epoch_num, epoch_num,
                                werk_shared::truncate(&t.desired, 50));
                            items.push(ActionItem::new(action_id, title)
                                .with_description("open in logbase")
                                .with_category("\u{2500}")
                                .with_tags(&["address", "logbase", "epoch"]));
                            address_resolved = true;
                        }
                    }
                }
                _ => {} // Other address types handled by existing tension search
            }
        }
    }

    // Tensions — only when there's a query to search for (skip if address already resolved)
    if !query.is_empty() && !address_resolved {
        let tension_items = if query.starts_with('#') {
            search_by_short_code(&query[1..], store, search_index)
        } else if query.chars().all(|c| c.is_ascii_digit()) {
            search_by_short_code(query, store, search_index)
        } else {
            // Combine FrankenSearch (semantic) with substring (prefix) matches.
            // Substring catches "busi"→"business" that FrankenSearch may miss.
            let mut seen = std::collections::HashSet::new();
            let mut combined = Vec::new();

            // Substring matches first — direct text matches are most expected
            for item in search_via_substring(query, store, search_index) {
                if let Some(id) = item.id.strip_prefix(TENSION_ID_PREFIX) {
                    seen.insert(id.to_string());
                }
                combined.push(item);
            }

            // FrankenSearch adds semantic/fuzzy hits not already found
            if let Some(idx) = search_index {
                for item in search_via_frankensearch(query, idx) {
                    if let Some(id) = item.id.strip_prefix(TENSION_ID_PREFIX) {
                        if seen.contains(id) {
                            continue;
                        }
                    }
                    combined.push(item);
                }
            }

            combined.truncate(15);
            combined
        };
        items.extend(tension_items);
    }

    items
}

/// Build a tension ActionItem with consistent layout:
/// title: "◇ #30 desire text"  (status glyph left, code, desire)
/// description: "← #15 Parent"  (parent ref on right)
fn tension_action_item(
    id: &str,
    desired: &str,
    short_code: Option<i32>,
    status: sd_core::TensionStatus,
    parent_ref: &str,
) -> ActionItem {
    // Status glyph goes in the category badge — rendered separately by the
    // widget, outside the title's match-highlight logic. This avoids the
    // widget's byte-vs-char position bug with non-ASCII title characters.
    let badge = match status {
        sd_core::TensionStatus::Active => "\u{25C7}",    // ◇
        sd_core::TensionStatus::Resolved => "\u{2713}",  // ✓
        sd_core::TensionStatus::Released => "\u{223C}",   // ∼
    };
    let title = match short_code {
        Some(c) => format!("#{} {}", c, desired),
        None => desired.to_string(),
    };
    ActionItem::new(format!("{}{}", TENSION_ID_PREFIX, id), title)
        .with_description(parent_ref)
        .with_category(badge)
        .with_tags(&["tension"])
}

/// Convert FrankenSearch hits to ActionItems.
fn search_via_frankensearch(query: &str, index: &sd_core::SearchIndex) -> Vec<ActionItem> {
    let hits = index.search(query, 15);
    hits.into_iter()
        .filter(|hit| hit.status == sd_core::TensionStatus::Active)
        .map(|hit| {
            let parent_ref = index.compact_parent_ref(hit.parent_id.as_deref())
                .unwrap_or_else(|| "root".to_string());
            tension_action_item(&hit.doc_id, &hit.desired, hit.short_code, hit.status, &parent_ref)
        })
        .collect()
}

/// Substring search — case-insensitive match against desire and reality text.
fn search_via_substring(query: &str, store: &sd_core::Store, search_index: Option<&sd_core::SearchIndex>) -> Vec<ActionItem> {
    let q = query.to_lowercase();
    let tensions = store.list_tensions().unwrap_or_default();
    let mut results: Vec<_> = tensions.iter()
        .filter(|t| t.status == sd_core::TensionStatus::Active)
        .filter(|t| t.desired.to_lowercase().contains(&q) || t.actual.to_lowercase().contains(&q))
        .take(15)
        .map(|t| {
            let parent_ref = search_index
                .and_then(|idx| idx.compact_parent_ref(t.parent_id.as_deref()))
                .unwrap_or_else(|| "root".to_string());
            tension_action_item(&t.id, &t.desired, t.short_code, t.status, &parent_ref)
        })
        .collect();
    results.truncate(15);
    results
}

/// Search by short code prefix (e.g. "4" matches #4, #40, #42...).
fn search_by_short_code(
    code_prefix: &str,
    store: &sd_core::Store,
    search_index: Option<&sd_core::SearchIndex>,
) -> Vec<ActionItem> {
    let prefix_num: i32 = match code_prefix.parse() {
        Ok(n) => n,
        Err(_) => return Vec::new(), // non-numeric after #
    };
    let tensions = store.list_tensions().unwrap_or_default();
    tensions.iter()
        .filter(|t| {
            t.short_code.map_or(false, |sc| {
                sc.to_string().starts_with(&prefix_num.to_string())
            })
        })
        .take(15)
        .map(|t| {
            let parent_ref = search_index
                .and_then(|idx| idx.compact_parent_ref(t.parent_id.as_deref()))
                .unwrap_or_else(|| "root".to_string());
            tension_action_item(&t.id, &t.desired, t.short_code, t.status, &parent_ref)
        })
        .collect()
}

fn make_action(id: &str) -> Option<ActionItem> {
    let action = match id {
        "add" => ActionItem::new("add", "Add child tension")
            .with_description("Create a new tension under the current parent")
            .with_tags(&["create", "new", "child"]),
        "ascend" => ActionItem::new("ascend", "Go to parent")
            .with_description("Navigate up to the parent tension")
            .with_tags(&["up", "back", "parent"]),
        "descend" => ActionItem::new("descend", "Descend into tension")
            .with_description("Navigate into the focused tension's children")
            .with_tags(&["into", "focus", "enter"]),
        "move" => ActionItem::new("move", "Move tension")
            .with_description("Reparent the focused tension")
            .with_tags(&["reparent", "relocate"]),
        "resolve" => ActionItem::new("resolve", "Resolve tension")
            .with_description("Close the gap — desire met reality")
            .with_tags(&["close", "done", "complete"]),
        "release" => ActionItem::new("release", "Release tension")
            .with_description("Let it go — acknowledge without closing")
            .with_tags(&["drop", "abandon", "let go"]),
        "edit_desire" => ActionItem::new("edit_desire", "Edit desire")
            .with_description("Change what is wanted")
            .with_tags(&["edit", "desire", "want"]),
        "edit_reality" => ActionItem::new("edit_reality", "Edit reality")
            .with_description("Update what is actual")
            .with_tags(&["edit", "reality", "actual"]),
        "edit_horizon" => ActionItem::new("edit_horizon", "Edit horizon")
            .with_description("Set when this matters")
            .with_tags(&["edit", "horizon", "deadline", "when"]),
        "note" => ActionItem::new("note", "Add note")
            .with_description("Annotate the focused tension")
            .with_tags(&["annotate", "comment", "note"]),
        "undo" => ActionItem::new("undo", "Undo last gesture")
            .with_description("Revert the most recent action")
            .with_tags(&["undo", "revert", "back"]),
        "redo" => ActionItem::new("redo", "Redo last undo")
            .with_description("Re-apply the most recently undone action")
            .with_tags(&["redo", "forward"]),
        "help" => ActionItem::new("help", "Toggle help")
            .with_description("Show keyboard shortcuts and commands")
            .with_tags(&["help", "keys", "shortcuts"]),
        "survey" => ActionItem::new("survey", "Switch to survey view")
            .with_description("Time-first overview of all tensions")
            .with_tags(&["survey", "overview", "time"]),
        "logbase" => ActionItem::new("logbase", "Open logbase")
            .with_description("Epoch history for the focused tension")
            .with_tags(&["logbase", "log", "history", "epochs"]),
        "hold" => ActionItem::new("hold", "Hold tension")
            .with_description("Remove positioning — place in held zone")
            .with_tags(&["hold", "park", "pause"]),
        "quit" => ActionItem::new("quit", "Quit")
            .with_description("Exit the instrument")
            .with_tags(&["exit", "close", "quit"]),
        _ => return None,
    };
    Some(action)
}
