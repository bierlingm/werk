//! The Operative Instrument application state.

use std::collections::HashMap;

use sd_core::{DynamicsEngine, Tension, TensionStatus};
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
    pub parent_phase: sd_core::CreativeCyclePhase,
    pub siblings: Vec<FieldEntry>,
    pub vlist: VirtualList,

    // Gaze
    pub gaze: Option<GazeState>,
    pub gaze_data: Option<GazeData>,
    pub full_gaze_data: Option<FullGazeData>,

    // Input
    pub input_mode: InputMode,
    pub input_buffer: String,

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
    pub fn new(engine: DynamicsEngine, all_entries: Vec<FieldEntry>) -> Self {
        let total_count = all_entries.len();
        let total_active = all_entries
            .iter()
            .filter(|e| e.status == TensionStatus::Active)
            .count();

        let mut app = Self {
            engine,
            parent_id: None,
            parent_tension: None,
            parent_phase: sd_core::CreativeCyclePhase::Germination,
            siblings: Vec::new(),
            vlist: VirtualList::new(0),
            gaze: None,
            gaze_data: None,
            full_gaze_data: None,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
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
            parent_phase: sd_core::CreativeCyclePhase::Germination,
            siblings: Vec::new(),
            vlist: VirtualList::new(0),
            gaze: None,
            gaze_data: None,
            full_gaze_data: None,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
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

        // Compute parent phase
        if let Some(ref pid) = self.parent_id {
            let cd = self.engine.compute_full_dynamics_for_tension(pid);
            self.parent_phase = cd
                .map(|c| c.phase.phase)
                .unwrap_or(sd_core::CreativeCyclePhase::Germination);
        }

        let now = chrono::Utc::now();
        let window = chrono::Duration::days(7);

        // Compute activity per tension
        let mut activity_map: HashMap<String, Vec<f64>> = HashMap::new();
        for t in &tensions {
            for m in self.engine.store().get_mutations(&t.id).unwrap_or_default() {
                if m.timestamp() >= now - window {
                    let bucket = (now - m.timestamp()).num_days().min(6) as usize;
                    activity_map
                        .entry(m.tension_id().to_string())
                        .or_insert_with(|| vec![0.0; 7])[6 - bucket] += 1.0;
                }
            }
        }

        self.siblings = tensions
            .iter()
            .filter(|t| match self.filter {
                Filter::Active => t.status == TensionStatus::Active,
                Filter::All => true,
            })
            .map(|t| {
                let computed = self.engine.compute_full_dynamics_for_tension(&t.id);
                let activity = activity_map.remove(&t.id).unwrap_or_default();
                let has_children = !self
                    .engine
                    .store()
                    .get_children(&t.id)
                    .unwrap_or_default()
                    .is_empty();
                FieldEntry::from_tension(t, &computed, activity, has_children)
            })
            .collect();

        // Rebuild vlist — preserve cursor position and gaze
        let old_cursor = self.vlist.cursor;
        let old_gaze = self.gaze.clone();
        let old_gaze_id = old_gaze.as_ref()
            .and_then(|g| {
                // Look up ID from the OLD siblings list before it was replaced
                // (siblings was already replaced above, so this won't work — use saved ID)
                None::<String>
            });

        // Save gazed tension ID before siblings are replaced (they already were above)
        // We need to find the old gaze ID from the NEW siblings if possible
        // Actually — the gaze index may be stale. Clone the gaze state and try to restore by ID.
        let saved_gaze_info: Option<(String, bool)> = self.gaze.as_ref().and_then(|g| {
            // The siblings were ALREADY rebuilt above, so g.index may be wrong.
            // But the gaze_data still has the tension info if present.
            self.gaze_data.as_ref().map(|gd| {
                // Find the ID from the old data — use the desired text as fallback
                // Actually, we stored the tension ID nowhere in GazeData.
                // Let's just use the old sibling list... which is gone.
                // Simplest fix: check if cursor is still valid and just keep it.
                (String::new(), g.full)
            })
        });

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
    }

    /// Descend into a tension's children.
    pub fn descend(&mut self, id: &str) {
        self.parent_id = Some(id.to_string());
        self.load_siblings();
        self.vlist.cursor = 0;
    }

    /// Ascend to parent level. Cursor lands on the tension we just left.
    pub fn ascend(&mut self) {
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
                let glyph = glyphs::status_glyph(t.status, {
                    let cd = self.engine.compute_full_dynamics_for_tension(&t.id);
                    cd.map(|c| c.phase.phase)
                        .unwrap_or(sd_core::CreativeCyclePhase::Germination)
                });
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
        let computed = self.engine.compute_full_dynamics_for_tension(id);

        // Children preview — collect IDs first to avoid borrow conflicts
        let children_tensions = self.engine.store().get_children(id).unwrap_or_default();
        let active_children: Vec<_> = children_tensions
            .iter()
            .filter(|c| c.status == TensionStatus::Active)
            .take(5)
            .cloned()
            .collect();

        let mut children: Vec<ChildPreview> = Vec::new();
        for c in &active_children {
            let cd = self.engine.compute_full_dynamics_for_tension(&c.id);
            let (phase, tendency) = cd
                .map(|d| (d.phase.phase, d.tendency.tendency))
                .unwrap_or((
                    sd_core::CreativeCyclePhase::Germination,
                    sd_core::StructuralTendency::Stagnant,
                ));
            children.push(ChildPreview {
                id: c.id.clone(),
                desired: c.desired.clone(),
                phase,
                tendency,
                status: c.status,
            });
        }

        let magnitude = computed.as_ref().and_then(|cd| cd.structural_tension.as_ref().map(|st| st.magnitude));
        let conflict = computed.as_ref().and_then(|cd| {
            cd.conflict.as_ref().map(|c| {
                match c.pattern {
                    sd_core::ConflictPattern::CompetingTensions => "competing tensions".to_string(),
                    sd_core::ConflictPattern::AsymmetricActivity => "asymmetric activity".to_string(),
                }
            })
        });
        let neglect = computed.as_ref().and_then(|cd| {
            cd.neglect.as_ref().map(|n| {
                match n.neglect_type {
                    sd_core::NeglectType::ParentNeglectsChildren => "children neglected".to_string(),
                    sd_core::NeglectType::ChildrenNeglected => "children neglected".to_string(),
                }
            })
        });
        let oscillation = computed.as_ref().and_then(|cd| {
            cd.oscillation.as_ref().map(|o| format!("{} reversals", o.reversals))
        });

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

        Some(GazeData {
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            horizon,
            children,
            magnitude,
            conflict,
            neglect,
            oscillation,
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

    /// Compute full gaze data (dynamics + history) for a tension.
    pub fn compute_full_gaze(&mut self, id: &str) -> Option<FullGazeData> {
        let computed = self.engine.compute_full_dynamics_for_tension(id)?;

        let phase = crate::glyphs::phase_word(computed.phase.phase).to_string();
        let tendency = crate::glyphs::tendency_word(computed.tendency.tendency).to_string();
        let magnitude = computed.structural_tension.as_ref().map(|st| st.magnitude);

        let orientation = computed.orientation.as_ref().map(|o| {
            match o.orientation {
                sd_core::Orientation::Creative => "creative".to_string(),
                sd_core::Orientation::ProblemSolving => "problem-solving".to_string(),
                sd_core::Orientation::ReactiveResponsive => "reactive".to_string(),
            }
        });
        let conflict = computed.conflict.as_ref().map(|c| {
            match c.pattern {
                sd_core::ConflictPattern::CompetingTensions => "competing tensions".to_string(),
                sd_core::ConflictPattern::AsymmetricActivity => "asymmetric activity".to_string(),
            }
        });
        let neglect = computed.neglect.as_ref().map(|n| {
            match n.neglect_type {
                sd_core::NeglectType::ParentNeglectsChildren |
                sd_core::NeglectType::ChildrenNeglected => "children neglected".to_string(),
            }
        });
        let oscillation = computed.oscillation.as_ref().map(|o| {
            format!("{} reversals, magnitude {:.2}", o.reversals, o.magnitude)
        });
        let resolution = computed.resolution.as_ref().map(|r| {
            let trend = match r.trend {
                sd_core::ResolutionTrend::Accelerating => "accelerating",
                sd_core::ResolutionTrend::Steady => "steady",
                sd_core::ResolutionTrend::Decelerating => "decelerating",
            };
            format!("{}, velocity {:.3}", trend, r.velocity)
        });
        let compensating_strategy = computed.compensating_strategy.as_ref().map(|cs| {
            match cs.strategy_type {
                sd_core::CompensatingStrategyType::TolerableConflict => "tolerable conflict".to_string(),
                sd_core::CompensatingStrategyType::ConflictManipulation => "conflict manipulation".to_string(),
                sd_core::CompensatingStrategyType::WillpowerManipulation => "willpower manipulation".to_string(),
            }
        });
        let assimilation = if computed.assimilation.depth != sd_core::AssimilationDepth::None {
            Some(match computed.assimilation.depth {
                sd_core::AssimilationDepth::Shallow => "shallow".to_string(),
                sd_core::AssimilationDepth::Deep => "deep".to_string(),
                sd_core::AssimilationDepth::None => unreachable!(),
            })
        } else {
            None
        };
        let horizon_drift = if computed.horizon_drift.change_count > 0 {
            Some(match computed.horizon_drift.drift_type {
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

        // History
        let mutations = self.engine.store().get_mutations(id).unwrap_or_default();
        let now = chrono::Utc::now();
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
            phase,
            tendency,
            magnitude,
            orientation,
            conflict,
            neglect,
            oscillation,
            resolution,
            compensating_strategy,
            assimilation,
            horizon_drift,
            history,
        })
    }

    /// Build agent context string for a tension.
    pub fn build_agent_context(&mut self, tension_id: &str) -> String {
        let short_id = &tension_id[..12.min(tension_id.len())];
        let mut ctx = String::new();
        if let Ok(Some(t)) = self.engine.store().get_tension(tension_id) {
            ctx.push_str(&format!("Tension ID: {}\n", tension_id));
            ctx.push_str(&format!("Desired: {}\n", t.desired));
            ctx.push_str(&format!("Reality: {}\n", t.actual));
            if let Some(ref h) = t.horizon {
                ctx.push_str(&format!("Horizon: {}\n", h));
            }
            ctx.push_str(&format!("Status: {}\n", t.status));

            // Add dynamics
            if let Some(cd) = self.engine.compute_full_dynamics_for_tension(tension_id) {
                ctx.push_str(&format!("Phase: {} | Tendency: {}\n",
                    crate::glyphs::phase_word(cd.phase.phase),
                    crate::glyphs::tendency_word(cd.tendency.tendency),
                ));
                if let Some(ref st) = cd.structural_tension {
                    ctx.push_str(&format!("Magnitude: {:.2}\n", st.magnitude));
                }
                if cd.conflict.is_some() { ctx.push_str("Conflict: present\n"); }
                if cd.neglect.is_some() { ctx.push_str("Neglect: detected\n"); }
            }

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

            if let Some(cd) = self.engine.compute_full_dynamics_for_tension(tension_id) {
                ctx.push_str(&format!("Phase: {} | Tendency: {}\n",
                    crate::glyphs::phase_word(cd.phase.phase),
                    crate::glyphs::tendency_word(cd.tendency.tendency),
                ));
                if let Some(ref st) = cd.structural_tension {
                    ctx.push_str(&format!("Magnitude: {:.2}\n", st.magnitude));
                }
                if cd.conflict.is_some() { ctx.push_str("Conflict: present\n"); }
                if cd.neglect.is_some() { ctx.push_str("Neglect: detected\n"); }
            }

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
        let base = 1 + 2 + 2;
        let children = self.gaze_data.as_ref()
            .map(|g| if g.children.is_empty() { 0 } else { g.children.len() + 1 })
            .unwrap_or(0);
        let extras = self.gaze_data.as_ref()
            .map(|g| {
                let mut n = 0;
                if g.magnitude.is_some() { n += 2; }
                if g.conflict.is_some() { n += 1; }
                if g.neglect.is_some() { n += 1; }
                if g.oscillation.is_some() { n += 1; }
                n
            })
            .unwrap_or(0);
        base + children + extras
    }

    fn full_gaze_height_for_refresh(&self) -> usize {
        self.quick_gaze_height_for_refresh() + 10 + self.full_gaze_data.as_ref()
            .map(|d| d.history.len().min(15) + 2)
            .unwrap_or(0)
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
