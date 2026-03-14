use crate::msg::Msg;

pub struct InputOverlay {
    pub prompt: String,
    pub buffer: String,
    pub cursor: usize,
}

impl InputOverlay {
    pub fn new(prompt: String, prefill: String) -> Self {
        let cursor = prefill.len();
        Self {
            prompt,
            buffer: prefill,
            cursor,
        }
    }
}

pub enum InputMode {
    Normal,
    TextInput(InputContext),
    Confirm(ConfirmAction),
    MovePicker(MovePickerState),
    Reflect,
}

pub enum InputContext {
    UpdateReality(String),
    UpdateDesire(String),
    AddNote(String),
    SetHorizon(String),
    AddTensionDesired { parent_id: Option<String> },
    /// Create child: desired state step (carries parent_id)
    CreateChildDesired(String),
    /// Create child: horizon step (carries parent_id, desired)
    CreateChildHorizon(String, String),
    /// Create child: actual state step (carries parent_id, desired, horizon)
    CreateChildActual { parent_id: String, desired: String, horizon: Option<String> },
    /// Create parent: desired state step (carries child_id)
    CreateParentDesired(String),
    /// Create parent: horizon step (carries child_id, desired)
    CreateParentHorizon(String, String),
    /// Create parent: actual state step (carries child_id, desired, horizon)
    CreateParentActual { child_id: String, desired: String, horizon: Option<String> },
    AgentPrompt(String),
    SetRecurrence(String),
    SetSnooze(String),
}

pub enum ConfirmAction {
    Delete { id: String, desired: String },
}

pub struct MovePickerState {
    pub tension_id: String,
    pub candidates: Vec<(String, String)>,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Welcome,
    Dashboard,
    Detail,
    TreeView,
    Neighborhood,
    Timeline,
    Focus,
    DynamicsSummary,
    Agent(String),
}

#[derive(Debug, Clone)]
pub struct PaletteAction {
    pub name: &'static str,
    pub description: &'static str,
    pub msg: Option<Msg>,
}

pub struct CommandPaletteState {
    pub query: String,
    pub cursor: usize,
    pub selected: usize,
    pub actions: Vec<PaletteAction>,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor: 0,
            selected: 0,
            actions: all_palette_actions(),
        }
    }

    pub fn filtered_actions(&self) -> Vec<&PaletteAction> {
        if self.query.is_empty() {
            return self.actions.iter().collect();
        }
        let q = self.query.to_lowercase();
        self.actions
            .iter()
            .filter(|a| {
                a.name.to_lowercase().contains(&q) || a.description.to_lowercase().contains(&q)
            })
            .collect()
    }
}

pub fn all_palette_actions() -> Vec<PaletteAction> {
    vec![
        PaletteAction { name: "add", description: "Create a new tension", msg: Some(Msg::StartAddTension) },
        PaletteAction { name: "reality", description: "Update current state", msg: Some(Msg::StartUpdateReality) },
        PaletteAction { name: "desire", description: "Update desired state", msg: Some(Msg::StartUpdateDesire) },
        PaletteAction { name: "resolve", description: "Mark as resolved", msg: Some(Msg::StartResolve) },
        PaletteAction { name: "release", description: "Release (let go)", msg: Some(Msg::StartRelease) },
        PaletteAction { name: "delete", description: "Delete tension", msg: Some(Msg::StartDelete) },
        PaletteAction { name: "move", description: "Reparent tension", msg: Some(Msg::StartMove) },
        PaletteAction { name: "child", description: "Create child tension", msg: Some(Msg::CreateChild) },
        PaletteAction { name: "parent", description: "Create parent tension", msg: Some(Msg::CreateParent) },
        PaletteAction { name: "note", description: "Add a note", msg: Some(Msg::StartAddNote) },
        PaletteAction { name: "horizon", description: "Set horizon", msg: Some(Msg::StartSetHorizon) },
        PaletteAction { name: "tree", description: "Switch to tree view", msg: Some(Msg::SwitchTree) },
        PaletteAction { name: "dashboard", description: "Switch to dashboard", msg: Some(Msg::SwitchDashboard) },
        PaletteAction { name: "agent", description: "Open agent view", msg: Some(Msg::StartAgent) },
        PaletteAction { name: "timeline", description: "Toggle timeline panel", msg: Some(Msg::ToggleTimeline) },
        PaletteAction { name: "health", description: "Toggle health overlay", msg: Some(Msg::ToggleHealthOverlay) },
        PaletteAction { name: "reflect", description: "Free-form writing about a tension", msg: Some(Msg::StartReflect) },
        PaletteAction { name: "snooze", description: "Snooze tension until a date", msg: Some(Msg::StartSnooze) },
        PaletteAction { name: "insights", description: "Behavioral pattern analysis", msg: Some(Msg::ShowInsights) },
        PaletteAction { name: "trajectory", description: "Field trajectory overview", msg: Some(Msg::ShowTrajectory) },
        PaletteAction { name: "help", description: "Show help", msg: Some(Msg::ToggleHelp) },
        PaletteAction { name: "quit", description: "Exit werk", msg: Some(Msg::Quit) },
    ]
}
