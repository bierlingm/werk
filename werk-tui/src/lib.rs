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
    compute_urgency, ComputedDynamics, CreativeCyclePhase, DynamicsEngine, Forest,
    StructuralTendency, Tension, TensionStatus,
};
use werk_shared::{relative_time, truncate, Workspace};

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

/// Filter mode for the tension list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Active,
    All,
    Resolved,
    Released,
}

impl Filter {
    fn next(self) -> Self {
        match self {
            Filter::Active => Filter::All,
            Filter::All => Filter::Resolved,
            Filter::Resolved => Filter::Released,
            Filter::Released => Filter::Active,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Filter::Active => "Active",
            Filter::All => "All",
            Filter::Resolved => "Resolved",
            Filter::Released => "Released",
        }
    }
}

/// The view currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Detail,
    TreeView,
}

/// A single item in the tree view.
#[derive(Debug, Clone)]
pub struct TreeItem {
    pub tension_id: String,
    pub short_id: String,
    pub desired: String,
    pub phase: String,
    pub movement: String,
    pub horizon_display: String,
    pub urgency: Option<f64>,
    pub depth: usize,
    pub connector: String, // e.g. "  ", "|-", "|  |-", etc.
    pub tier: UrgencyTier,
}

/// Display data for a single mutation in the detail view.
#[derive(Debug, Clone)]
pub struct MutationDisplay {
    pub relative_time: String,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: String,
}

/// Dynamics display for the detail view.
#[derive(Debug, Clone)]
pub struct DetailDynamics {
    pub phase: String,
    pub movement: String,
    pub magnitude: Option<f64>,
    pub urgency: Option<f64>,
    pub neglect: Option<String>,
    pub conflict: Option<String>,
    // Verbose fields
    pub oscillation: Option<String>,
    pub resolution: Option<String>,
    pub orientation: Option<String>,
    pub compensating_strategy: Option<String>,
    pub assimilation_depth: Option<String>,
    pub horizon_drift: Option<String>,
}

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
                    KeyCode::Enter => Msg::OpenDetail,
                    KeyCode::Escape => Msg::Back,
                    KeyCode::Char('1') => Msg::SwitchDashboard,
                    KeyCode::Char('2') | KeyCode::Char('t') => Msg::SwitchTree,
                    KeyCode::Char('f') => Msg::CycleFilter,
                    KeyCode::Char('v') => Msg::ToggleVerbose,
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
    engine: DynamicsEngine,
    tensions: Vec<TensionRow>,
    selected: usize,
    active_view: View,
    show_resolved: bool,
    show_help: bool,
    filter: Filter,
    verbose: bool,
    #[allow(dead_code)]
    status_message: Option<String>,
    total_active: usize,
    total_resolved: usize,
    total_released: usize,
    total_neglected: usize,
    total_urgent: usize,

    // Detail view state
    detail_tension: Option<Tension>,
    detail_scroll: u16,
    detail_mutations: Vec<MutationDisplay>,
    detail_children: Vec<TensionRow>,
    detail_dynamics: Option<DetailDynamics>,

    // Tree view state
    tree_selected: usize,
    tree_items: Vec<TreeItem>,
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
            selected: 0,
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

            tree_selected: 0,
            tree_items: Vec::new(),
        }
    }

    /// Visible tensions based on current filter.
    fn visible_tensions(&self) -> Vec<&TensionRow> {
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
            .collect()
    }

    /// Load detail data for a given tension ID.
    fn load_detail(&mut self, tension_id: &str) {
        let now = Utc::now();

        // Get the full tension
        let tension = match self.engine.store().get_tension(tension_id) {
            Ok(Some(t)) => t,
            _ => return,
        };

        // Compute dynamics
        let computed = self.engine.compute_full_dynamics_for_tension(tension_id);

        // Load mutations (last 10)
        let mutations = self.engine.store().get_mutations(tension_id).unwrap_or_default();
        let mut mutation_displays: Vec<MutationDisplay> = mutations
            .iter()
            .rev()
            .take(10)
            .map(|m| {
                MutationDisplay {
                    relative_time: relative_time(m.timestamp(), now),
                    field: m.field().to_string(),
                    old_value: m.old_value().map(|s| s.to_string()),
                    new_value: m.new_value().to_string(),
                }
            })
            .collect();
        mutation_displays.reverse();

        // Load children
        let all_tensions = self.engine.store().list_tensions().unwrap_or_default();
        let children: Vec<TensionRow> = all_tensions
            .iter()
            .filter(|t| t.parent_id.as_deref() == Some(tension_id))
            .map(|t| build_tension_row(&mut self.engine, t, now))
            .collect();

        // Build dynamics display
        let detail_dynamics = computed.map(|cd| build_detail_dynamics(&cd));

        self.detail_tension = Some(tension);
        self.detail_scroll = 0;
        self.detail_mutations = mutation_displays;
        self.detail_children = children;
        self.detail_dynamics = detail_dynamics;
    }

    /// Build tree items from the store.
    fn build_tree_items(&mut self) {
        let tensions = self.engine.store().list_tensions().unwrap_or_default();
        let forest = match Forest::from_tensions(tensions) {
            Ok(f) => f,
            Err(_) => return,
        };

        let now = Utc::now();
        let mut items = Vec::new();
        let root_ids = forest.root_ids().to_vec();
        let root_count = root_ids.len();

        for (i, root_id) in root_ids.iter().enumerate() {
            let is_last_root = i == root_count - 1;
            self.build_tree_recursive(
                &forest,
                root_id,
                &mut items,
                0,
                is_last_root,
                String::new(),
                now,
            );
        }

        self.tree_items = items;
        if self.tree_selected >= self.tree_items.len() && !self.tree_items.is_empty() {
            self.tree_selected = self.tree_items.len() - 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_tree_recursive(
        &mut self,
        forest: &Forest,
        node_id: &str,
        items: &mut Vec<TreeItem>,
        depth: usize,
        is_last: bool,
        prefix: String,
        now: chrono::DateTime<Utc>,
    ) {
        let node = match forest.find(node_id) {
            Some(n) => n,
            None => return,
        };

        let tension = &node.tension;
        let computed = self.engine.compute_full_dynamics_for_tension(&tension.id);
        let urgency = compute_urgency(tension, now).map(|u| u.value);

        let (phase, movement, neglected) = match &computed {
            Some(cd) => {
                let p = phase_char(cd.phase.phase);
                let m = movement_char(cd.tendency.tendency);
                let n = cd.neglect.is_some();
                (p, m, n)
            }
            None => ("?", "\u{25CB}", false),
        };

        let horizon_display = format_horizon(tension, now);

        let tier = compute_tier(tension, urgency, neglected, now);

        let connector = if depth == 0 {
            if is_last { "\u{2514}\u{2500}\u{2500} ".to_string() } else { "\u{251C}\u{2500}\u{2500} ".to_string() }
        } else {
            let branch = if is_last { "\u{2514}\u{2500}\u{2500} " } else { "\u{251C}\u{2500}\u{2500} " };
            format!("{}{}", prefix, branch)
        };

        let child_prefix = if depth == 0 {
            if is_last { "    ".to_string() } else { "\u{2502}   ".to_string() }
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}\u{2502}   ", prefix)
        };

        items.push(TreeItem {
            tension_id: tension.id.clone(),
            short_id: tension.id.chars().take(6).collect(),
            desired: tension.desired.clone(),
            phase: phase.to_string(),
            movement: movement.to_string(),
            horizon_display,
            urgency,
            depth,
            connector,
            tier,
        });

        let child_ids: Vec<String> = node.children.clone();
        let child_count = child_ids.len();
        for (ci, child_id) in child_ids.iter().enumerate() {
            let is_last_child = ci == child_count - 1;
            self.build_tree_recursive(
                forest,
                child_id,
                items,
                depth + 1,
                is_last_child,
                child_prefix.clone(),
                now,
            );
        }
    }
}

impl Model for WerkApp {
    type Message = Msg;

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::MoveDown => {
                match self.active_view {
                    View::Dashboard => {
                        let visible = self.visible_tensions().len();
                        if visible > 0 && self.selected < visible - 1 {
                            self.selected += 1;
                        }
                    }
                    View::TreeView => {
                        let count = self.tree_items.len();
                        if count > 0 && self.tree_selected < count - 1 {
                            self.tree_selected += 1;
                        }
                    }
                    View::Detail => {
                        // handled by ScrollDetailDown
                    }
                }
                Cmd::None
            }
            Msg::MoveUp => {
                match self.active_view {
                    View::Dashboard => {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    View::TreeView => {
                        if self.tree_selected > 0 {
                            self.tree_selected -= 1;
                        }
                    }
                    View::Detail => {
                        // handled by ScrollDetailUp
                    }
                }
                Cmd::None
            }
            Msg::ScrollDetailDown => {
                if self.active_view == View::Detail {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                }
                Cmd::None
            }
            Msg::ScrollDetailUp => {
                if self.active_view == View::Detail {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
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
            Msg::OpenDetail => {
                match self.active_view {
                    View::Dashboard => {
                        let visible = self.visible_tensions();
                        if let Some(row) = visible.get(self.selected) {
                            let id = row.id.clone();
                            self.load_detail(&id);
                            self.active_view = View::Detail;
                        }
                    }
                    View::TreeView => {
                        if let Some(item) = self.tree_items.get(self.tree_selected) {
                            let id = item.tension_id.clone();
                            self.load_detail(&id);
                            self.active_view = View::Detail;
                        }
                    }
                    View::Detail => {}
                }
                Cmd::None
            }
            Msg::Back => {
                match self.active_view {
                    View::Detail | View::TreeView => {
                        self.active_view = View::Dashboard;
                    }
                    View::Dashboard => {}
                }
                Cmd::None
            }
            Msg::SwitchDashboard => {
                self.active_view = View::Dashboard;
                Cmd::None
            }
            Msg::SwitchTree => {
                self.build_tree_items();
                self.active_view = View::TreeView;
                Cmd::None
            }
            Msg::CycleFilter => {
                self.filter = self.filter.next();
                let visible = self.visible_tensions().len();
                if visible > 0 && self.selected >= visible {
                    self.selected = visible - 1;
                } else if visible == 0 {
                    self.selected = 0;
                }
                Cmd::None
            }
            Msg::ToggleVerbose => {
                self.verbose = !self.verbose;
                Cmd::None
            }
            Msg::Quit => Cmd::Quit,
            Msg::Noop => Cmd::None,
        }
    }

    fn view(&self, frame: &mut Frame<'_>) {
        let area = Rect::new(0, 0, frame.width(), frame.height());

        match self.active_view {
            View::Dashboard => {
                let layout = Flex::vertical().constraints([
                    Constraint::Fixed(1),
                    Constraint::Fill,
                    Constraint::Fixed(1),
                ]);
                let rects = layout.split(area);

                self.render_title_bar(&rects[0], frame);
                self.render_tension_list(&rects[1], frame);
                self.render_dashboard_hints(&rects[2], frame);
            }
            View::Detail => {
                let layout = Flex::vertical().constraints([
                    Constraint::Fixed(1),
                    Constraint::Fill,
                    Constraint::Fixed(1),
                ]);
                let rects = layout.split(area);

                self.render_detail_title(&rects[0], frame);
                self.render_detail_body(&rects[1], frame);
                self.render_detail_hints(&rects[2], frame);
            }
            View::TreeView => {
                let layout = Flex::vertical().constraints([
                    Constraint::Fixed(1),
                    Constraint::Fill,
                    Constraint::Fixed(1),
                ]);
                let rects = layout.split(area);

                self.render_tree_title(&rects[0], frame);
                self.render_tree_body(&rects[1], frame);
                self.render_tree_hints(&rects[2], frame);
            }
        }

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
const CLR_CYAN: PackedRgba = PackedRgba::rgb(80, 200, 220);
const CLR_BG_DARK: PackedRgba = PackedRgba::rgb(30, 30, 30);

impl WerkApp {
    // ── Dashboard rendering ──────────────────────────────────────

    fn render_title_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let filter_label = if self.filter != Filter::Active {
            format!("  [{}]", self.filter.label())
        } else {
            String::new()
        };
        let status = format!(
            " werk  |  {} active  {} urgent  {} neglected  {} resolved  {} released{}",
            self.total_active,
            self.total_urgent,
            self.total_neglected,
            self.total_resolved,
            self.total_released,
            filter_label,
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

    fn render_dashboard_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = format!(
            " j/k navigate  Enter detail  t tree  f filter[{}]  R resolved  q quit  ? help",
            self.filter.label()
        );
        let style = Style::new().fg(CLR_MID_GRAY);
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&hints, style)]));
        paragraph.render(*area, frame);
    }

    // ── Detail rendering ─────────────────────────────────────────

    fn render_detail_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let title = match &self.detail_tension {
            Some(t) => {
                let short_id: String = t.id.chars().take(8).collect();
                format!(
                    " {}  {}",
                    truncate(&t.desired, area.width.saturating_sub(12) as usize),
                    short_id,
                )
            }
            None => " Detail".to_string(),
        };
        let style = Style::new().fg(CLR_WHITE).bold();
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&title, style)]));
        paragraph.render(*area, frame);
    }

    fn render_detail_body(&self, area: &Rect, frame: &mut Frame<'_>) {
        let mut lines: Vec<Line> = Vec::new();

        if let Some(tension) = &self.detail_tension {
            let now = Utc::now();

            // Info section
            lines.push(Line::from(""));
            lines.push(Line::from_spans([
                Span::styled("  Desired  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(&tension.desired, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
            lines.push(Line::from_spans([
                Span::styled("  Actual   ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(&tension.actual, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
            lines.push(Line::from_spans([
                Span::styled("  Status   ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(tension.status.to_string(), Style::new().fg(CLR_LIGHT_GRAY)),
                Span::styled("       Created  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(
                    relative_time(tension.created_at, now),
                    Style::new().fg(CLR_LIGHT_GRAY),
                ),
            ]));
            let horizon_str = match &tension.horizon {
                Some(h) => {
                    let remaining = h.range_end().signed_duration_since(now).num_days();
                    if remaining < 0 {
                        format!("{}  ({}d past)", h, -remaining)
                    } else if remaining == 0 {
                        format!("{}  (today)", h)
                    } else {
                        format!("{}  ({}d remaining)", h, remaining)
                    }
                }
                None => "\u{2014}".to_string(),
            };
            lines.push(Line::from_spans([
                Span::styled("  Horizon  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(horizon_str, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));

            // Dynamics section
            lines.push(Line::from(""));
            lines.push(Line::from_spans([Span::styled(
                "  \u{2500}\u{2500} Dynamics \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                Style::new().fg(CLR_DIM_GRAY),
            )]));

            if let Some(dyn_display) = &self.detail_dynamics {
                // Phase + Movement line
                lines.push(Line::from_spans([
                    Span::styled("  Phase       ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(&dyn_display.phase, Style::new().fg(CLR_LIGHT_GRAY)),
                    Span::styled("        Movement    ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(&dyn_display.movement, Style::new().fg(CLR_LIGHT_GRAY)),
                ]));

                // Magnitude bar
                if let Some(mag) = dyn_display.magnitude {
                    let bar = render_bar(mag, 10);
                    lines.push(Line::from_spans([
                        Span::styled("  Magnitude   ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(bar, Style::new().fg(CLR_CYAN)),
                        Span::styled(format!(" {:.2}", mag), Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }

                // Urgency bar
                if let Some(urg) = dyn_display.urgency {
                    let bar = render_bar(urg.min(1.0), 10);
                    lines.push(Line::from_spans([
                        Span::styled("  Urgency     ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(bar, Style::new().fg(
                            if urg > 0.75 { CLR_RED_SOFT } else { CLR_YELLOW_SOFT }
                        )),
                        Span::styled(format!(" {:.0}%", (urg * 100.0).min(999.0)), Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }

                // Conflict (only if present)
                if let Some(conflict) = &dyn_display.conflict {
                    lines.push(Line::from_spans([
                        Span::styled("  Conflict    ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(conflict, Style::new().fg(CLR_RED_SOFT)),
                    ]));
                }

                // Neglect (only if present)
                if let Some(neglect) = &dyn_display.neglect {
                    lines.push(Line::from_spans([
                        Span::styled("  Neglect     ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(neglect, Style::new().fg(CLR_YELLOW_SOFT)),
                    ]));
                }

                // Verbose dynamics
                if self.verbose {
                    lines.push(Line::from(""));
                    lines.push(Line::from_spans([Span::styled(
                        "  \u{2500}\u{2500} Verbose Dynamics \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(CLR_DIM_GRAY),
                    )]));

                    if let Some(v) = &dyn_display.oscillation {
                        lines.push(Line::from_spans([
                            Span::styled("  Oscillation         ", Style::new().fg(CLR_MID_GRAY)),
                            Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                        ]));
                    }
                    if let Some(v) = &dyn_display.resolution {
                        lines.push(Line::from_spans([
                            Span::styled("  Resolution          ", Style::new().fg(CLR_MID_GRAY)),
                            Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                        ]));
                    }
                    if let Some(v) = &dyn_display.orientation {
                        lines.push(Line::from_spans([
                            Span::styled("  Orientation         ", Style::new().fg(CLR_MID_GRAY)),
                            Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                        ]));
                    }
                    if let Some(v) = &dyn_display.compensating_strategy {
                        lines.push(Line::from_spans([
                            Span::styled("  Compensating Strat  ", Style::new().fg(CLR_MID_GRAY)),
                            Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                        ]));
                    }
                    if let Some(v) = &dyn_display.assimilation_depth {
                        lines.push(Line::from_spans([
                            Span::styled("  Assimilation Depth  ", Style::new().fg(CLR_MID_GRAY)),
                            Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                        ]));
                    }
                    if let Some(v) = &dyn_display.horizon_drift {
                        lines.push(Line::from_spans([
                            Span::styled("  Horizon Drift       ", Style::new().fg(CLR_MID_GRAY)),
                            Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                        ]));
                    }
                }
            } else {
                lines.push(Line::from_spans([Span::styled(
                    "  No dynamics computed",
                    Style::new().fg(CLR_DIM_GRAY),
                )]));
            }

            // History section
            lines.push(Line::from(""));
            let history_header = format!(
                "  \u{2500}\u{2500} History ({}) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                self.detail_mutations.len()
            );
            lines.push(Line::from_spans([Span::styled(
                &history_header,
                Style::new().fg(CLR_DIM_GRAY),
            )]));

            if self.detail_mutations.is_empty() {
                lines.push(Line::from_spans([Span::styled(
                    "  No mutations recorded",
                    Style::new().fg(CLR_DIM_GRAY),
                )]));
            } else {
                for m in &self.detail_mutations {
                    let value_display = match &m.old_value {
                        Some(old) => format!(
                            "\"{}\" -> \"{}\"",
                            truncate(old, 20),
                            truncate(&m.new_value, 30)
                        ),
                        None => format!("\"{}\"", truncate(&m.new_value, 50)),
                    };
                    lines.push(Line::from_spans([
                        Span::styled(
                            format!("  {:<14}", m.relative_time),
                            Style::new().fg(CLR_DIM_GRAY),
                        ),
                        Span::styled(
                            format!("[{}]  ", m.field),
                            Style::new().fg(CLR_CYAN),
                        ),
                        Span::styled(value_display, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
            }

            // Children section
            if !self.detail_children.is_empty() {
                lines.push(Line::from(""));
                let children_header = format!(
                    "  \u{2500}\u{2500} Children ({}) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                    self.detail_children.len()
                );
                lines.push(Line::from_spans([Span::styled(
                    &children_header,
                    Style::new().fg(CLR_DIM_GRAY),
                )]));

                for child in &self.detail_children {
                    let desired_trunc = truncate(&child.desired, 40);
                    lines.push(Line::from_spans([
                        Span::styled(
                            format!("  {}  ", child.short_id),
                            Style::new().fg(CLR_DIM_GRAY),
                        ),
                        Span::styled(
                            format!("[{}] {} ", child.phase, child.movement),
                            Style::new().fg(CLR_MID_GRAY),
                        ),
                        Span::styled(desired_trunc, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
            }

            lines.push(Line::from(""));
        } else {
            lines.push(Line::from("  No tension selected"));
        }

        let text = Text::from_lines(lines);
        let paragraph = Paragraph::new(text).scroll((self.detail_scroll, 0));
        paragraph.render(*area, frame);
    }

    fn render_detail_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let verbose_label = if self.verbose { "v-" } else { "v+" };
        let hints = format!(
            " Esc back  j/k scroll  {} verbose  1 dashboard  t tree  q quit  ? help",
            verbose_label,
        );
        let style = Style::new().fg(CLR_MID_GRAY);
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&hints, style)]));
        paragraph.render(*area, frame);
    }

    // ── Tree rendering ───────────────────────────────────────────

    fn render_tree_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let status = format!(
            " Tree  |  {} tensions  {} roots",
            self.tree_items.len(),
            self.tree_items.iter().filter(|i| i.depth == 0).count(),
        );
        let style = Style::new().fg(CLR_LIGHT_GRAY).bold();
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&status, style)]));
        paragraph.render(*area, frame);
    }

    fn render_tree_body(&self, area: &Rect, frame: &mut Frame<'_>) {
        if self.tree_items.is_empty() {
            let msg = Paragraph::new("  No tensions found.");
            msg.render(*area, frame);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        for (idx, item) in self.tree_items.iter().enumerate() {
            let is_selected = idx == self.tree_selected;
            let marker = if is_selected { ">" } else { " " };

            let urgency_str = match item.urgency {
                Some(u) => format!("{:>3.0}%", (u * 100.0).min(999.0)),
                None => "  --".to_string(),
            };

            let desired_width = (area.width as usize)
                .saturating_sub(item.connector.chars().count() + 2 + 4 + 4 + 8 + 12 + 5);
            let desired_trunc = truncate(&item.desired, desired_width.max(10));

            let (line_style, desired_style) = if is_selected {
                (
                    Style::new().fg(CLR_WHITE).bold(),
                    Style::new().fg(CLR_WHITE).bold(),
                )
            } else {
                match item.tier {
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

            lines.push(Line::from_spans([
                Span::styled(format!("{} ", marker), line_style),
                Span::styled(&item.connector, Style::new().fg(CLR_DIM_GRAY)),
                Span::styled(format!("[{}] {} ", item.phase, item.movement), line_style),
                Span::styled(format!("{}  ", item.short_id), line_style),
                Span::styled(
                    format!("{:<width$} ", desired_trunc, width = desired_width),
                    desired_style,
                ),
                Span::styled(format!("{:>11} ", item.horizon_display), line_style),
                Span::styled(urgency_str, line_style),
            ]));
        }

        let text = Text::from_lines(lines);
        let scroll = self.tree_scroll_offset(area.height);
        let paragraph = Paragraph::new(text).scroll((scroll, 0));
        paragraph.render(*area, frame);
    }

    fn tree_scroll_offset(&self, viewport_height: u16) -> u16 {
        let selected = self.tree_selected as u16;
        let vp = viewport_height.saturating_sub(2);
        selected.saturating_sub(vp)
    }

    fn render_tree_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = " j/k navigate  Enter detail  Esc dashboard  1 dashboard  f filter  q quit  ? help";
        let style = Style::new().fg(CLR_MID_GRAY);
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(hints, style)]));
        paragraph.render(*area, frame);
    }

    // ── Help overlay ─────────────────────────────────────────────

    fn render_help_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let help_width = 60u16.min(area.width.saturating_sub(4));
        let help_height = 18u16.min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(help_width)) / 2;
        let y = (area.height.saturating_sub(help_height)) / 2;
        let help_area = Rect::new(x, y, help_width, help_height);

        let help_lines = vec![
            Line::from_spans([Span::styled(
                " werk \u{2014} structural dynamics TUI",
                Style::new().bold(),
            )]),
            Line::from(""),
            Line::from("  j / Down    Move down"),
            Line::from("  k / Up      Move up"),
            Line::from("  Enter       Open detail view"),
            Line::from("  Esc         Go back"),
            Line::from("  1           Dashboard view"),
            Line::from("  2 / t       Tree view"),
            Line::from("  f           Cycle filter (Active/All/Resolved/Released)"),
            Line::from("  v           Toggle verbose dynamics"),
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

// ============================================================================
// Formatting helpers
// ============================================================================

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

fn render_bar(value: f64, width: usize) -> String {
    let filled = ((value * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    )
}

fn phase_char(phase: CreativeCyclePhase) -> &'static str {
    match phase {
        CreativeCyclePhase::Germination => "G",
        CreativeCyclePhase::Assimilation => "A",
        CreativeCyclePhase::Completion => "C",
        CreativeCyclePhase::Momentum => "M",
    }
}

fn phase_name(phase: CreativeCyclePhase) -> &'static str {
    match phase {
        CreativeCyclePhase::Germination => "Germination",
        CreativeCyclePhase::Assimilation => "Assimilation",
        CreativeCyclePhase::Completion => "Completion",
        CreativeCyclePhase::Momentum => "Momentum",
    }
}

fn movement_char(tendency: StructuralTendency) -> &'static str {
    match tendency {
        StructuralTendency::Advancing => "\u{2192}",
        StructuralTendency::Oscillating => "\u{2194}",
        StructuralTendency::Stagnant => "\u{25CB}",
    }
}

fn movement_name(tendency: StructuralTendency) -> &'static str {
    match tendency {
        StructuralTendency::Advancing => "Advancing",
        StructuralTendency::Oscillating => "Oscillating",
        StructuralTendency::Stagnant => "Stagnant",
    }
}

fn format_horizon(tension: &Tension, now: chrono::DateTime<Utc>) -> String {
    match &tension.horizon {
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
    }
}

fn compute_tier(
    tension: &Tension,
    urgency: Option<f64>,
    neglected: bool,
    now: chrono::DateTime<Utc>,
) -> UrgencyTier {
    if tension.status == TensionStatus::Resolved || tension.status == TensionStatus::Released {
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
    }
}

fn build_detail_dynamics(cd: &ComputedDynamics) -> DetailDynamics {
    let phase = phase_name(cd.phase.phase).to_string();
    let movement = format!("{} {}", movement_char(cd.tendency.tendency), movement_name(cd.tendency.tendency));
    let magnitude = cd.structural_tension.as_ref().map(|st| st.magnitude);
    let urgency = cd.urgency.as_ref().map(|u| u.value);

    let neglect = cd.neglect.as_ref().map(|n| {
        let ntype = match n.neglect_type {
            sd_core::NeglectType::ParentNeglectsChildren => "Parent neglects children",
            sd_core::NeglectType::ChildrenNeglected => "Children neglected",
        };
        format!("{} (ratio: {:.2})", ntype, n.activity_ratio)
    });

    let conflict = cd.conflict.as_ref().map(|c| {
        let pattern = match c.pattern {
            sd_core::ConflictPattern::AsymmetricActivity => "Asymmetric activity",
            sd_core::ConflictPattern::CompetingTensions => "Competing tensions",
        };
        pattern.to_string()
    });

    let oscillation = cd.oscillation.as_ref().map(|o| {
        format!("{} reversals, magnitude {:.2}", o.reversals, o.magnitude)
    });

    let resolution = cd.resolution.as_ref().map(|r| {
        let trend = match r.trend {
            sd_core::ResolutionTrend::Accelerating => "accelerating",
            sd_core::ResolutionTrend::Steady => "steady",
            sd_core::ResolutionTrend::Decelerating => "decelerating",
        };
        format!("velocity {:.4}, {}", r.velocity, trend)
    });

    let orientation = cd.orientation.as_ref().map(|o| {
        let orient = match o.orientation {
            sd_core::Orientation::Creative => "Creative",
            sd_core::Orientation::ProblemSolving => "Problem-solving",
            sd_core::Orientation::ReactiveResponsive => "Reactive/Responsive",
        };
        format!(
            "{} (creative: {:.0}%, problem: {:.0}%, reactive: {:.0}%)",
            orient,
            o.evidence.creative_ratio * 100.0,
            o.evidence.problem_solving_ratio * 100.0,
            o.evidence.reactive_ratio * 100.0,
        )
    });

    let compensating_strategy = cd.compensating_strategy.as_ref().map(|cs| {
        let stype = match cs.strategy_type {
            sd_core::CompensatingStrategyType::TolerableConflict => "Tolerable conflict",
            sd_core::CompensatingStrategyType::ConflictManipulation => "Conflict manipulation",
            sd_core::CompensatingStrategyType::WillpowerManipulation => "Willpower manipulation",
        };
        format!("{}, persisted {}s", stype, cs.persistence_seconds)
    });

    let assimilation_depth = {
        let depth = match cd.assimilation.depth {
            sd_core::AssimilationDepth::Shallow => "Shallow",
            sd_core::AssimilationDepth::Deep => "Deep",
            sd_core::AssimilationDepth::None => "None",
        };
        if cd.assimilation.depth != sd_core::AssimilationDepth::None {
            Some(format!(
                "{} (freq: {:.2}, trend: {:.2})",
                depth, cd.assimilation.mutation_frequency, cd.assimilation.frequency_trend
            ))
        } else {
            None
        }
    };

    let horizon_drift = {
        let dtype = match cd.horizon_drift.drift_type {
            sd_core::HorizonDriftType::Stable => "Stable",
            sd_core::HorizonDriftType::Tightening => "Tightening",
            sd_core::HorizonDriftType::Postponement => "Postponement",
            sd_core::HorizonDriftType::RepeatedPostponement => "Repeated postponement",
            sd_core::HorizonDriftType::Loosening => "Loosening",
            sd_core::HorizonDriftType::Oscillating => "Oscillating",
        };
        if cd.horizon_drift.change_count > 0 {
            Some(format!(
                "{} ({} changes, net shift {}s)",
                dtype, cd.horizon_drift.change_count, cd.horizon_drift.net_shift_seconds
            ))
        } else {
            None
        }
    };

    DetailDynamics {
        phase,
        movement,
        magnitude,
        urgency,
        neglect,
        conflict,
        oscillation,
        resolution,
        orientation,
        compensating_strategy,
        assimilation_depth,
        horizon_drift,
    }
}

fn build_tension_row(
    engine: &mut DynamicsEngine,
    tension: &Tension,
    now: chrono::DateTime<Utc>,
) -> TensionRow {
    let short_id = tension.id.chars().take(6).collect::<String>();
    let computed = engine.compute_full_dynamics_for_tension(&tension.id);

    let (phase, movement, neglected, magnitude) = match &computed {
        Some(cd) => {
            let p = phase_char(cd.phase.phase);
            let m = movement_char(cd.tendency.tendency);
            let negl = cd.neglect.is_some();
            let mag = cd.structural_tension.as_ref().map(|st| st.magnitude);
            (p, m, negl, mag)
        }
        None => ("?", "\u{25CB}", false, None),
    };

    let urgency = compute_urgency(tension, now).map(|u| u.value);
    let horizon_display = format_horizon(tension, now);
    let tier = compute_tier(tension, urgency, neglected, now);

    TensionRow {
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
    }
}

// ============================================================================
// Data loading
// ============================================================================

/// Load all tensions from the workspace and compute dynamics.
/// Returns (engine, rows) so the engine persists in WerkApp.
pub fn load_tensions() -> Result<(DynamicsEngine, Vec<TensionRow>), String> {
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
        rows.push(build_tension_row(&mut engine, tension, now));
    }

    rows.sort_by(|a, b| {
        a.tier.cmp(&b.tier).then_with(|| {
            let ua = a.urgency.unwrap_or(-1.0);
            let ub = b.urgency.unwrap_or(-1.0);
            ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    Ok((engine, rows))
}

// ============================================================================
// Public run function
// ============================================================================

/// Launch the TUI dashboard.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (engine, tensions) = load_tensions().unwrap_or_else(|_| {
        // Create an in-memory engine as fallback
        let engine = DynamicsEngine::new_in_memory()
            .expect("failed to create in-memory engine");
        (engine, Vec::new())
    });
    let app = WerkApp::new(engine, tensions);
    App::fullscreen(app).run()?;
    Ok(())
}
