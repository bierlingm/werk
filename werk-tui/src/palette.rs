//! CommandPalette integration — unified search/command/navigation surface.
//!
//! Opens on Ctrl+K. Fuzzy-searches registered actions with Bayesian scoring.
//! Actions dispatch to the same code paths as direct keybindings.
//!
//! **Learning**: A FeedbackCollector (from frankensearch-fusion) tracks which
//! actions the practitioner selects. Boosts decay exponentially so recent
//! usage matters more. The boost map persists across sessions via StateRegistry.

use ftui::widgets::command_palette::{ActionItem, CommandPalette};

use crate::feedback::{FeedbackCollector, FeedbackConfig};

/// Palette action IDs in default display order.
/// Reordered at startup by feedback boosts — most-used actions float to top.
const ACTION_IDS: &[&str] = &[
    "add", "resolve", "release", "descend", "ascend",
    "edit_desire", "edit_reality", "edit_horizon", "note", "move",
    "undo", "redo", "search", "help", "survey", "hold", "quit",
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
        "search" => ActionItem::new("search", "Search tensions")
            .with_description("Full-text search across all tensions")
            .with_tags(&["find", "search", "lookup"]),
        "help" => ActionItem::new("help", "Toggle help")
            .with_description("Show keyboard shortcuts and commands")
            .with_tags(&["help", "keys", "shortcuts"]),
        "survey" => ActionItem::new("survey", "Switch to survey view")
            .with_description("Time-first overview of all tensions")
            .with_tags(&["survey", "overview", "time"]),
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
