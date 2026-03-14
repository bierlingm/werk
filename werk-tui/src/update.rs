use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;

use ftui::{Cmd, Event, Frame, KeyCode, KeyEvent, KeyEventKind, Modifiers, Model};
use ftui::layout::{Constraint, Flex, Rect};
use ftui::runtime::{Every, Subscription};

use sd_core::{
    compute_urgency, ComputedDynamics, Forest, Mutation, TensionStatus,
};
use werk_shared::{relative_time, truncate, Config, StructuredResponse, Workspace};

use crate::agent::execute_agent_capture;
use crate::horizon::parse_horizon;
use crate::app::WerkApp;
use crate::helpers::{
    build_detail_dynamics, build_tension_row, build_tension_row_from_computed,
    compute_tier, format_horizon, movement_char, phase_char, phase_name,
};
use ftui::widgets::command_palette::PaletteAction;
use ftui::widgets::textarea::TextArea;
use crate::input::{
    ConfirmAction, InputContext, InputMode, InputOverlay,
    MovePickerState, View,
};
use crate::msg::Msg;
use crate::types::{
    MutationDisplay, MutationKind, Toast, ToastSeverity, TreeItem, UrgencyTier,
    MAX_VISIBLE_TOASTS, URGENCY_ALERT_THRESHOLD,
};

impl WerkApp {
    /// Load detail data for a given tension ID.
    pub(crate) fn load_detail(&mut self, tension_id: &str) {
        let now = Utc::now();

        let tension = match self.engine.store().get_tension(tension_id) {
            Ok(Some(t)) => t,
            _ => return,
        };

        let computed = self.engine.compute_full_dynamics_for_tension(tension_id);

        let mutations = self.engine.store().get_mutations(tension_id).unwrap_or_default();
        let mut mutation_displays: Vec<MutationDisplay> = mutations
            .iter()
            .rev()
            .take(10)
            .map(|m| {
                let field = m.field().to_string();
                let kind = match field.as_str() {
                    "created" => MutationKind::Created,
                    "status" => MutationKind::StatusChange,
                    "parent_id" => MutationKind::ParentChange,
                    "horizon" => MutationKind::HorizonChange,
                    "note" => MutationKind::Note,
                    _ => MutationKind::FieldUpdate,
                };

                let new_value = m.new_value().to_string();
                let old_value = m.old_value().map(|s| s.to_string());

                // Resolve human-readable labels for parent references
                let resolved_label = if kind == MutationKind::ParentChange {
                    self.engine
                        .store()
                        .get_tension(&new_value)
                        .ok()
                        .flatten()
                        .map(|t| truncate(&t.desired, 40).to_string())
                } else {
                    None
                };

                // Format horizon values as human-readable dates
                let (old_value, new_value) = if kind == MutationKind::HorizonChange {
                    let fmt = |v: &str| -> String {
                        // Try to parse as ISO date and format more readably
                        chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d")
                            .map(|d| d.format("%b %d, %Y").to_string())
                            .unwrap_or_else(|_| v.to_string())
                    };
                    (old_value.map(|o| fmt(&o)), fmt(&new_value))
                } else {
                    (old_value, new_value)
                };

                MutationDisplay {
                    relative_time: relative_time(m.timestamp(), now),
                    field,
                    kind,
                    old_value,
                    new_value,
                    resolved_label,
                }
            })
            .collect();
        mutation_displays.reverse();

        let all_tensions = self.engine.store().list_tensions().unwrap_or_default();
        let children: Vec<_> = all_tensions
            .iter()
            .filter(|t| t.parent_id.as_deref() == Some(tension_id))
            .map(|t| build_tension_row(&mut self.engine, t, now))
            .collect();

        let detail_dynamics = computed.map(|cd| {
            let mut dd = build_detail_dynamics(&cd);
            // Build forecast from resolution data
            dd.forecast_line = if let Some(ref resolution) = cd.resolution {
                if let (Some(required_vel), Some(is_sufficient)) = (resolution.required_velocity, resolution.is_sufficient) {
                    let ratio = if required_vel > 0.0 { resolution.velocity / required_vel } else { 0.0 };
                    if is_sufficient {
                        let magnitude = cd.structural_tension.as_ref().map(|st| st.magnitude).unwrap_or(0.0);
                        if resolution.velocity > 0.0 && magnitude > 0.0 {
                            let secs_remaining = (magnitude / resolution.velocity) as i64;
                            let forecast_date = chrono::Utc::now() + chrono::Duration::seconds(secs_remaining);
                            Some((
                                format!("On track \u{2014} resolving ~{}", forecast_date.format("%b %d")),
                                crate::theme::CLR_GREEN,
                            ))
                        } else {
                            None
                        }
                    } else {
                        Some((
                            format!("Behind \u{2014} {:.0}% of required pace", ratio * 100.0),
                            crate::theme::CLR_RED_SOFT,
                        ))
                    }
                } else {
                    None
                }
            } else {
                None
            };
            dd
        });

        // Load parent tension
        let parent = tension.parent_id.as_ref().and_then(|pid| {
            self.engine.store().get_tension(pid).ok().flatten()
        });

        // Build ancestor chain by walking up parent_id links
        let mut ancestors = Vec::new();
        let mut current_id = tension.parent_id.clone();
        let mut seen = std::collections::HashSet::new();
        while let Some(pid) = current_id {
            if !seen.insert(pid.clone()) { break; }  // cycle protection
            if let Ok(Some(parent_t)) = self.engine.store().get_tension(&pid) {
                ancestors.push((parent_t.id.clone(), parent_t.desired.clone()));
                current_id = parent_t.parent_id.clone();
            } else {
                break;
            }
        }
        ancestors.reverse();  // root-first order

        self.detail.tension = Some(tension);
        self.detail.scroll = 0;
        self.detail.cursor = 0;
        self.detail.mutations = mutation_displays;
        self.detail.children = children;
        self.detail.dynamics = detail_dynamics;
        self.detail.parent = parent;
        self.detail.ancestors = ancestors;
    }

    /// Build the flat list of navigable tensions for the neighborhood view.
    /// Order: parent, SELECTED, siblings, children.
    pub(crate) fn build_neighborhood_items(&mut self, center_id: &str) {
        let tensions = self.engine.store().list_tensions().unwrap_or_default();
        let forest = match Forest::from_tensions(tensions) {
            Ok(f) => f,
            Err(_) => {
                self.neighborhood_items.clear();
                return;
            }
        };

        let node = match forest.find(center_id) {
            Some(n) => n,
            None => {
                self.neighborhood_items.clear();
                return;
            }
        };

        let mut items: Vec<(String, String)> = Vec::new();

        // Parent
        if let Some(pid) = &node.tension.parent_id {
            if forest.find(pid).is_some() {
                items.push((pid.clone(), "Parent".to_string()));
            }
        }

        // Selected (always index after parent, or 0)
        let selected_idx = items.len();
        items.push((center_id.to_string(), "SELECTED".to_string()));

        // Siblings
        for sib in forest.siblings(center_id).unwrap_or_default() {
            items.push((sib.tension.id.clone(), "Sibling".to_string()));
        }

        // Children
        for child in forest.children(center_id).unwrap_or_default() {
            items.push((child.tension.id.clone(), "Child".to_string()));
        }

        self.neighborhood_items = items;
        self.neighborhood_state.borrow_mut().select(Some(selected_idx));
    }

    /// Build tree items from the store.
    pub(crate) fn build_tree_items(&mut self) {
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
        if self.tree_selected() >= self.tree_items.len() && !self.tree_items.is_empty() {
            self.set_tree_selected(self.tree_items.len() - 1);
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

impl WerkApp {
    fn handle_welcome_confirm(&mut self) -> Cmd<Msg> {
        let global = self.welcome_selected == 1;
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match Workspace::init(&cwd, global) {
            Ok(workspace) => {
                match workspace.open_store() {
                    Ok(store) => {
                        self.engine = sd_core::DynamicsEngine::with_store(store);
                        self.tensions = Vec::new();
                        self.active_view = View::Dashboard;
                        self.push_toast(
                            if global {
                                "Global workspace created at ~/.werk/".to_string()
                            } else {
                                "Workspace created at .werk/".to_string()
                            },
                            ToastSeverity::Info,
                        );
                    }
                    Err(e) => {
                        self.push_toast(format!("Error opening store: {}", e), ToastSeverity::Alert);
                    }
                }
            }
            Err(e) => {
                self.push_toast(format!("Error creating workspace: {}", e), ToastSeverity::Alert);
            }
        }
        Cmd::None
    }

    pub(crate) fn selected_tension_id(&self) -> Option<String> {
        match &self.active_view {
            View::Dashboard => {
                let visible = self.visible_tensions();
                visible.get(self.selected()).map(|r| r.id.clone())
            }
            View::Detail => self.detail.tension.as_ref().map(|t| t.id.clone()),
            View::TreeView => self
                .tree_items
                .get(self.tree_selected())
                .map(|i| i.tension_id.clone()),
            View::Agent(id) => Some(id.clone()),
            View::Focus => self.detail.tension.as_ref().map(|t| t.id.clone()),
            View::Neighborhood => {
                let cursor = self.neighborhood_state.borrow().selected().unwrap_or(0);
                self.neighborhood_items.get(cursor).map(|(id, _)| id.clone())
                    .or_else(|| self.neighborhood_tension_id.clone())
            }
            View::Timeline | View::DynamicsSummary | View::Welcome => None,
        }
    }

    /// When all children of a composite tension are resolved or released,
    /// auto-resolve the parent and recurse upward.
    fn check_parent_auto_resolution(&mut self, tension_id: &str) {
        let tension = self.engine.store().get_tension(tension_id).ok().flatten();
        let parent_id = tension.and_then(|t| t.parent_id.clone());

        if let Some(pid) = parent_id {
            let all_tensions = self.engine.store().list_tensions().unwrap_or_default();
            let children: Vec<_> = all_tensions
                .iter()
                .filter(|t| t.parent_id.as_deref() == Some(&pid))
                .collect();
            let all_done = !children.is_empty()
                && children.iter().all(|c| {
                    c.status == TensionStatus::Resolved
                        || c.status == TensionStatus::Released
                });
            if all_done {
                if self.engine.store().update_status(&pid, TensionStatus::Resolved).is_ok() {
                    if let Ok(Some(p)) = self.engine.store().get_tension(&pid) {
                        self.push_toast(
                            format!(
                                "All children done \u{2014} '{}' auto-resolved",
                                truncate(&p.desired, 30)
                            ),
                            ToastSeverity::Info,
                        );
                    }
                    // Recurse: parent resolution might cascade further
                    self.check_parent_auto_resolution(&pid);
                }
            }
        }
    }

    /// Compute a what-if preview of cascading effects for resolving/releasing a tension.
    fn compute_what_if_preview(
        &self,
        id: &str,
        tension: &sd_core::Tension,
        action: crate::app::WhatIfAction,
    ) -> crate::app::WhatIfPreview {
        // Find active children that would continue without this parent resolved
        let children = self.engine.store().get_children(id).unwrap_or_default();
        let active_children: Vec<String> = children
            .iter()
            .filter(|c| c.status == TensionStatus::Active)
            .map(|c| truncate(&c.desired, 40).to_string())
            .collect();
        let children_count = active_children.len();

        // Check if resolving/releasing would auto-resolve the parent
        let mut auto_resolved = Vec::new();
        if let Some(ref pid) = tension.parent_id {
            let all_tensions = self.engine.store().list_tensions().unwrap_or_default();
            let siblings: Vec<_> = all_tensions
                .iter()
                .filter(|t| t.parent_id.as_deref() == Some(pid))
                .collect();
            let all_would_be_done = !siblings.is_empty()
                && siblings.iter().all(|s| {
                    s.id == id
                        || s.status == TensionStatus::Resolved
                        || s.status == TensionStatus::Released
                });
            if all_would_be_done {
                if let Ok(Some(parent)) = self.engine.store().get_tension(pid) {
                    if parent.status == TensionStatus::Active {
                        auto_resolved.push(truncate(&parent.desired, 40).to_string());
                    }
                }
            }
        }

        crate::app::WhatIfPreview {
            tension_id: id.to_string(),
            tension_desired: tension.desired.clone(),
            action,
            orphaned_children: active_children,
            auto_resolved_parents: auto_resolved,
            children_count,
        }
    }

    pub(crate) fn push_toast(&mut self, message: String, severity: ToastSeverity) {
        self.toasts.push(Toast::new(message, severity));
        while self.toasts.len() > MAX_VISIBLE_TOASTS {
            self.toasts.remove(0);
        }
    }

    fn expire_toasts(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    fn process_dynamics_events(&mut self, computed: &ComputedDynamics, desired: &str) {
        for event in &computed.events {
            let (message, severity) = match event {
                sd_core::Event::OscillationDetected { .. } => (
                    format!("Oscillation detected: {}", truncate(desired, 30)),
                    ToastSeverity::Warning,
                ),
                sd_core::Event::ResolutionAchieved { .. } => (
                    format!("Resolution achieved: {}", truncate(desired, 30)),
                    ToastSeverity::Info,
                ),
                sd_core::Event::NeglectDetected { tension_ids, .. } => (
                    format!(
                        "{} tension{} neglected",
                        tension_ids.len(),
                        if tension_ids.len() == 1 { " is being" } else { "s are being" }
                    ),
                    ToastSeverity::Warning,
                ),
                sd_core::Event::UrgencyThresholdCrossed { crossed_above, .. } => {
                    if *crossed_above {
                        (
                            format!("{} is now urgent", truncate(desired, 30)),
                            ToastSeverity::Alert,
                        )
                    } else {
                        (
                            format!("{} no longer urgent", truncate(desired, 30)),
                            ToastSeverity::Info,
                        )
                    }
                }
                sd_core::Event::ConflictDetected { tension_ids, .. } => (
                    format!(
                        "Conflict between {} sibling tensions",
                        tension_ids.len()
                    ),
                    ToastSeverity::Warning,
                ),
                sd_core::Event::LifecycleTransition {
                    old_phase,
                    new_phase,
                    ..
                } => (
                    format!(
                        "Phase: {} \u{2192} {}",
                        phase_name(*old_phase),
                        phase_name(*new_phase)
                    ),
                    ToastSeverity::Info,
                ),
                sd_core::Event::HorizonDriftDetected { drift_type, .. } => (
                    format!("Horizon drifting: {:?}", drift_type),
                    ToastSeverity::Warning,
                ),
                sd_core::Event::CompensatingStrategyDetected {
                    strategy_type, ..
                } => (
                    format!("Compensating strategy: {:?}", strategy_type),
                    ToastSeverity::Info,
                ),
                sd_core::Event::OscillationResolved { .. } => (
                    format!("Oscillation resolved: {}", truncate(desired, 30)),
                    ToastSeverity::Info,
                ),
                sd_core::Event::NeglectResolved { .. } => (
                    format!("No longer neglected: {}", truncate(desired, 30)),
                    ToastSeverity::Info,
                ),
                sd_core::Event::ConflictResolved { .. } => (
                    "Conflict resolved".to_string(),
                    ToastSeverity::Info,
                ),
                _ => continue,
            };
            self.push_toast(message, severity);
        }
    }

    fn check_urgency_changes(&mut self) {
        let tensions = self.engine.store().list_tensions().unwrap_or_default();
        let now = Utc::now();
        let mut new_urgencies = HashMap::new();

        for tension in &tensions {
            if tension.status != TensionStatus::Active {
                continue;
            }
            if let Some(urgency) = compute_urgency(tension, now) {
                let was_above = self
                    .previous_urgencies
                    .get(&tension.id)
                    .map(|&u| u >= URGENCY_ALERT_THRESHOLD)
                    .unwrap_or(false);
                let is_above = urgency.value >= URGENCY_ALERT_THRESHOLD;

                if is_above && !was_above {
                    self.push_toast(
                        format!("{} is now urgent", truncate(&tension.desired, 30)),
                        ToastSeverity::Alert,
                    );
                }
                new_urgencies.insert(tension.id.clone(), urgency.value);
            }
        }

        self.previous_urgencies = new_urgencies;
    }

    pub(crate) fn reload_data(&mut self) {
        let now = Utc::now();
        let tensions = self.engine.store().list_tensions().unwrap_or_default();

        // Compute per-tension activity from mutations (7-day window)
        let window = chrono::Duration::days(7);
        let mut activity_map: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
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

        let mut rows: Vec<_> = Vec::with_capacity(tensions.len());
        for t in &tensions {
            let computed = self.engine.compute_full_dynamics_for_tension(&t.id);
            if let Some(ref cd) = computed {
                self.process_dynamics_events(cd, &t.desired);
            }
            let activity = activity_map.remove(&t.id).unwrap_or_default();
            rows.push(build_tension_row_from_computed(&computed, t, now, activity));
        }

        self.check_urgency_changes();
        rows.sort_by(|a, b| {
            a.tier.cmp(&b.tier).then_with(|| {
                let ua = a.urgency.unwrap_or(-1.0);
                let ub = b.urgency.unwrap_or(-1.0);
                ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        self.total_active = rows.iter().filter(|t| t.tier == UrgencyTier::Active).count();
        self.total_resolved = rows.iter().filter(|t| t.tier == UrgencyTier::Resolved).count();
        self.total_released = rows.iter().filter(|t| t.status == "Released").count();
        self.total_neglected = rows.iter().filter(|t| t.tier == UrgencyTier::Neglected).count();
        self.total_urgent = rows.iter().filter(|t| t.tier == UrgencyTier::Urgent).count();
        self.tensions = rows;

        // Projection caching: recompute every 5 minutes
        let should_recompute = self.last_projection_time
            .map(|t| (now - t).num_seconds() > 300)
            .unwrap_or(true);

        if should_recompute {
            let all_tensions = self.engine.store().list_tensions().unwrap_or_default();
            let all_mutations = self.engine.store().all_mutations().unwrap_or_default();
            let thresholds = sd_core::ProjectionThresholds::default();
            let projection = sd_core::project_field(&all_tensions, &all_mutations, &thresholds, now);

            // Populate trajectory on each TensionRow from 1-month projection
            for row in &mut self.tensions {
                if let Some((_, projs)) = projection.tension_projections.iter()
                    .find(|(id, _)| *id == row.id)
                {
                    // Use 1-month projection's trajectory (index 1)
                    if let Some(proj) = projs.get(1) {
                        row.trajectory = Some(proj.trajectory);
                    }
                }
            }

            self.field_projection = Some(projection);
            self.last_projection_time = Some(now);
        }

        let visible = self.visible_tensions().len();
        if visible > 0 && self.selected() >= visible {
            self.set_selected(visible - 1);
        }

        if self.active_view == View::Detail {
            if let Some(t) = &self.detail.tension {
                let id = t.id.clone();
                self.load_detail(&id);
            }
        }

        // Auto-load detail for the selected tension (used by split-pane dashboard)
        if self.active_view == View::Dashboard {
            let visible = self.visible_tensions();
            if let Some(row) = visible.get(self.selected()) {
                let id = row.id.clone();
                self.load_detail(&id);
            }
        }

        if self.active_view == View::TreeView {
            self.build_tree_items();
        }

        // Recompute the lever
        self.lever = crate::lever::compute_lever(&mut self.engine);
    }

    /// Spawn a background task that watches the database file for external modifications.
    /// Returns a `Cmd` that will produce `Msg::ExternalChange` when the file changes.
    pub(crate) fn setup_file_watcher(&self) -> Cmd<Msg> {
        let db_path = match self.engine.store().path() {
            Some(p) => p.to_path_buf(),
            None => return Cmd::None, // in-memory store, nothing to watch
        };

        // Watch the parent directory (.werk/) since some editors do atomic
        // writes (rename) which can confuse per-file watches.
        let watch_dir = match db_path.parent() {
            Some(d) => d.to_path_buf(),
            None => return Cmd::None,
        };
        let db_file_name = match db_path.file_name() {
            Some(n) => n.to_os_string(),
            None => return Cmd::None,
        };

        Cmd::task_named("file_watcher", move || {
            use notify::{Watcher, RecursiveMode, Event as NotifyEvent};
            use std::sync::mpsc;
            use std::time::Duration;

            let (tx, rx) = mpsc::channel();
            let target_name = db_file_name.clone();
            let mut watcher = match notify::recommended_watcher(
                move |res: Result<NotifyEvent, notify::Error>| {
                    if let Ok(event) = res {
                        // Only trigger on modifications/creates that touch the db file
                        if event.kind.is_modify() || event.kind.is_create() {
                            let dominated = event.paths.iter().any(|p| {
                                p.file_name().map_or(false, |n| n == target_name)
                            });
                            if dominated {
                                tx.send(()).ok();
                            }
                        }
                    }
                },
            ) {
                Ok(w) => w,
                Err(_) => return Msg::Noop,
            };

            if watcher.watch(&watch_dir, RecursiveMode::NonRecursive).is_err() {
                return Msg::Noop;
            }

            // Block until a change arrives
            if rx.recv().is_err() {
                return Msg::Noop;
            }

            // Debounce: wait 200ms then drain any queued notifications
            std::thread::sleep(Duration::from_millis(200));
            while rx.try_recv().is_ok() {}

            Msg::ExternalChange
        })
    }

    fn enter_text_input(&mut self, context: InputContext, prompt: String, prefill: String) {
        self.text_input_widget.set_value(&prefill);
        self.input_overlay = Some(InputOverlay::new(prompt, prefill));
        self.input_mode = InputMode::TextInput(context);
    }

    fn enter_confirm(&mut self, action: ConfirmAction, prompt: String) {
        self.input_overlay = Some(InputOverlay::new(prompt, String::new()));
        self.input_mode = InputMode::Confirm(action);
    }

    fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_overlay = None;
        self.text_input_widget.clear();
    }

    fn handle_text_input_key(&mut self, code: KeyCode) {
        if self.input_overlay.is_none() {
            return;
        }

        let event = Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers: Modifiers::NONE,
        });
        self.text_input_widget.handle_event(&event);

        // Sync widget value back to overlay buffer
        if let Some(ref mut overlay) = self.input_overlay {
            overlay.buffer = self.text_input_widget.value().to_string();
            overlay.cursor = overlay.buffer.len(); // approximate sync
        }
    }

    fn handle_submit(&mut self) -> Cmd<Msg> {
        let buffer = if self.input_overlay.is_some() {
            self.text_input_widget.value().to_string()
        } else {
            self.cancel_input();
            return Cmd::None;
        };

        let mode = std::mem::replace(&mut self.input_mode, InputMode::Normal);
        self.input_overlay = None;
        self.text_input_widget.clear();

        match mode {
            InputMode::TextInput(ctx) => self.dispatch_text_submit(ctx, buffer),
            InputMode::Confirm(action) => {
                self.input_mode = InputMode::Confirm(action);
                Cmd::None
            }
            InputMode::MovePicker(_) => {
                Cmd::None
            }
            InputMode::Normal => Cmd::None,
            InputMode::Reflect => Cmd::None,
        }
    }

    fn dispatch_text_submit(&mut self, ctx: InputContext, buffer: String) -> Cmd<Msg> {
        // Horizon steps allow empty input (skip = no horizon)
        let allows_empty = matches!(
            ctx,
            InputContext::CreateChildHorizon(..)
            | InputContext::CreateParentHorizon(..)
            | InputContext::SetRecurrence(..)
        );
        if buffer.trim().is_empty() && !allows_empty {
            self.status_toast = Some("Input cannot be empty".to_string());
            return Cmd::None;
        }

        match ctx {
            InputContext::UpdateReality(id) => {
                match self.engine.store().update_actual(&id, buffer.trim()) {
                    Ok(()) => {
                        self.status_toast = Some("Reality updated".to_string());
                        self.reload_data();
                    }
                    Err(e) => {
                        self.status_toast = Some(format!("Error: {}", e));
                    }
                }
            }
            InputContext::UpdateDesire(id) => {
                match self.engine.store().update_desired(&id, buffer.trim()) {
                    Ok(()) => {
                        self.status_toast = Some("Desire updated".to_string());
                        self.reload_data();
                    }
                    Err(e) => {
                        self.status_toast = Some(format!("Error: {}", e));
                    }
                }
            }
            InputContext::AddNote(id) => {
                let mutation = Mutation::new(
                    id,
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    buffer.trim().to_owned(),
                );
                match self.engine.store().record_mutation(&mutation) {
                    Ok(()) => {
                        self.status_toast = Some("Note added".to_string());
                        self.reload_data();
                    }
                    Err(e) => {
                        self.status_toast = Some(format!("Error: {}", e));
                    }
                }
            }
            InputContext::SetHorizon(id) => {
                let trimmed = buffer.trim();
                let horizon = if trimmed.eq_ignore_ascii_case("none") {
                    None
                } else {
                    match parse_horizon(trimmed) {
                        Ok(h) => Some(h),
                        Err(e) => {
                            self.status_toast = Some(format!(
                                "Invalid horizon: {}. Use: 2026, 2026-03, 2026-03-15",
                                e
                            ));
                            return Cmd::None;
                        }
                    }
                };
                match self.engine.store().update_horizon(&id, horizon) {
                    Ok(()) => {
                        self.status_toast = Some("Horizon updated".to_string());
                        self.reload_data();
                    }
                    Err(e) => {
                        self.status_toast = Some(format!("Error: {}", e));
                    }
                }
            }
            InputContext::AddTensionDesired { parent_id } => {
                let (desired, horizon_str, actual) = parse_quick_add(&buffer);
                let horizon_parsed = horizon_str.and_then(|h| {
                    match parse_horizon(&h) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            self.status_toast = Some(format!(
                                "Invalid horizon: {}. Use: 2026, 2026-03, +2w, etc.",
                                e
                            ));
                            None
                        }
                    }
                });
                let actual = actual.unwrap_or_default();
                match self
                    .engine
                    .store()
                    .create_tension_full(&desired, &actual, parent_id, horizon_parsed)
                {
                    Ok(t) => {
                        self.status_toast =
                            Some(format!("Created: {}", truncate(&t.desired, 40)));
                        self.reload_data();
                    }
                    Err(e) => {
                        self.status_toast = Some(format!("Error: {}", e));
                    }
                }
            }
            // ── Create Child flow ─────────────────────────────
            InputContext::CreateChildDesired(parent_id) => {
                let desired = buffer.trim().to_owned();
                // Show parent horizon as context if available
                let prompt = if let Ok(Some(parent)) = self.engine.store().get_tension(&parent_id) {
                    if let Some(h) = &parent.horizon {
                        format!("Horizon for child (parent horizon: {}):", h)
                    } else {
                        "Horizon for child (when? or empty to skip):".to_string()
                    }
                } else {
                    "Horizon for child (when? or empty to skip):".to_string()
                };
                self.enter_text_input(
                    InputContext::CreateChildHorizon(parent_id, desired),
                    prompt,
                    String::new(),
                );
            }
            InputContext::CreateChildHorizon(parent_id, desired) => {
                let trimmed = buffer.trim();
                let horizon = if trimmed.is_empty() {
                    // Inherit parent horizon when child has none
                    self.engine.store().get_tension(&parent_id)
                        .ok().flatten()
                        .and_then(|p| p.horizon.map(|h| h.to_string()))
                } else {
                    match parse_horizon(trimmed) {
                        Ok(h) => Some(h.to_string()),
                        Err(e) => {
                            self.status_toast = Some(format!(
                                "Invalid horizon: {}. Use: 2026, 2026-03, next month, etc.",
                                e
                            ));
                            return Cmd::None;
                        }
                    }
                };
                self.enter_text_input(
                    InputContext::CreateChildActual { parent_id, desired, horizon },
                    "Current reality for child:".to_string(),
                    String::new(),
                );
            }
            InputContext::CreateChildActual { parent_id, desired, horizon } => {
                let actual = buffer.trim().to_owned();
                let horizon_parsed = horizon.as_deref().and_then(|h| parse_horizon(h).ok());
                match self
                    .engine
                    .store()
                    .create_tension_full(&desired, &actual, Some(parent_id), horizon_parsed)
                {
                    Ok(t) => {
                        self.status_toast =
                            Some(format!("Created child: {}", truncate(&t.desired, 40)));
                        self.reload_data();
                    }
                    Err(e) => {
                        self.status_toast = Some(format!("Error: {}", e));
                    }
                }
            }

            // ── Create Parent flow ──────────────────────────────
            InputContext::CreateParentDesired(child_id) => {
                let desired = buffer.trim().to_owned();
                let prompt = if let Ok(Some(child)) = self.engine.store().get_tension(&child_id) {
                    if let Some(h) = &child.horizon {
                        format!("Horizon for parent (child horizon: {}, parent should be >=):", h)
                    } else {
                        "Horizon for parent (when? or empty to skip):".to_string()
                    }
                } else {
                    "Horizon for parent (when? or empty to skip):".to_string()
                };
                self.enter_text_input(
                    InputContext::CreateParentHorizon(child_id, desired),
                    prompt,
                    String::new(),
                );
            }
            InputContext::CreateParentHorizon(child_id, desired) => {
                let trimmed = buffer.trim();
                let horizon = if trimmed.is_empty() {
                    None
                } else {
                    match parse_horizon(trimmed) {
                        Ok(h) => Some(h.to_string()),
                        Err(e) => {
                            self.status_toast = Some(format!(
                                "Invalid horizon: {}. Use: 2026, 2026-03, next month, etc.",
                                e
                            ));
                            return Cmd::None;
                        }
                    }
                };
                self.enter_text_input(
                    InputContext::CreateParentActual { child_id, desired, horizon },
                    "Current reality for parent:".to_string(),
                    String::new(),
                );
            }
            InputContext::CreateParentActual { child_id, desired, horizon } => {
                let actual = buffer.trim().to_owned();
                let horizon_parsed = horizon.as_deref().and_then(|h| parse_horizon(h).ok());
                // Create the new parent tension (as a root initially)
                match self
                    .engine
                    .store()
                    .create_tension_full(&desired, &actual, None, horizon_parsed)
                {
                    Ok(new_parent) => {
                        // Reparent the child under the new parent
                        match self
                            .engine
                            .store()
                            .update_parent(&child_id, Some(&new_parent.id))
                        {
                            Ok(()) => {
                                self.status_toast = Some(format!(
                                    "Created parent: {} (child reparented)",
                                    truncate(&new_parent.desired, 30)
                                ));
                                self.reload_data();
                            }
                            Err(e) => {
                                self.status_toast = Some(format!(
                                    "Parent created but reparent failed: {}",
                                    e
                                ));
                                self.reload_data();
                            }
                        }
                    }
                    Err(e) => {
                        self.status_toast = Some(format!("Error: {}", e));
                    }
                }
            }

            InputContext::SetSnooze(tension_id) => {
                let input = buffer.trim().to_string();
                match parse_horizon(&input) {
                    Ok(horizon) => {
                        let date_str = horizon.to_string();
                        // For month/year precision, use range_start to get a concrete date.
                        let snooze_date = horizon.range_start().format("%Y-%m-%d").to_string();
                        let mutation = Mutation::new(
                            tension_id,
                            Utc::now(),
                            "snoozed_until".to_string(),
                            None,
                            snooze_date.clone(),
                        );
                        match self.engine.store().record_mutation(&mutation) {
                            Ok(()) => {
                                self.reload_data();
                                self.push_toast(
                                    format!("Snoozed until {}", date_str),
                                    ToastSeverity::Info,
                                );
                            }
                            Err(e) => {
                                self.status_toast = Some(format!("Error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.status_toast = Some(format!(
                            "Invalid date: {}. Use: +3d, friday, 2026-04-01, etc.",
                            e
                        ));
                    }
                }
            }

            InputContext::AgentPrompt(tension_id) => {
                let prompt = buffer.trim().to_owned();
                self.agent.running = true;
                self.agent.output = vec!["Running agent...".to_string()];
                self.agent.scroll = 0;
                self.agent.mutations = Vec::new();
                self.agent.mutation_selected = Vec::new();
                self.agent.mutation_cursor = 0;
                self.agent.response_text = None;

                return self.spawn_agent_task(tension_id, prompt);
            }

            InputContext::SetRecurrence(tension_id) => {
                let input = buffer.trim().to_string();
                if input.is_empty() {
                    // Clear recurrence
                    self.engine.store().record_mutation(&Mutation::new(
                        tension_id, Utc::now(), "recurrence".to_string(), None, "none".to_string(),
                    )).ok();
                    self.status_toast = Some("Recurrence cleared".to_string());
                } else {
                    // Validate it parses as a horizon
                    if crate::horizon::parse_horizon(&input).is_ok() {
                        self.engine.store().record_mutation(&Mutation::new(
                            tension_id, Utc::now(), "recurrence".to_string(), None, input,
                        )).ok();
                        self.status_toast = Some("Recurrence set".to_string());
                    } else {
                        self.status_toast = Some(format!(
                            "Invalid interval: use +1w, +2w, +1m, etc."
                        ));
                    }
                }
                self.reload_data();
            }

            // SetSnooze is handled above
        }
        Cmd::None
    }

    fn handle_recurrence(&mut self, tension_id: &str) {
        let mutations = self.engine.store().get_mutations(tension_id).unwrap_or_default();
        let recurrence = mutations.iter().rev()
            .find(|m| m.field() == "recurrence" && m.new_value() != "none")
            .map(|m| m.new_value().to_string());

        if let Some(interval) = recurrence {
            let tension = self.engine.store().get_tension(tension_id).ok().flatten();
            if let Some(t) = tension {
                let new_horizon = crate::horizon::parse_horizon(&interval).ok();
                let new_id = self.engine.store().create_tension_full(
                    &t.desired, "", t.parent_id.clone(), new_horizon,
                ).ok();
                if let Some(new_t) = new_id {
                    // Copy recurrence to new tension
                    self.engine.store().record_mutation(&Mutation::new(
                        new_t.id.clone(), Utc::now(), "recurrence".to_string(), None, interval.clone(),
                    )).ok();
                    self.toasts.push(Toast::new(
                        format!("Recurring: new '{}' created", truncate(&t.desired, 25)),
                        ToastSeverity::Info,
                    ));
                }
            }
        }
    }

    fn handle_confirm(&mut self, yes: bool) {
        if !yes {
            self.cancel_input();
            return;
        }

        let mode = std::mem::replace(&mut self.input_mode, InputMode::Normal);
        self.input_overlay = None;

        if let InputMode::Confirm(action) = mode {
            match action {
                ConfirmAction::Delete { id, desired: _ } => {
                    match self.engine.store().delete_tension(&id) {
                        Ok(()) => {
                            self.status_toast = Some("Tension deleted".to_string());
                            if self.active_view == View::Detail {
                                self.detail.tension = None;
                                self.detail.nav_stack.clear();
                                self.active_view = View::Dashboard;
                            }
                            self.reload_data();
                        }
                        Err(e) => {
                            self.status_toast = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
    }

    fn handle_move_picker_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let InputMode::MovePicker(ref state) = self.input_mode {
                    let count = state.candidates.len();
                    let current = self.move_picker_state.borrow().selected().unwrap_or(0);
                    if count > 0 && current < count - 1 {
                        self.move_picker_state.borrow_mut().select(Some(current + 1));
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let current = self.move_picker_state.borrow().selected().unwrap_or(0);
                if current > 0 {
                    self.move_picker_state.borrow_mut().select(Some(current - 1));
                }
            }
            KeyCode::Enter => {
                let selected_idx = self.move_picker_state.borrow().selected().unwrap_or(0);
                let mode = std::mem::replace(&mut self.input_mode, InputMode::Normal);
                self.input_overlay = None;
                *self.move_picker_state.borrow_mut() = ftui::widgets::list::ListState::default();
                if let InputMode::MovePicker(state) = mode {
                    if let Some((target_id, _)) = state.candidates.get(selected_idx) {
                        let new_parent = if target_id == "__ROOT__" {
                            None
                        } else {
                            Some(target_id.as_str())
                        };
                        match self
                            .engine
                            .store()
                            .update_parent(&state.tension_id, new_parent)
                        {
                            Ok(()) => {
                                self.status_toast = Some("Tension moved".to_string());
                                self.reload_data();
                            }
                            Err(e) => {
                                self.status_toast = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Escape => {
                self.cancel_input();
            }
            _ => {}
        }
    }

    fn build_move_candidates(&self, tension_id: &str) -> Vec<(String, String)> {
        let tensions = self.engine.store().list_tensions().unwrap_or_default();

        let mut descendants = std::collections::HashSet::new();
        let mut stack = vec![tension_id.to_owned()];
        while let Some(current) = stack.pop() {
            for t in &tensions {
                if t.parent_id.as_deref() == Some(&current) && !descendants.contains(&t.id) {
                    descendants.insert(t.id.clone());
                    stack.push(t.id.clone());
                }
            }
        }

        let mut candidates = vec![("__ROOT__".to_string(), "(root - no parent)".to_string())];
        for t in &tensions {
            if t.id != tension_id && !descendants.contains(&t.id) {
                let label = format!(
                    "{}  {}",
                    &t.id[..6.min(t.id.len())],
                    truncate(&t.desired, 50),
                );
                candidates.push((t.id.clone(), label));
            }
        }
        candidates
    }

    // ── Agent integration ────────────────────────────────────

    fn spawn_agent_task(&self, tension_id: String, prompt: String) -> Cmd<Msg> {
        let context_json = self.build_agent_context(&tension_id);
        let full_prompt = self.build_agent_prompt(&tension_id, &prompt, &context_json);

        let agent_cmd = match self.resolve_agent_cmd() {
            Ok(cmd) => cmd,
            Err(e) => {
                return Cmd::Msg(Msg::AgentResponseReceived(Err(e)));
            }
        };

        Cmd::task_named("agent", move || {
            let result = execute_agent_capture(&agent_cmd, &full_prompt);
            Msg::AgentResponseReceived(result)
        })
    }

    fn resolve_agent_cmd(&self) -> std::result::Result<String, String> {
        let workspace = Workspace::discover().map_err(|e| format!("workspace error: {}", e))?;
        let config =
            Config::load(&workspace).map_err(|e| format!("config error: {}", e))?;
        match config.get("agent.command") {
            Some(cmd) => Ok(cmd.clone()),
            None => Err(
                "No agent command configured. Use `werk config set agent.command <cmd>`"
                    .to_string(),
            ),
        }
    }

    fn build_agent_context(&self, tension_id: &str) -> String {
        let tension = match self.engine.store().get_tension(tension_id) {
            Ok(Some(t)) => t,
            _ => return "{}".to_string(),
        };

        let mut ctx = serde_json::Map::new();
        ctx.insert("id".to_string(), serde_json::Value::String(tension.id.clone()));
        ctx.insert(
            "desired".to_string(),
            serde_json::Value::String(tension.desired.clone()),
        );
        ctx.insert(
            "actual".to_string(),
            serde_json::Value::String(tension.actual.clone()),
        );
        ctx.insert(
            "status".to_string(),
            serde_json::Value::String(tension.status.to_string()),
        );
        if let Some(h) = &tension.horizon {
            ctx.insert("horizon".to_string(), serde_json::Value::String(h.to_string()));
        }
        if let Some(pid) = &tension.parent_id {
            ctx.insert("parent_id".to_string(), serde_json::Value::String(pid.clone()));
        }
        ctx.insert(
            "created_at".to_string(),
            serde_json::Value::String(tension.created_at.to_rfc3339()),
        );

        serde_json::Value::Object(ctx).to_string()
    }

    fn build_agent_prompt(&self, tension_id: &str, user_prompt: &str, context_json: &str) -> String {
        format!(
            "You are helping manage a structural tension.\n\n\
             Context:\n{}\n\n\
             User message: {}\n\n\
             IMPORTANT: Respond in YAML format with two sections:\n\
             1. 'mutations' array: suggested changes to the tension forest\n\
             2. 'response' string: your advice in prose\n\n\
             Supported mutation actions:\n\
             - update_actual: {{tension_id, new_value, reasoning}}\n\
             - create_child: {{parent_id, desired, actual, reasoning}}\n\
             - add_note: {{tension_id, text}}\n\
             - update_status: {{tension_id, new_status, reasoning}}\n\
             - update_desired: {{tension_id, new_value, reasoning}}\n\n\
             Only suggest mutations you're confident about. \
             If nothing should change, return empty mutations: [].\n\n\
             Wrap your YAML in --- markers. Example:\n\
             ---\n\
             mutations:\n\
               - action: update_actual\n\
                 tension_id: {tid}\n\
                 new_value: \"Updated state\"\n\
                 reasoning: \"Progress made\"\n\
             response: |\n\
               Your advice here.\n\
             ---\n\n\
             If you cannot produce YAML, respond in plain text.",
            context_json, user_prompt, tid = tension_id
        )
    }

    fn apply_agent_mutations(&mut self) -> usize {
        let mut applied = 0;
        let mutations: Vec<_> = self
            .agent.mutations
            .iter()
            .enumerate()
            .filter(|(i, _)| self.agent.mutation_selected.get(*i).copied().unwrap_or(false))
            .map(|(_, m)| m.clone())
            .collect();

        for mutation in &mutations {
            match self.apply_single_agent_mutation(mutation) {
                Ok(()) => applied += 1,
                Err(e) => {
                    self.push_toast(format!("Error: {}", e), ToastSeverity::Alert);
                }
            }
        }

        applied
    }

    fn apply_single_agent_mutation(
        &mut self,
        mutation: &werk_shared::AgentMutation,
    ) -> std::result::Result<(), String> {
        match mutation {
            werk_shared::AgentMutation::UpdateActual {
                tension_id,
                new_value,
                ..
            } => {
                self.engine
                    .store()
                    .update_actual(tension_id, new_value)
                    .map_err(|e| e.to_string())?;
            }
            werk_shared::AgentMutation::CreateChild {
                parent_id,
                desired,
                actual,
                ..
            } => {
                self.engine
                    .store()
                    .create_tension_full(desired, actual, Some(parent_id.clone()), {
                    // Inherit parent horizon if available
                    let parent_horizon = self.engine.store().get_tension(parent_id)
                        .ok().flatten()
                        .and_then(|p| p.horizon);
                    parent_horizon
                })
                    .map_err(|e| e.to_string())?;
            }
            werk_shared::AgentMutation::AddNote {
                tension_id, text, ..
            } => {
                self.engine
                    .store()
                    .record_mutation(&Mutation::new(
                        tension_id.clone(),
                        Utc::now(),
                        "note".to_owned(),
                        None,
                        text.clone(),
                    ))
                    .map_err(|e| e.to_string())?;
            }
            werk_shared::AgentMutation::UpdateStatus {
                tension_id,
                new_status,
                ..
            } => {
                let status = match new_status.to_lowercase().as_str() {
                    "resolved" => TensionStatus::Resolved,
                    "released" => TensionStatus::Released,
                    "active" => TensionStatus::Active,
                    other => {
                        return Err(format!(
                            "unknown status: '{}' (expected Active, Resolved, or Released)",
                            other
                        ));
                    }
                };
                self.engine
                    .store()
                    .update_status(tension_id, status)
                    .map_err(|e| e.to_string())?;
                if status == TensionStatus::Resolved || status == TensionStatus::Released {
                    self.check_parent_auto_resolution(tension_id);
                }
            }
            werk_shared::AgentMutation::UpdateDesired {
                tension_id,
                new_value,
                ..
            } => {
                self.engine
                    .store()
                    .update_desired(tension_id, new_value)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn normal_key_to_msg(&self, code: KeyCode, shift: bool) -> Msg {
        if matches!(self.active_view, View::Agent(_)) {
            return match code {
                KeyCode::Char('j') | KeyCode::Down => Msg::MoveDown,
                KeyCode::Char('k') | KeyCode::Up => Msg::MoveUp,
                KeyCode::Enter => Msg::AgentToggleMutation(self.agent.mutation_cursor),
                KeyCode::Char('a') => Msg::AgentApplySelected,
                KeyCode::Char('1') => Msg::AgentToggleMutation(0),
                KeyCode::Char('2') => Msg::AgentToggleMutation(1),
                KeyCode::Char('3') => Msg::AgentToggleMutation(2),
                KeyCode::Char('4') => Msg::AgentToggleMutation(3),
                KeyCode::Char('5') => Msg::AgentToggleMutation(4),
                KeyCode::Char('6') => Msg::AgentToggleMutation(5),
                KeyCode::Char('7') => Msg::AgentToggleMutation(6),
                KeyCode::Char('8') => Msg::AgentToggleMutation(7),
                KeyCode::Char('9') => Msg::AgentToggleMutation(8),
                KeyCode::Escape => Msg::Back,
                KeyCode::Char('q') => Msg::Quit,
                KeyCode::Char('?') => Msg::ToggleHelp,
                _ => Msg::Noop,
            };
        }

        match code {
            KeyCode::Char('j') | KeyCode::Down => Msg::MoveDown,
            KeyCode::Char('k') | KeyCode::Up => Msg::MoveUp,
            KeyCode::Char('?') => Msg::ToggleHelp,
            KeyCode::Char('q') => Msg::Quit,
            KeyCode::Enter => Msg::OpenDetail,
            KeyCode::Escape => Msg::Back,
            KeyCode::Tab | KeyCode::BackTab => {
                match self.active_view {
                    View::Dashboard => Msg::SwitchTree,
                    View::TreeView => Msg::SwitchDashboard,
                    _ => Msg::SwitchDashboard,
                }
            }
            KeyCode::Char('f') => Msg::CycleFilter,
            KeyCode::Char('r') => Msg::StartUpdateReality,
            KeyCode::Char('d') => Msg::StartUpdateDesire,
            KeyCode::Char('n') => Msg::StartAddNote,
            KeyCode::Char('h') => Msg::StartSetHorizon,
            KeyCode::Char('a') => Msg::StartAddTension,
            KeyCode::Char('c') => Msg::CreateChild,
            KeyCode::Char('p') => Msg::CreateParent,
            KeyCode::Char('R') if shift => Msg::StartResolve,
            KeyCode::Char('X') if shift => Msg::StartRelease,
            KeyCode::Char('m') => Msg::StartMove,
            KeyCode::Char('T') if shift && matches!(self.active_view, View::Dashboard | View::Detail | View::TreeView) => Msg::ToggleTimeline,
            KeyCode::Char('g') if self.active_view == View::Detail => Msg::StartAgent,
            KeyCode::Delete | KeyCode::Backspace
                if self.active_view == View::Detail =>
            {
                Msg::StartDelete
            }
            KeyCode::Char('D') if shift && matches!(self.active_view, View::Dashboard | View::Detail | View::TreeView) => Msg::ToggleHealthOverlay,
            KeyCode::Char('w') if matches!(self.active_view, View::Dashboard | View::Detail) => Msg::StartReflect,
            KeyCode::Char('Y') if shift && self.active_view == View::Detail => Msg::StartSetRecurrence,
            KeyCode::Char('u') => Msg::Undo,
            KeyCode::Char('z') => Msg::StartSnooze,
            KeyCode::Char('Z') if shift => Msg::ToggleShowSnoozed,
            KeyCode::Char('L') if shift => Msg::ShowLever,
            KeyCode::Char(':') => Msg::OpenCommandPalette,
            KeyCode::Char('/') => Msg::OpenSearch,
            KeyCode::Char('!') => Msg::TickerJump(0),
            KeyCode::Char('@') => Msg::TickerJump(1),
            KeyCode::Char('#') => Msg::TickerJump(2),
            _ => Msg::Noop,
        }
    }

    fn handle_reflect_key(&mut self, code: KeyCode, mods: Modifiers) -> Cmd<Msg> {
        match code {
            KeyCode::Escape => {
                self.reflect.textarea = None;
                self.reflect.tension_id = None;
                self.input_mode = InputMode::Normal;
            }
            _ => {
                if let Some(ref mut textarea) = self.reflect.textarea {
                    let alt = mods.contains(Modifiers::ALT);
                    let super_key = mods.contains(Modifiers::SUPER);

                    // macOS: Option+arrow = word nav, Cmd+arrow = line nav
                    // ftui TextArea uses Ctrl+arrow for word nav
                    let (mapped_code, mapped_mods) = if alt {
                        match code {
                            // Option+Left/Right → Ctrl+Left/Right (word nav)
                            KeyCode::Left | KeyCode::Right => {
                                let mut m = (mods - Modifiers::ALT) | Modifiers::CTRL;
                                m -= Modifiers::SHIFT; // keep shift if present
                                if mods.contains(Modifiers::SHIFT) { m |= Modifiers::SHIFT; }
                                (code, m)
                            }
                            // Option+Backspace → Ctrl+Backspace (word delete)
                            KeyCode::Backspace => (code, (mods - Modifiers::ALT) | Modifiers::CTRL),
                            _ => (code, mods),
                        }
                    } else if super_key {
                        match code {
                            // Cmd+Left → Home, Cmd+Right → End
                            KeyCode::Left => (KeyCode::Home, mods - Modifiers::SUPER),
                            KeyCode::Right => (KeyCode::End, mods - Modifiers::SUPER),
                            // Cmd+Up → top, Cmd+Down → bottom
                            KeyCode::Up => (KeyCode::Home, (mods - Modifiers::SUPER) | Modifiers::CTRL),
                            KeyCode::Down => (KeyCode::End, (mods - Modifiers::SUPER) | Modifiers::CTRL),
                            // Cmd+Backspace → Ctrl+K equivalent (delete line)
                            KeyCode::Backspace => (KeyCode::Char('k'), (mods - Modifiers::SUPER) | Modifiers::CTRL),
                            _ => (code, mods),
                        }
                    } else {
                        (code, mods)
                    };

                    let event = Event::Key(
                        KeyEvent::new(mapped_code).with_modifiers(mapped_mods),
                    );
                    textarea.handle_event(&event);
                }
            }
        }
        Cmd::None
    }
}

impl Model for WerkApp {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        self.setup_file_watcher()
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        self.expire_toasts();

        if !matches!(msg, Msg::Noop | Msg::Tick | Msg::DynamicsEvent(_, _)) {
            self.status_toast = None;
        }

        if let Msg::RawKey(code, mods) = msg {
            let shift = mods.contains(Modifiers::SHIFT);
            if self.active_view == View::Welcome {
                match code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.welcome_selected = 1;
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.welcome_selected = 0;
                    }
                    KeyCode::Enter => {
                        return self.handle_welcome_confirm();
                    }
                    KeyCode::Char('q') => return Cmd::Quit,
                    _ => {}
                }
                return Cmd::None;
            }

            if self.command_palette.is_visible() {
                let event = Event::Key(
                    KeyEvent::new(code).with_modifiers(mods),
                );
                if let Some(action) = self.command_palette.handle_event(&event) {
                    match action {
                        PaletteAction::Execute(id) => {
                            self.command_palette.close();
                            if let Some(msg) = Self::palette_id_to_msg(&id) {
                                return self.update(msg);
                            }
                        }
                        PaletteAction::Dismiss => {
                            self.command_palette.close();
                        }
                    }
                }
                return Cmd::None;
            }

            if self.show_lever_overlay {
                match code {
                    KeyCode::Escape | KeyCode::Char('L') => {
                        self.show_lever_overlay = false;
                    }
                    KeyCode::Char('q') => return Cmd::Quit,
                    _ => {}
                }
                return Cmd::None;
            }

            if self.show_health_overlay {
                match code {
                    KeyCode::Escape | KeyCode::Char('D') => {
                        self.show_health_overlay = false;
                    }
                    KeyCode::Char('q') => return Cmd::Quit,
                    _ => {}
                }
                return Cmd::None;
            }

            if self.show_insights_overlay {
                match code {
                    KeyCode::Escape => {
                        self.show_insights_overlay = false;
                    }
                    KeyCode::Char('q') => return Cmd::Quit,
                    _ => {}
                }
                return Cmd::None;
            }

            if self.show_trajectory_overlay {
                match code {
                    KeyCode::Escape => {
                        self.show_trajectory_overlay = false;
                    }
                    KeyCode::Char('q') => return Cmd::Quit,
                    _ => {}
                }
                return Cmd::None;
            }

            if self.what_if_preview.is_some() {
                match code {
                    KeyCode::Char('R') if shift => {
                        if matches!(self.what_if_preview.as_ref().map(|p| &p.action), Some(crate::app::WhatIfAction::Resolve)) {
                            let preview = self.what_if_preview.take().unwrap();
                            if self.engine.store().update_status(&preview.tension_id, TensionStatus::Resolved).is_ok() {
                                self.handle_recurrence(&preview.tension_id);
                                self.check_parent_auto_resolution(&preview.tension_id);
                                self.pending_undo = Some(crate::app::UndoAction {
                                    description: format!("Resolved '{}'", truncate(&preview.tension_desired, 30)),
                                    tension_id: preview.tension_id.clone(),
                                    previous_status: "Active".to_string(),
                                    expires_at: std::time::Instant::now() + std::time::Duration::from_secs(5),
                                });
                                self.push_toast(
                                    format!("Resolved '{}' \u{2014} press u to undo", truncate(&preview.tension_desired, 25)),
                                    ToastSeverity::Info,
                                );
                                self.reload_data();
                            }
                        }
                    }
                    KeyCode::Char('X') if shift => {
                        if matches!(self.what_if_preview.as_ref().map(|p| &p.action), Some(crate::app::WhatIfAction::Release)) {
                            let preview = self.what_if_preview.take().unwrap();
                            if self.engine.store().update_status(&preview.tension_id, TensionStatus::Released).is_ok() {
                                self.check_parent_auto_resolution(&preview.tension_id);
                                self.pending_undo = Some(crate::app::UndoAction {
                                    description: format!("Released '{}'", truncate(&preview.tension_desired, 30)),
                                    tension_id: preview.tension_id.clone(),
                                    previous_status: "Active".to_string(),
                                    expires_at: std::time::Instant::now() + std::time::Duration::from_secs(5),
                                });
                                self.push_toast(
                                    format!("Released '{}' \u{2014} press u to undo", truncate(&preview.tension_desired, 25)),
                                    ToastSeverity::Info,
                                );
                                self.reload_data();
                            }
                        }
                    }
                    KeyCode::Escape => {
                        self.what_if_preview = None;
                    }
                    KeyCode::Char('q') => return Cmd::Quit,
                    _ => {}
                }
                return Cmd::None;
            }

            if self.search.active {
                match code {
                    KeyCode::Escape => {
                        self.search.active = false;
                        self.search.query = None;
                        self.search.buffer.clear();
                        self.search.cursor = 0;
                        self.search.input_widget.clear();
                        let visible = self.visible_tensions().len();
                        if visible > 0 && self.selected() >= visible {
                            self.set_selected(visible - 1);
                        }
                    }
                    KeyCode::Enter => {
                        // Dismiss the search overlay but keep the filter active
                        self.search.active = false;
                        // Ensure selection is valid within filtered results
                        let visible = self.visible_tensions().len();
                        if visible > 0 && self.selected() >= visible {
                            self.set_selected(visible.saturating_sub(1));
                        }
                    }
                    other => {
                        // Delegate to the search TextInput widget
                        let event = Event::Key(KeyEvent {
                            code: other,
                            kind: KeyEventKind::Press,
                            modifiers: mods,
                        });
                        self.search.input_widget.handle_event(&event);

                        // Sync widget value back to search state
                        self.search.buffer = self.search.input_widget.value().to_string();
                        self.search.cursor = self.search.buffer.len();
                        self.search.query = if self.search.buffer.is_empty() {
                            None
                        } else {
                            Some(self.search.buffer.clone())
                        };
                        self.set_selected(0);
                    }
                }
                return Cmd::None;
            }

            match &self.input_mode {
                InputMode::TextInput(_) => {
                    match code {
                        KeyCode::Enter => return self.handle_submit(),
                        KeyCode::Escape => self.cancel_input(),
                        other => self.handle_text_input_key(other),
                    }
                    return Cmd::None;
                }
                InputMode::Confirm(_) => {
                    match code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => self.handle_confirm(true),
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Escape => {
                            self.handle_confirm(false);
                        }
                        _ => {}
                    }
                    return Cmd::None;
                }
                InputMode::MovePicker(_) => {
                    self.handle_move_picker_key(code);
                    return Cmd::None;
                }
                InputMode::Normal => {
                    let mapped = self.normal_key_to_msg(code, shift);
                    return self.update(mapped);
                }
                InputMode::Reflect => {
                    return self.handle_reflect_key(code, mods);
                }
            }
        }

        match msg {
            Msg::MoveDown => {
                match &self.active_view {
                    View::Dashboard => {
                        let visible = self.visible_tensions().len();
                        if visible > 0 && self.selected() < visible - 1 {
                            self.set_selected(self.selected() + 1);
                        }
                        // Auto-load detail for split-pane preview
                        let vis = self.visible_tensions();
                        if let Some(row) = vis.get(self.selected()) {
                            let id = row.id.clone();
                            self.load_detail(&id);
                        }
                    }
                    View::TreeView => {
                        let count = self.tree_items.len();
                        if count > 0 && self.tree_selected() < count - 1 {
                            self.set_tree_selected(self.tree_selected() + 1);
                        }
                    }
                    View::Detail => {
                        let max = self.detail_item_count().saturating_sub(1);
                        self.detail.cursor = self.detail.cursor.saturating_add(1).min(max);
                    }
                    View::Agent(_) => {
                        if !self.agent.mutations.is_empty()
                            && self.agent.mutation_cursor < self.agent.mutations.len() - 1
                        {
                            self.agent.mutation_cursor += 1;
                        }
                    }
                    View::Focus => {
                        // Cycle to next active tension
                        let visible = self.visible_tensions();
                        if let Some(current_id) = self.detail.tension.as_ref().map(|t| t.id.clone()) {
                            let current_idx = visible.iter().position(|r| r.id == current_id).unwrap_or(0);
                            if current_idx + 1 < visible.len() {
                                let next_id = visible[current_idx + 1].id.clone();
                                self.load_detail(&next_id);
                            }
                        }
                    }
                    View::Neighborhood => {
                        let len = self.neighborhood_items.len();
                        if len > 0 {
                            let mut state = self.neighborhood_state.borrow_mut();
                            let cur = state.selected().unwrap_or(0);
                            if cur < len - 1 {
                                state.select(Some(cur + 1));
                            }
                        }
                    }
                    View::Timeline | View::DynamicsSummary | View::Welcome => {}
                }
                Cmd::None
            }
            Msg::MoveUp => {
                match &self.active_view {
                    View::Dashboard => {
                        if self.selected() > 0 {
                            self.set_selected(self.selected() - 1);
                        }
                        // Auto-load detail for split-pane preview
                        let vis = self.visible_tensions();
                        if let Some(row) = vis.get(self.selected()) {
                            let id = row.id.clone();
                            self.load_detail(&id);
                        }
                    }
                    View::TreeView => {
                        if self.tree_selected() > 0 {
                            self.set_tree_selected(self.tree_selected() - 1);
                        }
                    }
                    View::Detail => {
                        self.detail.cursor = self.detail.cursor.saturating_sub(1);
                    }
                    View::Agent(_) => {
                        if self.agent.mutation_cursor > 0 {
                            self.agent.mutation_cursor -= 1;
                        }
                    }
                    View::Focus => {
                        // Cycle to previous active tension
                        let visible = self.visible_tensions();
                        if let Some(current_id) = self.detail.tension.as_ref().map(|t| t.id.clone()) {
                            let current_idx = visible.iter().position(|r| r.id == current_id).unwrap_or(0);
                            if current_idx > 0 {
                                let prev_id = visible[current_idx - 1].id.clone();
                                self.load_detail(&prev_id);
                            }
                        }
                    }
                    View::Neighborhood => {
                        let mut state = self.neighborhood_state.borrow_mut();
                        let cur = state.selected().unwrap_or(0);
                        if cur > 0 {
                            state.select(Some(cur - 1));
                        }
                    }
                    View::Timeline | View::DynamicsSummary | View::Welcome => {}
                }
                Cmd::None
            }
            Msg::ScrollDetailDown => {
                if self.active_view == View::Detail {
                    self.detail.scroll = self.detail.scroll.saturating_add(1);
                }
                Cmd::None
            }
            Msg::ScrollDetailUp => {
                if self.active_view == View::Detail {
                    self.detail.scroll = self.detail.scroll.saturating_sub(1);
                }
                Cmd::None
            }
            Msg::ToggleHelp => {
                self.show_help = !self.show_help;
                Cmd::None
            }
            Msg::OpenDetail => {
                match &self.active_view {
                    View::Dashboard => {
                        let visible = self.visible_tensions();
                        if let Some(row) = visible.get(self.selected()) {
                            let id = row.id.clone();
                            self.detail.nav_stack.clear();
                            self.load_detail(&id);
                            self.active_view = View::Detail;
                        }
                    }
                    View::TreeView => {
                        if let Some(item) = self.tree_items.get(self.tree_selected()) {
                            let id = item.tension_id.clone();
                            self.detail.nav_stack.clear();
                            self.load_detail(&id);
                            self.active_view = View::Detail;
                        }
                    }
                    View::Detail => {
                        // Navigate into child at cursor, if cursor is on a child item
                        let mutations_count = self.detail.mutations.len();
                        let children_start = 2 + mutations_count;
                        if self.detail.cursor >= children_start {
                            let child_idx = self.detail.cursor - children_start;
                            if let Some(child) = self.detail.children.get(child_idx) {
                                let child_id = child.id.clone();
                                if let Some(ref current) = self.detail.tension {
                                    self.detail.nav_stack.push(current.id.clone());
                                }
                                self.load_detail(&child_id);
                            }
                        }
                    }
                    View::Neighborhood => {
                        let cursor = self.neighborhood_state.borrow().selected().unwrap_or(0);
                        if let Some((id, role)) = self.neighborhood_items.get(cursor).cloned() {
                            if role == "SELECTED" {
                                // Enter on the center tension opens detail
                                self.detail.nav_stack.clear();
                                self.load_detail(&id);
                                self.active_view = View::Detail;
                            } else {
                                // Enter on any other item re-centers neighborhood on it
                                self.neighborhood_tension_id = Some(id.clone());
                                self.build_neighborhood_items(&id);
                            }
                        }
                    }
                    View::Agent(_) | View::Timeline | View::Focus | View::DynamicsSummary | View::Welcome => {}
                }
                Cmd::None
            }
            Msg::Back => {
                // If search filter is active (overlay closed), clear it first
                if self.search.query.is_some() && !self.search.active {
                    self.search.query = None;
                    self.search.buffer.clear();
                    self.search.cursor = 0;
                    self.search.input_widget.clear();
                    // Revalidate selection within unfiltered list
                    let visible = self.visible_tensions().len();
                    if visible > 0 && self.selected() >= visible {
                        self.set_selected(visible.saturating_sub(1));
                    }
                    return Cmd::None;
                }
                match &self.active_view {
                    View::Agent(tid) => {
                        let id = tid.clone();
                        self.load_detail(&id);
                        self.active_view = View::Detail;
                    }
                    View::Detail => {
                        if let Some(prev_id) = self.detail.nav_stack.pop() {
                            self.load_detail(&prev_id);
                            // stay in Detail view
                        } else {
                            self.active_view = View::Dashboard;
                        }
                    }
                    View::TreeView => {
                        self.active_view = View::Dashboard;
                    }
                    View::Neighborhood | View::Timeline | View::DynamicsSummary => {
                        self.active_view = View::Dashboard;
                    }
                    View::Focus => {
                        self.active_view = View::Dashboard;
                    }
                    View::Dashboard | View::Welcome => {}
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
            Msg::ViewNeighborhood => {
                // Legacy: redirect to Detail view (neighborhood absorbed into detail)
                if let Some(id) = self.selected_tension_id() {
                    self.load_detail(&id);
                    self.active_view = View::Detail;
                }
                Cmd::None
            }
            Msg::ViewTimeline => {
                // Legacy: redirect to Dashboard with timeline panel visible
                self.show_timeline = true;
                self.active_view = View::Dashboard;
                Cmd::None
            }
            Msg::ViewFocus => {
                // Legacy: redirect to Detail view (focus absorbed into detail)
                if let Some(id) = self.selected_tension_id() {
                    self.load_detail(&id);
                    self.active_view = View::Detail;
                }
                Cmd::None
            }
            Msg::ToggleTimeline => {
                self.show_timeline = !self.show_timeline;
                Cmd::None
            }
            Msg::ToggleHealthOverlay => {
                self.show_health_overlay = !self.show_health_overlay;
                Cmd::None
            }
            Msg::ShowInsights => {
                let now = Utc::now();
                let since = now - chrono::Duration::days(30);
                let all_mutations = self.engine.store().all_mutations().unwrap_or_default();
                let recent: Vec<_> = all_mutations.iter()
                    .filter(|m| m.timestamp() >= since)
                    .collect();

                use ftui::text::{Line, Span};
                use ftui::style::Style;
                use crate::theme::*;

                let mut lines: Vec<Line> = vec![
                    Line::from_spans([Span::styled(
                        " Behavioral Insights (30 days)".to_string(),
                        Style::new().fg(CLR_CYAN).bold(),
                    )]),
                    Line::from("".to_string()),
                ];

                // 1. Attention distribution
                let mut per_tension: HashMap<String, usize> = HashMap::new();
                for m in &recent {
                    *per_tension.entry(m.tension_id().to_string()).or_insert(0) += 1;
                }
                if !per_tension.is_empty() {
                    lines.push(Line::from_spans([Span::styled(
                        " Attention".to_string(),
                        Style::new().fg(CLR_CYAN).bold(),
                    )]));
                    let mut sorted: Vec<_> = per_tension.iter().collect();
                    sorted.sort_by(|a, b| b.1.cmp(a.1));
                    for (id, count) in sorted.iter().take(3) {
                        let desired = self.engine.store().get_tension(id).ok().flatten()
                            .map(|t| truncate(&t.desired, 25).to_string())
                            .unwrap_or_else(|| id[..8.min(id.len())].to_string());
                        lines.push(Line::from(format!("   {} — {} updates", desired, count)));
                    }
                    if let Some((id, count)) = sorted.last() {
                        if sorted.len() > 3 {
                            let desired = self.engine.store().get_tension(id).ok().flatten()
                                .map(|t| truncate(&t.desired, 25).to_string())
                                .unwrap_or_default();
                            lines.push(Line::from(format!("   {} — {} update (least)", desired, count)));
                        }
                    }
                    lines.push(Line::from("".to_string()));
                }

                // 2. Oscillation patterns
                let tensions = self.engine.store().list_tensions().unwrap_or_default();
                let oscillating_count = tensions.iter()
                    .filter(|t| t.status != TensionStatus::Resolved && t.status != TensionStatus::Released)
                    .filter(|t| {
                        self.engine.compute_full_dynamics_for_tension(&t.id)
                            .map(|cd| cd.oscillation.is_some())
                            .unwrap_or(false)
                    })
                    .count();
                if oscillating_count > 0 {
                    lines.push(Line::from_spans([Span::styled(
                        " Patterns".to_string(),
                        Style::new().fg(CLR_CYAN).bold(),
                    )]));
                    lines.push(Line::from(format!("   {} tensions show oscillation", oscillating_count)));
                    lines.push(Line::from("".to_string()));
                }

                // 3. Day-of-week activity
                use chrono::Datelike;
                let mut day_counts = [0u32; 7];
                for m in &recent {
                    let weekday = m.timestamp().weekday().num_days_from_monday() as usize;
                    day_counts[weekday] += 1;
                }
                let max_day = *day_counts.iter().max().unwrap_or(&1).max(&1);
                lines.push(Line::from_spans([Span::styled(
                    " Activity by Day".to_string(),
                    Style::new().fg(CLR_CYAN).bold(),
                )]));
                let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                let mut day_spans: Vec<Span<'static>> = vec![Span::styled("   ".to_string(), Style::new())];
                for (i, day) in days.iter().enumerate() {
                    let bar_len = ((day_counts[i] as f64 / max_day as f64) * 5.0).round() as usize;
                    let bar: String = "\u{2588}".repeat(bar_len);
                    let pad: String = " ".repeat(5 - bar_len);
                    day_spans.push(Span::styled(
                        format!("{} {}{} ", day, bar, pad),
                        Style::new().fg(CLR_CYAN),
                    ));
                }
                lines.push(Line::from_spans(day_spans));

                lines.push(Line::from("".to_string()));
                lines.push(Line::from_spans([Span::styled(
                    "  Press Esc to close".to_string(),
                    Style::new().fg(CLR_DIM_GRAY),
                )]));

                self.insights_lines = lines;
                self.show_insights_overlay = true;
                Cmd::None
            }
            Msg::ShowTrajectory => {
                use ftui::text::{Line, Span};
                use ftui::style::Style;
                use crate::theme::*;

                let now = Utc::now();

                // Ensure projection is fresh
                if self.field_projection.is_none() {
                    let all_tensions = self.engine.store().list_tensions().unwrap_or_default();
                    let all_mutations = self.engine.store().all_mutations().unwrap_or_default();
                    let thresholds = sd_core::ProjectionThresholds::default();
                    let projection = sd_core::project_field(&all_tensions, &all_mutations, &thresholds, now);
                    self.field_projection = Some(projection);
                    self.last_projection_time = Some(now);
                }

                let mut lines: Vec<Line> = vec![
                    Line::from_spans([Span::styled(
                        " Field Trajectory Overview".to_string(),
                        Style::new().fg(CLR_CYAN).bold(),
                    )]),
                    Line::from("".to_string()),
                ];

                if let Some(ref fp) = self.field_projection {
                    // Structural funnel per horizon
                    let horizon_labels = [
                        (sd_core::ProjectionHorizon::OneWeek, "+1 week"),
                        (sd_core::ProjectionHorizon::OneMonth, "+1 month"),
                        (sd_core::ProjectionHorizon::ThreeMonths, "+3 months"),
                    ];
                    for (horizon, label) in &horizon_labels {
                        if let Some(buckets) = fp.funnel.get(horizon) {
                            lines.push(Line::from_spans([Span::styled(
                                format!(" {} ({})", label, buckets.total),
                                Style::new().fg(CLR_CYAN).bold(),
                            )]));
                            if buckets.total > 0 {
                                lines.push(Line::from_spans([
                                    Span::styled("   \u{2193} Resolving  ", Style::new().fg(CLR_MID_GRAY)),
                                    Span::styled(format!("{}", buckets.resolving), Style::new().fg(CLR_GREEN)),
                                ]));
                                lines.push(Line::from_spans([
                                    Span::styled("   \u{2014} Stalling   ", Style::new().fg(CLR_MID_GRAY)),
                                    Span::styled(format!("{}", buckets.stalling), Style::new().fg(CLR_DIM_GRAY)),
                                ]));
                                lines.push(Line::from_spans([
                                    Span::styled("   ~ Drifting   ", Style::new().fg(CLR_MID_GRAY)),
                                    Span::styled(format!("{}", buckets.drifting), Style::new().fg(CLR_YELLOW)),
                                ]));
                                lines.push(Line::from_spans([
                                    Span::styled("   \u{21cc} Oscillating", Style::new().fg(CLR_MID_GRAY)),
                                    Span::styled(format!(" {}", buckets.oscillating), Style::new().fg(CLR_RED_SOFT)),
                                ]));
                            }
                            lines.push(Line::from("".to_string()));
                        }
                    }

                    // Urgency collisions
                    if !fp.urgency_collisions.is_empty() {
                        lines.push(Line::from_spans([Span::styled(
                            " Urgency Collisions".to_string(),
                            Style::new().fg(CLR_RED_SOFT).bold(),
                        )]));
                        for collision in fp.urgency_collisions.iter().take(3) {
                            let ids: Vec<String> = collision.tension_ids.iter()
                                .map(|id| {
                                    self.engine.store().get_tension(id).ok().flatten()
                                        .map(|t| truncate(&t.desired, 18).to_string())
                                        .unwrap_or_else(|| id[..6.min(id.len())].to_string())
                                })
                                .collect();
                            lines.push(Line::from_spans([
                                Span::styled(
                                    format!("   peak {:.0}%  ", collision.peak_combined_urgency * 100.0),
                                    Style::new().fg(CLR_RED_SOFT),
                                ),
                                Span::styled(ids.join(", "), Style::new().fg(CLR_LIGHT_GRAY)),
                            ]));
                        }
                        lines.push(Line::from("".to_string()));
                    }
                }

                lines.push(Line::from_spans([Span::styled(
                    "  Press Esc to close".to_string(),
                    Style::new().fg(CLR_DIM_GRAY),
                )]));

                self.trajectory_lines = lines;
                self.show_trajectory_overlay = true;
                Cmd::None
            }
            Msg::CycleFilter => {
                self.filter = self.filter.next();
                let visible = self.visible_tensions().len();
                if visible > 0 && self.selected() >= visible {
                    self.set_selected(visible - 1);
                } else if visible == 0 {
                    self.set_selected(0);
                }
                Cmd::None
            }
            Msg::StartUpdateReality => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Update reality for \"{}\":",
                            truncate(&t.desired, 40)
                        );
                        let prefill = t.actual.clone();
                        self.enter_text_input(
                            InputContext::UpdateReality(id),
                            prompt,
                            prefill,
                        );
                    }
                }
                Cmd::None
            }
            Msg::StartUpdateDesire => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Update desire for \"{}\":",
                            truncate(&t.desired, 40)
                        );
                        let prefill = t.desired.clone();
                        self.enter_text_input(
                            InputContext::UpdateDesire(id),
                            prompt,
                            prefill,
                        );
                    }
                }
                Cmd::None
            }
            Msg::StartAddNote => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Add note for \"{}\":",
                            truncate(&t.desired, 40)
                        );
                        self.enter_text_input(
                            InputContext::AddNote(id),
                            prompt,
                            String::new(),
                        );
                    }
                }
                Cmd::None
            }
            Msg::StartSetHorizon => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Set horizon for \"{}\" (2026, 2026-03, 2026-03-15, or none):",
                            truncate(&t.desired, 30)
                        );
                        let prefill = t
                            .horizon
                            .as_ref()
                            .map(|h| h.to_string())
                            .unwrap_or_default();
                        self.enter_text_input(
                            InputContext::SetHorizon(id),
                            prompt,
                            prefill,
                        );
                    }
                }
                Cmd::None
            }
            Msg::StartAddTension => {
                let parent_id = if self.active_view == View::Detail {
                    self.detail.tension.as_ref().map(|t| t.id.clone())
                } else {
                    None
                };
                let prompt = if parent_id.is_some() {
                    "Add sub-tension: desired [+2w] [| actual]".to_string()
                } else {
                    "Add: desired [+2w] [| actual]".to_string()
                };
                self.enter_text_input(
                    InputContext::AddTensionDesired { parent_id },
                    prompt,
                    String::new(),
                );
                Cmd::None
            }
            Msg::CreateChild => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Desired state for child of \"{}\":",
                            truncate(&t.desired, 30)
                        );
                        self.enter_text_input(
                            InputContext::CreateChildDesired(id),
                            prompt,
                            String::new(),
                        );
                    }
                }
                Cmd::None
            }
            Msg::CreateParent => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Desired state for parent of \"{}\":",
                            truncate(&t.desired, 30)
                        );
                        self.enter_text_input(
                            InputContext::CreateParentDesired(id),
                            prompt,
                            String::new(),
                        );
                    }
                }
                Cmd::None
            }
            Msg::StartResolve => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        if t.status == TensionStatus::Active {
                            let preview = self.compute_what_if_preview(
                                &id, &t, crate::app::WhatIfAction::Resolve,
                            );
                            self.what_if_preview = Some(preview);
                        }
                    }
                }
                Cmd::None
            }
            Msg::StartRelease => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        if t.status == TensionStatus::Active {
                            let preview = self.compute_what_if_preview(
                                &id, &t, crate::app::WhatIfAction::Release,
                            );
                            self.what_if_preview = Some(preview);
                        }
                    }
                }
                Cmd::None
            }
            Msg::StartDelete => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Delete \"{}\"? (y/n)",
                            truncate(&t.desired, 40)
                        );
                        self.enter_confirm(
                            ConfirmAction::Delete {
                                id,
                                desired: t.desired.clone(),
                            },
                            prompt,
                        );
                    }
                }
                Cmd::None
            }
            Msg::StartSetRecurrence => {
                if let Some(ref t) = self.detail.tension {
                    let id = t.id.clone();
                    self.enter_text_input(
                        InputContext::SetRecurrence(id),
                        "Recurrence interval (e.g. +1w, +2w, +1m, or empty to clear):".into(),
                        String::new(),
                    );
                }
                Cmd::None
            }
            Msg::StartSnooze => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        let prompt = format!(
                            "Snooze \"{}\" until (e.g. +3d, friday, 2026-04-01):",
                            truncate(&t.desired, 30)
                        );
                        self.enter_text_input(
                            InputContext::SetSnooze(id),
                            prompt,
                            String::new(),
                        );
                    }
                }
                Cmd::None
            }
            Msg::ToggleShowSnoozed => {
                self.show_snoozed = !self.show_snoozed;
                let label = if self.show_snoozed { "Showing snoozed tensions" } else { "Hiding snoozed tensions" };
                self.push_toast(label.to_string(), ToastSeverity::Info);
                Cmd::None
            }
            Msg::StartMove => {
                if let Some(id) = self.selected_tension_id() {
                    let candidates = self.build_move_candidates(&id);
                    self.input_overlay = Some(InputOverlay::new(
                        "Move tension - select new parent (j/k/Enter):".to_string(),
                        String::new(),
                    ));
                    {
                        let mut mps = self.move_picker_state.borrow_mut();
                        *mps = ftui::widgets::list::ListState::default();
                        mps.select(Some(0));
                    }
                    self.input_mode = InputMode::MovePicker(MovePickerState {
                        tension_id: id,
                        candidates,
                        selected: 0,
                    });
                }
                Cmd::None
            }

            Msg::InputChar(_)
            | Msg::InputBackspace
            | Msg::InputDelete
            | Msg::InputLeft
            | Msg::InputRight
            | Msg::InputHome
            | Msg::InputEnd
            | Msg::InputSubmit
            | Msg::InputCancel
            | Msg::ConfirmYes
            | Msg::ConfirmNo
            | Msg::PickerUp
            | Msg::PickerDown
            | Msg::PickerSelect
            | Msg::PickerCancel => Cmd::None,

            Msg::StartAgent => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(t)) = self.engine.store().get_tension(&id) {
                        self.active_view = View::Agent(id.clone());
                        self.agent.output = Vec::new();
                        self.agent.scroll = 0;
                        self.agent.mutations = Vec::new();
                        self.agent.mutation_selected = Vec::new();
                        self.agent.mutation_cursor = 0;
                        self.agent.running = false;
                        self.agent.response_text = None;

                        let prompt = format!(
                            "Enter prompt for agent ({}):",
                            truncate(&t.desired, 30)
                        );
                        self.enter_text_input(
                            InputContext::AgentPrompt(id),
                            prompt,
                            String::new(),
                        );
                    }
                }
                Cmd::None
            }
            Msg::AgentResponseReceived(result) => {
                self.agent.running = false;
                match result {
                    Ok(response_text) => {
                        self.agent.output = response_text.lines().map(|l| l.to_string()).collect();
                        self.agent.scroll = 0;

                        if let Some(structured) = StructuredResponse::from_response(&response_text) {
                            self.agent.response_text = Some(structured.response.clone());
                            self.agent.mutations = structured.mutations;
                            self.agent.mutation_selected =
                                vec![true; self.agent.mutations.len()];
                            self.agent.mutation_cursor = 0;

                            if self.agent.mutations.is_empty() {
                                self.push_toast(
                                    "Agent responded (no mutations suggested)".to_string(),
                                    ToastSeverity::Info,
                                );
                            } else {
                                self.push_toast(
                                    format!(
                                        "Agent suggested {} change(s)",
                                        self.agent.mutations.len()
                                    ),
                                    ToastSeverity::Info,
                                );
                            }
                        } else {
                            self.agent.response_text = Some(response_text);
                            self.push_toast(
                                "Agent responded (plain text)".to_string(),
                                ToastSeverity::Info,
                            );
                        }
                    }
                    Err(e) => {
                        self.agent.output = vec![format!("Error: {}", e)];
                        self.push_toast(
                            format!("Agent error: {}", truncate(&e, 40)),
                            ToastSeverity::Alert,
                        );
                    }
                }
                Cmd::None
            }
            Msg::AgentToggleMutation(idx) => {
                if idx < self.agent.mutation_selected.len() {
                    self.agent.mutation_selected[idx] = !self.agent.mutation_selected[idx];
                }
                Cmd::None
            }
            Msg::AgentApplySelected => {
                if self.agent.mutations.is_empty() {
                    return Cmd::None;
                }
                let count = self.apply_agent_mutations();
                if count > 0 {
                    self.push_toast(
                        format!("Applied {} agent change(s)", count),
                        ToastSeverity::Info,
                    );
                    self.reload_data();
                    if let View::Agent(ref tid) = self.active_view {
                        let id = tid.clone();
                        self.load_detail(&id);
                        self.active_view = View::Detail;
                    }
                } else {
                    self.push_toast("No mutations selected".to_string(), ToastSeverity::Warning);
                }
                Cmd::None
            }
            Msg::AgentScrollUp => {
                self.agent.scroll = self.agent.scroll.saturating_sub(1);
                Cmd::None
            }
            Msg::AgentScrollDown => {
                self.agent.scroll = self.agent.scroll.saturating_add(1);
                Cmd::None
            }

            Msg::Tick => {
                // Expire pending undo after deadline
                if let Some(ref undo) = self.pending_undo {
                    if undo.expires_at <= std::time::Instant::now() {
                        self.pending_undo = None;
                    }
                }
                // Check for expired snoozes and notify
                let today = Utc::now().date_naive();
                let expired_names: Vec<String> = self.tensions.iter()
                    .filter_map(|t| {
                        let mutations = self.engine.store().get_mutations(&t.id).unwrap_or_default();
                        mutations.iter().rev()
                            .find(|m| m.field() == "snoozed_until")
                            .and_then(|m| chrono::NaiveDate::parse_from_str(m.new_value(), "%Y-%m-%d").ok())
                            .filter(|&d| d == today)
                            .map(|_| truncate(&t.desired, 30).to_string())
                    })
                    .collect();
                for name in expired_names {
                    self.push_toast(
                        format!("Snooze expired: {}", name),
                        ToastSeverity::Info,
                    );
                }
                self.reload_data();
                Cmd::None
            }
            Msg::ExternalChange => {
                self.reload_data();
                self.push_toast(
                    "External change detected — reloaded".to_string(),
                    ToastSeverity::Info,
                );
                // Re-arm the watcher
                self.setup_file_watcher()
            }
            Msg::DynamicsEvent(message, severity) => {
                self.push_toast(message, severity);
                Cmd::None
            }

            Msg::WelcomeSelect | Msg::WelcomeConfirm => Cmd::None,

            Msg::OpenCommandPalette => {
                self.command_palette.open();
                Cmd::None
            }
            Msg::OpenSearch => {
                self.search.active = true;
                self.search.buffer.clear();
                self.search.cursor = 0;
                self.search.query = None;
                self.search.input_widget.clear();
                Cmd::None
            }

            Msg::ShowLever => {
                self.show_lever_overlay = !self.show_lever_overlay;
                Cmd::None
            }

            Msg::Undo => {
                if let Some(undo) = self.pending_undo.take() {
                    if undo.expires_at > std::time::Instant::now() {
                        if self.engine.store().update_status(&undo.tension_id, TensionStatus::Active).is_ok() {
                            self.push_toast(
                                format!("Undone: {}", undo.description),
                                ToastSeverity::Info,
                            );
                            self.reload_data();
                        }
                    }
                }
                Cmd::None
            }

            Msg::ViewDynamics => {
                // Legacy: redirect to health overlay on current view
                self.show_health_overlay = true;
                Cmd::None
            }
            Msg::StartReflect => {
                if let Some(id) = self.selected_tension_id() {
                    if let Ok(Some(_t)) = self.engine.store().get_tension(&id) {
                        self.load_detail(&id);
                        self.reflect.textarea = Some(
                            TextArea::new()
                                .with_placeholder("Write your reflections...")
                                .with_focus(true)
                                .with_soft_wrap(true)
                        );
                        self.reflect.tension_id = Some(id);
                        self.input_mode = InputMode::Reflect;
                    }
                }
                Cmd::None
            }
            Msg::ReflectSubmit => {
                let buffer_text = self.reflect.textarea.as_ref().map(|ta| ta.text());
                if let (Some(buffer), Some(tid)) = (buffer_text, self.reflect.tension_id.take()) {
                    self.reflect.textarea = None;
                    let reflect_text = buffer.trim().to_owned();
                    if !reflect_text.is_empty() {
                        self.input_mode = InputMode::Normal;
                        self.active_view = View::Agent(tid.clone());
                        self.agent.output = vec!["Running agent with reflect...".to_string()];
                        self.agent.scroll = 0;
                        self.agent.mutations = Vec::new();
                        self.agent.mutation_selected = Vec::new();
                        self.agent.mutation_cursor = 0;
                        self.agent.running = true;
                        self.agent.response_text = None;
                        return self.spawn_agent_task(tid, reflect_text);
                    } else {
                        self.input_mode = InputMode::Normal;
                    }
                } else {
                    self.input_mode = InputMode::Normal;
                }
                Cmd::None
            }

            Msg::TickerJump(n) => {
                let mut urgent: Vec<&crate::types::TensionRow> = self
                    .tensions
                    .iter()
                    .filter(|t| !matches!(t.tier, UrgencyTier::Resolved))
                    .filter(|t| t.urgency.is_some())
                    .collect();
                urgent.sort_by(|a, b| {
                    b.urgency
                        .unwrap()
                        .partial_cmp(&a.urgency.unwrap())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                if let Some(row) = urgent.get(n) {
                    let id = row.id.clone();
                    self.detail.nav_stack.clear();
                    self.load_detail(&id);
                    self.active_view = View::Detail;
                }
                Cmd::None
            }

            Msg::RawKey(_, _) => Cmd::None,
            Msg::Quit => Cmd::Quit,
            Msg::Noop => Cmd::None,
        }
    }

    fn subscriptions(&self) -> Vec<Box<dyn Subscription<Msg>>> {
        vec![Box::new(Every::new(Duration::from_secs(60), || Msg::Tick))]
    }

    fn view(&self, frame: &mut Frame<'_>) {
        frame.set_cursor_visible(false);
        frame.set_cursor(None);

        let area = Rect::new(0, 0, frame.width(), frame.height());
        let hide_hints = area.height < 10 || !matches!(self.input_mode, InputMode::Normal);
        let show_lever = area.height >= 10 && self.lever.is_some();

        match &self.active_view {
            View::Welcome => {
                self.render_welcome_screen(area, frame);
                return;
            }
            View::Dashboard => {
                let use_split = area.width >= 120;

                if use_split {
                    // Split layout: dashboard left (40%), detail right (60%)
                    let left_width = (area.width as f64 * 0.4) as u16;
                    let right_width = area.width - left_width;

                    let left_area = Rect::new(area.x, area.y, left_width, area.height);
                    let right_area = Rect::new(area.x + left_width, area.y, right_width, area.height);

                    // Left side: status bar + tension list + hints
                    let mut left_constraints: Vec<Constraint> = Vec::new();
                    left_constraints.push(Constraint::Fixed(1)); // status bar
                    left_constraints.push(Constraint::Fill);     // list
                    if show_lever { left_constraints.push(Constraint::Fixed(1)); }
                    if !hide_hints { left_constraints.push(Constraint::Fixed(1)); }

                    let left_layout = Flex::vertical().constraints(left_constraints);
                    let left_rects = left_layout.split(left_area);
                    let mut idx = 0;
                    self.render_status_bar(&left_rects[idx], frame); idx += 1;
                    self.render_tension_list(&left_rects[idx], frame); idx += 1;
                    if show_lever { self.render_lever_bar(&left_rects[idx], frame); idx += 1; }
                    if !hide_hints { self.render_dashboard_hints(&left_rects[idx], frame); }

                    // Right side: detail title + detail body
                    if self.detail.tension.is_some() {
                        let right_layout = Flex::vertical().constraints([
                            Constraint::Fixed(1), // title
                            Constraint::Fill,     // body
                        ]);
                        let right_rects = right_layout.split(right_area);
                        self.render_detail_title(&right_rects[0], frame);
                        self.render_detail_body_responsive(&right_rects[1], frame);
                    }
                } else {
                    // Single-pane layout (existing behavior)
                    let mut constraints: Vec<Constraint> = Vec::new();
                    constraints.push(Constraint::Fixed(1)); // merged status bar
                    constraints.push(Constraint::Fill);     // content
                    if self.show_timeline { constraints.push(Constraint::Fixed(7)); } // timeline panel
                    if show_lever { constraints.push(Constraint::Fixed(1)); }
                    if !hide_hints { constraints.push(Constraint::Fixed(1)); }

                    let layout = Flex::vertical().constraints(constraints);
                    let rects = layout.split(area);
                    let mut idx = 0;
                    self.render_status_bar(&rects[idx], frame); idx += 1;
                    self.render_tension_list(&rects[idx], frame); idx += 1;
                    if self.show_timeline { self.render_timeline_body(&rects[idx], frame); idx += 1; }
                    if show_lever { self.render_lever_bar(&rects[idx], frame); idx += 1; }
                    if !hide_hints { self.render_dashboard_hints(&rects[idx], frame); }
                }
            }
            View::Detail => {
                let mut constraints: Vec<Constraint> = Vec::new();
                constraints.push(Constraint::Fixed(1)); // title bar
                constraints.push(Constraint::Fill);     // content
                if show_lever { constraints.push(Constraint::Fixed(1)); }
                if !hide_hints { constraints.push(Constraint::Fixed(1)); }

                let layout = Flex::vertical().constraints(constraints);
                let rects = layout.split(area);
                let mut idx = 0;
                self.render_detail_title(&rects[idx], frame); idx += 1;
                self.render_detail_body_responsive(&rects[idx], frame); idx += 1;
                if show_lever { self.render_lever_bar(&rects[idx], frame); idx += 1; }
                if !hide_hints { self.render_detail_hints(&rects[idx], frame); }
            }
            View::TreeView => {
                let mut constraints: Vec<Constraint> = Vec::new();
                constraints.push(Constraint::Fixed(1)); // title bar
                constraints.push(Constraint::Fill);     // content
                if show_lever { constraints.push(Constraint::Fixed(1)); }
                if !hide_hints { constraints.push(Constraint::Fixed(1)); }

                let layout = Flex::vertical().constraints(constraints);
                let rects = layout.split(area);
                let mut idx = 0;
                self.render_tree_title(&rects[idx], frame); idx += 1;
                self.render_tree_body(&rects[idx], frame); idx += 1;
                if show_lever { self.render_lever_bar(&rects[idx], frame); idx += 1; }
                if !hide_hints { self.render_tree_hints(&rects[idx], frame); }
            }
            // Legacy views: redirect rendering to Dashboard fallback
            View::Neighborhood | View::Timeline | View::Focus | View::DynamicsSummary => {
                // These views have been absorbed into the primary views.
                // If reached (e.g. stale state), render as Dashboard.
                let mut constraints: Vec<Constraint> = Vec::new();
                constraints.push(Constraint::Fixed(1)); // merged status bar
                constraints.push(Constraint::Fill);     // content
                if show_lever { constraints.push(Constraint::Fixed(1)); }
                if !hide_hints { constraints.push(Constraint::Fixed(1)); }

                let layout = Flex::vertical().constraints(constraints);
                let rects = layout.split(area);
                let mut idx = 0;
                self.render_status_bar(&rects[idx], frame); idx += 1;
                self.render_tension_list(&rects[idx], frame); idx += 1;
                if show_lever { self.render_lever_bar(&rects[idx], frame); idx += 1; }
                if !hide_hints { self.render_dashboard_hints(&rects[idx], frame); }
            }
            View::Agent(tension_id) => {
                let layout = Flex::vertical().constraints([
                    Constraint::Fixed(1),
                    Constraint::Fill,
                    Constraint::Fixed(5),
                    Constraint::Fixed(1),
                ]);
                let rects = layout.split(area);

                self.render_agent_title(tension_id, &rects[0], frame);
                self.render_agent_body(&rects[1], frame);
                self.render_agent_context(tension_id, &rects[2], frame);
                if !hide_hints {
                    self.render_agent_hints(&rects[3], frame);
                }
            }
        }

        if self.show_lever_overlay {
            self.render_lever_detail_overlay(area, frame);
        }

        if self.show_health_overlay {
            self.render_health_overlay(area, frame);
        }

        if self.show_insights_overlay {
            self.render_insights_overlay(area, frame);
        }

        if self.show_trajectory_overlay {
            self.render_trajectory_overlay(area, frame);
        }

        if self.what_if_preview.is_some() {
            self.render_what_if_overlay(area, frame);
        }

        if self.show_help {
            self.render_help_overlay(area, frame);
        }

        if self.command_palette.is_visible() {
            self.render_command_palette(area, frame);
        }

        if self.search.active {
            self.render_search_overlay(area, frame);
        }

        if matches!(self.input_mode, InputMode::Reflect) {
            self.render_reflect_overlay(area, frame);
            // Show cursor for TextArea editing
            frame.set_cursor_visible(true);
        }

        self.render_input_overlay(area, frame);
        if matches!(self.input_mode, InputMode::TextInput(_)) {
            frame.set_cursor_visible(true);
        }
        self.render_toasts(area, frame);
    }
}

/// Parse a quick-add input line into (desired, optional_horizon_str, optional_actual).
///
/// Format: `desired state [horizon] [| actual state]`
///
/// Examples:
///   "Ship v2"                       -> ("Ship v2", None, None)
///   "Ship v2 +2w"                   -> ("Ship v2", Some("+2w"), None)
///   "Ship v2 | still drafting"      -> ("Ship v2", None, Some("still drafting"))
///   "Ship v2 +2w | still drafting"  -> ("Ship v2", Some("+2w"), Some("still drafting"))
fn parse_quick_add(input: &str) -> (String, Option<String>, Option<String>) {
    use crate::horizon::parse_horizon;

    // Split on | for actual state
    let parts: Vec<&str> = input.splitn(2, '|').collect();
    let before_pipe = parts[0].trim();
    let actual = parts
        .get(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Try to extract horizon from the end of the desired portion.
    // Only match if the last word parses as a valid horizon.
    let words: Vec<&str> = before_pipe.rsplitn(2, ' ').collect();
    if words.len() == 2 {
        let candidate = words[0]; // last word
        if parse_horizon(candidate).is_ok() {
            let desired = words[1].trim().to_string();
            if !desired.is_empty() {
                return (desired, Some(candidate.to_string()), actual);
            }
        }
    }

    (before_pipe.to_string(), None, actual)
}
