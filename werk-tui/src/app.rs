//! The Operative Instrument application state.

use ftui::widgets::input::TextInput;
use sd_core::{DynamicsEngine, Store, Tension, TensionStatus};
use werk_shared::truncate;

use crate::glyphs;
use crate::state::*;
use crate::vlist::VirtualList;

/// The main application struct.
pub struct InstrumentApp {
    pub engine: DynamicsEngine,

    // Navigation
    pub parent_id: Option<String>,
    pub parent_tension: Option<Tension>,
    /// Parent's temporal indicator (six dots), computed on load_siblings
    pub parent_temporal_indicator: String,
    pub parent_temporal_urgency: f64,
    pub parent_horizon_label: Option<String>,
    /// How long ago the parent's desire was last articulated
    pub parent_desire_age: Option<String>,
    /// How long ago the parent's reality was last checked
    pub parent_reality_age: Option<String>,
    pub siblings: Vec<FieldEntry>,
    pub vlist: VirtualList,

    // Gaze
    pub gaze: Option<GazeState>,
    pub gaze_data: Option<GazeData>,
    pub full_gaze_data: Option<FullGazeData>,

    // Input
    pub input_mode: InputMode,
    pub input_buffer: String,
    /// TextInput widget for inline editing (edit mode only).
    pub text_input: TextInput,

    // Search
    pub search_state: Option<crate::search::SearchState>,

    // Filter
    pub filter: Filter,

    // Agent
    pub agent_mutations: Vec<werk_shared::AgentMutation>,
    pub agent_mutation_selected: Vec<bool>,
    pub agent_mutation_cursor: usize,
    pub agent_response_text: Option<String>,
    pub agent_tension_id: Option<String>,
    pub agent_last_user_message: Option<String>,

    // Chrome
    pub transient: Option<TransientMessage>,
    pub show_help: bool,

    // Watch insights
    pub pending_insights: Vec<crate::state::InsightData>,
    pub insight_cursor: usize,
    pub pending_insight_count: usize,

    // Change detection — only reload when db file changes
    pub db_modified: Option<std::time::SystemTime>,

    // Pre-computed for rendering (updated during navigation changes)
    pub breadcrumb_cache: Vec<(String, String)>,
    pub total_active: usize,
    pub total_count: usize,

    // Alerts — stateless, recomputed on load_siblings
    pub alerts: Vec<crate::state::Alert>,
    pub alert_cursor: usize,

    // Reordering — stores original positions for cancel
    pub reorder_original: Vec<(String, Option<i32>)>,
}

/// Filter for the field view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Active,
    All,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::Active => "active",
            Filter::All => "all",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Filter::Active => Filter::All,
            Filter::All => Filter::Active,
        }
    }
}

impl InstrumentApp {
    /// Create a new app. Starts at the Field (root level).
    pub fn new(store: Store, all_entries: Vec<FieldEntry>) -> Self {
        let engine = DynamicsEngine::with_store(store);
        let total_count = all_entries.len();
        let total_active = all_entries
            .iter()
            .filter(|e| e.status == TensionStatus::Active)
            .count();

        let mut app = Self {
            engine,
            parent_id: None,
            parent_tension: None,
            parent_temporal_indicator: String::new(),
            parent_temporal_urgency: 0.0,
            parent_horizon_label: None,
            parent_desire_age: None,
            parent_reality_age: None,
            siblings: Vec::new(),
            vlist: VirtualList::new(0),
            gaze: None,
            gaze_data: None,
            full_gaze_data: None,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            text_input: TextInput::new()
                .with_style(crate::theme::STYLES.text_bold)
                .with_cursor_style(ftui::style::Style::new().fg(crate::theme::CLR_CYAN))
                .with_placeholder_style(crate::theme::STYLES.dim),
            search_state: None,
            agent_mutations: Vec::new(),
            agent_mutation_selected: Vec::new(),
            agent_mutation_cursor: 0,
            agent_response_text: None,
            agent_tension_id: None,
            agent_last_user_message: None,
            filter: Filter::Active,
            transient: None,
            show_help: false,
            pending_insights: Vec::new(),
            insight_cursor: 0,
            pending_insight_count: 0,
            db_modified: None,
            breadcrumb_cache: Vec::new(),
            total_active,
            total_count,
            alerts: Vec::new(),
            alert_cursor: 0,
            reorder_original: Vec::new(),
        };
        app.load_siblings();
        app.refresh_pending_insight_count();
        app
    }

    /// Create an app in empty/welcome state.
    pub fn new_empty() -> Self {
        let engine = DynamicsEngine::new_in_memory().expect("failed to create in-memory engine");
        Self {
            engine,
            parent_id: None,
            parent_tension: None,
            parent_temporal_indicator: String::new(),
            parent_temporal_urgency: 0.0,
            parent_horizon_label: None,
            parent_desire_age: None,
            parent_reality_age: None,
            siblings: Vec::new(),
            vlist: VirtualList::new(0),
            gaze: None,
            gaze_data: None,
            full_gaze_data: None,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            text_input: TextInput::new()
                .with_style(crate::theme::STYLES.text_bold)
                .with_cursor_style(ftui::style::Style::new().fg(crate::theme::CLR_CYAN))
                .with_placeholder_style(crate::theme::STYLES.dim),
            search_state: None,
            agent_mutations: Vec::new(),
            agent_mutation_selected: Vec::new(),
            agent_mutation_cursor: 0,
            agent_response_text: None,
            agent_tension_id: None,
            agent_last_user_message: None,
            filter: Filter::Active,
            transient: None,
            show_help: false,
            pending_insights: Vec::new(),
            insight_cursor: 0,
            pending_insight_count: 0,
            db_modified: None,
            breadcrumb_cache: Vec::new(),
            total_active: 0,
            total_count: 0,
            alerts: Vec::new(),
            alert_cursor: 0,
            reorder_original: Vec::new(),
        }
    }

    /// Load siblings for the current parent_id. If None, load roots.
    pub fn load_siblings(&mut self) {
        let tensions = match &self.parent_id {
            Some(pid) => self.engine.store().get_children(pid).unwrap_or_default(),
            None => self.engine.store().get_roots().unwrap_or_default(),
        };

        // Load parent tension if descended
        self.parent_tension = self
            .parent_id
            .as_ref()
            .and_then(|pid| self.engine.store().get_tension(pid).ok().flatten());

        let now = chrono::Utc::now();

        // Compute parent temporal data for descended view header/footer
        if let Some(ref parent) = self.parent_tension {
            let mutations = self.engine.store()
                .get_mutations(&parent.id).unwrap_or_default();

            let last_reality = mutations.iter().rev()
                .find(|m| m.field() == "actual" || m.field() == "created")
                .map(|m| m.timestamp().to_owned())
                .unwrap_or(parent.created_at);

            let last_desire = mutations.iter().rev()
                .find(|m| m.field() == "desired" || m.field() == "created")
                .map(|m| m.timestamp().to_owned())
                .unwrap_or(parent.created_at);

            let horizon_end = parent.horizon.as_ref().map(|h| h.range_end());
            let (indicator, urgency) = crate::glyphs::temporal_indicator(last_reality, horizon_end, now);
            self.parent_temporal_indicator = indicator;
            self.parent_temporal_urgency = urgency;

            let now_year = chrono::Datelike::year(&now);
            self.parent_horizon_label = parent.horizon.as_ref()
                .map(|h| crate::glyphs::compact_horizon(h, now_year));

            self.parent_desire_age = Some(crate::glyphs::relative_time(last_desire, now));
            self.parent_reality_age = Some(crate::glyphs::relative_time(last_reality, now));
        } else {
            self.parent_temporal_indicator = String::new();
            self.parent_temporal_urgency = 0.0;
            self.parent_horizon_label = None;
            self.parent_desire_age = None;
            self.parent_reality_age = None;
        }

        // Sort: positioned DESC (from SQL), then unpositioned by horizon range_end
        let mut filtered: Vec<_> = tensions
            .iter()
            .filter(|t| match self.filter {
                Filter::Active => t.status == TensionStatus::Active,
                Filter::All => true,
            })
            .cloned()
            .collect();

        // The SQL already gives us positioned DESC first, then unpositioned by created_at.
        // Re-sort only the unpositioned group by horizon range_end (deadline).
        let first_unpositioned = filtered.iter().position(|t| t.position.is_none());
        if let Some(start) = first_unpositioned {
            filtered[start..].sort_by(|a, b| {
                match (&a.horizon, &b.horizon) {
                    (Some(ha), Some(hb)) => {
                        let end_ord = ha.range_end().cmp(&hb.range_end());
                        if end_ord != std::cmp::Ordering::Equal {
                            return end_ord;
                        }
                        ha.precision_level().cmp(&hb.precision_level())
                    }
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.created_at.cmp(&b.created_at),
                }
            });
        }

        self.siblings = filtered
            .iter()
            .map(|t| {
                let has_children = !self
                    .engine
                    .store()
                    .get_children(&t.id)
                    .unwrap_or_default()
                    .is_empty();
                // Find last reality update from mutation history
                let last_reality_update = self.engine.store()
                    .get_mutations(&t.id)
                    .unwrap_or_default()
                    .iter()
                    .rev()
                    .find(|m| m.field() == "actual" || m.field() == "created")
                    .map(|m| m.timestamp().to_owned())
                    .unwrap_or(t.created_at);
                FieldEntry::from_tension(t, last_reality_update, has_children, now)
            })
            .collect();

        // Rebuild vlist — preserve cursor position and gaze
        let old_cursor = self.vlist.cursor;
        let old_gaze = self.gaze.clone();

        self.vlist.rebuild(self.siblings.len());
        self.vlist.cursor = old_cursor.min(self.siblings.len().saturating_sub(1));

        // If gaze was open, try to keep it at the same cursor position
        if let Some(gaze) = old_gaze {
            if gaze.index < self.siblings.len() {
                let id = self.siblings[gaze.index].id.clone();
                self.gaze = Some(GazeState { index: gaze.index, full: gaze.full });
                self.gaze_data = self.compute_gaze(&id);
                let height = if gaze.full {
                    self.full_gaze_height_for_refresh()
                } else {
                    self.quick_gaze_height_for_refresh()
                };
                self.vlist.set_height(gaze.index, height);
            } else {
                self.gaze = None;
                self.gaze_data = None;
            }
        }

        // Update totals
        let all = self.engine.store().list_tensions().unwrap_or_default();
        self.total_count = all.len();
        self.total_active = all.iter().filter(|t| t.status == TensionStatus::Active).count();

        // Refresh breadcrumb cache
        self.breadcrumb_cache = self.breadcrumb();

        // Compute alerts
        self.compute_alerts();
    }

    /// Compute stateless alerts from current tension state.
    fn compute_alerts(&mut self) {
        use crate::state::{Alert, AlertKind};
        let mut alerts = Vec::new();
        let now = chrono::Utc::now();

        if let Some(ref parent) = self.parent_tension {
            // Neglect: no reality check in 3+ weeks
            let mutations = self.engine.store()
                .get_mutations(&parent.id).unwrap_or_default();
            let last_reality = mutations.iter().rev()
                .find(|m| m.field() == "actual" || m.field() == "created")
                .map(|m| m.timestamp().to_owned())
                .unwrap_or(parent.created_at);
            let weeks = now.signed_duration_since(last_reality).num_weeks();
            if weeks >= 3 {
                alerts.push(Alert {
                    kind: AlertKind::Neglect { weeks },
                    message: format!("neglected {} weeks", weeks),
                    action_hint: "update reality".to_string(),
                });
            }

            // Horizon past
            if let Some(ref h) = parent.horizon {
                let end = h.range_end();
                let past_days = now.signed_duration_since(end).num_days();
                if past_days > 0 {
                    alerts.push(Alert {
                        kind: AlertKind::HorizonPast { days: past_days },
                        message: format!("horizon past {} days", past_days),
                        action_hint: "extend or resolve".to_string(),
                    });
                }
            }
        }

        // Root-level alert: multiple root tensions
        if self.parent_id.is_none() {
            let roots = self.engine.store().get_roots().unwrap_or_default();
            let active_roots = roots.iter()
                .filter(|t| t.status == TensionStatus::Active)
                .count();
            if active_roots > 1 {
                alerts.push(Alert {
                    kind: AlertKind::MultipleRoots { count: active_roots },
                    message: format!("{} root tensions \u{2014} no senior organizing principle", active_roots),
                    action_hint: "create a parent for all / move inside another".to_string(),
                });
            }
        }

        self.alerts = alerts;
        self.alert_cursor = 0;
    }

    /// Descend into a tension's children.
    pub fn descend(&mut self, id: &str) {
        self.parent_id = Some(id.to_string());
        self.load_siblings();
        self.gaze = None;
        self.gaze_data = None;
        self.full_gaze_data = None;
        self.vlist.cursor = 0;
    }

    /// Ascend to parent level. Cursor lands on the tension we just left.
    pub fn ascend(&mut self) {
        let old_parent_id = self.parent_id.take();

        // Close gaze
        self.gaze = None;
        self.gaze_data = None;
        self.full_gaze_data = None;

        // Find the grandparent
        if let Some(ref pid) = old_parent_id {
            if let Ok(Some(parent)) = self.engine.store().get_tension(pid) {
                self.parent_id = parent.parent_id.clone();
            }
        }

        self.load_siblings();

        // Set cursor to the tension we ascended from
        if let Some(ref old_pid) = old_parent_id {
            if let Some(idx) = self.siblings.iter().position(|s| s.id == *old_pid) {
                self.vlist.cursor = idx;
            }
        }
    }

    /// Get the currently selected entry.
    pub fn selected_entry(&self) -> Option<&FieldEntry> {
        self.siblings.get(self.vlist.cursor)
    }

    /// The action target: gazed tension if gaze is active, else selected.
    pub fn action_target(&self) -> Option<&FieldEntry> {
        if let Some(ref gaze) = self.gaze {
            self.siblings.get(gaze.index)
        } else {
            self.selected_entry()
        }
    }

    /// Build breadcrumb path from current parent up to root.
    pub fn breadcrumb(&mut self) -> Vec<(String, String)> {
        let mut crumbs = Vec::new();
        let mut current_id = self.parent_id.clone();
        while let Some(ref id) = current_id {
            if let Ok(Some(t)) = self.engine.store().get_tension(id) {
                let glyph = glyphs::status_glyph(t.status);
                crumbs.push((glyph.to_string(), t.desired.clone()));
                current_id = t.parent_id.clone();
            } else {
                break;
            }
        }
        crumbs.reverse(); // root first
        crumbs
    }

    /// Compute quick Gaze data for a tension.
    pub fn compute_gaze(&mut self, id: &str) -> Option<GazeData> {
        let tension = self.engine.store().get_tension(id).ok()??;

        // Children preview — collect IDs first to avoid borrow conflicts
        let children_tensions = self.engine.store().get_children(id).unwrap_or_default();
        let active_children: Vec<_> = children_tensions
            .iter()
            .filter(|c| c.status == TensionStatus::Active)
            .take(8)
            .cloned()
            .collect();

        let mut children: Vec<ChildPreview> = Vec::new();
        for c in &active_children {
            children.push(ChildPreview {
                id: c.id.clone(),
                desired: c.desired.clone(),
                status: c.status,
                position: c.position,
            });
        }

        // Horizon display
        let now = chrono::Utc::now();
        let horizon = tension.horizon.as_ref().map(|h| {
            let days = h.range_end().signed_duration_since(now).num_days();
            if days < 0 {
                format!("{} ({}d past)", h, -days)
            } else if days == 0 {
                format!("{} (today)", h)
            } else {
                format!("{} ({}d)", h, days)
            }
        });

        // Last event — most recent mutation for this tension
        let last_event = self.engine.store().get_mutations(&tension.id).ok().and_then(|mutations| {
            mutations.last().map(|m| {
                let elapsed = now.signed_duration_since(m.timestamp().to_owned());
                let time_str = if elapsed.num_minutes() < 1 {
                    "just now".to_string()
                } else if elapsed.num_hours() < 1 {
                    format!("{}m ago", elapsed.num_minutes())
                } else if elapsed.num_hours() < 24 {
                    format!("{}h ago", elapsed.num_hours())
                } else {
                    format!("{}d ago", elapsed.num_days())
                };
                format!("{} {}", m.field(), time_str)
            })
        });

        // Created date display
        let created_at = tension.created_at.format("%Y-%m-%d").to_string();

        Some(GazeData {
            id: tension.id.clone(),
            actual: tension.actual.clone(),
            horizon,
            created_at,
            children,
            last_event,
        })
    }

    /// Create a tension with a horizon string (e.g. "2026-W13" or "2026-03-20").
    pub fn create_tension_with_horizon(&mut self, name: &str, desire: &str, reality: &str, horizon_str: &str) {
        let desired = if desire.is_empty() { name } else { desire };
        let parent = self.parent_id.clone();

        // Try to parse horizon (supports natural language like "tomorrow", "2w", "eom")
        let horizon = crate::horizon::parse_horizon(horizon_str).ok();

        let result = self.engine.create_tension_full(desired, reality, parent, horizon);

        if let Ok(tension) = result {
            self.set_transient(format!("created: {}", truncate(&tension.desired, 30)));
            self.load_siblings();
            if let Some(idx) = self.siblings.iter().position(|s| s.id == tension.id) {
                self.vlist.cursor = idx;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Reordering — grab-and-drop interaction
    // -----------------------------------------------------------------------

    /// Enter reorder mode: grab the selected tension for repositioning.
    /// Shift+J/K enters this mode; j/k moves visually; Enter commits; Esc cancels.
    pub fn enter_reorder(&mut self) {
        if self.siblings.is_empty() { return; }
        let cursor = self.vlist.cursor;
        let tension_id = self.siblings[cursor].id.clone();

        // Store original positions for cancel
        self.reorder_original = self.siblings.iter()
            .map(|s| (s.id.clone(), s.position))
            .collect();

        self.input_mode = InputMode::Reordering { tension_id };
        // Close gaze if open
        if self.gaze.is_some() {
            self.gaze = None;
            self.gaze_data = None;
            self.full_gaze_data = None;
            self.vlist.reset_heights();
        }
    }

    /// Move the grabbed tension up one position (visual swap only, no engine writes).
    pub fn reorder_move_up(&mut self) {
        if self.siblings.is_empty() || self.vlist.cursor == 0 { return; }
        let cursor = self.vlist.cursor;
        self.siblings.swap(cursor, cursor - 1);
        self.vlist.cursor = cursor - 1;
    }

    /// Move the grabbed tension down one position (visual swap only, no engine writes).
    pub fn reorder_move_down(&mut self) {
        if self.siblings.is_empty() || self.vlist.cursor >= self.siblings.len() - 1 { return; }
        let cursor = self.vlist.cursor;
        self.siblings.swap(cursor, cursor + 1);
        self.vlist.cursor = cursor + 1;
    }

    /// Commit the reorder: write final positions to engine as a single logical action.
    /// Preserves the positioned/unpositioned boundary. Moving a tension below the
    /// boundary unpositions it; moving one above positions it.
    pub fn reorder_commit(&mut self) {
        let tension_id = match &self.input_mode {
            InputMode::Reordering { tension_id } => tension_id.clone(),
            _ => String::new(),
        };

        // Count how many items were originally positioned
        let originally_positioned = self.reorder_original.iter()
            .filter(|(_, pos)| pos.is_some())
            .count();

        // Was the grabbed tension originally positioned?
        let grabbed_was_positioned = self.reorder_original.iter()
            .any(|(id, pos)| id == &tension_id && pos.is_some());

        // Find where the grabbed tension ended up
        let grabbed_index = self.siblings.iter()
            .position(|s| s.id == tension_id)
            .unwrap_or(0);

        // Compute the boundary: how many items should be positioned in the result.
        // The boundary is the original count, adjusted if the grabbed item crossed it.
        let boundary = if grabbed_was_positioned && grabbed_index >= originally_positioned {
            // Grabbed item moved out of positioned group → boundary shrinks by 1
            originally_positioned.saturating_sub(1)
        } else if !grabbed_was_positioned && grabbed_index < originally_positioned {
            // Grabbed item moved into positioned group → boundary grows by 1
            originally_positioned + 1
        } else {
            originally_positioned
        };

        // Assign positions above boundary, None below
        for (i, sibling) in self.siblings.iter().enumerate() {
            if i < boundary {
                let pos = (boundary - i) as i32;
                let _ = self.engine.update_position(&sibling.id, Some(pos));
            } else {
                let _ = self.engine.update_position(&sibling.id, None);
            }
        }

        self.reorder_original.clear();
        self.input_mode = InputMode::Normal;
        self.load_siblings();

        // Restore cursor to the moved tension
        if let Some(idx) = self.siblings.iter().position(|s| s.id == tension_id) {
            self.vlist.cursor = idx;
        }
        self.set_transient("position updated");
    }

    /// Cancel the reorder: restore original positions and cursor.
    pub fn reorder_cancel(&mut self) {
        // Get the original tension ID to restore cursor
        let tension_id = match &self.input_mode {
            InputMode::Reordering { tension_id } => tension_id.clone(),
            _ => String::new(),
        };

        // Restore original positions from snapshot
        for (id, pos) in &self.reorder_original {
            let _ = self.engine.update_position(id, *pos);
        }
        self.reorder_original.clear();
        self.input_mode = InputMode::Normal;
        self.load_siblings();

        // Restore cursor to the original tension
        if let Some(idx) = self.siblings.iter().position(|s| s.id == tension_id) {
            self.vlist.cursor = idx;
        }
    }


    /// Compute full gaze data (facts + history) for a tension.
    pub fn compute_full_gaze(&mut self, id: &str) -> Option<FullGazeData> {
        let tension = self.engine.store().get_tension(id).ok()??;
        let now = chrono::Utc::now();

        // Urgency from horizon
        let urgency = self.engine.compute_urgency(&tension).map(|u| {
            format!("{:.0}%", u.value * 100.0)
        });

        // Horizon drift from mutation history
        let mutations = self.engine.store().get_mutations(id).unwrap_or_default();
        let drift = sd_core::detect_horizon_drift(id, &mutations);
        let horizon_drift = if drift.change_count > 0 {
            Some(match drift.drift_type {
                sd_core::HorizonDriftType::Stable => "stable".to_string(),
                sd_core::HorizonDriftType::Tightening => "tightening".to_string(),
                sd_core::HorizonDriftType::Postponement => "postponement".to_string(),
                sd_core::HorizonDriftType::RepeatedPostponement => "repeated postponement".to_string(),
                sd_core::HorizonDriftType::Loosening => "loosening".to_string(),
                sd_core::HorizonDriftType::Oscillating => "oscillating".to_string(),
            })
        } else {
            None
        };

        // Closure: proportion of children resolved
        let children = self.engine.store().get_children(id).unwrap_or_default();
        let closure = if !children.is_empty() {
            let resolved = children.iter().filter(|c| c.status == TensionStatus::Resolved).count();
            Some(format!("{}/{}", resolved, children.len()))
        } else {
            None
        };

        // History
        let history: Vec<HistoryEntry> = mutations
            .iter()
            .rev() // most recent first
            .take(20)
            .map(|m| {
                let relative = werk_shared::relative_time(m.timestamp(), now);
                let desc = match m.field() {
                    "desired" => format!("desire: \"{}\"", truncate(m.new_value(), 70)),
                    "actual" => format!("reality: \"{}\"", truncate(m.new_value(), 70)),
                    "status" => format!("status \u{2192} {}", m.new_value()),
                    "note" => format!("note: \"{}\"", truncate(m.new_value(), 70)),
                    "parent_id" => "parent changed".to_string(),
                    "horizon" => format!("horizon \u{2192} {}", m.new_value()),
                    "created" => "created".to_string(),
                    other => format!("{}: {}", other, truncate(m.new_value(), 60)),
                };
                HistoryEntry {
                    relative_time: relative,
                    description: desc,
                }
            })
            .collect();

        Some(FullGazeData {
            urgency,
            horizon_drift,
            closure,
            history,
        })
    }

    /// Build agent context string for a tension.
    pub fn build_agent_context(&mut self, tension_id: &str) -> String {
        let _short_id = &tension_id[..12.min(tension_id.len())];
        let mut ctx = String::new();
        if let Ok(Some(t)) = self.engine.store().get_tension(tension_id) {
            ctx.push_str(&format!("Tension ID: {}\n", tension_id));
            ctx.push_str(&format!("Desired: {}\n", t.desired));
            ctx.push_str(&format!("Reality: {}\n", t.actual));
            if let Some(ref h) = t.horizon {
                ctx.push_str(&format!("Horizon: {}\n", h));
            }
            ctx.push_str(&format!("Status: {}\n", t.status));

            // Add children
            let children = self.engine.store().get_children(tension_id).unwrap_or_default();
            if !children.is_empty() {
                ctx.push_str(&format!("\nChildren ({} active):\n",
                    children.iter().filter(|c| c.status == sd_core::TensionStatus::Active).count()
                ));
                for c in children.iter().filter(|c| c.status == sd_core::TensionStatus::Active).take(8) {
                    ctx.push_str(&format!("  - {}\n", c.desired));
                }
            }

            // Recent history
            let mutations = self.engine.store().get_mutations(tension_id).unwrap_or_default();
            if !mutations.is_empty() {
                ctx.push_str("\nRecent history:\n");
                let now = chrono::Utc::now();
                for m in mutations.iter().rev().take(5) {
                    let rel = werk_shared::relative_time(m.timestamp(), now);
                    ctx.push_str(&format!("  {} {}: {}\n", rel, m.field(), truncate(m.new_value(), 50)));
                }
            }

            // Structured response instructions
            ctx.push_str(&format!(r#"

IMPORTANT: To suggest changes, include a YAML block at the END of your response between --- markers.
Put ALL your prose/advice BEFORE the YAML block. Example:

Your advice and analysis here...

---
mutations:
  - action: update_actual
    tension_id: "{tid}"
    new_value: "the new reality"
    reasoning: "why"
response: |
  Brief summary of suggestions.
---

Available actions:
  update_actual   - tension_id, new_value, reasoning
  update_desired  - tension_id, new_value, reasoning
  create_child    - parent_id: "{tid}", desired, actual, reasoning
  update_status   - tension_id, new_status: "Resolved"|"Released", reasoning
  add_note        - tension_id, text
  set_horizon     - tension_id, horizon (e.g. "2026-04", "2w", "eom"), reasoning
  move_tension    - tension_id, new_parent_id (or null for root), reasoning
  create_parent   - child_id: "{tid}", desired, actual, reasoning

Multiple mutations allowed. If no changes needed, respond with plain text only (no YAML block).
The YAML block MUST be the last thing in your response.
"#,
                tid = tension_id,
            ));
        }
        ctx
    }

    /// Build agent context for clipboard handoff (no YAML mutation instructions).
    pub fn build_agent_context_for_clipboard(&mut self, tension_id: &str) -> String {
        let mut ctx = String::new();
        if let Ok(Some(t)) = self.engine.store().get_tension(tension_id) {
            ctx.push_str(&format!("Tension: {}\n", t.desired));
            ctx.push_str(&format!("Desired: {}\n", t.desired));
            ctx.push_str(&format!("Reality: {}\n", t.actual));
            if let Some(ref h) = t.horizon {
                ctx.push_str(&format!("Horizon: {}\n", h));
            }
            ctx.push_str(&format!("Status: {}\n", t.status));

            let children = self.engine.store().get_children(tension_id).unwrap_or_default();
            if !children.is_empty() {
                ctx.push_str(&format!("\nChildren ({} active):\n",
                    children.iter().filter(|c| c.status == sd_core::TensionStatus::Active).count()
                ));
                for c in children.iter().filter(|c| c.status == sd_core::TensionStatus::Active).take(8) {
                    ctx.push_str(&format!("  - {}\n", c.desired));
                }
            }

            let mutations = self.engine.store().get_mutations(tension_id).unwrap_or_default();
            if !mutations.is_empty() {
                ctx.push_str("\nRecent history:\n");
                let now = chrono::Utc::now();
                for m in mutations.iter().rev().take(5) {
                    let rel = werk_shared::relative_time(m.timestamp(), now);
                    ctx.push_str(&format!("  {} {}: {}\n", rel, m.field(), truncate(m.new_value(), 50)));
                }
            }
        }
        ctx
    }

    /// Copy text to system clipboard.
    pub fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        use std::process::{Command, Stdio};
        use std::io::Write;
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        Ok(())
    }

    /// Get the configured agent command.
    pub fn get_agent_command(&self) -> Option<String> {
        let workspace = werk_shared::Workspace::discover().ok()?;
        let config = werk_shared::Config::load(&workspace).ok()?;
        config.get("agent.command").cloned()
    }

    /// Apply selected agent mutations to the store.
    pub fn apply_selected_mutations(&mut self) {
        let tension_id = self.agent_tension_id.clone();
        for (i, mutation) in self.agent_mutations.iter().enumerate() {
            if !self.agent_mutation_selected.get(i).copied().unwrap_or(false) {
                continue;
            }
            if let Some(ref tid) = tension_id {
                match mutation {
                    werk_shared::AgentMutation::UpdateActual { new_value, .. } => {
                        let _ = self.engine.update_actual(tid, new_value);
                    }
                    werk_shared::AgentMutation::UpdateDesired { new_value, .. } => {
                        let _ = self.engine.update_desired(tid, new_value);
                    }
                    werk_shared::AgentMutation::CreateChild { desired, actual, .. } => {
                        let _ = self.engine.create_tension_with_parent(desired, actual, Some(tid.clone()));
                    }
                    werk_shared::AgentMutation::UpdateStatus { new_status, .. } => {
                        if new_status == "Resolved" {
                            let _ = self.engine.resolve(tid);
                        } else if new_status == "Released" {
                            let _ = self.engine.release(tid);
                        }
                    }
                    werk_shared::AgentMutation::AddNote { text, .. } => {
                        let _ = self.engine.store().record_mutation(
                            &sd_core::Mutation::new(
                                tid.clone(),
                                chrono::Utc::now(),
                                "note".to_owned(),
                                None,
                                text.clone(),
                            ),
                        );
                    }
                    werk_shared::AgentMutation::SetHorizon { horizon, .. } => {
                        if let Ok(h) = crate::horizon::parse_horizon(horizon) {
                            let _ = self.engine.update_horizon(tid, Some(h));
                        }
                    }
                    werk_shared::AgentMutation::MoveTension { new_parent_id, .. } => {
                        let _ = self.engine.update_parent(tid, new_parent_id.as_deref());
                    }
                    werk_shared::AgentMutation::CreateParent { child_id, desired, actual, .. } => {
                        // Create a new parent tension, then reparent the child under it
                        let current_parent = self.engine.store().get_tension(child_id)
                            .ok().flatten().and_then(|t| t.parent_id.clone());
                        if let Ok(parent) = self.engine.create_tension_with_parent(
                            desired, actual, current_parent
                        ) {
                            let _ = self.engine.update_parent(child_id, Some(&parent.id));
                        }
                    }
                }
            }
        }
        let applied = self.agent_mutation_selected.iter().filter(|&&s| s).count();
        self.set_transient(format!("applied {} mutations", applied));
        self.agent_mutations.clear();
        self.agent_mutation_selected.clear();
        self.agent_response_text = None;
        self.agent_tension_id = None;
        self.load_siblings();
    }

    /// Check if the database file has been modified since last check.
    /// Returns true if data should be reloaded.
    pub fn db_has_changed(&mut self) -> bool {
        let db_path = std::env::current_dir()
            .ok()
            .and_then(|mut d| {
                loop {
                    let candidate = d.join(".werk").join("sd.db");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                    if !d.pop() {
                        return None;
                    }
                }
            });

        if let Some(path) = db_path {
            if let Ok(meta) = std::fs::metadata(&path) {
                if let Ok(modified) = meta.modified() {
                    let changed = self.db_modified.map(|prev| modified != prev).unwrap_or(true);
                    self.db_modified = Some(modified);
                    return changed;
                }
            }
        }
        false
    }

    /// Quick gaze height estimate for refresh (doesn't conflict with borrows).
    fn quick_gaze_height_for_refresh(&self) -> usize {
        let mut h = 2; // panel top + bottom border
        if let Some(ref data) = self.gaze_data {
            h += data.children.len().max(1);
            if !data.actual.is_empty() {
                h += 2;
            }
        } else {
            h += 1;
        }
        h
    }

    fn full_gaze_height_for_refresh(&self) -> usize {
        let mut h = self.quick_gaze_height_for_refresh();
        if let Some(ref full) = self.full_gaze_data {
            h += 1; // separator
            let dyn_count = full.urgency.is_some() as usize
                + full.horizon_drift.is_some() as usize
                + full.closure.is_some() as usize;
            let dyn_count = dyn_count.max(1); // at least 1 row
            let hist_count = full.history.len().min(dyn_count.max(3));
            h += dyn_count.max(hist_count);
        }
        h
    }

    /// Set a transient message on the lever.
    #[allow(dead_code)]
    pub fn set_transient(&mut self, text: impl Into<String>) {
        self.transient = Some(TransientMessage::new(text));
    }

    /// Check how many pending watch insights exist on disk.
    pub fn refresh_pending_insight_count(&mut self) {
        if let Ok(workspace) = werk_shared::Workspace::discover() {
            let pending_dir = workspace.werk_dir().join("watch").join("pending");
            if pending_dir.exists() {
                let count = std::fs::read_dir(&pending_dir)
                    .map(|entries| {
                        entries
                            .flatten()
                            .filter(|e| {
                                let path = e.path();
                                if path.extension().map(|ext| ext == "yaml").unwrap_or(false) {
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        // Quick check: not reviewed
                                        content.contains("reviewed: false")
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            })
                            .count()
                    })
                    .unwrap_or(0);
                self.pending_insight_count = count;
            }
        }
    }

    /// Load all pending insights from disk into memory for review.
    pub fn load_pending_insights(&mut self) {
        use crate::state::InsightData;

        self.pending_insights.clear();
        self.insight_cursor = 0;

        // Find the workspace .werk directory by walking up
        let pending_dir = std::env::current_dir()
            .ok()
            .and_then(|mut d| {
                loop {
                    let candidate = d.join(".werk").join("watch").join("pending");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                    if !d.pop() {
                        return None;
                    }
                }
            });

        let pending_dir = match pending_dir {
            Some(d) => d,
            None => return,
        };

        let mut entries: Vec<_> = std::fs::read_dir(&pending_dir)
            .map(|rd| rd.flatten().collect())
            .unwrap_or_default();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let path = entry.path();
            if path.extension().map(|ext| ext == "yaml").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if !content.contains("reviewed: false") {
                        continue;
                    }

                    // Simple line-based parsing — more robust than serde_yaml for this format
                    let get_field = |field: &str| -> String {
                        content.lines()
                            .find(|l| l.starts_with(&format!("{}: ", field)))
                            .map(|l| l[field.len() + 2..].trim().trim_matches('\'').trim_matches('"').to_string())
                            .unwrap_or_default()
                    };

                    let tension_id = get_field("tension_id");
                    let trigger = get_field("trigger");
                    let timestamp = get_field("timestamp");
                    let tension_desired = get_field("tension_desired");

                    // Response is multi-line — extract between "response:" and next top-level key
                    let response = extract_yaml_multiline(&content, "response");

                    // Extract mutations section
                    let mutation_count = content.matches("- action:").count();
                    let mutation_text = extract_mutations_section(&content);

                    self.pending_insights.push(InsightData {
                        file_path: path,
                        tension_id,
                        tension_desired,
                        trigger,
                        response,
                        mutation_count,
                        mutation_text,
                        timestamp,
                        expanded: false,
                    });
                }
            }
        }
    }

    /// Accept the current insight: mark as reviewed, apply any note mutation.
    pub fn accept_current_insight(&mut self) {
        if let Some(insight) = self.pending_insights.get(self.insight_cursor) {
            // Mark as reviewed on disk
            if let Ok(content) = std::fs::read_to_string(&insight.file_path) {
                let new_content = content.replace("reviewed: false", "reviewed: true");
                let _ = std::fs::write(&insight.file_path, new_content);
            }

            // Add a note mutation to record the insight
            if !insight.tension_id.is_empty() {
                let note_text = format!(
                    "watch: {} -- {}",
                    insight.trigger,
                    truncate(&insight.response, 100),
                );
                let _ = self.engine.store().record_mutation(
                    &sd_core::Mutation::new(
                        insight.tension_id.clone(),
                        chrono::Utc::now(),
                        "note".to_owned(),
                        None,
                        note_text,
                    ),
                );
            }
        }

        // Remove from list and advance
        if !self.pending_insights.is_empty() {
            self.pending_insights.remove(self.insight_cursor);
            if self.insight_cursor >= self.pending_insights.len() && self.insight_cursor > 0 {
                self.insight_cursor -= 1;
            }
        }
        self.pending_insight_count = self.pending_insights.len();

        if self.pending_insights.is_empty() {
            self.input_mode = InputMode::Normal;
            self.set_transient("all insights reviewed");
            self.load_siblings();
        }
    }

    /// Dismiss the current insight: mark as reviewed without applying.
    pub fn dismiss_current_insight(&mut self) {
        if let Some(insight) = self.pending_insights.get(self.insight_cursor) {
            // Mark as reviewed on disk
            if let Ok(content) = std::fs::read_to_string(&insight.file_path) {
                let new_content = content.replace("reviewed: false", "reviewed: true");
                let _ = std::fs::write(&insight.file_path, new_content);
            }
        }

        // Remove from list and advance
        if !self.pending_insights.is_empty() {
            self.pending_insights.remove(self.insight_cursor);
            if self.insight_cursor >= self.pending_insights.len() && self.insight_cursor > 0 {
                self.insight_cursor -= 1;
            }
        }
        self.pending_insight_count = self.pending_insights.len();

        if self.pending_insights.is_empty() {
            self.input_mode = InputMode::Normal;
            self.set_transient("all insights dismissed");
            self.load_siblings();
        }
    }
}

/// Extract the mutations section from a pending insight YAML file.
fn extract_mutations_section(content: &str) -> String {
    let mut result = String::new();
    let mut in_mutations = false;

    for line in content.lines() {
        if line.starts_with("mutations:") {
            in_mutations = true;
            continue;
        }
        if in_mutations {
            if line.starts_with("- ") || line.starts_with("  ") {
                // Format nicely: "action: add_note" -> "add_note"
                let trimmed = line.trim();
                if trimmed.starts_with("- action:") {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&format!("\u{25B8} {}", trimmed.trim_start_matches("- action:").trim()));
                } else if trimmed.starts_with("action:") {
                    // skip
                } else {
                    result.push_str(&format!("  {}\n", trimmed));
                }
            } else {
                // Hit a non-indented line — end of mutations section
                break;
            }
        }
    }
    result
}

/// Extract a multi-line YAML value (e.g. response field that spans multiple lines).
fn extract_yaml_multiline(content: &str, field: &str) -> String {
    let prefix = format!("{}:", field);
    let mut lines = content.lines();
    let mut result = String::new();
    let mut found = false;

    for line in &mut lines {
        if line.starts_with(&prefix) {
            // Check for inline value
            let rest = line[prefix.len()..].trim();
            if rest.starts_with("|-") || rest.starts_with("|") {
                // Block scalar — read indented lines
                found = true;
                continue;
            } else if !rest.is_empty() {
                // Inline value
                return rest.trim_matches('\'').trim_matches('"').to_string();
            }
            found = true;
            continue;
        }
        if found {
            // Read indented continuation lines until we hit a non-indented line
            if line.starts_with(' ') || line.starts_with('\t') || line.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(line.trim_start());
            } else {
                break;
            }
        }
    }
    result
}
