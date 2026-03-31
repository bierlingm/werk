//! Focus graph for spatial navigation.
//!
//! Phase 2: structure only — nodes registered, connections wired.
//! Phase 4 (#164) wires this to keybindings and actual navigation.

use ftui::layout::Rect;
use ftui::widgets::{FocusGraph, FocusId, FocusNode, NavDirection};

/// Well-known focus node IDs.
/// Using constants so they're stable across graph rebuilds.
pub const FOCUS_DESIRE: FocusId = 1;
pub const FOCUS_ROUTE: FocusId = 2;
pub const FOCUS_CONSOLE: FocusId = 3;
pub const FOCUS_HELD: FocusId = 4;
pub const FOCUS_INPUT_POINT: FocusId = 5;
pub const FOCUS_ACCUMULATED: FocusId = 6;
pub const FOCUS_REALITY: FocusId = 7;

/// Named focus region IDs for readability.
pub struct FocusIds {
    pub desire: FocusId,
    pub route: FocusId,
    pub console: FocusId,
    pub held: FocusId,
    pub input_point: FocusId,
    pub accumulated: FocusId,
    pub reality: FocusId,
}

impl Default for FocusIds {
    fn default() -> Self {
        Self {
            desire: FOCUS_DESIRE,
            route: FOCUS_ROUTE,
            console: FOCUS_CONSOLE,
            held: FOCUS_HELD,
            input_point: FOCUS_INPUT_POINT,
            accumulated: FOCUS_ACCUMULATED,
            reality: FOCUS_REALITY,
        }
    }
}

/// Focus graph state — skeleton for Phase 2.
pub struct FocusState {
    pub graph: FocusGraph,
    pub ids: FocusIds,
    pub active: FocusId,
}

impl FocusState {
    pub fn new() -> Self {
        let mut graph = FocusGraph::new();
        let ids = FocusIds::default();

        // Register all spatial zones as focus nodes.
        // Bounds are Rect::default() — updated each frame when pane rects are computed.
        graph.insert(FocusNode::new(ids.desire, Rect::default()));
        graph.insert(FocusNode::new(ids.route, Rect::default()));
        graph.insert(FocusNode::new(ids.console, Rect::default()));
        graph.insert(FocusNode::new(ids.held, Rect::default()));
        graph.insert(FocusNode::new(ids.input_point, Rect::default()));
        graph.insert(FocusNode::new(ids.accumulated, Rect::default()));
        graph.insert(FocusNode::new(ids.reality, Rect::default()));

        // Vertical connections — the spatial law axis (desire → reality).
        graph.connect(ids.desire, NavDirection::Down, ids.route);
        graph.connect(ids.route, NavDirection::Down, ids.console);
        graph.connect(ids.console, NavDirection::Down, ids.held);
        graph.connect(ids.held, NavDirection::Down, ids.input_point);
        graph.connect(ids.input_point, NavDirection::Down, ids.accumulated);
        graph.connect(ids.accumulated, NavDirection::Down, ids.reality);

        // Reverse connections (reality → desire).
        graph.connect(ids.reality, NavDirection::Up, ids.accumulated);
        graph.connect(ids.accumulated, NavDirection::Up, ids.input_point);
        graph.connect(ids.input_point, NavDirection::Up, ids.held);
        graph.connect(ids.held, NavDirection::Up, ids.console);
        graph.connect(ids.console, NavDirection::Up, ids.route);
        graph.connect(ids.route, NavDirection::Up, ids.desire);

        Self {
            graph,
            ids,
            active: FOCUS_ROUTE, // Start in the route zone (field center).
        }
    }
}
