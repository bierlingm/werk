//! ProgramSimulator integration tests for the Operative Instrument TUI.
//!
//! Tests verify gesture integrity via the simulator: events are injected,
//! model state is inspected, and frames can be captured for snapshot testing.
//!
//! Key insight: In the add flow, Enter creates immediately with intelligent
//! defaults. Tab advances to the next field. This matches "gesture as unit
//! of change" — Enter is the commit point.

use ftui_runtime::simulator::ProgramSimulator;
use werk_tui::InstrumentApp;
use werk_tui::msg::Msg;
use werk_tui::state::InputMode;

fn test_app() -> InstrumentApp {
    InstrumentApp::new_empty()
}

fn send(sim: &mut ProgramSimulator<InstrumentApp>, msg: Msg) -> bool {
    sim.send(msg);
    sim.is_running()
}

fn type_text(sim: &mut ProgramSimulator<InstrumentApp>, text: &str) {
    for ch in text.chars() {
        send(sim, Msg::Char(ch));
    }
}

#[test]
fn add_tension_enter_creates_immediately() {
    let mut sim = ProgramSimulator::new(test_app());
    sim.init();

    // Start add gesture
    send(&mut sim, Msg::StartAdd);
    assert!(matches!(sim.model().input_mode, InputMode::Adding(_)));

    // Verify we're in Adding mode
    assert!(
        matches!(sim.model().input_mode, InputMode::Adding(_)),
        "should be in Adding mode, got {:?}",
        sim.model().input_mode
    );

    // Type desire and press Enter — creates immediately with defaults
    type_text(&mut sim, "Ship the feature");
    assert_eq!(
        sim.model().input_buffer, "Ship the feature",
        "input buffer should have typed text, got '{}'",
        sim.model().input_buffer
    );
    send(&mut sim, Msg::Submit);

    assert!(
        matches!(sim.model().input_mode, InputMode::Normal),
        "Enter on desire should create and return to Normal, got {:?}",
        sim.model().input_mode
    );
    assert!(
        !sim.model().siblings.is_empty(),
        "expected at least one sibling after add"
    );
    assert_eq!(sim.model().siblings[0].desired, "Ship the feature");
}

#[test]
fn add_tension_tab_advances_fields() {
    let mut sim = ProgramSimulator::new(test_app());
    sim.init();

    send(&mut sim, Msg::StartAdd);
    type_text(&mut sim, "Build API");

    // Tab advances to reality step
    send(&mut sim, Msg::Tab);
    assert!(
        matches!(sim.model().input_mode, InputMode::Adding(werk_tui::state::AddStep::Reality { .. })),
        "Tab should advance to Reality step"
    );

    type_text(&mut sim, "Prototyped");

    // Tab advances to horizon step
    send(&mut sim, Msg::Tab);
    assert!(
        matches!(sim.model().input_mode, InputMode::Adding(werk_tui::state::AddStep::Horizon { .. })),
        "Tab should advance to Horizon step"
    );

    // Enter on horizon creates
    send(&mut sim, Msg::Submit);
    assert!(matches!(sim.model().input_mode, InputMode::Normal));
    assert!(!sim.model().siblings.is_empty());
    assert_eq!(sim.model().siblings[0].desired, "Build API");
    assert_eq!(sim.model().siblings[0].actual, "Prototyped");
}

#[test]
fn navigate_and_resolve() {
    let mut sim = ProgramSimulator::new(test_app());
    sim.init();

    // Add a tension
    send(&mut sim, Msg::StartAdd);
    type_text(&mut sim, "Task to resolve");
    send(&mut sim, Msg::Submit);
    assert!(matches!(sim.model().input_mode, InputMode::Normal));
    assert!(!sim.model().siblings.is_empty());

    // Start resolve
    send(&mut sim, Msg::StartResolve);
    assert!(
        matches!(sim.model().input_mode, InputMode::Confirming(_)),
        "expected Confirming mode after StartResolve"
    );

    // Confirm with 'y'
    send(&mut sim, Msg::Char('y'));
    assert!(matches!(sim.model().input_mode, InputMode::Normal));

    // Should have a resolved tension
    let resolved = sim.model().siblings.iter()
        .filter(|s| s.status == sd_core::TensionStatus::Resolved)
        .count();
    assert_eq!(resolved, 1, "expected one resolved tension");
}

#[test]
fn undo_after_add() {
    let mut sim = ProgramSimulator::new(test_app());
    sim.init();

    assert!(sim.model().siblings.is_empty());

    // Add a tension
    send(&mut sim, Msg::StartAdd);
    type_text(&mut sim, "Undo me");
    send(&mut sim, Msg::Submit);
    assert!(!sim.model().siblings.is_empty());

    // Undo
    send(&mut sim, Msg::Undo);
    assert!(matches!(sim.model().input_mode, InputMode::Normal));
    assert!(sim.is_running());
}

#[test]
fn quit_stops_simulation() {
    let mut sim = ProgramSimulator::new(test_app());
    sim.init();
    assert!(sim.is_running());
    send(&mut sim, Msg::Quit);
    assert!(!sim.is_running());
}

#[test]
fn inspector_toggle() {
    let mut sim = ProgramSimulator::new(test_app());
    sim.init();
    assert!(!sim.model().show_inspector);
    send(&mut sim, Msg::InspectorToggle);
    assert!(sim.model().show_inspector);
    send(&mut sim, Msg::InspectorToggle);
    assert!(!sim.model().show_inspector);
}

#[test]
fn frame_capture_does_not_panic() {
    let mut sim = ProgramSimulator::new(test_app());
    sim.init();
    let _buf = sim.capture_frame(80, 24);
    assert_eq!(sim.frame_count(), 1);
}
