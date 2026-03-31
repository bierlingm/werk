//! CommandPalette integration — unified search/command/navigation surface.
//!
//! Opens on Ctrl+K. Fuzzy-searches registered actions with Bayesian scoring.
//! Actions dispatch to the same code paths as direct keybindings.

use ftui::widgets::command_palette::{ActionItem, CommandPalette};

/// Build a CommandPalette pre-loaded with all available actions.
pub fn build_palette() -> CommandPalette {
    let mut palette = CommandPalette::new()
        .with_max_visible(12);

    // Structure
    palette.register_action(
        ActionItem::new("add", "Add child tension")
            .with_description("Create a new tension under the current parent")
            .with_tags(&["create", "new", "child"]),
    );
    palette.register_action(
        ActionItem::new("ascend", "Go to parent")
            .with_description("Navigate up to the parent tension")
            .with_tags(&["up", "back", "parent"]),
    );
    palette.register_action(
        ActionItem::new("descend", "Descend into tension")
            .with_description("Navigate into the focused tension's children")
            .with_tags(&["into", "focus", "enter"]),
    );
    palette.register_action(
        ActionItem::new("move", "Move tension")
            .with_description("Reparent the focused tension")
            .with_tags(&["reparent", "relocate"]),
    );

    // Action
    palette.register_action(
        ActionItem::new("resolve", "Resolve tension")
            .with_description("Close the gap — desire met reality")
            .with_tags(&["close", "done", "complete"]),
    );
    palette.register_action(
        ActionItem::new("release", "Release tension")
            .with_description("Let it go — acknowledge without closing")
            .with_tags(&["drop", "abandon", "let go"]),
    );
    palette.register_action(
        ActionItem::new("edit_desire", "Edit desire")
            .with_description("Change what is wanted")
            .with_tags(&["edit", "desire", "want"]),
    );
    palette.register_action(
        ActionItem::new("edit_reality", "Edit reality")
            .with_description("Update what is actual")
            .with_tags(&["edit", "reality", "actual"]),
    );
    palette.register_action(
        ActionItem::new("edit_horizon", "Edit horizon")
            .with_description("Set when this matters")
            .with_tags(&["edit", "horizon", "deadline", "when"]),
    );
    palette.register_action(
        ActionItem::new("note", "Add note")
            .with_description("Annotate the focused tension")
            .with_tags(&["annotate", "comment", "note"]),
    );

    // Chrome
    palette.register_action(
        ActionItem::new("undo", "Undo last gesture")
            .with_description("Revert the most recent action")
            .with_tags(&["undo", "revert", "back"]),
    );
    palette.register_action(
        ActionItem::new("redo", "Redo last undo")
            .with_description("Re-apply the most recently undone action")
            .with_tags(&["redo", "forward"]),
    );
    palette.register_action(
        ActionItem::new("search", "Search tensions")
            .with_description("Full-text search across all tensions")
            .with_tags(&["find", "search", "lookup"]),
    );
    palette.register_action(
        ActionItem::new("help", "Toggle help")
            .with_description("Show keyboard shortcuts and commands")
            .with_tags(&["help", "keys", "shortcuts"]),
    );
    palette.register_action(
        ActionItem::new("survey", "Switch to survey view")
            .with_description("Time-first overview of all tensions")
            .with_tags(&["survey", "overview", "time"]),
    );
    palette.register_action(
        ActionItem::new("hold", "Hold tension")
            .with_description("Remove positioning — place in held zone")
            .with_tags(&["hold", "park", "pause"]),
    );
    palette.register_action(
        ActionItem::new("quit", "Quit")
            .with_description("Exit the instrument")
            .with_tags(&["exit", "close", "quit"]),
    );

    palette
}
