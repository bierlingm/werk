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

    /// Raw event passthrough — carries full modifier info for TextInput widget.
    RawEvent(Event),

    // Yaw
    BackTab, // Shift+Tab — return yaw (go back without pivoting)

    // Zoom
    ShiftSubmit, // Shift+Enter — orient zoom (V9)

    // Chrome
    ToggleHelp,
    Search,
    CycleFilter,
    Undo,
    Redo,
    OpenPalette,

    // System
    Resize { width: u16, height: u16 },
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
                // Keys with Ctrl/Alt/Super modifiers (except Shift-only) get passed through
                // as RawEvent so TextInput can handle word-level operations.
                let has_modifier = key.modifiers.intersects(
                    Modifiers::CTRL | Modifiers::ALT | Modifiers::SUPER
                );
                if has_modifier {
                    // Intercept specific Ctrl combos before generic RawEvent passthrough
                    if key.ctrl() {
                        match key.code {
                            KeyCode::Char('z') if key.shift() => return Msg::Redo,
                            KeyCode::Char('z') => return Msg::Undo,
                            KeyCode::Char('k') => return Msg::OpenPalette,
                            _ => {}
                        }
                    }
                    return Msg::RawEvent(Event::Key(key));
                }

                match key.code {
                    KeyCode::Char(c) if key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT => {
                        Msg::Char(c)
                    }
                    KeyCode::Enter if key.shift() => Msg::ShiftSubmit,
                    KeyCode::Enter => Msg::Submit,
                    KeyCode::Escape => Msg::Cancel,
                    KeyCode::Backspace => Msg::Backspace,
                    KeyCode::Delete => Msg::RawEvent(Event::Key(key)),
                    KeyCode::Tab => Msg::Tab,
                    KeyCode::BackTab => Msg::BackTab,
                    KeyCode::Up if key.shift() => Msg::MoveUp,
                    KeyCode::Down if key.shift() => Msg::MoveDown,
                    KeyCode::Up => Msg::Up,
                    KeyCode::Down => Msg::Down,
                    KeyCode::Left => Msg::RawEvent(Event::Key(key)),
                    KeyCode::Right => Msg::RawEvent(Event::Key(key)),
                    KeyCode::Home => Msg::RawEvent(Event::Key(key)),
                    KeyCode::End => Msg::RawEvent(Event::Key(key)),
                    _ => Msg::Noop,
                }
            }
            Event::Paste(_) => Msg::RawEvent(event),
            Event::Resize { width, height } => Msg::Resize { width, height },
            _ => Msg::Noop,
        }
    }
}
