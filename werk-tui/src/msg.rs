//! Messages the Operative Instrument can process.

use ftui::{Event, KeyCode, Modifiers};

#[derive(Debug, Clone)]
pub enum Msg {
    // Navigation
    Up,
    Down,
    Descend,
    Ascend,
    JumpTop,
    JumpBottom,

    // Depths
    ToggleGaze,
    ExpandGaze, // Tab inside gaze -> full dynamics

    // Acts
    StartAdd,
    StartEdit,
    StartNote,
    StartResolve,
    StartRelease,
    StartMove,
    MoveUp,    // Shift+K — move tension toward vision
    MoveDown,  // Shift+J — move tension toward reality

    // Text input (shared across all input modes)
    Char(char),
    Backspace,
    Submit,
    Cancel,
    Tab, // switch fields in edit mode

    // Agent
    InvokeAgent,
    AgentClipboard,
    AgentResponse(Result<String, String>),

    // Insights
    OpenInsights,

    // Chrome
    ToggleHelp,
    Search,
    CycleFilter,
    Undo,

    // System
    DataChanged,
    Tick,
    Quit,
    Noop,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key) => {
                if key.ctrl() && key.code == KeyCode::Char('c') {
                    return Msg::Quit;
                }
                // All other key routing happens in update() based on InputMode.
                // We pass through as Char for printable keys in input modes,
                // and dispatch specific Msg for navigation keys in Normal mode.
                // This requires the update() function to handle RawKey routing.
                match key.code {
                    KeyCode::Char(c) if key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT => {
                        Msg::Char(c)
                    }
                    KeyCode::Enter => Msg::Submit,
                    KeyCode::Escape => Msg::Cancel,
                    KeyCode::Backspace => Msg::Backspace,
                    KeyCode::Tab => Msg::Tab,
                    KeyCode::Up => Msg::Up,
                    KeyCode::Down => Msg::Down,
                    KeyCode::Left => Msg::Ascend,
                    KeyCode::Right => Msg::Descend,
                    _ => Msg::Noop,
                }
            }
            _ => Msg::Noop,
        }
    }
}
