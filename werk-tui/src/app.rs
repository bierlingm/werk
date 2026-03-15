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

/// The kind of action previewed in the what-if overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhatIfAction {
    Resolve,
    Release,
}

/// A what-if counterfactual preview shown before resolving or releasing.
pub struct WhatIfPreview {
    pub tension_id: String,
    pub tension_desired: String,
    pub action: WhatIfAction,
    pub orphaned_children: Vec<String>,
    pub auto_resolved_parents: Vec<String>,
    pub children_count: usize,
}

/// A pending undo action for resolve/release that expires after a timeout.
pub struct UndoAction {
    pub description: String,
    pub tension_id: String,
    pub previous_status: String,
    pub expires_at: std::time::Instant,
}

/// View-specific state for the Detail view.
pub struct DetailState {
    pub(crate) tension: Option<Tension>,
    pub(crate) scroll: u16,
    pub(crate) cursor: usize,  // index into the flat list of navigable items
    pub(crate) mutations: Vec<MutationDisplay>,
    pub(crate) children: Vec<TensionRow>,
    pub(crate) dynamics: Option<DetailDynamics>,
    pub(crate) parent: Option<Tension>,
    pub(crate) ancestors: Vec<(String, String)>,  // (id, desired), root-first
    pub(crate) nav_stack: Vec<String>,            // for back-navigation
}

/// View-specific state for the Agent view.
pub struct AgentState {
    pub(crate) output: Vec<String>,
    pub(crate) scroll: u16,
    pub(crate) mutations: Vec<AgentMutation>,
    pub(crate) mutation_selected: Vec<bool>,
    pub(crate) mutation_cursor: usize,
    pub(crate) running: bool,
    pub(crate) response_text: Option<String>,
}

/// View-specific state for search.
pub struct SearchState {
    pub(crate) query: Option<String>,
    pub(crate) buffer: String,
    pub(crate) cursor: usize,
    pub(crate) active: bool,
    pub(crate) input_widget: TextInput,
}

/// View-specific state for the Reflect overlay.
pub struct ReflectState {
    pub(crate) textarea: Option<TextArea>,
    pub(crate) tension_id: Option<String>,
}

/// The main TUI application.
pub struct WerkApp {
    pub(crate) engine: DynamicsEngine,
    pub(crate) tensions: Vec<TensionRow>,
    pub(crate) dashboard_state: RefCell<TableState>,
    pub(crate) active_view: View,
    pub(crate) show_help: bool,
    pub(crate) filter: Filter,
    #[allow(dead_code)]
    pub(crate) status_message: Option<String>,
    pub(crate) total_active: usize,
    pub(crate) total_resolved: usize,
    pub(crate) total_released: usize,
    pub(crate) total_neglected: usize,
    pub(crate) total_urgent: usize,

    // View-specific state
    pub(crate) detail: DetailState,
    pub(crate) agent: AgentState,
    pub(crate) search: SearchState,
    pub(crate) reflect: ReflectState,

    // Neighborhood view state
    pub(crate) neighborhood_tension_id: Option<String>,
    pub(crate) neighborhood_items: Vec<(String, String)>, // (tension_id, role label)
    pub(crate) neighborhood_state: RefCell<ListState>,

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

    // Phase 6: Welcome screen
    pub(crate) welcome_selected: usize,

    // Phase 6: Command palette (native ftui widget)
    pub(crate) command_palette: CommandPalette,

    // Phase 9/11: Lever
    pub(crate) lever: Option<LeverResult>,
    pub(crate) show_lever_overlay: bool,

    // View consolidation: toggleable panels/overlays
    pub(crate) show_timeline: bool,
    pub(crate) show_health_overlay: bool,

    // Native ftui widget state for input overlay migration
    pub(crate) text_input_widget: TextInput,
    pub(crate) move_picker_state: RefCell<ListState>,

    // Undo support for resolve/release
    pub(crate) pending_undo: Option<UndoAction>,

    // Snooze: hide snoozed tensions from dashboard
    pub(crate) show_snoozed: bool,

    // What-if counterfactual preview before resolve/release
    pub(crate) what_if_preview: Option<WhatIfPreview>,

    // Adjustable split pane ratio (Phase 4)
    pub(crate) split_ratio: f64,

    // Behavioral pattern insights overlay
    pub(crate) show_insights_overlay: bool,
    pub(crate) insights_lines: Vec<ftui::text::Line>,

    // Trajectory overlay
    pub(crate) show_trajectory_overlay: bool,
    pub(crate) trajectory_lines: Vec<ftui::text::Line>,

    // Projection cache (recomputed every 5 minutes)
    pub(crate) field_projection: Option<sd_core::FieldProjection>,
    pub(crate) last_projection_time: Option<chrono::DateTime<chrono::Utc>>,
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
            show_help: false,
            filter: Filter::Active,
            status_message: None,
            total_active,
            total_resolved,
            total_released,
            total_neglected,
            total_urgent,

            detail: DetailState {
                tension: None,
                scroll: 0,
                cursor: 0,
                mutations: Vec::new(),
                children: Vec::new(),
                dynamics: None,
                parent: None,
                ancestors: Vec::new(),
                nav_stack: Vec::new(),
            },

            agent: AgentState {
                output: Vec::new(),
                scroll: 0,
                mutations: Vec::new(),
                mutation_selected: Vec::new(),
                mutation_cursor: 0,
                running: false,
                response_text: None,
            },

            search: SearchState {
                query: None,
                buffer: String::new(),
                cursor: 0,
                active: false,
                input_widget: TextInput::new(),
            },

            reflect: ReflectState {
                textarea: None,
                tension_id: None,
            },

            neighborhood_tension_id: None,
            neighborhood_items: Vec::new(),
            neighborhood_state: RefCell::new(ListState::default()),

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

            welcome_selected: 0,
            command_palette: Self::build_command_palette(),

            lever: None,
            show_lever_overlay: false,

            show_timeline: false,
            show_health_overlay: false,

            text_input_widget: TextInput::new(),
            move_picker_state: RefCell::new(ListState::default()),

            pending_undo: None,

            show_snoozed: false,

            show_insights_overlay: false,
            insights_lines: Vec::new(),

            show_trajectory_overlay: false,
            trajectory_lines: Vec::new(),

            field_projection: None,
            last_projection_time: None,

            what_if_preview: None,

            split_ratio: 0.4,
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
            "timeline" => Some(Msg::ToggleTimeline),
            "health" => Some(Msg::ToggleHealthOverlay),
            "reflect" => Some(Msg::StartReflect),
            "snooze" => Some(Msg::StartSnooze),
            "insights" => Some(Msg::ShowInsights),
            "trajectory" => Some(Msg::ShowTrajectory),
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

    /// Count of navigable items in the Detail view.
    /// cursor 0 = Info, cursor 1 = Dynamics, then mutations, then children.
    pub(crate) fn detail_item_count(&self) -> usize {
        let mut count = 2; // Info section + Dynamics section
        count += self.detail.mutations.len();
        count += self.detail.children.len();
        count.max(1)
    }

    /// Count of currently snoozed tensions (uses cached flag on TensionRow).
    pub(crate) fn snoozed_count(&self) -> usize {
        self.tensions.iter().filter(|t| t.snoozed).count()
    }

    /// Visible tensions based on current filter, search query, and snooze state.
    /// Snooze state is cached on TensionRow during reload_data() to avoid per-call SQLite queries.
    pub(crate) fn visible_tensions(&self) -> Vec<&TensionRow> {
        self.tensions
            .iter()
            .filter(|t| self.show_snoozed || !t.snoozed)
            .filter(|t| match self.filter {
                Filter::Active => t.tier != UrgencyTier::Resolved,
                Filter::All => true,
                Filter::Resolved => t.status == "Resolved",
                Filter::Released => t.status == "Released",
            })
            .filter(|t| {
                if let Some(ref q) = self.search.query {
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
