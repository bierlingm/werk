#![forbid(unsafe_code)]

//! werk-tui: FrankenTUI dashboard for structural dynamics.

use chrono::Utc;

use ftui::{App, Cmd, Event, Frame, KeyCode, Model, PackedRgba};
use ftui::layout::{Constraint, Flex, Rect};
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use sd_core::{
    compute_urgency, CreativeCyclePhase, DynamicsEngine, StructuralTendency,
    TensionStatus,
};
use werk_shared::{truncate, Workspace};

// ============================================================================
// Data types
// ============================================================================

/// Urgency tier for display grouping and sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UrgencyTier {
    Urgent,
    Active,
    Neglected,
    Resolved,
}

/// A single tension row prepared for display.
#[derive(Debug, Clone)]
pub struct TensionRow {
    pub id: String,
    pub short_id: String,
    pub desired: String,
    pub actual: String,
    pub status: String,
    pub phase: String,
    pub movement: String,
    pub urgency: Option<f64>,
    pub magnitude: Option<f64>,
    pub neglected: bool,
    pub horizon_display: String,
    pub tier: UrgencyTier,
}

/// The view currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
}

/// Messages the app can process.
#[derive(Debug, Clone)]
pub enum Msg {
    MoveUp,
    MoveDown,
    ToggleResolved,
    ToggleHelp,
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
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => Msg::MoveDown,
                    KeyCode::Char('k') | KeyCode::Up => Msg::MoveUp,
                    KeyCode::Char('R') => Msg::ToggleResolved,
                    KeyCode::Char('?') => Msg::ToggleHelp,
                    KeyCode::Char('q') => Msg::Quit,
                    _ => Msg::Noop,
                }
            }
            _ => Msg::Noop,
        }
    }
}

// ============================================================================
// Application state
// ============================================================================

/// The main TUI application.
pub struct WerkApp {
    tensions: Vec<TensionRow>,
    selected: usize,
    #[allow(dead_code)]
    active_view: View,
    show_resolved: bool,
    show_help: bool,
    #[allow(dead_code)]
    status_message: Option<String>,
    total_active: usize,
    total_resolved: usize,
    total_released: usize,
    total_neglected: usize,
    total_urgent: usize,
}

impl WerkApp {
    /// Create a new WerkApp with preloaded tension rows.
    pub fn new(tensions: Vec<TensionRow>) -> Self {
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
            tensions,
            selected: 0,
            active_view: View::Dashboard,
            show_resolved: false,
            show_help: false,
            status_message: None,
            total_active,
            total_resolved,
            total_released,
            total_neglected,
            total_urgent,
        }
    }

    /// Visible tensions based on current filter.
    fn visible_tensions(&self) -> Vec<&TensionRow> {
        self.tensions
            .iter()
            .filter(|t| {
                if self.show_resolved {
                    true
                } else {
                    t.tier != UrgencyTier::Resolved
                }
            })
            .collect()
    }
}

impl Model for WerkApp {
    type Message = Msg;

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::MoveDown => {
                let visible = self.visible_tensions().len();
                if visible > 0 && self.selected < visible - 1 {
                    self.selected += 1;
                }
                Cmd::None
            }
            Msg::MoveUp => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Cmd::None
            }
            Msg::ToggleResolved => {
                self.show_resolved = !self.show_resolved;
                let visible = self.visible_tensions().len();
                if visible > 0 && self.selected >= visible {
                    self.selected = visible - 1;
                }
                Cmd::None
            }
            Msg::ToggleHelp => {
                self.show_help = !self.show_help;
                Cmd::None
            }
            Msg::Quit => Cmd::Quit,
            Msg::Noop => Cmd::None,
        }
    }

    fn view(&self, frame: &mut Frame<'_>) {
        let area = Rect::new(0, 0, frame.width(), frame.height());

        let layout = Flex::vertical().constraints([
            Constraint::Fixed(1),
            Constraint::Fill,
            Constraint::Fixed(1),
        ]);
        let rects = layout.split(area);

        self.render_title_bar(&rects[0], frame);
        self.render_tension_list(&rects[1], frame);
        self.render_key_hints(&rects[2], frame);

        if self.show_help {
            self.render_help_overlay(area, frame);
        }
    }
}

// ============================================================================
// Rendering helpers
// ============================================================================

// Color constants
const CLR_WHITE: PackedRgba = PackedRgba::rgb(255, 255, 255);
const CLR_LIGHT_GRAY: PackedRgba = PackedRgba::rgb(200, 200, 200);
const CLR_MID_GRAY: PackedRgba = PackedRgba::rgb(120, 120, 120);
const CLR_DIM_GRAY: PackedRgba = PackedRgba::rgb(100, 100, 100);
const CLR_RED: PackedRgba = PackedRgba::rgb(255, 80, 80);
const CLR_RED_SOFT: PackedRgba = PackedRgba::rgb(255, 100, 100);
const CLR_GREEN: PackedRgba = PackedRgba::rgb(80, 200, 120);
const CLR_YELLOW: PackedRgba = PackedRgba::rgb(255, 200, 60);
const CLR_YELLOW_SOFT: PackedRgba = PackedRgba::rgb(200, 180, 80);
const CLR_BG_DARK: PackedRgba = PackedRgba::rgb(30, 30, 30);

impl WerkApp {
    fn render_title_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let status = format!(
            " werk  |  {} active  {} urgent  {} neglected  {} resolved  {} released",
            self.total_active,
            self.total_urgent,
            self.total_neglected,
            self.total_resolved,
            self.total_released,
        );
        let style = Style::new().fg(CLR_LIGHT_GRAY).bold();
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&status, style)]));
        paragraph.render(*area, frame);
    }

    fn render_tension_list(&self, area: &Rect, frame: &mut Frame<'_>) {
        let visible = self.visible_tensions();
        if visible.is_empty() {
            let msg = Paragraph::new("  No tensions found. Use `werk add` to create one.");
            msg.render(*area, frame);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        let mut current_tier: Option<UrgencyTier> = None;

        for (idx, row) in visible.iter().enumerate() {
            if current_tier != Some(row.tier) {
                current_tier = Some(row.tier);
                let (header, header_style) = match row.tier {
                    UrgencyTier::Urgent => (" URGENT", Style::new().fg(CLR_RED).bold()),
                    UrgencyTier::Active => (" ACTIVE", Style::new().fg(CLR_GREEN).bold()),
                    UrgencyTier::Neglected => (" NEGLECTED", Style::new().fg(CLR_YELLOW).bold()),
                    UrgencyTier::Resolved => (" RESOLVED", Style::new().fg(CLR_MID_GRAY).bold()),
                };
                if !lines.is_empty() {
                    lines.push(Line::from(""));
                }
                lines.push(Line::from_spans([Span::styled(header, header_style)]));
            }

            let is_selected = idx == self.selected;
            let line = format_tension_line(row, is_selected, area.width as usize);
            lines.push(line);
        }

        let text = Text::from_lines(lines);
        let paragraph = Paragraph::new(text).scroll((self.scroll_offset(area.height), 0));
        paragraph.render(*area, frame);
    }

    fn scroll_offset(&self, viewport_height: u16) -> u16 {
        let visible = self.visible_tensions();
        let mut line_of_selected: u16 = 0;
        let mut current_tier: Option<UrgencyTier> = None;

        for (idx, row) in visible.iter().enumerate() {
            if current_tier != Some(row.tier) {
                current_tier = Some(row.tier);
                if idx > 0 {
                    line_of_selected += 1; // blank line
                }
                line_of_selected += 1; // header
            }
            if idx == self.selected {
                break;
            }
            line_of_selected += 1;
        }

        let vp = viewport_height.saturating_sub(2);
        line_of_selected.saturating_sub(vp)
    }

    fn render_key_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = " j/k navigate  R show/hide resolved  q quit  ? help";
        let style = Style::new().fg(CLR_MID_GRAY);
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(hints, style)]));
        paragraph.render(*area, frame);
    }

    fn render_help_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let help_width = 50u16.min(area.width.saturating_sub(4));
        let help_height = 12u16.min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(help_width)) / 2;
        let y = (area.height.saturating_sub(help_height)) / 2;
        let help_area = Rect::new(x, y, help_width, help_height);

        let help_lines = vec![
            Line::from_spans([Span::styled(
                " werk \u{2014} structural dynamics dashboard",
                Style::new().bold(),
            )]),
            Line::from(""),
            Line::from("  j / Down    Move down"),
            Line::from("  k / Up      Move up"),
            Line::from("  R           Toggle resolved tensions"),
            Line::from("  ?           Toggle this help"),
            Line::from("  q / Ctrl+C  Quit"),
            Line::from(""),
            Line::from_spans([Span::styled("  Press ? to close", Style::new().dim())]),
        ];

        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let paragraph = Paragraph::new(Text::from_lines(help_lines)).style(bg_style);
        paragraph.render(help_area, frame);
    }
}

fn format_tension_line(row: &TensionRow, selected: bool, width: usize) -> Line {
    let marker = if selected { ">" } else { " " };
    let phase_str = format!("[{}]", row.phase);

    // Urgency bar (6 chars wide)
    let urgency_bar = match row.urgency {
        Some(u) => {
            let filled = ((u * 6.0).round() as usize).min(6);
            let empty = 6 - filled;
            format!(
                "{}{}",
                "\u{2588}".repeat(filled),
                "\u{2591}".repeat(empty),
            )
        }
        None => "------".to_string(),
    };

    let urgency_pct = match row.urgency {
        Some(u) => format!("{:>3.0}%", (u * 100.0).min(999.0)),
        None => "  --".to_string(),
    };

    // Truncate desired to fit
    let fixed_width = 4 + 4 + 2 + 12 + 2 + 7 + 2 + 5;
    let desired_width = width.saturating_sub(fixed_width).max(10);
    let desired_trunc = truncate(&row.desired, desired_width);

    let (line_style, desired_style) = if selected {
        (
            Style::new().fg(CLR_WHITE).bold(),
            Style::new().fg(CLR_WHITE).bold(),
        )
    } else {
        match row.tier {
            UrgencyTier::Urgent => (
                Style::new().fg(CLR_RED_SOFT),
                Style::new().fg(CLR_RED_SOFT),
            ),
            UrgencyTier::Active => (
                Style::new().fg(CLR_LIGHT_GRAY),
                Style::new().fg(CLR_LIGHT_GRAY),
            ),
            UrgencyTier::Neglected => (
                Style::new().fg(CLR_YELLOW_SOFT),
                Style::new().fg(CLR_YELLOW_SOFT),
            ),
            UrgencyTier::Resolved => (
                Style::new().fg(CLR_DIM_GRAY),
                Style::new().fg(CLR_DIM_GRAY).dim(),
            ),
        }
    };

    Line::from_spans([
        Span::styled(format!("{} ", marker), line_style),
        Span::styled(format!("{} ", phase_str), line_style),
        Span::styled(format!("{} ", row.movement), line_style),
        Span::styled(
            format!("{:<width$} ", desired_trunc, width = desired_width),
            desired_style,
        ),
        Span::styled(format!("{:>11} ", row.horizon_display), line_style),
        Span::styled(format!("{} ", urgency_bar), line_style),
        Span::styled(urgency_pct, line_style),
    ])
}

// ============================================================================
// Data loading
// ============================================================================

/// Load all tensions from the workspace and compute dynamics.
pub fn load_tensions() -> Result<Vec<TensionRow>, String> {
    let workspace = Workspace::discover().map_err(|e| e.to_string())?;
    let store = workspace.open_store().map_err(|e| e.to_string())?;
    let mut engine = DynamicsEngine::with_store(store);

    let tensions = engine
        .store()
        .list_tensions()
        .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let mut rows: Vec<TensionRow> = Vec::with_capacity(tensions.len());

    for tension in &tensions {
        let short_id = tension.id.chars().take(6).collect::<String>();

        let computed = engine.compute_full_dynamics_for_tension(&tension.id);

        let (phase, movement, neglected, magnitude) = match &computed {
            Some(cd) => {
                let p = match cd.phase.phase {
                    CreativeCyclePhase::Germination => "G",
                    CreativeCyclePhase::Assimilation => "A",
                    CreativeCyclePhase::Completion => "C",
                    CreativeCyclePhase::Momentum => "M",
                };
                let m = match cd.tendency.tendency {
                    StructuralTendency::Advancing => "\u{2192}",
                    StructuralTendency::Oscillating => "\u{2194}",
                    StructuralTendency::Stagnant => "\u{25CB}",
                };
                let negl = cd.neglect.is_some();
                let mag = cd.structural_tension.as_ref().map(|st| st.magnitude);
                (p, m, negl, mag)
            }
            None => ("?", "\u{25CB}", false, None),
        };

        let urgency = compute_urgency(tension, now).map(|u| u.value);

        let horizon_display = match &tension.horizon {
            Some(h) => {
                let days = h.range_end().signed_duration_since(now).num_days();
                if days < 0 {
                    format!("{}d past", -days)
                } else if days == 0 {
                    "today".to_string()
                } else if days <= 30 {
                    format!("{}d", days)
                } else {
                    h.to_string()
                }
            }
            None => "\u{2014}".to_string(),
        };

        let tier = if tension.status == TensionStatus::Resolved
            || tension.status == TensionStatus::Released
        {
            UrgencyTier::Resolved
        } else if urgency.map(|u| u > 0.75).unwrap_or(false)
            || tension
                .horizon
                .as_ref()
                .map(|h| h.range_end() < now)
                .unwrap_or(false)
        {
            UrgencyTier::Urgent
        } else if neglected {
            UrgencyTier::Neglected
        } else {
            UrgencyTier::Active
        };

        rows.push(TensionRow {
            id: tension.id.clone(),
            short_id,
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            status: tension.status.to_string(),
            phase: phase.to_string(),
            movement: movement.to_string(),
            urgency,
            magnitude,
            neglected,
            horizon_display,
            tier,
        });
    }

    rows.sort_by(|a, b| {
        a.tier.cmp(&b.tier).then_with(|| {
            let ua = a.urgency.unwrap_or(-1.0);
            let ub = b.urgency.unwrap_or(-1.0);
            ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    Ok(rows)
}

// ============================================================================
// Public run function
// ============================================================================

/// Launch the TUI dashboard.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let tensions = load_tensions().unwrap_or_default();
    let app = WerkApp::new(tensions);
    App::fullscreen(app).run()?;
    Ok(())
}
