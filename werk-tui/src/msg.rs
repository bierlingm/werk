use ftui::{Event, KeyCode, Modifiers};
use crate::types::ToastSeverity;

/// Messages the app can process.
#[derive(Debug, Clone)]
pub enum Msg {
    // Existing
    MoveUp,
    MoveDown,
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
    StartSetRecurrence,

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

    // View consolidation: toggleable panels
    ToggleTimeline,
    ToggleHealthOverlay,

    // Phase 15A: Reflect
    StartReflect,
    ReflectSubmit,

    // Ticker jump: jump to Nth most urgent tension (0-indexed)
    TickerJump(usize),

    // Snooze
    StartSnooze,
    ToggleShowSnoozed,

    // Phase 9: Lever
    ShowLever,

    // Undo last resolve/release
    Undo,

    // Behavioral pattern insights
    ShowInsights,

    // Trajectory overlay
    ShowTrajectory,

    // Adjustable split pane ratio
    SplitWider,
    SplitNarrower,

    // Filesystem watcher detected external db change
    ExternalChange,

    // Raw key event for mode-based routing (carries full modifiers)
    RawKey(KeyCode, Modifiers),
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
                Msg::RawKey(key.code, key.modifiers)
            }
            _ => Msg::Noop,
        }
    }
}
