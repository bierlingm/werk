//! LETO-inspired TUI prototype — survey + deck, real data, ftui widgets only.
//!
//! Run: cargo run -p werk-tui --example leto_prototype
//!
//! Tab switches views. j/k navigate. l/Enter descend. h/Backspace ascend. q quit.

use std::cell::RefCell;
use std::collections::HashMap;

use chrono::{Datelike, Utc};

use ftui::layout::{Constraint, Flex, Rect};
use ftui::style::Style;
use ftui::text::{Line, Span, Text};
use ftui::widgets::block::Block;
use ftui::widgets::borders::{BorderType, Borders};
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::progress::ProgressBar;
use ftui::widgets::sparkline::Sparkline;
use ftui::widgets::status_line::{StatusItem, StatusLine};
use ftui::widgets::{StatefulWidget, Widget};
use ftui::{Cmd, Event, Frame, KeyCode, Model, PackedRgba, Program, ProgramConfig};

use werk_core::{Store, TensionStatus};
use werk_shared::Workspace;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

// Aligned with production theme (dark terminal)
const BG: PackedRgba = PackedRgba::rgb(0, 0, 0);
const SURFACE: PackedRgba = PackedRgba::rgb(35, 42, 42); // #232A2A
const FG: PackedRgba = PackedRgba::rgb(220, 220, 220); // #DCDCDC
const BRIGHT: PackedRgba = PackedRgba::rgb(255, 255, 255); // white
const SUBDUED: PackedRgba = PackedRgba::rgb(100, 100, 100); // #646464 — muted text
const DIM: PackedRgba = PackedRgba::rgb(160, 160, 160); // #A0A0A0 — text_subtle, readable
const BORDER: PackedRgba = PackedRgba::rgb(60, 60, 70); // #3C3C46
const AMBER: PackedRgba = PackedRgba::rgb(200, 170, 60); // #C8AA3C
const AMBER_HI: PackedRgba = PackedRgba::rgb(230, 200, 80);
const AMBER_LO: PackedRgba = PackedRgba::rgb(120, 95, 25);
const RED: PackedRgba = PackedRgba::rgb(220, 90, 90); // #DC5A5A
const GREEN: PackedRgba = PackedRgba::rgb(80, 190, 120); // #50BE78
const CYAN: PackedRgba = PackedRgba::rgb(80, 190, 210); // #50BED2
const SEL_BG: PackedRgba = PackedRgba::rgb(35, 42, 42); // same as SURFACE

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Item {
    id: String,
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: TensionStatus,
    parent_id: Option<String>,
    position: Option<i32>,
    children: usize,
    urgency: f64,
    horizon_label: Option<String>,
    age_label: String,
    band: Band,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Band {
    Overdue,
    Imminent,
    Approaching,
    Later,
    Unframed,
}

impl Band {
    fn name(&self) -> &'static str {
        match self {
            Band::Overdue => "OVERDUE",
            Band::Imminent => "IMMINENT",
            Band::Approaching => "APPROACHING",
            Band::Later => "LATER",
            Band::Unframed => "UNFRAMED",
        }
    }
    fn accent(&self) -> PackedRgba {
        match self {
            Band::Overdue => RED,
            Band::Imminent => AMBER_HI,
            Band::Approaching => AMBER,
            Band::Later => CYAN,
            Band::Unframed => SUBDUED,
        }
    }
    fn border(&self) -> PackedRgba {
        match self {
            Band::Overdue => PackedRgba::rgb(100, 45, 45),
            Band::Imminent => PackedRgba::rgb(100, 85, 30),
            Band::Approaching => PackedRgba::rgb(80, 65, 22),
            Band::Later => PackedRgba::rgb(35, 75, 85),
            Band::Unframed => BORDER,
        }
    }
}

fn urgency_color(u: f64) -> PackedRgba {
    if u > 1.3 {
        RED
    } else if u > 1.0 {
        AMBER_HI
    } else if u > 0.7 {
        AMBER
    } else if u > 0.3 {
        AMBER_LO
    } else {
        PackedRgba::rgb(70, 110, 70)
    } // low urgency green, readable on black
}

fn compact_age(secs: i64) -> String {
    let s = secs.unsigned_abs();
    if s < 3600 {
        format!("{}m", s / 60)
    } else if s < 86400 {
        format!("{}h", s / 3600)
    } else if s < 86400 * 14 {
        format!("{}d", s / 86400)
    } else if s < 86400 * 63 {
        format!("{}w", s / (86400 * 7))
    } else {
        format!("{}mo", s / (86400 * 30))
    }
}

fn load_items(store: &Store) -> Vec<Item> {
    let tensions = store.list_tensions().unwrap_or_default();
    let now = Utc::now();
    let now_year = now.year();

    let mut child_counts: HashMap<String, usize> = HashMap::new();
    for t in &tensions {
        if let Some(ref pid) = t.parent_id {
            *child_counts.entry(pid.clone()).or_default() += 1;
        }
    }

    let compact_horizon = |h: &werk_core::Horizon| -> String {
        use werk_core::HorizonKind;
        let mn = |m: u32| -> &'static str {
            match m {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "?",
            }
        };
        match h.kind() {
            HorizonKind::Year(y) => format!("{y}"),
            HorizonKind::Month(y, m) => {
                if y == now_year {
                    mn(m).into()
                } else {
                    format!("{} {}", mn(m), y % 100)
                }
            }
            HorizonKind::Day(d) => format!("{} {}", mn(d.month()), d.day()),
            HorizonKind::DateTime(dt) => format!("{} {}", mn(dt.month()), dt.day()),
        }
    };

    tensions
        .iter()
        .map(|t| {
            let horizon_end = t.horizon.as_ref().map(|h| h.range_end());
            let horizon_label = t.horizon.as_ref().map(|h| compact_horizon(h));

            let urgency = match horizon_end {
                Some(end) => {
                    let window =
                        end.signed_duration_since(t.created_at).num_seconds().max(1) as f64;
                    let elapsed =
                        now.signed_duration_since(t.created_at).num_seconds().max(0) as f64;
                    elapsed / window
                }
                None => {
                    let weeks = now.signed_duration_since(t.created_at).num_weeks() as f64;
                    (weeks / 6.0).min(1.0)
                }
            };

            let band = match horizon_end {
                None => Band::Unframed,
                Some(end) => {
                    let hours = (end - now).num_hours() as f64;
                    if hours < 0.0 {
                        Band::Overdue
                    } else if hours <= 168.0 {
                        Band::Imminent
                    } else if hours <= 720.0 {
                        Band::Approaching
                    } else {
                        Band::Later
                    }
                }
            };

            let age_secs = now.signed_duration_since(t.created_at).num_seconds();
            let age_label = compact_age(age_secs);

            Item {
                id: t.id.clone(),
                short_code: t.short_code,
                desired: t.desired.clone(),
                actual: t.actual.clone(),
                status: t.status,
                parent_id: t.parent_id.clone(),
                position: t.position,
                children: child_counts.get(&t.id).copied().unwrap_or(0),
                urgency,
                horizon_label,
                age_label,
                band,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn trunc(s: &str, max: usize) -> String {
    let first = s.lines().next().unwrap_or(s).trim();
    if first.chars().count() <= max {
        first.to_string()
    } else {
        let t: String = first.chars().take(max.saturating_sub(1)).collect();
        format!("{t}\u{2026}")
    }
}

fn display_id(item: &Item) -> String {
    item.short_code
        .map(|sc| format!("#{sc}"))
        .unwrap_or_else(|| format!("#{}", &item.id[..4.min(item.id.len())]))
}

fn status_glyph(item: &Item) -> &'static str {
    match item.status {
        TensionStatus::Active if item.position.is_some() => "\u{25c6}", // ◆ positioned
        TensionStatus::Active => "\u{2727}",                            // ✧ held
        TensionStatus::Resolved => "\u{2726}", // ✦ resolved (production glyph)
        TensionStatus::Released => "\u{00b7}", // · released (production glyph)
    }
}

fn glyph_color(item: &Item) -> PackedRgba {
    match item.status {
        TensionStatus::Active if item.urgency > 1.0 => AMBER,
        TensionStatus::Active if item.position.is_none() => SUBDUED,
        TensionStatus::Active => CYAN,
        TensionStatus::Resolved => GREEN,
        TensionStatus::Released => SUBDUED,
    }
}

// ---------------------------------------------------------------------------
// View mode
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum View {
    Survey,
    Deck,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Msg {
    Up,
    Down,
    Descend,
    Ascend,
    SwitchView,
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
                    KeyCode::Char('q') => Msg::Quit,
                    KeyCode::Char('j') | KeyCode::Down => Msg::Down,
                    KeyCode::Char('k') | KeyCode::Up => Msg::Up,
                    KeyCode::Char('l') | KeyCode::Enter => Msg::Descend,
                    KeyCode::Char('h') | KeyCode::Backspace => Msg::Ascend,
                    KeyCode::Tab => Msg::SwitchView,
                    _ => Msg::Noop,
                }
            }
            _ => Msg::Noop,
        }
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    items: Vec<Item>,
    view: View,
    cursor: usize,
    deck_parent: Option<String>,
    // Per-band list states for survey
    band_states: HashMap<Band, RefCell<ListState>>,
    // List state for deck children
    deck_list_state: RefCell<ListState>,
}

impl App {
    fn new(items: Vec<Item>) -> Self {
        let mut band_states = HashMap::new();
        for &b in &[
            Band::Overdue,
            Band::Imminent,
            Band::Approaching,
            Band::Later,
            Band::Unframed,
        ] {
            band_states.insert(b, RefCell::new(ListState::default()));
        }
        Self {
            items,
            view: View::Survey,
            cursor: 0,
            deck_parent: None,
            band_states,
            deck_list_state: RefCell::new(ListState::default()),
        }
    }

    fn survey_items(&self) -> Vec<&Item> {
        let mut v: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| i.status == TensionStatus::Active)
            .collect();
        v.sort_by(|a, b| {
            a.band.cmp(&b.band).then(
                b.urgency
                    .partial_cmp(&a.urgency)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        v
    }

    fn survey_band_groups(&self) -> Vec<(Band, Vec<&Item>)> {
        let items = self.survey_items();
        let mut groups: Vec<(Band, Vec<&Item>)> = Vec::new();
        for item in items {
            if groups.last().map(|(b, _)| *b != item.band).unwrap_or(true) {
                groups.push((item.band, Vec::new()));
            }
            groups.last_mut().unwrap().1.push(item);
        }
        groups
    }

    /// Which band and offset within that band does the cursor point to?
    fn cursor_band_position(&self) -> Option<(Band, usize)> {
        let mut offset = 0;
        for (band, group) in self.survey_band_groups() {
            if self.cursor < offset + group.len() {
                return Some((band, self.cursor - offset));
            }
            offset += group.len();
        }
        None
    }

    fn deck_children(&self) -> Vec<&Item> {
        let pid = self.deck_parent.as_deref();
        let mut v: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| {
                i.parent_id.as_deref() == pid
                // At root (no parent), only show active items to avoid dumping everything
                && (pid.is_some() || i.status == TensionStatus::Active)
            })
            .collect();
        v.sort_by(|a, b| {
            let rank = |i: &&Item| -> u8 {
                match i.status {
                    TensionStatus::Active if i.position.is_some() => 0,
                    TensionStatus::Active => 1,
                    TensionStatus::Resolved => 2,
                    TensionStatus::Released => 3,
                }
            };
            rank(a)
                .cmp(&rank(b))
                .then_with(|| a.position.unwrap_or(9999).cmp(&b.position.unwrap_or(9999)))
        });
        v
    }

    fn deck_parent_item(&self) -> Option<&Item> {
        self.deck_parent
            .as_ref()
            .and_then(|pid| self.items.iter().find(|i| i.id == *pid))
    }

    fn visible_count(&self) -> usize {
        match self.view {
            View::Survey => self.survey_items().len(),
            View::Deck => self.deck_children().len(),
        }
    }

    fn clamp_cursor(&mut self) {
        let n = self.visible_count();
        if n == 0 {
            self.cursor = 0;
        } else if self.cursor >= n {
            self.cursor = n - 1;
        }
    }

    fn content_rect(&self, full: Rect) -> Rect {
        let max_w: u16 = 130.min(full.width.saturating_sub(2));
        let x = full.x + (full.width.saturating_sub(max_w)) / 2;
        Rect::new(x, full.y, max_w, full.height)
    }

    /// Sync band ListStates from the global cursor (1 row per item).
    fn sync_survey_states(&self) {
        let (active_band, active_offset) =
            self.cursor_band_position().unwrap_or((Band::Overdue, 0));
        for (&band, state) in &self.band_states {
            let mut s = state.borrow_mut();
            if band == active_band {
                s.select(Some(active_offset));
            } else {
                s.selected = None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Survey view
// ---------------------------------------------------------------------------

fn render_survey(app: &App, frame: &mut Frame<'_>, area: Rect) {
    let groups = app.survey_band_groups();

    if groups.is_empty() {
        Paragraph::new(Text::from(Line::from(Span::styled(
            "  no active tensions.",
            Style::new().fg(SUBDUED),
        ))))
        .render(area, frame);
        return;
    }

    // Layout: sparkline(1) + bands (capped, largest gets Fill) + status(1)
    let max_band_idx = groups
        .iter()
        .enumerate()
        .max_by_key(|(_, (_, g))| g.len())
        .map(|(i, _)| i)
        .unwrap_or(0);
    let cap = 12u16;

    let mut constraints: Vec<Constraint> = vec![Constraint::Fixed(1)];
    for (i, (_, group)) in groups.iter().enumerate() {
        if i == max_band_idx {
            constraints.push(Constraint::Fill);
        } else {
            let h = (group.len() as u16 + 2).min(cap + 2);
            constraints.push(Constraint::Fixed(h));
        }
    }
    constraints.push(Constraint::Fixed(1));

    let slots = Flex::vertical().constraints(constraints).split(area);
    let mut si = 0;

    app.sync_survey_states();

    // Sparkline
    let spark_data: Vec<f64> = app.survey_items().iter().map(|i| i.urgency).collect();
    Sparkline::new(&spark_data)
        .style(Style::new().fg(AMBER))
        .max(2.0)
        .render(slots[si], frame);
    si += 1;

    // Band lists
    let active_band = app.cursor_band_position().map(|(b, _)| b);

    for (band, group) in &groups {
        let slot = slots[si];
        si += 1;

        let is_active = active_band == Some(*band);
        let inner_w = slot.width.saturating_sub(2) as usize;

        // Layout: [horizon 7][glyph 2][arrow 2] name... [age 4][id 5]
        let left_cols = 11usize;
        let right_cols = 9usize;
        let name_budget = inner_w.saturating_sub(left_cols + right_cols);

        let list_items: Vec<ListItem> = group
            .iter()
            .map(|item| {
                let g = status_glyph(item);
                let gc = glyph_color(item);
                let did = display_id(item);
                let arrow = if item.children > 0 { "\u{2192}" } else { " " };
                let hlabel = item.horizon_label.as_deref().unwrap_or("");
                let name = trunc(&item.desired, name_budget);
                let name_pad = name_budget.saturating_sub(name.chars().count());
                let hlabel_c = if item.urgency > 1.0 {
                    band.accent()
                } else {
                    SUBDUED
                };

                let line = Line::from_spans([
                    Span::styled(format!("{:<7}", hlabel), Style::new().fg(hlabel_c)),
                    Span::styled(format!("{g} "), Style::new().fg(gc)),
                    Span::styled(format!("{arrow} "), Style::new().fg(SUBDUED)),
                    Span::styled(name, Style::new().fg(FG)),
                    Span::styled(" ".repeat(name_pad), Style::new()),
                    Span::styled(format!(" {:>3}", item.age_label), Style::new().fg(SUBDUED)),
                    Span::styled(format!(" {:>4}", did), Style::new().fg(SUBDUED)),
                ]);
                ListItem::new(line).marker("")
            })
            .collect();

        let title = format!(" {} ({}) ", band.name(), group.len());
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(if is_active {
                band.accent()
            } else {
                band.border()
            }))
            .title(title.as_str())
            .style(Style::new().bg(BG));

        let list = List::new(list_items)
            .block(block)
            .style(Style::new().fg(FG).bg(BG))
            .highlight_style(Style::new().fg(BRIGHT).bg(SEL_BG).bold());

        if let Some(state_cell) = app.band_states.get(band) {
            let mut state = state_cell.borrow_mut();
            StatefulWidget::render(&list, slot, frame, &mut state);
        }
    }

    // Status bar
    let status_slot = *slots.last().unwrap();
    let active_count = app.survey_items().len();
    let overdue_count = app
        .survey_items()
        .iter()
        .filter(|i| i.band == Band::Overdue)
        .count();
    let left = if overdue_count > 0 {
        format!("{active_count} active \u{00B7} {overdue_count} overdue")
    } else {
        format!("{active_count} active")
    };
    StatusLine::new()
        .style(Style::new().fg(SUBDUED).bg(SURFACE))
        .left(StatusItem::Text(Box::leak(left.into_boxed_str())))
        .right(StatusItem::Text("Tab\u{00A0}deck"))
        .right(StatusItem::Text("j/k\u{00A0}move"))
        .right(StatusItem::Text("l\u{00A0}enter"))
        .render(status_slot, frame);
}

// ---------------------------------------------------------------------------
// Deck view
// ---------------------------------------------------------------------------

fn render_deck(app: &App, frame: &mut Frame<'_>, area: Rect) {
    let cw = area.width as usize;
    let children = app.deck_children();
    let parent = app.deck_parent_item();

    let has_reality = parent.map(|p| !p.actual.is_empty()).unwrap_or(false);
    let has_children = !children.is_empty();
    let resolved = children
        .iter()
        .filter(|c| c.status == TensionStatus::Resolved)
        .count();
    let total = children.len();

    // Layout: desire(2) + progress(1) + sparkline(1) + table(fill) + reality(1-2) + status(1)
    let mut constraints = vec![
        Constraint::Fixed(2), // desire
        Constraint::Fixed(1), // progress bar
        Constraint::Fixed(1), // sparkline
        Constraint::Fill,     // children table
    ];
    if has_reality {
        constraints.push(Constraint::Fixed(2)); // blank + reality
    }
    constraints.push(Constraint::Fixed(1)); // status

    let slots = Flex::vertical().constraints(constraints).split(area);
    let mut si = 0;

    // --- Desire ---
    {
        let slot = slots[si];
        si += 1;
        let lines = match parent {
            Some(p) => {
                let did = display_id(p);
                let hlabel = p.horizon_label.as_deref().unwrap_or("");
                let right = format!("{did}  {hlabel}");
                let desire = trunc(&p.desired, cw.saturating_sub(right.len() + 10));
                let gap = cw.saturating_sub(7 + desire.len() + 1 + right.len());
                vec![
                    Line::from_spans([
                        Span::styled("\u{25c6} ", Style::new().fg(CYAN)),
                        Span::styled(desire, Style::new().fg(BRIGHT).bold()),
                        Span::styled(" ".repeat(gap.max(1)), Style::new()),
                        Span::styled(right, Style::new().fg(SUBDUED)),
                    ]),
                    Line::from_spans([
                        Span::styled("  ", Style::new()),
                        Span::styled(
                            "\u{2501}".repeat(cw.saturating_sub(2)),
                            Style::new().fg(BORDER),
                        ),
                    ]),
                ]
            }
            None => {
                let top_active = children.len();
                let count = format!("{top_active} top-level active");
                let gap = cw.saturating_sub(5 + 1 + count.len());
                vec![
                    Line::from_spans([
                        Span::styled("FIELD", Style::new().fg(CYAN).bold()),
                        Span::styled(" ".repeat(gap.max(1)), Style::new()),
                        Span::styled(count, Style::new().fg(SUBDUED)),
                    ]),
                    Line::from_spans([
                        Span::styled("  ", Style::new()),
                        Span::styled(
                            "\u{2501}".repeat(cw.saturating_sub(2)),
                            Style::new().fg(BORDER),
                        ),
                    ]),
                ]
            }
        };
        Paragraph::new(Text::from_lines(lines)).render(slot, frame);
    }

    // --- Progress bar (closure ratio) ---
    {
        let slot = slots[si];
        si += 1;
        if total > 0 {
            let ratio = resolved as f64 / total as f64;
            let label_str = format!("{resolved}/{total}");
            ProgressBar::new()
                .ratio(ratio)
                .label(Box::leak(label_str.into_boxed_str()))
                .style(Style::new().fg(SUBDUED).bg(BG))
                .gauge_style(if resolved == total {
                    Style::new().fg(GREEN)
                } else {
                    Style::new().fg(AMBER_LO)
                })
                .render(slot, frame);
        }
    }

    // --- Sparkline ---
    {
        let slot = slots[si];
        si += 1;
        let data: Vec<f64> = children.iter().map(|c| c.urgency).collect();
        if !data.is_empty() {
            Sparkline::new(&data)
                .style(Style::new().fg(AMBER))
                .max(2.0)
                .render(slot, frame);
        }
    }

    // --- Children list ---
    {
        let slot = slots[si];
        si += 1;
        if !has_children {
            let msg = if parent.is_some() {
                "  no children. press a to add."
            } else {
                "  nothing here. press a to name what matters."
            };
            Paragraph::new(Text::from(Line::from(Span::styled(
                msg,
                Style::new().fg(SUBDUED),
            ))))
            .render(slot, frame);
        } else {
            let inner_w = slot.width.saturating_sub(2) as usize;
            // Layout: [horizon 7][glyph 2][arrow 2] name... [age 4][id 5]
            let left_cols = 11usize;
            let right_cols = 9usize;
            let name_budget = inner_w.saturating_sub(left_cols + right_cols);

            let list_items: Vec<ListItem> = children
                .iter()
                .map(|item| {
                    let is_done = matches!(
                        item.status,
                        TensionStatus::Resolved | TensionStatus::Released
                    );
                    let is_held = item.status == TensionStatus::Active && item.position.is_none();
                    let gc = glyph_color(item);
                    let g = status_glyph(item);
                    let did = display_id(item);
                    let arrow = if item.children > 0 { "\u{2192}" } else { " " };

                    let name_style = if is_done {
                        Style::new().fg(SUBDUED)
                    } else if is_held {
                        Style::new().fg(DIM)
                    } else {
                        Style::new().fg(BRIGHT).bold()
                    };

                    let name = trunc(&item.desired, name_budget);
                    let name_pad = name_budget.saturating_sub(name.chars().count());
                    let hlabel = item.horizon_label.as_deref().unwrap_or("");

                    if is_done {
                        let line = Line::from_spans([
                            Span::styled(format!("{:<7}", hlabel), Style::new().fg(SUBDUED)),
                            Span::styled(format!("{g} "), Style::new().fg(gc)),
                            Span::styled("  ", Style::new()),
                            Span::styled(name, name_style),
                            Span::styled(" ".repeat(name_pad), Style::new()),
                            Span::styled("    ", Style::new()),
                            Span::styled(format!(" {:>4}", did), Style::new().fg(SUBDUED)),
                        ]);
                        ListItem::new(line).marker("")
                    } else {
                        let hlabel_c = if item.urgency > 1.0 {
                            urgency_color(item.urgency)
                        } else {
                            SUBDUED
                        };

                        let line = Line::from_spans([
                            Span::styled(format!("{:<7}", hlabel), Style::new().fg(hlabel_c)),
                            Span::styled(format!("{g} "), Style::new().fg(gc)),
                            Span::styled(format!("{arrow} "), Style::new().fg(SUBDUED)),
                            Span::styled(name, name_style),
                            Span::styled(" ".repeat(name_pad), Style::new()),
                            Span::styled(
                                format!(" {:>3}", item.age_label),
                                Style::new().fg(SUBDUED),
                            ),
                            Span::styled(format!(" {:>4}", did), Style::new().fg(SUBDUED)),
                        ]);
                        ListItem::new(line).marker("")
                    }
                })
                .collect();

            let block = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(BORDER))
                .style(Style::new().bg(BG));

            let list = List::new(list_items)
                .block(block)
                .style(Style::new().fg(FG).bg(BG))
                .highlight_style(Style::new().fg(BRIGHT).bg(SEL_BG).bold());

            // Clamp list area to content
            let content_h = (children.len() as u16 + 2).min(slot.height);
            let list_area = Rect::new(slot.x, slot.y, slot.width, content_h);

            let mut state = app.deck_list_state.borrow_mut();
            state.select(Some(app.cursor));
            StatefulWidget::render(&list, list_area, frame, &mut state);
        }
    }

    // --- Reality ---
    if has_reality {
        let slot = slots[si];
        si += 1;
        if let Some(p) = parent {
            let reality = trunc(&p.actual, cw.saturating_sub(12));
            Paragraph::new(Text::from_lines(vec![
                Line::from_spans([
                    Span::styled("  ", Style::new()),
                    Span::styled(
                        "\u{2501}".repeat(cw.saturating_sub(2)),
                        Style::new().fg(BORDER),
                    ),
                ]),
                Line::from_spans([
                    Span::styled("\u{25c7} ", Style::new().fg(SUBDUED)),
                    Span::styled(reality, Style::new().fg(DIM)),
                ]),
            ]))
            .render(slot, frame);
        }
    }

    // --- Status ---
    {
        let slot = *slots.last().unwrap();
        let loc = match &app.deck_parent {
            Some(_) => app
                .deck_parent_item()
                .map(|p| display_id(p))
                .unwrap_or_else(|| "?".into()),
            None => "root".into(),
        };
        StatusLine::new()
            .style(Style::new().fg(SUBDUED).bg(SURFACE))
            .left(StatusItem::Text(Box::leak(
                format!("deck \u{00B7} {loc}").into_boxed_str(),
            )))
            .right(StatusItem::Text("Tab\u{00A0}survey"))
            .right(StatusItem::Text("h\u{00A0}up"))
            .right(StatusItem::Text("l\u{00A0}enter"))
            .right(StatusItem::Text("j/k\u{00A0}move"))
            .render(slot, frame);
    }

    let _ = si; // suppress warning
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

impl Model for App {
    type Message = Msg;
    fn init(&mut self) -> Cmd<Msg> {
        Cmd::none()
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Quit => return Cmd::Quit,
            Msg::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            Msg::Down => {
                let n = self.visible_count();
                if self.cursor + 1 < n {
                    self.cursor += 1;
                }
            }
            Msg::SwitchView => {
                self.view = if self.view == View::Survey {
                    View::Deck
                } else {
                    View::Survey
                };
                self.cursor = 0;
                self.clamp_cursor();
            }
            Msg::Descend => {
                let target_id = match self.view {
                    View::Survey => {
                        let items = self.survey_items();
                        items
                            .get(self.cursor)
                            .filter(|i| i.children > 0)
                            .map(|i| i.id.clone())
                    }
                    View::Deck => {
                        let children = self.deck_children();
                        children
                            .get(self.cursor)
                            .filter(|i| i.children > 0)
                            .map(|i| i.id.clone())
                    }
                };
                if let Some(id) = target_id {
                    self.deck_parent = Some(id);
                    self.view = View::Deck;
                    self.cursor = 0;
                    self.clamp_cursor();
                }
            }
            Msg::Ascend => {
                if self.view == View::Deck {
                    if let Some(ref pid) = self.deck_parent.clone() {
                        let grandparent = self
                            .items
                            .iter()
                            .find(|i| i.id == *pid)
                            .and_then(|i| i.parent_id.clone());
                        self.deck_parent = grandparent;
                        self.cursor = 0;
                        self.clamp_cursor();
                    }
                }
            }
            Msg::Noop => {}
        }
        Cmd::none()
    }

    fn view(&self, frame: &mut Frame<'_>) {
        let full = Rect::new(0, 0, frame.width(), frame.height());
        frame.buffer.fill(
            full,
            ftui::Cell::from_char(' ').with_fg(SUBDUED).with_bg(BG),
        );
        let area = self.content_rect(full);

        match self.view {
            View::Survey => render_survey(self, frame, area),
            View::Deck => render_deck(self, frame, area),
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::discover().map_err(|e| format!("No werk workspace: {e}"))?;
    let store = workspace
        .open_store()
        .map_err(|e| format!("Store error: {e}"))?;
    let items = load_items(&store);
    let app = App::new(items);
    let mut program = Program::with_config(app, ProgramConfig::fullscreen())?;
    program.run()?;
    Ok(())
}
