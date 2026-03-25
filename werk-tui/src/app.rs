//! The Operative Instrument application state.

use ftui::widgets::input::TextInput;
use sd_core::{Engine, Store, Tension, TensionStatus};
use werk_shared::truncate;

use crate::glyphs;
use crate::state::*;

/// The main application struct.
pub struct InstrumentApp {
    pub engine: Engine,

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

    // Input
    pub input_mode: InputMode,
    pub input_buffer: String,
    /// TextInput widget for inline editing (edit mode only).
    pub text_input: TextInput,

    // Search
    pub search_state: Option<crate::search::SearchState>,

    // Chrome
    pub transient: Option<TransientMessage>,
    pub show_help: bool,

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

    // Deck cursor — V2: tracks position in the frontier's flat selectable list
    pub deck_cursor: crate::deck::DeckCursor,

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

    // Focus zoom (V7): detail of the currently focused child.
    pub deck_zoom: crate::deck::ZoomLevel,
    pub focused_detail: Option<crate::deck::FocusedDetail>,

    // Pathway palette state — active when InputMode::Pathway.
    pub pathway_state: Option<crate::state::PathwayState>,

    // Cached frontier — computed once per frame, shared between render and navigation.
    pub cached_frontier: Option<crate::deck::Frontier>,

    // Session telemetry — records every significant action for debugging.
    pub session_log: crate::session_log::SessionLog,
}

impl InstrumentApp {
    /// Create a new app. Starts at the Field (root level).
    pub fn new(store: Store, all_entries: Vec<FieldEntry>) -> Self {
        let engine = Engine::with_store(store);
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
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            text_input: TextInput::new()
                .with_style(crate::theme::STYLES.text_bold)
                .with_cursor_style(ftui::style::Style::new().fg(crate::theme::CLR_CYAN))
                .with_placeholder_style(crate::theme::STYLES.dim),
            search_state: None,
            transient: None,
            show_help: false,
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
            deck_cursor: crate::deck::DeckCursor::default(),
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
            pathway_state: None,
            cached_frontier: None,
            session_log: crate::session_log::SessionLog::new(),
        };
        app.load_siblings();
        app
    }

    /// Create an app in empty/welcome state.
    pub fn new_empty() -> Self {
        let engine = Engine::new_in_memory().expect("failed to create in-memory engine"); // ubs:ignore in-memory SQLite cannot fail
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
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            text_input: TextInput::new()
                .with_style(crate::theme::STYLES.text_bold)
                .with_cursor_style(ftui::style::Style::new().fg(crate::theme::CLR_CYAN))
                .with_placeholder_style(crate::theme::STYLES.dim),
            search_state: None,
            transient: None,
            show_help: false,
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
            deck_cursor: crate::deck::DeckCursor::default(),
            last_render_lines: std::cell::Cell::new(40),
            trajectory_mode: false,
            epoch_boundary: None,
            deck_config: crate::deck::DeckConfig::default(),
            deck_zoom: crate::deck::ZoomLevel::Normal,
            focused_detail: None,
            pathway_state: None,
            cached_frontier: None,
            session_log: crate::session_log::SessionLog::new(),
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
            // Cache grandparent display for deck breadcrumb
            self.grandparent_display = parent.parent_id.as_ref().and_then(|gp_id| {
                self.engine.store().get_tension(gp_id).ok().flatten().map(|gp| {
                    (werk_shared::display_id(gp.short_code, &gp.id), gp.desired.clone())
                })
            });

            // Cache mutation count for deck log indicator
            self.parent_mutation_count = mutations.len();

            // V5: Compute epoch boundary — last epoch timestamp (lightweight query)
            self.epoch_boundary = self.engine.store()
                .get_last_epoch_timestamp(&parent.id)
                .ok()
                .flatten();
        } else {
            self.parent_temporal_indicator = String::new();
            self.parent_temporal_urgency = 0.0;
            self.parent_horizon_label = None;
            self.parent_desire_age = None;
            self.parent_reality_age = None;
            self.grandparent_display = None;
            self.parent_mutation_count = 0;
            self.epoch_boundary = None;
        }

        // Sort: positioned DESC (from SQL), then unpositioned by horizon range_end.
        // Descended views include all children for frontier classification.
        // Root level shows only active (resolved/released root tensions are historical).
        let mut filtered: Vec<_> = if self.parent_id.is_some() {
            tensions.to_vec()
        } else {
            tensions.iter()
                .filter(|t| t.status == TensionStatus::Active)
                .cloned()
                .collect()
        };

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

        // Recompute cached frontier and clamp deck cursor
        self.recompute_frontier();
        let count = self.cached_frontier.as_ref().map(|f| f.selectable_count()).unwrap_or(0);
        self.deck_cursor.clamp(count);

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
    pub fn create_tension_with_horizon(&mut self, name: &str, desire: &str, reality: &str, horizon_str: &str) {
        let desired = if desire.is_empty() { name } else { desire };
        let parent = self.parent_id.clone();

        // Try to parse horizon (supports natural language like "tomorrow", "2w", "eom")
        let horizon = crate::horizon::parse_horizon(horizon_str).ok();

        let has_horizon = horizon.is_some();
        let result = self.engine.create_tension_full(desired, reality, parent, horizon);

        if let Ok(tension) = result {
            self.set_transient(format!("created: {}", truncate(&tension.desired, 30)));
            self.load_siblings();
            if let Some(idx) = self.siblings.iter().position(|s| s.id == tension.id) {
                self.deck_cursor_to_sibling(idx);
            }
            // Check for containment violation if created with a horizon under a parent
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
        frontier.compute_expansion(self.last_render_lines.get());
        self.cached_frontier = Some(frontier);
    }

    /// Get the cached frontier, recomputing if invalidated.
    pub fn ensure_frontier(&mut self) -> &crate::deck::Frontier {
        if self.cached_frontier.is_none() {
            self.recompute_frontier();
        }
        self.cached_frontier.as_ref().unwrap()
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
            format!("ENTER cursor={} id={} deck_cursor={} positions=[{}]",
                cursor, &tension_id, self.deck_cursor.index, positions.join(", ")));

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
        self.cached_frontier = None; // invalidate — siblings mutated
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
        self.cached_frontier = None; // invalidate — siblings mutated
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

        // Assign positions to active items based on array order.
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

        for (id, pos) in &self.reorder_original {
            let _ = self.engine.update_position(id, *pos);
        }
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
                let old = match m.old_value() {
                    Some(v) => v.to_string(),
                    None => continue,
                };
                match field {
                    "desired" | "actual" | "status" | "horizon" => {}
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
            self.load_siblings();
        } else {
            self.set_transient(if is_redo { "nothing to redo" } else { "nothing to undo" });
        }
    }

    /// Set a transient message on the lever.
    #[allow(dead_code)]
    pub fn set_transient(&mut self, text: impl Into<String>) {
        self.transient = Some(TransientMessage::new(text));
    }

    /// Dump the session log to .werk/session.log.
    pub fn dump_session_log(&mut self) {
        self.session_log.record(crate::session_log::Category::Session, "log dumped by user");
        match self.session_log.dump_to_file() {
            Ok(path) => self.set_transient(format!("log \u{2192} {}", path.display())),
            Err(e) => self.set_transient(format!("log dump failed: {}", e)),
        }
    }
}

impl Drop for InstrumentApp {
    fn drop(&mut self) {
        self.session_log.record(crate::session_log::Category::Session,
            format!("session ended ({} events)", self.session_log.total_count()));
        let _ = self.session_log.dump_to_file();
    }
}
