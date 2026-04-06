//! Centralized help system — single source of truth for all keybindings.
//!
//! Populates an ftui HelpRegistry with every keybinding across views and modes.
//! The `?` overlay renders directly via Spans/Paragraph to bypass degradation checks.
//! Registry and HintRanker are available for future adaptive hint learning.

use ftui::widgets::help::{HelpCategory, HelpEntry};
use ftui::widgets::help_registry::{HelpContent, HelpId, HelpRegistry, Keybinding};
use ftui::widgets::hint_ranker::{HintContext, HintRanker, RankerConfig};

use crate::state::ViewOrientation;

// ---------------------------------------------------------------------------
// HelpId constants — one per view, one global
// ---------------------------------------------------------------------------

pub const HELP_GLOBAL: HelpId = HelpId(1);
pub const HELP_DECK: HelpId = HelpId(10);
pub const HELP_SURVEY: HelpId = HelpId(20);
pub const HELP_LOGBASE: HelpId = HelpId(30);

// ---------------------------------------------------------------------------
// Registry population
// ---------------------------------------------------------------------------

/// Build and populate the help registry with all keybindings.
pub fn build_registry() -> HelpRegistry {
    let mut reg = HelpRegistry::new();

    // Global — available in every view
    reg.register(HELP_GLOBAL, HelpContent {
        short: "Global keybindings".into(),
        long: Some("Keys that work in every view".into()),
        keybindings: vec![
            Keybinding::new("q", "quit"),
            Keybinding::new("/", "open palette"),
            Keybinding::new("Ctrl+K", "open palette"),
            Keybinding::new("?", "help"),
        ],
        see_also: vec![],
    });

    // Deck view (Stream orientation)
    reg.register(HELP_DECK, HelpContent {
        short: "Deck — structure-first view".into(),
        long: Some("Navigate and act on tensions in the tree".into()),
        keybindings: vec![
            // Navigation
            Keybinding::new("j/k", "move up/down"),
            Keybinding::new("l / Enter", "descend into"),
            Keybinding::new("h / Bksp", "ascend out"),
            Keybinding::new("g/G", "jump to top/bottom"),
            Keybinding::new("Space", "gaze (peek)"),
            Keybinding::new("1-9", "act on alert"),
            // Reorder
            Keybinding::new("Shift+J/K", "reorder position"),
            // Acts
            Keybinding::new("a", "add tension"),
            Keybinding::new("e", "edit (desire/reality/horizon)"),
            Keybinding::new("n", "add note"),
            Keybinding::new("m", "move / reparent"),
            Keybinding::new("r", "resolve"),
            Keybinding::new("x", "release"),
            Keybinding::new("o", "reopen"),
            Keybinding::new("p", "hold / unhold"),
            Keybinding::new("u/U", "undo / redo"),
            Keybinding::new("y", "copy ID"),
            Keybinding::new("f", "filter"),
            // View switch
            Keybinding::new("Tab", "switch to survey"),
            Keybinding::new("L", "open logbase"),
        ],
        see_also: vec![HELP_SURVEY, HELP_LOGBASE],
    });

    // Survey view
    reg.register(HELP_SURVEY, HelpContent {
        short: "Survey — time-first view".into(),
        long: Some("All active tensions organized by temporal urgency".into()),
        keybindings: vec![
            // Navigation
            Keybinding::new("j/k", "move cursor"),
            Keybinding::new("J/K", "jump between bands"),
            Keybinding::new("g/G", "top/bottom of band"),
            Keybinding::new("Enter", "descend into tension"),
            // Acts
            Keybinding::new("r", "resolve"),
            Keybinding::new("x", "release"),
            Keybinding::new("e", "edit"),
            Keybinding::new("n", "add note"),
            // View switch
            Keybinding::new("Tab", "pivot to deck at tension"),
            Keybinding::new("Shift+Tab", "return to deck"),
            Keybinding::new("L", "open logbase"),
            Keybinding::new("Esc", "return to deck"),
        ],
        see_also: vec![HELP_DECK, HELP_LOGBASE],
    });

    // Logbase view
    reg.register(HELP_LOGBASE, HelpContent {
        short: "Logbase — epoch stream".into(),
        long: Some("History of mutations for a single tension".into()),
        keybindings: vec![
            // Navigation
            Keybinding::new("j/k", "scroll events"),
            Keybinding::new("J/K", "jump between epochs"),
            Keybinding::new("Enter / Space", "expand event detail"),
            // View switch
            Keybinding::new("Tab", "switch to survey"),
            Keybinding::new("Shift+Tab", "return to origin"),
            Keybinding::new("L", "close logbase"),
            Keybinding::new("Esc", "return to origin"),
        ],
        see_also: vec![HELP_DECK, HELP_SURVEY],
    });

    reg
}


// ---------------------------------------------------------------------------
// Data access — returns grouped keybindings for manual rendering
// ---------------------------------------------------------------------------

/// A group of keybindings under a category header.
pub struct KeyGroup {
    pub label: &'static str,
    pub bindings: Vec<(&'static str, &'static str)>, // (key, description)
}

/// Get all keybinding groups for the `?` overlay in a given view.
/// Returns global entries first, then view-specific entries grouped by category.
pub fn overlay_groups(view: ViewOrientation) -> Vec<KeyGroup> {
    let mut groups = vec![
        KeyGroup {
            label: "Global",
            bindings: global_entries().iter().map(|e| (leak_str(&e.key), leak_str(&e.desc))).collect(),
        },
    ];

    // Group view entries by category
    let entries = view_entries(view);
    let mut nav = Vec::new();
    let mut edit = Vec::new();
    let mut view_switch = Vec::new();

    for e in &entries {
        let pair = (leak_str(&e.key), leak_str(&e.desc));
        match e.category {
            HelpCategory::Navigation => nav.push(pair),
            HelpCategory::Editing => edit.push(pair),
            HelpCategory::View => view_switch.push(pair),
            _ => nav.push(pair),
        }
    }

    if !nav.is_empty() {
        groups.push(KeyGroup { label: "Navigation", bindings: nav });
    }
    if !edit.is_empty() {
        groups.push(KeyGroup { label: "Acts", bindings: edit });
    }
    if !view_switch.is_empty() {
        groups.push(KeyGroup { label: "View", bindings: view_switch });
    }

    groups
}

/// Leak a String to get a &'static str. Fine for help data — allocated once at startup.
fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

// ---------------------------------------------------------------------------
// Entry data — the actual keybinding definitions
// ---------------------------------------------------------------------------

fn global_entries() -> Vec<HelpEntry> {
    vec![
        entry("q", "quit", HelpCategory::Global),
        entry("/", "palette", HelpCategory::Global),
        entry("?", "help", HelpCategory::Global),
    ]
}

fn view_entries(view: ViewOrientation) -> Vec<HelpEntry> {
    match view {
        ViewOrientation::Stream => deck_entries(),
        ViewOrientation::Survey => survey_entries(),
        ViewOrientation::Logbase => logbase_entries(),
    }
}

fn deck_entries() -> Vec<HelpEntry> {
    vec![
        // Navigation
        entry("j/k", "move up/down", HelpCategory::Navigation),
        entry("l / Enter", "descend into", HelpCategory::Navigation),
        entry("h / Bksp", "ascend out", HelpCategory::Navigation),
        entry("g/G", "jump to top/bottom", HelpCategory::Navigation),
        entry("Space", "gaze (peek)", HelpCategory::Navigation),
        entry("Shift+J/K", "reorder position", HelpCategory::Navigation),
        entry("1-9", "act on alert", HelpCategory::Navigation),
        // Acts
        entry("a", "add tension", HelpCategory::Editing),
        entry("e", "edit (desire/reality/horizon)", HelpCategory::Editing),
        entry("n", "add note", HelpCategory::Editing),
        entry("m", "move / reparent", HelpCategory::Editing),
        entry("r", "resolve", HelpCategory::Editing),
        entry("x", "release", HelpCategory::Editing),
        entry("o", "reopen", HelpCategory::Editing),
        entry("p", "hold / unhold", HelpCategory::Editing),
        entry("u/U", "undo / redo", HelpCategory::Editing),
        entry("y", "copy ID", HelpCategory::Editing),
        entry("f", "filter", HelpCategory::Editing),
        // View
        entry("Tab", "switch to survey", HelpCategory::View),
        entry("L", "open logbase", HelpCategory::View),
    ]
}

fn survey_entries() -> Vec<HelpEntry> {
    vec![
        // Navigation
        entry("j/k", "move cursor", HelpCategory::Navigation),
        entry("J/K", "jump between bands", HelpCategory::Navigation),
        entry("g/G", "top/bottom of band", HelpCategory::Navigation),
        entry("Enter", "descend into tension", HelpCategory::Navigation),
        // Acts
        entry("r", "resolve", HelpCategory::Editing),
        entry("x", "release", HelpCategory::Editing),
        entry("e", "edit", HelpCategory::Editing),
        entry("n", "add note", HelpCategory::Editing),
        // View
        entry("Tab", "pivot to deck at tension", HelpCategory::View),
        entry("Shift+Tab", "return to deck", HelpCategory::View),
        entry("L", "open logbase", HelpCategory::View),
        entry("Esc", "return to deck", HelpCategory::View),
    ]
}

fn logbase_entries() -> Vec<HelpEntry> {
    vec![
        // Navigation
        entry("j/k", "scroll events", HelpCategory::Navigation),
        entry("J/K", "jump between epochs", HelpCategory::Navigation),
        entry("Enter / Space", "expand event detail", HelpCategory::Navigation),
        // View
        entry("Tab", "switch to survey", HelpCategory::View),
        entry("Shift+Tab", "return to origin", HelpCategory::View),
        entry("L", "close logbase", HelpCategory::View),
        entry("Esc", "return to origin", HelpCategory::View),
    ]
}

fn entry(key: &str, desc: &str, category: HelpCategory) -> HelpEntry {
    HelpEntry::new(key, desc).with_category(category)
}

// ---------------------------------------------------------------------------
// HintRanker setup
// ---------------------------------------------------------------------------

/// Build a HintRanker pre-loaded with all keybindings and static priorities.
/// Priority ordering: navigation keys first (most useful for new users),
/// then acts, then view switches.
pub fn build_ranker(registry: &HelpRegistry) -> HintRanker {
    let mut ranker = HintRanker::new(RankerConfig::default());

    // Register hints from each view with decreasing priority
    let views = [
        (HELP_DECK, HintContext::Mode("deck".into())),
        (HELP_SURVEY, HintContext::Mode("survey".into())),
        (HELP_LOGBASE, HintContext::Mode("logbase".into())),
        (HELP_GLOBAL, HintContext::Global),
    ];

    for (help_id, context) in views {
        if let Some(content) = registry.peek(help_id) {
            for (i, kb) in content.keybindings.iter().enumerate() {
                let label = format!("{} {}", kb.key, kb.action);
                let cost = 1.0; // uniform display cost
                let priority = (100 - i.min(99)) as u32; // earlier = higher priority
                ranker.register(&label, cost, context.clone(), priority);
            }
        }
    }

    ranker
}

// ---------------------------------------------------------------------------
// Shortcut lookup — for palette action annotations
// ---------------------------------------------------------------------------

/// Map palette action IDs to their keyboard shortcuts.
pub fn action_shortcut(action_id: &str) -> Option<&'static str> {
    match action_id {
        "add" => Some("a"),
        "resolve" => Some("r"),
        "release" => Some("x"),
        "descend" => Some("l"),
        "ascend" => Some("h"),
        "edit_desire" => Some("e"),
        "edit_reality" => Some("e"),
        "edit_horizon" => Some("e"),
        "note" => Some("n"),
        "move" => Some("m"),
        "undo" => Some("u"),
        "redo" => Some("U"),
        "survey" => Some("Tab"),
        "logbase" => Some("L"),
        "hold" => Some("p"),
        "quit" => Some("q"),
        _ => None,
    }
}
