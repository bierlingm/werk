use ftui::{Event, KeyCode};
use crate::types::ToastSeverity;

/// Messages the app can process.
#[derive(Debug, Clone)]
pub enum Msg {
    // Existing
    MoveUp,
    MoveDown,
    ToggleResolved,
    ToggleHelp,
    Quit,
    Noop,

    // New navigation
    OpenDetail,
    Back,
    SwitchDashboard,
    SwitchTree,

    // Detail view
    ScrollDetailUp,
    ScrollDetailDown,

    // Filtering
    CycleFilter,

    // Verbose toggle
    ToggleVerbose,

    // Phase 3: CRUD operations
    StartUpdateReality,
    StartUpdateDesire,
    StartAddNote,
    StartSetHorizon,
    StartAddTension,
    StartResolve,
    StartRelease,
    StartDelete,
    StartMove,

    // Input overlay events
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputLeft,
    InputRight,
    InputHome,
    InputEnd,
    InputSubmit,
    InputCancel,

    // Confirm events
    ConfirmYes,
    ConfirmNo,

    // Move picker events
    PickerUp,
    PickerDown,
    PickerSelect,
    PickerCancel,

    // Phase 4: Dynamics events and periodic tick
    Tick,
    DynamicsEvent(String, ToastSeverity),

    // Phase 5: Agent integration
    StartAgent,
    AgentResponseReceived(std::result::Result<String, String>),
    AgentToggleMutation(usize),
    AgentApplySelected,
    AgentScrollUp,
    AgentScrollDown,

    // Phase 7: Quick Create + Adopt/Reparent
    CreateChild,
    CreateParent,

    // Phase 10: Neighborhood view
    ViewNeighborhood,

    // Phase 12: Timeline view
    ViewTimeline,

    // Phase 13: Focus mode
    ViewFocus,

    // Phase 6: Welcome screen
    WelcomeSelect,
    WelcomeConfirm,

    // Phase 6: Command palette & search
    OpenCommandPalette,
    OpenSearch,

    // Phase 14: Dynamics summary dashboard
    ViewDynamics,

    // Phase 15A: Reflect
    StartReflect,
    ReflectSubmit,

    // Phase 9: Lever
    ShowLever,

    // Raw key event for mode-based routing
    RawKey(KeyCode, bool),
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key) => {
                if key.ctrl() && key.code == KeyCode::Char('c') {
                    return Msg::Quit;
                }
                if key.ctrl() && key.code == KeyCode::Char('s') {
                    return Msg::ReflectSubmit;
                }
                Msg::RawKey(key.code, key.shift())
            }
            _ => Msg::Noop,
        }
    }
}
