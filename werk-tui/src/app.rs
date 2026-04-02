//! The Operative Instrument application state.

use std::sync::Arc;

use ftui::widgets::input::TextInput;
use ftui_runtime::state_persistence::StateRegistry;
use sd_core::{Engine, Store, Tension, TensionStatus};
use werk_shared::truncate;

use crate::glyphs;
use crate::state::*;

/// The main application struct.
pub struct InstrumentApp {
    pub engine: Engine,

    // Store session — identifies this TUI instance in the gesture/session model.
    // Created on startup, ended on drop. Each TUI holds its own session_id.
    pub session_id: Option<String>,

    // Navigation
    pub parent_id: Option<String>,
    pub parent_tension: Option<Tension>,
    pub parent_horizon_label: Option<String>,
    /// How long ago the parent's desire was last articulated
    pub parent_desire_age: Option<String>,
    /// How long ago the parent's reality was last checked
    pub parent_reality_age: Option<String>,
    pub siblings: Vec<FieldEntry>,

    // Input
    pub input_mode: InputMode,
    pub input_buffer: String,
    /// TextInput widget for inline editing (edit mode only).
    pub text_input: TextInput,

    // Search
    pub search_state: Option<crate::search::SearchState>,
    pub search_index: Option<sd_core::SearchIndex>,

    // Chrome
    pub show_help: bool,
    /// Inspector overlay — dev tool toggled by Ctrl+Shift+I.
    pub show_inspector: bool,

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

    // Deck cached data — computed during load_siblings, not during render
    pub grandparent_display: Option<(String, String)>, // (display_id, desired text)
    pub parent_mutation_count: usize,
    pub db_path_cache: Option<std::path::PathBuf>,

    // Cached render expansion — updated after each render so navigation uses
    // the same frontier expansion as the visible display.
    pub last_render_lines: std::cell::Cell<usize>,

    // Trajectory mode (Q30): when true, positioned resolved/released stay on route
    pub trajectory_mode: bool,

    // Epoch boundary (V5): timestamp of the last epoch close for the current parent.
    // Children resolved/released before this are excluded from accumulated.
    pub epoch_boundary: Option<chrono::DateTime<chrono::Utc>>,

    // Deck configuration (V6): read from deck.* config keys.
    pub deck_config: crate::deck::DeckConfig,

    // Focus zoom (V7): detail of the currently focused child or note.
    pub deck_zoom: crate::deck::ZoomLevel,
    pub focused_detail: Option<crate::deck::FocusedDetail>,
    pub focused_note: Option<crate::deck::FocusedNote>,

    // Parent notes for accumulated zone display
    pub parent_notes: Vec<crate::deck::AccumulatedItem>,

    // Pathway palette state — active when InputMode::Pathway.
    pub pathway_state: Option<crate::state::PathwayState>,

    // Frontier — always valid, recomputed after every data change.
    pub frontier: crate::deck::Frontier,

    // Summary expansion overrides — toggled by Enter on summary lines.
    // When true, compute_expansion's compression is overridden to show all items.
    pub route_expanded: bool,
    pub held_expanded: bool,
    pub accumulated_expanded: bool,

    // View orientation — Stream (deck) or Survey (time-first)
    pub view_orientation: crate::state::ViewOrientation,
    /// Cursor position in the survey item list.
    pub survey_cursor: usize,
    /// All active tensions for the survey view, sorted by time band.
    pub survey_items: Vec<crate::survey::SurveyItem>,
    /// Field-wide vitals for the NOW zone.
    pub field_vitals: crate::survey::FieldVitals,
    /// Saved stream state for Shift+Tab return (parent_id, focus node).
    pub pre_survey_state: Option<(Option<String>, ftui::widgets::FocusId)>,
    /// Survey band collapse/expand state.
    pub survey_tree_state: crate::survey_tree::SurveyTreeState,

    // Logbase view — epoch stream for a single tension.
    /// Which tension's logbase we're viewing.
    pub logbase_tension_id: Option<String>,
    /// Cached tension data for the logbase subject.
    pub logbase_tension: Option<sd_core::Tension>,
    /// All epochs for the logbase tension (chronological, oldest first).
    pub logbase_epochs: Vec<sd_core::store::EpochRecord>,
    /// Flat event stream: epochs + their mutations, ordered most-recent-first.
    pub logbase_events: Vec<crate::logbase::LogbaseEvent>,
    /// Provenance edges for the logbase tension.
    pub logbase_provenance: crate::logbase::LogbaseProvenance,
    /// List widget state for the event stream (selection + scroll offset).
    /// RefCell because StatefulWidget::render needs &mut but view() has &self.
    pub logbase_list_state: std::cell::RefCell<ftui::widgets::list::ListState>,
    /// Which epoch index is "focused" (gets fisheye expansion).
    pub logbase_focused_epoch: usize,
    /// Pre-built list items for the event stream (rebuilt on enter/epoch change).
    pub logbase_items: Vec<crate::logbase::LogbaseItem>,
    /// Cached header text lines (built once on enter, no store queries during render).
    pub logbase_header: Vec<(String, crate::logbase::HeaderStyle)>,
    /// Cached separator text.
    pub logbase_separator: String,
    /// Saved originating view state for L-return (orientation, parent_id, focus node).
    pub pre_logbase_state: Option<(crate::state::ViewOrientation, Option<String>, ftui::widgets::FocusId)>,

    // Session telemetry — records every significant action for debugging.
    pub session_log: crate::session_log::SessionLog,

    // Spatial layout — three-pane model with responsive breakpoints.
    pub layout: crate::layout::LayoutState,
    // Focus graph — skeleton for Phase 2, wired to navigation in Phase 4.
    pub focus_state: crate::focus::FocusState,

    // Theme — resolved at startup for the detected terminal mode.
    pub styles: crate::theme::InstrumentStyles,

    // Toast notifications — replaces TransientMessage.
    pub toasts: crate::toast::ToastQueue,

    // Gesture undo/redo history.
    pub gesture_history: crate::undo::GestureHistory,
    /// Pending gesture snapshot — captured by begin_gesture(), committed by end_gesture_tracked().
    pending_gesture: Option<(String, crate::undo::StateSnapshot)>,

    // Command palette — unified search/command/navigation surface.
    pub command_palette: ftui::widgets::command_palette::CommandPalette,
    /// Feedback collector for palette action learning.
    pub palette_feedback: crate::feedback::FeedbackCollector,

    // State persistence — file-backed registry for workspace save/restore.
    pub state_registry: Option<Arc<StateRegistry>>,
    /// Deferred cursor target from workspace restore — applied after focus graph rebuild.
    pub(crate) restore_cursor_target: Option<crate::deck::CursorTarget>,
}

impl InstrumentApp {
    /// Create a new app. Starts at the Field (root level).
    pub fn new(store: Store, all_entries: Vec<FieldEntry>, registry: Option<Arc<StateRegistry>>) -> Self {
        let engine = Engine::with_store(store);
        let session_id = engine.store().start_session().ok();
        let total_count = all_entries.len();
        let total_active = all_entries
            .iter()
            .filter(|e| e.status == TensionStatus::Active)
            .count();

        let search_index = sd_core::SearchIndex::build(&engine.store());

        // Resolve theme for detected terminal mode
        let theme = crate::theme::instrument_theme();
        let is_dark = ftui::Theme::detect_dark_mode();
        let resolved = theme.resolve(is_dark);
        let styles = crate::theme::InstrumentStyles::resolve(&resolved);

        let text_input = TextInput::new()
            .with_style(styles.text_bold)
            .with_cursor_style(ftui::style::Style::new().fg(styles.clr_cyan))
            .with_placeholder_style(styles.dim);

        let mut app = Self {
            engine,
            session_id,
            parent_id: None,
            parent_tension: None,
            parent_horizon_label: None,
            parent_desire_age: None,
            parent_reality_age: None,
            siblings: Vec::new(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            text_input,
            search_state: None,
            search_index,
            show_help: false,
            show_inspector: false,
            db_modified: None,
            breadcrumb_cache: Vec::new(),
            total_active,
            total_count,
            alerts: Vec::new(),
            alert_cursor: 0,
            reorder_original: Vec::new(),
            grandparent_display: None,
            parent_mutation_count: 0,
            db_path_cache: None,

            last_render_lines: std::cell::Cell::new(40),
            trajectory_mode: false,
            epoch_boundary: None,
            deck_config: {
                // Load deck config from workspace config.toml
                let config_path = std::env::current_dir()
                    .ok()
                    .map(|d| d.join(".werk").join("config.toml"))
                    .unwrap_or_default();
                let config = werk_shared::Config::load_from_path(&config_path).unwrap_or_default();
                crate::deck::DeckConfig::load(&config)
            },
            deck_zoom: crate::deck::ZoomLevel::Normal,
            focused_detail: None,
            focused_note: None,
            parent_notes: Vec::new(),
            pathway_state: None,
            frontier: crate::deck::Frontier::default(),
            route_expanded: false,
            held_expanded: false,
            accumulated_expanded: false,
            view_orientation: crate::state::ViewOrientation::Stream,
            survey_cursor: 0,
            survey_items: Vec::new(),
            field_vitals: crate::survey::FieldVitals::default(),
            pre_survey_state: None,
            survey_tree_state: crate::survey_tree::SurveyTreeState::new(),
            logbase_tension_id: None,
            logbase_tension: None,
            logbase_epochs: Vec::new(),
            logbase_events: Vec::new(),
            logbase_provenance: crate::logbase::LogbaseProvenance::default(),
            logbase_list_state: std::cell::RefCell::new(ftui::widgets::list::ListState::default()),
            logbase_focused_epoch: 0,
            logbase_items: Vec::new(),
            logbase_header: Vec::new(),
            logbase_separator: String::new(),
            pre_logbase_state: None,
            layout: {
                let mut ls = crate::layout::LayoutState::default();
                if let Ok((w, h)) = crossterm::terminal::size() {
                    ls.update_regime(w, h);
                }
                ls
            },
            focus_state: crate::focus::FocusState::new(),
            session_log: crate::session_log::SessionLog::new(),
            styles,
            toasts: crate::toast::ToastQueue::new(),
            gesture_history: crate::undo::GestureHistory::new(),
            pending_gesture: None,
            command_palette: crate::palette::build_palette(None), // rebuilt after feedback load
            palette_feedback: crate::palette::create_feedback_collector(),
            state_registry: registry,
            restore_cursor_target: None,
        };
        // Load feedback from persistence and rebuild palette with boosts
        if let Some(ref reg) = app.state_registry {
            let _ = reg.load();
            crate::persistence::load_feedback(reg, &mut app.palette_feedback);
            app.command_palette = crate::palette::build_palette(Some(&app.palette_feedback));
        }
        if let Some(ref sid) = app.session_id {
            app.session_log.set_store_session_id(sid.clone());
        }

        // Restore workspace state from persistence
        app.restore_workspace();

        app.load_siblings();
        app
    }

    /// Create an app in empty/welcome state.
    pub fn new_empty() -> Self {
        let engine = Engine::new_in_memory().expect("failed to create in-memory engine"); // ubs:ignore in-memory SQLite cannot fail

        // Resolve theme for detected terminal mode
        let theme = crate::theme::instrument_theme();
        let is_dark = ftui::Theme::detect_dark_mode();
        let resolved = theme.resolve(is_dark);
        let styles = crate::theme::InstrumentStyles::resolve(&resolved);

        let text_input = TextInput::new()
            .with_style(styles.text_bold)
            .with_cursor_style(ftui::style::Style::new().fg(styles.clr_cyan))
            .with_placeholder_style(styles.dim);

        Self {
            engine,
            session_id: None,
            parent_id: None,
            parent_tension: None,
            parent_horizon_label: None,
            parent_desire_age: None,
            parent_reality_age: None,
            siblings: Vec::new(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            text_input,
            search_state: None,
            search_index: None,
            show_help: false,
            show_inspector: false,
            db_modified: None,
            breadcrumb_cache: Vec::new(),
            total_active: 0,
            total_count: 0,
            alerts: Vec::new(),
            alert_cursor: 0,
            reorder_original: Vec::new(),
            grandparent_display: None,
            parent_mutation_count: 0,
            db_path_cache: None,

            last_render_lines: std::cell::Cell::new(40),
            trajectory_mode: false,
            epoch_boundary: None,
            deck_config: crate::deck::DeckConfig::default(),
            deck_zoom: crate::deck::ZoomLevel::Normal,
            focused_detail: None,
            focused_note: None,
            parent_notes: Vec::new(),
            pathway_state: None,
            frontier: crate::deck::Frontier::default(),
            route_expanded: false,
            held_expanded: false,
            accumulated_expanded: false,
            view_orientation: crate::state::ViewOrientation::Stream,
            survey_cursor: 0,
            survey_items: Vec::new(),
            field_vitals: crate::survey::FieldVitals::default(),
            pre_survey_state: None,
            survey_tree_state: crate::survey_tree::SurveyTreeState::new(),
            logbase_tension_id: None,
            logbase_tension: None,
            logbase_epochs: Vec::new(),
            logbase_events: Vec::new(),
            logbase_provenance: crate::logbase::LogbaseProvenance::default(),
            logbase_list_state: std::cell::RefCell::new(ftui::widgets::list::ListState::default()),
            logbase_focused_epoch: 0,
            logbase_items: Vec::new(),
            logbase_header: Vec::new(),
            logbase_separator: String::new(),
            pre_logbase_state: None,
            layout: crate::layout::LayoutState::default(),
            focus_state: crate::focus::FocusState::new(),
            session_log: crate::session_log::SessionLog::new(),
            styles,
            toasts: crate::toast::ToastQueue::new(),
            gesture_history: crate::undo::GestureHistory::new(),
            pending_gesture: None,
            command_palette: crate::palette::build_palette(None),
            palette_feedback: crate::palette::create_feedback_collector(),
            state_registry: None,
            restore_cursor_target: None,
        }
    }

    /// Begin a gesture linked to this TUI's session.
    /// Captures a state snapshot for undo. Call `end_gesture()` when done.
    pub fn begin_gesture(&mut self, description: &str) {
        // Capture snapshot before the gesture mutates anything
        let snapshot = self.capture_snapshot();
        self.pending_gesture = Some((description.to_string(), snapshot));

        let sid = self.session_id.clone();
        if let Some(ref sid) = sid {
            let _ = self.engine.begin_gesture_in_session(sid, Some(description));
        } else {
            let _ = self.engine.begin_gesture(Some(description));
        }
    }

    /// End the current gesture and commit its snapshot to undo history.
    pub fn end_gesture(&mut self) {
        let _ = self.engine.end_gesture();
        if let Some((desc, snapshot)) = self.pending_gesture.take() {
            self.gesture_history.push(desc, snapshot);
        }
    }

    /// Capture the current TUI state as a snapshot (for undo).
    pub fn capture_snapshot(&self) -> crate::undo::StateSnapshot {
        crate::undo::StateSnapshot {
            parent_id: self.parent_id.clone(),
            siblings: self.siblings.clone(),
            focus_active: self.focus_state.active,
            deck_zoom: self.deck_zoom.clone(),
            route_expanded: self.route_expanded,
            held_expanded: self.held_expanded,
            accumulated_expanded: self.accumulated_expanded,
        }
    }

    /// Restore TUI state from a snapshot (for undo/redo).
    pub fn restore_snapshot(&mut self, snap: crate::undo::StateSnapshot) {
        self.parent_id = snap.parent_id;
        self.siblings = snap.siblings;
        self.focus_state.active = snap.focus_active;
        self.deck_zoom = snap.deck_zoom;
        self.route_expanded = snap.route_expanded;
        self.held_expanded = snap.held_expanded;
        self.accumulated_expanded = snap.accumulated_expanded;
        // Reload from DB to ensure consistency
        self.load_siblings();
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

            let now_year = chrono::Datelike::year(&now);
            self.parent_horizon_label = parent.horizon.as_ref()
                .map(|h| crate::glyphs::compact_horizon(h, now_year));

            self.parent_desire_age = Some(crate::glyphs::relative_time(last_desire, now));
            self.parent_reality_age = Some(crate::glyphs::relative_time(last_reality, now));
            // Cache grandparent display for deck breadcrumb
            self.grandparent_display = parent.parent_id.as_ref().and_then(|gp_id| {
                self.engine.store().get_tension(gp_id).ok().flatten().map(|gp| {
                    (werk_shared::display_id(gp.short_code, &gp.id), gp.desired.clone())
                })
            });

            // Cache mutation count for deck log indicator
            self.parent_mutation_count = mutations.len();

            // Extract parent notes for accumulated zone display
            self.parent_notes = mutations.iter()
                .filter(|m| m.field() == "note")
                .map(|m| {
                    crate::deck::AccumulatedItem::Note {
                        text: m.new_value().to_string(),
                        age: crate::glyphs::relative_time(m.timestamp(), now),
                        timestamp: m.timestamp(),
                    }
                })
                .collect();

            // V5: Compute epoch boundary — last epoch timestamp (lightweight query)
            self.epoch_boundary = self.engine.store()
                .get_last_epoch_timestamp(&parent.id)
                .ok()
                .flatten();
        } else {
            self.parent_horizon_label = None;
            self.parent_desire_age = None;
            self.parent_reality_age = None;
            self.grandparent_display = None;
            self.parent_mutation_count = 0;
            self.parent_notes = Vec::new();
            self.epoch_boundary = None;
        }

        // Sort: positioned DESC (from SQL), then unpositioned by horizon range_end.
        // All levels include all children for frontier classification —
        // resolved/released appear in accumulated zone as field evolution evidence.
        let mut filtered: Vec<_> = tensions.to_vec();

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

        // Batch queries: count children and get mutation timestamps
        let child_ids: Vec<&str> = filtered.iter().map(|t| t.id.as_str()).collect();
        let children_counts = self.engine.store()
            .count_children_by_parent(&child_ids)
            .unwrap_or_default();
        let last_reality_updates = self.engine.store()
            .get_last_mutation_timestamps(&child_ids, &["actual", "created"])
            .unwrap_or_default();
        let last_status_changes = self.engine.store()
            .get_last_mutation_timestamps(&child_ids, &["status"])
            .unwrap_or_default();

        self.siblings = filtered
            .iter()
            .map(|t| {
                let child_count = children_counts.get(&t.id).copied().unwrap_or(0);
                let last_reality_update = last_reality_updates
                    .get(&t.id)
                    .copied()
                    .unwrap_or(t.created_at);
                let last_status_change = last_status_changes
                    .get(&t.id)
                    .copied()
                    .unwrap_or(t.created_at);
                FieldEntry::from_tension(t, last_reality_update, child_count, last_status_change, now)
            })
            .collect();

        // Update totals (COUNT queries, not loading all rows)
        let (total, active) = self.engine.store().count_tensions().unwrap_or((0, 0));
        self.total_count = total;
        self.total_active = active;

        // Refresh breadcrumb cache
        self.breadcrumb_cache = self.breadcrumb();

        // Compute alerts
        self.compute_alerts();

        // Reset expansion overrides on data change
        self.route_expanded = false;
        self.held_expanded = false;
        self.accumulated_expanded = false;

        // Recompute cached frontier and rebuild focus graph
        self.recompute_frontier();
        // focus_state.clamp_active() is called within rebuild_for_frontier

        // Rebuild search index (fast: ~1.6ms for 150 docs)
        self.search_index = sd_core::SearchIndex::build(self.engine.store());

        // Refresh db_modified so the next Tick doesn't treat our own writes as external changes
        self.refresh_db_modified();
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
        self.session_log.record(crate::session_log::Category::Nav, format!("DESCEND into {}", id));
        self.parent_id = Some(id.to_string());
        self.load_siblings();
        self.deck_cursor_reset();
    }

    /// Ascend to parent level. Cursor lands on the tension we just left.
    pub fn ascend(&mut self) {
        self.session_log.record(crate::session_log::Category::Nav, format!("ASCEND from {:?}", self.parent_id));
        let old_parent_id = self.parent_id.take();

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
                self.deck_cursor_to_sibling(idx);
            }
        } else {
            self.deck_cursor_reset();
        }
    }

    /// The action target: uses deck cursor to resolve the selected sibling.
    pub fn action_target(&self) -> Option<&FieldEntry> {
        self.deck_selected_sibling_index()
            .and_then(|idx| self.siblings.get(idx))
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

    /// Create a tension with a horizon string (e.g. "2026-W13" or "2026-03-20").
    pub fn create_tension_with_horizon(&mut self, desired: &str, actual: &str, horizon_str: &str) {
        let parent = self.parent_id.clone();
        let horizon = crate::horizon::parse_horizon(horizon_str).ok();
        let has_horizon = horizon.is_some();

        let desc = format!("create tension '{}'", truncate(desired, 40));
        self.begin_gesture(&desc);
        let result = self.engine.create_tension_full(desired, actual, parent, horizon);
        self.end_gesture();

        if let Ok(tension) = result {
            self.set_transient(format!("created: {}", truncate(&tension.desired, 30)));
            self.load_siblings();
            if let Some(idx) = self.siblings.iter().position(|s| s.id == tension.id) {
                self.deck_cursor_to_sibling(idx);
            }
            if has_horizon && self.parent_id.is_some() {
                self.check_containment_palette(&tension.id);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Pathway palettes — structural signal detection and presentation
    // -----------------------------------------------------------------------

    /// Check for containment violations after a horizon change.
    /// If a signal is found, enters Pathway mode with the first palette.
    pub fn check_containment_palette(&mut self, tension_id: &str) {
        if let Ok(palettes) = werk_shared::palette::detect_containment_palettes(
            self.engine.store(),
            tension_id,
        ) {
            if let Some((palette, context)) = palettes.into_iter().next() {
                self.pathway_state = Some(crate::state::PathwayState {
                    palette,
                    context,
                    cursor: 0,
                });
                self.input_mode = InputMode::Pathway;
            }
        }
    }

    /// Check for sequencing pressure after a position change.
    /// If a signal is found, enters Pathway mode with the first palette.
    pub fn check_sequencing_palette(&mut self, tension_id: &str) {
        if let Ok(palettes) = werk_shared::palette::detect_sequencing_palettes(
            self.engine.store(),
            tension_id,
        ) {
            if let Some((palette, context)) = palettes.into_iter().next() {
                self.pathway_state = Some(crate::state::PathwayState {
                    palette,
                    context,
                    cursor: 0,
                });
                self.input_mode = InputMode::Pathway;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Frontier caching
    // -----------------------------------------------------------------------

    /// Recompute the cached frontier from current siblings and expansion lines.
    /// Called after load_siblings and whenever siblings are mutated.
    /// During reorder mode, uses from_raw_order to show items in array order
    /// instead of classifying by stale position fields.
    pub fn recompute_frontier(&mut self) {
        let mut frontier = if matches!(self.input_mode, InputMode::Reordering { .. }) {
            crate::deck::Frontier::from_raw_order(&self.siblings, self.epoch_boundary)
        } else {
            crate::deck::Frontier::compute(
                &self.siblings,
                self.trajectory_mode,
                self.epoch_boundary,
            )
        };
        // Root level: children don't display as accumulated
        if self.parent_id.is_none() {
            frontier.accumulated.clear();
        }
        // Inject parent notes into accumulated zone, interleaved by timestamp
        if !self.parent_notes.is_empty() {
            frontier.inject_notes(self.parent_notes.clone(), &self.siblings);
        }
        frontier.compute_expansion(self.last_render_lines.get());
        // Set desire/reality anchor selectability based on whether we're descended
        frontier.has_desire_anchor = self.parent_tension.is_some();
        frontier.has_reality_anchor = self.parent_tension.as_ref()
            .map(|p| !p.actual.is_empty())
            .unwrap_or(false);
        // Apply user-toggled expansion overrides
        if self.route_expanded {
            frontier.show_route = frontier.route.len();
        }
        if self.held_expanded {
            frontier.show_held = frontier.held.len();
        }
        if self.accumulated_expanded {
            frontier.show_accumulated = frontier.accumulated.len();
        }
        // Rebuild focus graph from the new frontier
        let has_desire = frontier.has_desire_anchor;
        let has_reality = frontier.has_reality_anchor;
        self.focus_state.rebuild_for_frontier(&frontier, has_desire, has_reality);
        // Apply deferred cursor target from workspace restore
        if let Some(target) = self.restore_cursor_target.take() {
            if let Some(fid) = self.focus_state.focus_for(&target) {
                self.focus_state.active = fid;
            }
        }
        self.frontier = frontier;
    }

    // -----------------------------------------------------------------------
    // Reordering — grab-and-drop interaction
    // -----------------------------------------------------------------------

    /// Enter reorder mode: grab the selected tension for repositioning.
    /// Shift+J/K enters this mode; j/k moves visually; Enter commits; Esc cancels.
    /// Only active tensions can be reordered (resolved/released cannot).
    /// Returns true if reorder mode was entered, false if blocked.
    pub fn enter_reorder(&mut self) -> bool {
        if self.siblings.is_empty() { return false; }

        // Determine the sibling index of the selected item
        let cursor = match self.deck_selected_sibling_index() {
            Some(idx) => idx,
            None => {
                self.set_transient("nothing to reorder here");
                return false;
            }
        };

        // Only active tensions can be reordered
        let entry = &self.siblings[cursor];
        if entry.status != TensionStatus::Active {
            self.set_transient("only active steps can be repositioned");
            return false;
        }

        let tension_id = self.siblings[cursor].id.clone();

        // Store original positions for cancel
        self.reorder_original = self.siblings.iter()
            .map(|s| (s.id.clone(), s.position))
            .collect();

        self.input_mode = InputMode::Reordering { tension_id: tension_id.clone() };

        // Telemetry: log entry with state snapshot
        use crate::session_log::Category;
        let positions: Vec<String> = self.siblings.iter()
            .filter(|s| s.status == TensionStatus::Active)
            .map(|s| format!("{}:{}", s.short_code.unwrap_or(-1),
                s.position.map(|p| p.to_string()).unwrap_or_else(|| "held".into())))
            .collect();
        self.session_log.record(Category::Reorder,
            format!("ENTER cursor={} id={} focus={} positions=[{}]",
                cursor, &tension_id, self.focus_state.active, positions.join(", ")));

        true
    }

    /// Find the sibling index of the grabbed tension during reorder.
    pub fn reorder_grabbed_index(&self) -> Option<usize> {
        if let InputMode::Reordering { ref tension_id } = self.input_mode {
            self.siblings.iter().position(|s| s.id == *tension_id)
        } else {
            None
        }
    }

    /// Move the grabbed tension up one position (toward desire).
    ///
    /// During reorder, the grabbed item is tracked by tension_id.
    /// We find its current position in the siblings array, swap with the
    /// nearest active neighbor above, and invalidate the frontier cache.
    pub fn reorder_move_up(&mut self) {
        let cursor = match self.reorder_grabbed_index() {
            Some(idx) => idx,
            None => return,
        };
        if cursor == 0 { return; }

        // Find the nearest active sibling above
        let mut target = cursor - 1;
        while target > 0 && self.siblings[target].status != TensionStatus::Active {
            target -= 1;
        }
        if self.siblings[target].status != TensionStatus::Active {
            return; // no active sibling above
        }

        self.siblings.swap(cursor, target);
        self.recompute_frontier();
    }

    /// Move the grabbed tension down one position (toward reality).
    ///
    /// During reorder, the grabbed item is tracked by tension_id.
    pub fn reorder_move_down(&mut self) {
        let cursor = match self.reorder_grabbed_index() {
            Some(idx) => idx,
            None => return,
        };
        if cursor >= self.siblings.len() - 1 { return; }

        // Find the nearest active sibling below
        let mut target = cursor + 1;
        while target < self.siblings.len() - 1 && self.siblings[target].status != TensionStatus::Active {
            target += 1;
        }
        if self.siblings[target].status != TensionStatus::Active {
            return; // no active sibling below
        }

        self.siblings.swap(cursor, target);
        self.recompute_frontier();
    }

    /// Commit the reorder: write final positions to engine as a single logical action.
    /// Derives positions from final array order. The boundary between positioned
    /// and held shifts if the grabbed item crossed it.
    pub fn reorder_commit(&mut self) {
        let tension_id = match &self.input_mode {
            InputMode::Reordering { tension_id } => tension_id.clone(),
            _ => String::new(),
        };

        // Telemetry: log commit with final array state
        use crate::session_log::Category;
        let final_order: Vec<String> = self.siblings.iter()
            .filter(|s| s.status == TensionStatus::Active)
            .map(|s| format!("{}:{}", s.short_code.unwrap_or(-1),
                s.position.map(|p| p.to_string()).unwrap_or_else(|| "held".into())))
            .collect();
        self.session_log.record(Category::Reorder,
            format!("COMMIT id={} final_order=[{}]",
                &tension_id, final_order.join(", ")));

        // Count how many ACTIVE items were originally positioned
        // (reorder_original includes all siblings — filter to active only)
        let active_ids: std::collections::HashSet<&str> = self.siblings.iter()
            .filter(|s| s.status == TensionStatus::Active)
            .map(|s| s.id.as_str())
            .collect();
        let originally_positioned = self.reorder_original.iter()
            .filter(|(id, pos)| pos.is_some() && active_ids.contains(id.as_str()))
            .count();

        // Was the grabbed tension originally positioned?
        let grabbed_was_positioned = self.reorder_original.iter()
            .any(|(id, pos)| id == &tension_id && pos.is_some());

        // Find where the grabbed tension ended up among active items
        let grabbed_active_index = self.siblings.iter()
            .filter(|s| s.status == TensionStatus::Active)
            .position(|s| s.id == tension_id)
            .unwrap_or(0);

        // Compute the boundary: how many items should be positioned in the result.
        let boundary = if grabbed_was_positioned && grabbed_active_index >= originally_positioned {
            originally_positioned.saturating_sub(1)
        } else if !grabbed_was_positioned && grabbed_active_index < originally_positioned {
            originally_positioned + 1
        } else {
            originally_positioned
        };

        // Assign positions to active items based on array order — one gesture for the batch.
        self.begin_gesture("reorder siblings");
        let mut active_idx = 0usize;
        for sibling in self.siblings.iter() {
            if sibling.status != TensionStatus::Active {
                continue;
            }
            if active_idx < boundary {
                let pos = (boundary - active_idx) as i32;
                let _ = self.engine.update_position(&sibling.id, Some(pos));
            } else {
                let _ = self.engine.update_position(&sibling.id, None);
            }
            active_idx += 1;
        }
        self.end_gesture();

        self.reorder_original.clear();
        self.input_mode = InputMode::Normal;
        self.load_siblings();

        // Restore cursor to the moved tension
        if let Some(idx) = self.siblings.iter().position(|s| s.id == tension_id) {
            self.deck_cursor_to_sibling(idx);
        }

        // Check for sequencing pressure after reorder
        if !tension_id.is_empty() {
            self.check_sequencing_palette(&tension_id);
        }
        if !matches!(self.input_mode, InputMode::Pathway) {
            self.set_transient("position updated");
        }
    }

    /// Cancel the reorder: restore original positions and cursor.
    pub fn reorder_cancel(&mut self) {
        self.session_log.record(crate::session_log::Category::Reorder, "CANCEL");

        let tension_id = match &self.input_mode {
            InputMode::Reordering { tension_id } => tension_id.clone(),
            _ => String::new(),
        };

        self.begin_gesture("cancel reorder");
        for (id, pos) in &self.reorder_original {
            let _ = self.engine.update_position(id, *pos);
        }
        self.end_gesture();
        self.reorder_original.clear();
        self.input_mode = InputMode::Normal;
        self.load_siblings();

        if let Some(idx) = self.siblings.iter().position(|s| s.id == tension_id) {
            self.deck_cursor_to_sibling(idx);
        }
    }


    /// Build tension context for clipboard handoff.
    pub fn build_clipboard_context(&mut self, tension_id: &str) -> String {
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


    /// Check if the database file has been modified since last check.
    /// Returns true if data should be reloaded.
    pub fn db_has_changed(&mut self) -> bool {
        // Cache the db path on first call to avoid walking the filesystem every tick
        if self.db_path_cache.is_none() {
            self.db_path_cache = std::env::current_dir()
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
        }

        if let Some(ref path) = self.db_path_cache {
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

    /// Refresh the cached db_modified timestamp to the current DB file mtime.
    /// Called after TUI-initiated writes so the next Tick doesn't mistake our
    /// own mutations for external changes.
    fn refresh_db_modified(&mut self) {
        // Ensure db_path_cache is populated (may not be if called before first tick)
        if self.db_path_cache.is_none() {
            self.db_path_cache = std::env::current_dir()
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
        }
        if let Some(ref path) = self.db_path_cache {
            if let Ok(meta) = std::fs::metadata(path) {
                if let Ok(modified) = meta.modified() {
                    self.db_modified = Some(modified);
                }
            }
        }
    }

    /// Global undo/redo: find the most recent undoable mutation across all
    /// visible tensions + parent, and apply its old_value.
    /// Both undo and redo use the same mechanics (toggle behavior).
    pub fn global_undo_redo(&mut self, is_redo: bool) {
        use sd_core::TensionStatus;

        // Collect all tension IDs in scope: parent + all siblings
        let mut candidates: Vec<String> = self.siblings.iter().map(|s| s.id.clone()).collect();
        if let Some(ref pid) = self.parent_id {
            candidates.push(pid.clone());
        }

        // Find the most recent undoable mutation across all candidates
        let mut best: Option<(chrono::DateTime<chrono::Utc>, String, String, String)> = None; // (timestamp, tension_id, field, old_value)

        for id in &candidates {
            let mutations = self.engine.store().get_mutations(id).unwrap_or_default();
            for m in mutations.iter().rev() {
                let field = m.field();
                let old = m.old_value().map(|v| v.to_string()).unwrap_or_default();
                match field {
                    // For desired/actual, skip if old_value is empty (would clear the text)
                    "desired" | "actual" if old.is_empty() => continue,
                    "desired" | "actual" | "status" | "horizon" => {}
                    "created" => continue, // creation is never undoable
                    _ => continue,
                }
                let ts = m.timestamp().to_owned();
                if best.as_ref().map(|(bt, _, _, _)| ts > *bt).unwrap_or(true) {
                    best = Some((ts, id.clone(), field.to_string(), old));
                }
                break; // only check most recent undoable per tension
            }
        }

        let label = if is_redo { "restored" } else { "reverted" };

        if let Some((_ts, tension_id, field, old_value)) = best {
            let display_id = self.siblings.iter()
                .find(|s| s.id == tension_id)
                .and_then(|s| s.short_code)
                .map(|sc| format!("#{}", sc))
                .or_else(|| {
                    self.parent_tension.as_ref()
                        .filter(|p| p.id == tension_id)
                        .and_then(|p| p.short_code)
                        .map(|sc| format!("#{}", sc))
                })
                .unwrap_or_else(|| tension_id[..8].to_string());

            let gesture_desc = format!("{} {} {}", label, field, display_id);
            self.begin_gesture(&gesture_desc);

            match field.as_str() {
                "desired" => {
                    let _ = self.engine.update_desired(&tension_id, &old_value);
                    self.set_transient(format!("{} desire {}", display_id, label));
                }
                "actual" => {
                    let _ = self.engine.update_actual(&tension_id, &old_value);
                    self.set_transient(format!("{} reality {}", display_id, label));
                }
                "status" => {
                    let status = match old_value.as_str() {
                        "Active" => TensionStatus::Active,
                        "Resolved" => TensionStatus::Resolved,
                        "Released" => TensionStatus::Released,
                        _ => TensionStatus::Active,
                    };
                    let _ = self.engine.store().update_status(&tension_id, status);
                    self.set_transient(format!("{} status {}", display_id, label));
                }
                "horizon" => {
                    if old_value.is_empty() {
                        let _ = self.engine.update_horizon(&tension_id, None);
                    } else if let Ok(h) = crate::horizon::parse_horizon(&old_value) {
                        let _ = self.engine.update_horizon(&tension_id, Some(h));
                    }
                    self.set_transient(format!("{} horizon {}", display_id, label));
                }
                _ => {}
            }
            self.end_gesture();
            self.load_siblings();
        } else {
            self.set_transient(if is_redo { "nothing to redo" } else { "nothing to undo" });
        }
    }

    /// Push a toast notification (replaces old TransientMessage).
    pub fn set_transient(&mut self, text: impl Into<String>) {
        self.toasts.push_info(&text.into());
    }

    /// Dump the session log to .werk/session.log.
    pub fn dump_session_log(&mut self) {
        self.session_log.record(crate::session_log::Category::Session, "log dumped by user");
        match self.session_log.dump_to_file() {
            Ok(path) => self.set_transient(format!("log \u{2192} {}", path.display())),
            Err(e) => self.set_transient(format!("log dump failed: {}", e)),
        }
    }

    /// Save workspace state and palette feedback to the persistence registry.
    /// Called on every quit path so the practitioner returns to their reasoning surface.
    pub fn save_workspace(&self) {
        let Some(ref registry) = self.state_registry else { return };
        // Save palette feedback boosts
        crate::persistence::save_feedback(registry, &self.palette_feedback);
        let collapsed: Vec<crate::persistence::PersistedTimeBand> = self
            .survey_tree_state
            .collapsed_bands()
            .iter()
            .map(|b| crate::persistence::PersistedTimeBand::from(*b))
            .collect();
        let state = crate::persistence::WorkspaceState {
            parent_id: self.parent_id.clone(),
            cursor_target: self.focus_state.cursor_target().into(),
            view_orientation: self.view_orientation.into(),
            deck_zoom: self.deck_zoom.into(),
            route_expanded: self.route_expanded,
            held_expanded: self.held_expanded,
            accumulated_expanded: self.accumulated_expanded,
            collapsed_bands: collapsed,
        };
        crate::persistence::save_workspace(registry, &state);
    }

    /// Restore workspace state from the persistence registry.
    /// Called during construction, before load_siblings().
    fn restore_workspace(&mut self) {
        let Some(ref registry) = self.state_registry else { return };
        // Registry already loaded in constructor (for feedback). Just read.
        let Some(state) = crate::persistence::load_workspace(registry) else { return };

        self.parent_id = state.parent_id;
        self.view_orientation = state.view_orientation.into();
        self.deck_zoom = state.deck_zoom.into();
        self.route_expanded = state.route_expanded;
        self.held_expanded = state.held_expanded;
        self.accumulated_expanded = state.accumulated_expanded;

        // Restore collapsed bands
        for band in state.collapsed_bands {
            self.survey_tree_state.collapse(band.into());
        }

        // Focus target is restored after load_siblings() rebuilds the focus graph.
        // Store it temporarily so load_siblings can apply it.
        self.restore_cursor_target = Some(state.cursor_target.into());
    }

    /// Save workspace and quit — centralized quit path.
    pub fn save_and_quit(&self) -> ftui::Cmd<crate::msg::Msg> {
        self.save_workspace();
        ftui::Cmd::quit()
    }
}

impl Drop for InstrumentApp {
    fn drop(&mut self) {
        // End the store session (structural record)
        if let Some(ref sid) = self.session_id {
            let _ = self.engine.store().end_session(sid, None);
        }
        // Dump telemetry log (diagnostic record)
        self.session_log.record(crate::session_log::Category::Session,
            format!("session ended ({} events)", self.session_log.total_count()));
        let _ = self.session_log.dump_to_file();
    }
}
