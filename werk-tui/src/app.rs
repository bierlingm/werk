use std::cell::RefCell;
use std::collections::HashMap;

use ftui::widgets::command_palette::{CommandPalette, ActionItem};
use ftui::widgets::input::TextInput;
use ftui::widgets::list::ListState;
use ftui::widgets::table::TableState;
use ftui::widgets::textarea::TextArea;
use sd_core::DynamicsEngine;
use werk_shared::AgentMutation;

use crate::input::{InputMode, InputOverlay, View};
use crate::lever::LeverResult;
use crate::types::{
    DetailDynamics, Filter, MutationDisplay, TensionRow, Toast, UrgencyTier,
};
use sd_core::Tension;

/// The main TUI application.
pub struct WerkApp {
    pub(crate) engine: DynamicsEngine,
    pub(crate) tensions: Vec<TensionRow>,
    pub(crate) dashboard_state: RefCell<TableState>,
    pub(crate) active_view: View,
    pub(crate) show_resolved: bool,
    pub(crate) show_help: bool,
    pub(crate) filter: Filter,
    pub(crate) verbose: bool,
    #[allow(dead_code)]
    pub(crate) status_message: Option<String>,
    pub(crate) total_active: usize,
    pub(crate) total_resolved: usize,
    pub(crate) total_released: usize,
    pub(crate) total_neglected: usize,
    pub(crate) total_urgent: usize,

    // Detail view state
    pub(crate) detail_tension: Option<Tension>,
    pub(crate) detail_scroll: u16,
    pub(crate) detail_mutations: Vec<MutationDisplay>,
    pub(crate) detail_children: Vec<TensionRow>,
    pub(crate) detail_dynamics: Option<DetailDynamics>,
    pub(crate) detail_parent: Option<Tension>,
    pub(crate) detail_ancestors: Vec<(String, String)>,  // (id, desired), root-first
    pub(crate) detail_nav_stack: Vec<String>,            // for back-navigation

    // Neighborhood view state
    pub(crate) neighborhood_tension_id: Option<String>,

    // Tree view state
    pub(crate) tree_state: RefCell<ListState>,
    pub(crate) tree_items: Vec<crate::types::TreeItem>,

    // Phase 3: Input mode
    pub(crate) input_mode: InputMode,
    pub(crate) input_overlay: Option<InputOverlay>,
    pub(crate) status_toast: Option<String>,

    // Phase 4: Toasts and dynamics tracking
    pub(crate) toasts: Vec<Toast>,
    pub(crate) previous_urgencies: HashMap<String, f64>,

    // Phase 5: Agent integration
    pub(crate) agent_output: Vec<String>,
    pub(crate) agent_scroll: u16,
    pub(crate) agent_mutations: Vec<AgentMutation>,
    pub(crate) agent_mutation_selected: Vec<bool>,
    pub(crate) agent_mutation_cursor: usize,
    pub(crate) agent_running: bool,
    pub(crate) agent_response_text: Option<String>,

    // Phase 6: Welcome screen
    pub(crate) welcome_selected: usize,

    // Phase 6: Command palette (native ftui widget)
    pub(crate) command_palette: CommandPalette,

    // Phase 6: Search
    pub(crate) search_query: Option<String>,
    pub(crate) search_buffer: String,
    pub(crate) search_cursor: usize,
    pub(crate) search_active: bool,

    // Phase 9/11: Lever
    pub(crate) lever: Option<LeverResult>,
    pub(crate) show_lever_overlay: bool,

    // Phase 15A: Reflect (native ftui TextArea widget)
    pub(crate) reflect_textarea: Option<TextArea>,
    pub(crate) reflect_tension_id: Option<String>,

    // Native ftui widget state for input overlay migration
    pub(crate) text_input_widget: TextInput,
    pub(crate) move_picker_state: RefCell<ListState>,
    pub(crate) search_input_widget: TextInput,
}

impl WerkApp {
    /// Create a new WerkApp with a DynamicsEngine.
    pub fn new(engine: DynamicsEngine, tensions: Vec<TensionRow>) -> Self {
        let total_active = tensions.iter().filter(|t| t.tier == UrgencyTier::Active).count();
        let total_resolved = tensions.iter().filter(|t| t.tier == UrgencyTier::Resolved).count();
        let total_released = tensions
            .iter()
            .filter(|t| t.status == "Released")
            .count();
        let total_neglected = tensions
            .iter()
            .filter(|t| t.tier == UrgencyTier::Neglected)
            .count();
        let total_urgent = tensions.iter().filter(|t| t.tier == UrgencyTier::Urgent).count();

        Self {
            engine,
            tensions,
            dashboard_state: RefCell::new({
                let mut s = TableState::default();
                s.select(Some(0));
                s
            }),
            active_view: View::Dashboard,
            show_resolved: false,
            show_help: false,
            filter: Filter::Active,
            verbose: false,
            status_message: None,
            total_active,
            total_resolved,
            total_released,
            total_neglected,
            total_urgent,

            detail_tension: None,
            detail_scroll: 0,
            detail_mutations: Vec::new(),
            detail_children: Vec::new(),
            detail_dynamics: None,
            detail_parent: None,
            detail_ancestors: Vec::new(),
            detail_nav_stack: Vec::new(),

            neighborhood_tension_id: None,

            tree_state: RefCell::new({
                let mut s = ListState::default();
                s.select(Some(0));
                s
            }),
            tree_items: Vec::new(),

            input_mode: InputMode::Normal,
            input_overlay: None,
            status_toast: None,

            toasts: Vec::new(),
            previous_urgencies: HashMap::new(),

            agent_output: Vec::new(),
            agent_scroll: 0,
            agent_mutations: Vec::new(),
            agent_mutation_selected: Vec::new(),
            agent_mutation_cursor: 0,
            agent_running: false,
            agent_response_text: None,

            welcome_selected: 0,
            command_palette: Self::build_command_palette(),
            search_query: None,
            search_buffer: String::new(),
            search_cursor: 0,
            search_active: false,

            lever: None,
            show_lever_overlay: false,

            reflect_textarea: None,
            reflect_tension_id: None,

            text_input_widget: TextInput::new(),
            move_picker_state: RefCell::new(ListState::default()),
            search_input_widget: TextInput::new(),
        }
    }

    /// Create a WerkApp in welcome mode (no workspace found).
    pub fn new_welcome() -> Self {
        let engine = DynamicsEngine::new_in_memory()
            .expect("failed to create in-memory engine");
        let mut app = Self::new(engine, Vec::new());
        app.active_view = View::Welcome;
        app
    }

    /// Build and populate the native CommandPalette widget with all actions.
    fn build_command_palette() -> CommandPalette {
        use crate::input::all_palette_actions;
        let mut palette = CommandPalette::new().with_max_visible(14);
        for action in all_palette_actions() {
            let item = ActionItem::new(action.name, action.name)
                .with_description(action.description);
            palette.register_action(item);
        }
        palette
    }

    /// Map a command palette action ID back to the corresponding Msg.
    pub(crate) fn palette_id_to_msg(id: &str) -> Option<crate::msg::Msg> {
        use crate::msg::Msg;
        match id {
            "add" => Some(Msg::StartAddTension),
            "reality" => Some(Msg::StartUpdateReality),
            "desire" => Some(Msg::StartUpdateDesire),
            "resolve" => Some(Msg::StartResolve),
            "release" => Some(Msg::StartRelease),
            "delete" => Some(Msg::StartDelete),
            "move" => Some(Msg::StartMove),
            "child" => Some(Msg::CreateChild),
            "parent" => Some(Msg::CreateParent),
            "note" => Some(Msg::StartAddNote),
            "horizon" => Some(Msg::StartSetHorizon),
            "tree" => Some(Msg::SwitchTree),
            "dashboard" => Some(Msg::SwitchDashboard),
            "agent" => Some(Msg::StartAgent),
            "neighborhood" => Some(Msg::ViewNeighborhood),
            "timeline" => Some(Msg::ViewTimeline),
            "focus" => Some(Msg::ViewFocus),
            "health" => Some(Msg::ViewDynamics),
            "reflect" => Some(Msg::StartReflect),
            "help" => Some(Msg::ToggleHelp),
            "quit" => Some(Msg::Quit),
            _ => None,
        }
    }

    /// Get the current dashboard selected index.
    pub(crate) fn selected(&self) -> usize {
        self.dashboard_state.borrow().selected.unwrap_or(0)
    }

    /// Set the dashboard selected index.
    pub(crate) fn set_selected(&self, index: usize) {
        self.dashboard_state.borrow_mut().select(Some(index));
    }

    /// Get the current tree selected index.
    pub(crate) fn tree_selected(&self) -> usize {
        self.tree_state.borrow().selected.unwrap_or(0)
    }

    /// Set the tree selected index.
    pub(crate) fn set_tree_selected(&self, index: usize) {
        self.tree_state.borrow_mut().select(Some(index));
    }

    /// Visible tensions based on current filter and search query.
    pub(crate) fn visible_tensions(&self) -> Vec<&TensionRow> {
        self.tensions
            .iter()
            .filter(|t| match self.filter {
                Filter::Active => {
                    if self.show_resolved {
                        true
                    } else {
                        t.tier != UrgencyTier::Resolved
                    }
                }
                Filter::All => true,
                Filter::Resolved => t.status == "Resolved",
                Filter::Released => t.status == "Released",
            })
            .filter(|t| {
                if let Some(ref q) = self.search_query {
                    let q_lower = q.to_lowercase();
                    t.desired.to_lowercase().contains(&q_lower)
                        || t.actual.to_lowercase().contains(&q_lower)
                } else {
                    true
                }
            })
            .collect()
    }
}
