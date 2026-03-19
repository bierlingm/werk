//! Update logic for the Operative Instrument.

use std::time::Duration;

use ftui::{Cmd, Event, Frame, Model};
use ftui::layout::{Constraint, Flex, Rect};
use ftui::runtime::subscription::{Every, Subscription};

use crate::app::InstrumentApp;
use crate::msg::Msg;
use crate::state::*;

impl Model for InstrumentApp {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        Cmd::none()
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        // Clear expired transient messages
        if let Some(ref t) = self.transient {
            if t.is_expired() {
                self.transient = None;
            }
        }

        match &self.input_mode {
            InputMode::Normal => self.update_normal(msg),
            InputMode::Help => self.update_help(msg),
            InputMode::Adding(_) => self.update_adding(msg),
            InputMode::Editing { .. } => self.update_editing(msg),
            InputMode::Annotating { .. } => self.update_annotating(msg),
            InputMode::Confirming(_) => self.update_confirming(msg),
            InputMode::Searching => self.update_searching(msg),
            InputMode::Moving { .. } => self.update_moving(msg),
            InputMode::AgentPrompt { .. } => self.update_agent_prompt(msg),
            InputMode::ReviewingMutations => self.update_mutation_review(msg),
            InputMode::ReviewingInsights => self.update_insight_review(msg),
            InputMode::Reordering { .. } => self.update_reordering(msg),
        }
    }

    fn view(&self, frame: &mut Frame<'_>) {
        frame.set_cursor_visible(false);
        frame.set_cursor(None);

        let area = Rect::new(0, 0, frame.width(), frame.height());
        let show_hints = area.height >= 6;

        // Layout: content + lever + hints
        let mut constraints = vec![Constraint::Fill, Constraint::Fixed(1)];
        if show_hints {
            constraints.push(Constraint::Fixed(1));
        }

        let layout = Flex::vertical().constraints(constraints);
        let rects = layout.split(area);

        // Full-screen modes: render ONLY the overlay, skip the field entirely
        if matches!(self.input_mode, InputMode::Help) {
            crate::helpers::clear_area(frame, rects[0]);
            self.render_help(&rects[0], frame);
        } else if matches!(self.input_mode, InputMode::ReviewingMutations) {
            crate::helpers::clear_area(frame, rects[0]);
            self.render_mutation_review(&rects[0], frame);
        } else if matches!(self.input_mode, InputMode::ReviewingInsights) {
            crate::helpers::clear_area(frame, rects[0]);
            self.render_insight_review(&rects[0], frame);
        } else if matches!(self.input_mode, InputMode::Searching) {
            crate::helpers::clear_area(frame, rects[0]);
            self.render_search(&rects[0], frame);
        } else if matches!(self.input_mode, InputMode::Moving { .. }) {
            crate::helpers::clear_area(frame, rects[0]);
            self.render_search(&rects[0], frame);
        } else if matches!(self.input_mode, InputMode::AgentPrompt { .. }) && self.agent_response_text.is_some() {
            // Follow-up: show response with prompt overlay at bottom
            crate::helpers::clear_area(frame, rects[0]);
            self.render_mutation_review(&rects[0], frame);
            self.render_agent_prompt(&rects[0], frame);
        } else if self.siblings.is_empty() && self.parent_id.is_none()
            && !matches!(self.input_mode, InputMode::Adding(_))
        {
            self.render_empty(&rects[0], frame);
        } else if self.siblings.is_empty() && self.parent_id.is_some() {
            // Descended into a tension with no children — render_field handles this
            self.render_field(&rects[0], frame);
        } else {
            self.render_field(&rects[0], frame);
        }

        // Render inline overlays on top of the field (only for non-fullscreen modes)
        match &self.input_mode {
            InputMode::Adding(step) => {
                self.render_add_prompt(step, &rects[0], frame);
            }
            InputMode::Confirming(kind) => {
                self.render_confirm(kind, &rects[0], frame);
            }
            InputMode::Editing { field, .. } => {
                self.render_edit_prompt(field, &rects[0], frame);
            }
            InputMode::Annotating { .. } => {
                self.render_note_prompt(&rects[0], frame);
            }
            InputMode::AgentPrompt { .. } => {
                self.render_agent_prompt(&rects[0], frame);
            }
            // Full-screen modes already rendered above
            _ => {}
        }

        // Lever
        self.render_lever(&rects[1], frame);

        // Hints — show contextual hints for input modes
        if show_hints {
            match &self.input_mode {
                InputMode::Adding(_) => self.render_input_hints("Enter next  Esc skip/create  Bksp back", &rects[2], frame),
                InputMode::Confirming(_) => self.render_input_hints("y confirm  n cancel", &rects[2], frame),
                InputMode::Editing { .. } => self.render_input_hints("Enter save  Tab switch field  Esc cancel", &rects[2], frame),
                InputMode::Annotating { .. } => self.render_input_hints("Enter save  Esc cancel", &rects[2], frame),
                InputMode::Searching => self.render_input_hints("Enter jump  j/k navigate  Esc cancel", &rects[2], frame),
                InputMode::Moving { .. } => self.render_input_hints("Enter place here  j/k navigate  Esc cancel", &rects[2], frame),
                InputMode::Reordering { .. } => self.render_input_hints("Shift+J/K move  Enter drop  Esc cancel", &rects[2], frame),
                InputMode::AgentPrompt { .. } => self.render_input_hints("Enter send  ! clipboard  Esc cancel", &rects[2], frame),
                InputMode::ReviewingMutations => {
                    if self.agent_mutations.is_empty() {
                        self.render_input_hints("@ follow up  Esc close", &rects[2], frame);
                    } else {
                        self.render_input_hints("Space toggle  a apply  @ follow up  j/k navigate  Esc dismiss", &rects[2], frame);
                    }
                }
                InputMode::ReviewingInsights => {
                    self.render_input_hints("Space expand  a apply  d dismiss  j/k navigate  Esc close", &rects[2], frame);
                }
                _ => self.render_hints(&rects[2], frame),
            }
        }
    }

    fn subscriptions(&self) -> Vec<Box<dyn Subscription<Msg>>> {
        vec![Box::new(Every::new(Duration::from_secs(2), || Msg::Tick))]
    }
}

impl InstrumentApp {
    fn update_normal(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            // Navigation
            Msg::Char('k') | Msg::Up => {
                self.vlist.up();
                self.close_gaze();
                Cmd::none()
            }
            Msg::Char('j') | Msg::Down => {
                self.vlist.down();
                self.close_gaze();
                Cmd::none()
            }

            // Reorder: Shift+J/K enters grab mode and does first move
            Msg::Char('K') | Msg::MoveUp => {
                self.enter_reorder();
                self.reorder_move_up();
                Cmd::none()
            }
            Msg::Char('J') | Msg::MoveDown => {
                self.enter_reorder();
                self.reorder_move_down();
                Cmd::none()
            }

            Msg::Char('l') | Msg::Submit | Msg::Descend => {
                // Descend into any tension — gaze or not, with children or not
                // (descending into a childless tension shows empty with "a" to add)
                if let Some(entry) = self.action_target().cloned() {
                    self.descend(&entry.id);
                }
                Cmd::none()
            }
            Msg::Char('h') | Msg::Backspace | Msg::Ascend => {
                if self.parent_id.is_some() {
                    self.ascend();
                }
                Cmd::none()
            }

            Msg::Char('g') | Msg::JumpTop => {
                self.vlist.top();
                self.close_gaze();
                Cmd::none()
            }
            Msg::Char('G') | Msg::JumpBottom => {
                self.vlist.bottom();
                self.close_gaze();
                Cmd::none()
            }

            // Gaze
            Msg::Char(' ') | Msg::ToggleGaze => {
                self.toggle_gaze();
                Cmd::none()
            }
            // Tab is reserved for field cycling in edit mode — no action in normal mode
            Msg::Tab | Msg::ExpandGaze => Cmd::none(),

            // Acts
            Msg::Char('a') | Msg::StartAdd => {
                self.input_mode = InputMode::Adding(AddStep::Name);
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Char('e') | Msg::StartEdit => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active {
                        self.input_buffer = entry.desired.clone();
                        self.text_input.set_value(&entry.desired);
                        self.text_input.set_focused(true);
                        self.text_input.select_all();
                        self.input_mode = InputMode::Editing {
                            tension_id: entry.id,
                            field: EditField::Desire,
                        };
                    }
                }
                Cmd::none()
            }
            Msg::Char('n') | Msg::StartNote => {
                if let Some(entry) = self.action_target().cloned() {
                    self.input_buffer.clear();
                    self.input_mode = InputMode::Annotating {
                        tension_id: entry.id,
                    };
                }
                Cmd::none()
            }
            Msg::Char('r') | Msg::StartResolve => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active {
                        self.input_mode = InputMode::Confirming(ConfirmKind::Resolve {
                            tension_id: entry.id.clone(),
                            desired: entry.desired.clone(),
                        });
                    }
                }
                Cmd::none()
            }
            Msg::Char('x') | Msg::StartRelease => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active {
                        self.input_mode = InputMode::Confirming(ConfirmKind::Release {
                            tension_id: entry.id.clone(),
                            desired: entry.desired.clone(),
                        });
                    }
                }
                Cmd::none()
            }

            // Agent one-shot
            Msg::Char('@') | Msg::InvokeAgent => {
                if let Some(entry) = self.action_target().cloned() {
                    self.input_mode = InputMode::AgentPrompt { tension_id: entry.id };
                    self.input_buffer.clear();
                }
                Cmd::none()
            }

            // Undo last undoable mutation
            Msg::Char('u') | Msg::Undo => {
                if let Some(entry) = self.action_target().cloned() {
                    let mutations = self.engine.store().get_mutations(&entry.id).unwrap_or_default();
                    // Walk backwards to find the most recent undoable mutation
                    let mut undone = false;
                    for m in mutations.iter().rev() {
                        let field = m.field();
                        let old = m.old_value().map(|s| s.to_string());
                        match (field, old) {
                            ("desired", Some(old)) => {
                                let _ = self.engine.update_desired(&entry.id, &old);
                                self.set_transient("desire reverted");
                                undone = true;
                                break;
                            }
                            ("actual", Some(old)) => {
                                let _ = self.engine.update_actual(&entry.id, &old);
                                self.set_transient("reality reverted");
                                undone = true;
                                break;
                            }
                            ("status", Some(old)) => {
                                let status = match old.as_str() {
                                    "Active" => sd_core::TensionStatus::Active,
                                    "Resolved" => sd_core::TensionStatus::Resolved,
                                    "Released" => sd_core::TensionStatus::Released,
                                    _ => sd_core::TensionStatus::Active,
                                };
                                let _ = self.engine.store().update_status(&entry.id, status);
                                self.set_transient("status reverted");
                                undone = true;
                                break;
                            }
                            ("horizon", Some(old)) => {
                                if old.is_empty() {
                                    let _ = self.engine.update_horizon(&entry.id, None);
                                } else if let Ok(h) = crate::horizon::parse_horizon(&old) {
                                    let _ = self.engine.update_horizon(&entry.id, Some(h));
                                }
                                self.set_transient("horizon reverted");
                                undone = true;
                                break;
                            }
                            _ => continue, // skip non-undoable mutations (notes, created, etc.)
                        }
                    }
                    if !undone {
                        self.set_transient("nothing undoable (notes and creation can't be undone)");
                    } else {
                        self.load_siblings();
                    }
                }
                Cmd::none()
            }

            // Yank (copy tension ID to clipboard)
            Msg::Char('y') => {
                if let Some(entry) = self.action_target().cloned() {
                    let _ = self.copy_to_clipboard(&entry.id);
                    self.set_transient(format!("copied: {}", &entry.id[..12.min(entry.id.len())]));
                }
                Cmd::none()
            }

            // Reopen (for resolved/released)
            Msg::Char('o') => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status != sd_core::TensionStatus::Active {
                        let _ = self.engine.store().update_status(
                            &entry.id,
                            sd_core::TensionStatus::Active,
                        );
                        self.set_transient("reopened");
                        self.load_siblings();
                    }
                }
                Cmd::none()
            }

            // Search
            Msg::Char('/') | Msg::Search => {
                self.input_mode = InputMode::Searching;
                self.input_buffer.clear();
                self.search_state = Some(crate::search::SearchState::new());
                Cmd::none()
            }

            // Move by search
            Msg::Char('m') | Msg::StartMove => {
                if let Some(entry) = self.action_target().cloned() {
                    self.input_mode = InputMode::Moving { tension_id: entry.id };
                    self.input_buffer.clear();
                    self.search_state = Some(crate::search::SearchState::new());
                }
                Cmd::none()
            }

            // Filter
            Msg::Char('f') | Msg::CycleFilter => {
                self.filter = self.filter.cycle();
                self.load_siblings();
                self.set_transient(format!("filter: {}", self.filter.label()));
                Cmd::none()
            }

            // Insights
            Msg::Char('i') | Msg::OpenInsights => {
                // Always try to load — count may be stale
                self.load_pending_insights();
                if !self.pending_insights.is_empty() {
                    self.input_mode = InputMode::ReviewingInsights;
                } else {
                    self.set_transient("no pending insights");
                }
                Cmd::none()
            }

            // Help
            Msg::Char('?') | Msg::ToggleHelp => {
                self.input_mode = InputMode::Help;
                Cmd::none()
            }

            // Alert actions (1-9 in descended view)
            Msg::Char(c @ '1'..='9') if !self.alerts.is_empty() => {
                let idx = (c as usize) - ('1' as usize);
                if let Some(alert) = self.alerts.get(idx).cloned() {
                    match alert.kind {
                        crate::state::AlertKind::Neglect { .. } => {
                            // Open reality for editing on the parent
                            if let Some(ref pid) = self.parent_id {
                                let actual = self.parent_tension.as_ref()
                                    .map(|t| t.actual.clone()).unwrap_or_default();
                                self.input_buffer.clone_from(&actual);
                                self.text_input.set_value(&actual);
                                self.text_input.set_focused(true);
                                self.text_input.select_all();
                                self.input_mode = InputMode::Editing {
                                    tension_id: pid.clone(),
                                    field: EditField::Reality,
                                };
                            }
                        }
                        crate::state::AlertKind::HorizonPast { .. } => {
                            // Open horizon for editing on the parent
                            if let Some(ref pid) = self.parent_id {
                                let horizon_str = self.parent_tension.as_ref()
                                    .and_then(|t| t.horizon.as_ref().map(|h| h.to_string()))
                                    .unwrap_or_default();
                                self.input_buffer.clone_from(&horizon_str);
                                self.text_input.set_value(&horizon_str);
                                self.text_input.set_focused(true);
                                self.text_input.select_all();
                                self.input_mode = InputMode::Editing {
                                    tension_id: pid.clone(),
                                    field: EditField::Horizon,
                                };
                            }
                        }
                        crate::state::AlertKind::Oscillation | crate::state::AlertKind::Conflict => {
                            self.set_transient(format!("{}", alert.action_hint));
                        }
                        crate::state::AlertKind::MultipleRoots { .. } => {
                            self.set_transient("create a parent tension or reparent siblings");
                        }
                    }
                }
                Cmd::none()
            }

            // Quit
            Msg::Char('q') | Msg::Quit => Cmd::quit(),
            Msg::Cancel => {
                if self.gaze.is_some() {
                    self.close_gaze();
                }
                Cmd::none()
            }

            // Agent response received (from background task)
            Msg::AgentResponse(result) => {
                match result {
                    Ok(response) => {
                        // Try to parse structured response for mutations
                        if let Some(parsed) = werk_shared::StructuredResponse::from_response(&response) {
                            self.agent_response_text = Some(parsed.response.clone());
                            if !parsed.mutations.is_empty() {
                                self.agent_mutations = parsed.mutations.clone();
                                self.agent_mutation_selected = vec![true; self.agent_mutations.len()];
                                self.agent_mutation_cursor = 0;
                                self.input_mode = InputMode::ReviewingMutations;
                            } else {
                                self.set_transient("agent responded (no mutations)");
                            }
                        } else {
                            // Plain text response, no structured mutations
                            self.agent_response_text = Some(response);
                            self.input_mode = InputMode::ReviewingMutations;
                        }
                    }
                    Err(e) => {
                        self.set_transient(format!("agent error: {}", truncate_str(&e, 40)));
                    }
                }
                Cmd::none()
            }

            Msg::Tick => {
                if let Some(ref t) = self.transient {
                    if t.is_expired() {
                        self.transient = None;
                    }
                }
                // Only reload if the database file has changed (external modification)
                if self.db_has_changed() {
                    self.load_siblings();
                }
                self.refresh_pending_insight_count();
                Cmd::none()
            }

            // In normal mode, RawEvent carries Left/Right that should be navigation
            Msg::RawEvent(Event::Key(key)) => {
                match key.code {
                    ftui::KeyCode::Left => self.update_normal(Msg::Ascend),
                    ftui::KeyCode::Right => self.update_normal(Msg::Descend),
                    _ => Cmd::none(),
                }
            }

            _ => Cmd::none(),
        }
    }

    fn update_reordering(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            // Shift+J/K moves the grabbed tension
            Msg::Char('K') | Msg::MoveUp => {
                self.reorder_move_up();
                Cmd::none()
            }
            Msg::Char('J') | Msg::MoveDown => {
                self.reorder_move_down();
                Cmd::none()
            }
            // Enter/Space commits
            Msg::Submit | Msg::Char(' ') => {
                self.reorder_commit();
                Cmd::none()
            }
            // Esc cancels
            Msg::Cancel => {
                self.reorder_cancel();
                Cmd::none()
            }
            // Everything else: ignore (stay in reorder mode)
            _ => Cmd::none(),
        }
    }

    fn update_help(&mut self, msg: Msg) -> Cmd<Msg> {
        // Help overlay swallows ALL input — just closes on any key
        match msg {
            Msg::Noop | Msg::Tick => Cmd::none(),
            _ => {
                self.input_mode = InputMode::Normal;
                Cmd::none()
            }
        }
    }

    fn update_adding(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char(c) => {
                self.input_buffer.push(c);
                Cmd::none()
            }
            Msg::Backspace => {
                if self.input_buffer.is_empty() {
                    match &self.input_mode {
                        InputMode::Adding(AddStep::Name) => {
                            self.input_mode = InputMode::Normal;
                        }
                        InputMode::Adding(AddStep::Desire { name }) => {
                            self.input_buffer = name.clone();
                            self.input_mode = InputMode::Adding(AddStep::Name);
                        }
                        InputMode::Adding(AddStep::Reality { name, desire }) => {
                            let (n, d) = (name.clone(), desire.clone());
                            self.input_buffer = d;
                            self.input_mode = InputMode::Adding(AddStep::Desire { name: n });
                        }
                        InputMode::Adding(AddStep::Horizon { name, desire, reality }) => {
                            let (n, d, r) = (name.clone(), desire.clone(), reality.clone());
                            self.input_buffer = r;
                            self.input_mode = InputMode::Adding(AddStep::Reality { name: n, desire: d });
                        }
                        _ => {}
                    }
                } else {
                    self.input_buffer.pop();
                }
                Cmd::none()
            }
            Msg::Submit => {
                let buf = self.input_buffer.clone();
                if buf.is_empty() {
                    return Cmd::none();
                }
                match self.input_mode.clone() {
                    InputMode::Adding(AddStep::Name) => {
                        self.input_buffer.clear();
                        self.input_mode = InputMode::Adding(AddStep::Desire { name: buf });
                    }
                    InputMode::Adding(AddStep::Desire { name }) => {
                        // Pre-fill reality with parent's actual if we're creating a child
                        let prefill = self.parent_id.as_ref().and_then(|pid| {
                            self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                        }).unwrap_or_default();
                        self.input_buffer = prefill;
                        self.input_mode = InputMode::Adding(AddStep::Reality { name, desire: buf });
                    }
                    InputMode::Adding(AddStep::Reality { name, desire }) => {
                        self.input_buffer.clear();
                        self.input_mode = InputMode::Adding(AddStep::Horizon { name, desire, reality: buf });
                    }
                    InputMode::Adding(AddStep::Horizon { name, desire, reality }) => {
                        self.create_tension_with_horizon(&name, &desire, &reality, &buf);
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    }
                    _ => {}
                }
                Cmd::none()
            }
            Msg::Cancel | Msg::Tab => {
                // Esc/Tab: skip remaining optional steps, but reality is required
                match self.input_mode.clone() {
                    InputMode::Adding(AddStep::Name) => {
                        // Cancel entirely
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    }
                    InputMode::Adding(AddStep::Desire { name }) => {
                        // Skip desire (use name as desire), advance to reality
                        let prefill = self.parent_id.as_ref().and_then(|pid| {
                            self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                        }).unwrap_or_default();
                        self.input_buffer = prefill;
                        self.input_mode = InputMode::Adding(AddStep::Reality { name, desire: String::new() });
                    }
                    InputMode::Adding(AddStep::Reality { name, desire }) => {
                        // Reality is required — if buffer has content, use it and skip horizon
                        let buf = self.input_buffer.clone();
                        if !buf.is_empty() {
                            self.create_tension(&name, &desire, &buf);
                            self.input_mode = InputMode::Normal;
                            self.input_buffer.clear();
                        }
                        // If empty, stay on Reality step (don't skip)
                    }
                    InputMode::Adding(AddStep::Horizon { name, desire, reality }) => {
                        // Skip horizon, create with what we have
                        self.create_tension(&name, &desire, &reality);
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    }
                    _ => {}
                }
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn update_editing(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            // Intercept structural keys before TextInput
            Msg::Submit => {
                self.sync_text_input_to_buffer();
                self.save_current_edit_field();
                self.set_transient("saved");
                self.reload_after_edit();
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.text_input.set_focused(false);
                Cmd::none()
            }
            Msg::Tab => {
                // Save current field, cycle to next: desire → reality → horizon → desire
                self.sync_text_input_to_buffer();
                self.save_current_edit_field();
                if let InputMode::Editing { ref tension_id, ref field } = self.input_mode.clone() {
                    let new_field = match field {
                        EditField::Desire => EditField::Reality,
                        EditField::Reality => EditField::Horizon,
                        EditField::Horizon => EditField::Desire,
                    };
                    // Load the new field's content from the store (re-read to get saved value)
                    let new_buf = if let Ok(Some(t)) = self.engine.store().get_tension(tension_id) {
                        match new_field {
                            EditField::Desire => t.desired.clone(),
                            EditField::Reality => t.actual.clone(),
                            EditField::Horizon => t.horizon.map(|h| h.to_string()).unwrap_or_default(),
                        }
                    } else {
                        String::new()
                    };
                    self.input_buffer.clone_from(&new_buf);
                    self.text_input.set_value(&new_buf);
                    self.text_input.select_all();
                    self.input_mode = InputMode::Editing {
                        tension_id: tension_id.clone(),
                        field: new_field,
                    };
                }
                Cmd::none()
            }
            Msg::Cancel => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.text_input.set_focused(false);
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),

            // Forward everything else to TextInput via raw event
            Msg::RawEvent(ref event) => {
                self.text_input.handle_event(event);
                self.sync_text_input_to_buffer();
                Cmd::none()
            }

            // Char and Backspace: synthesize events for TextInput
            Msg::Char(c) => {
                let event = Event::Key(ftui::KeyEvent::new(ftui::KeyCode::Char(c)));
                self.text_input.handle_event(&event);
                self.sync_text_input_to_buffer();
                Cmd::none()
            }
            Msg::Backspace => {
                let event = Event::Key(ftui::KeyEvent::new(ftui::KeyCode::Backspace));
                self.text_input.handle_event(&event);
                self.sync_text_input_to_buffer();
                Cmd::none()
            }

            _ => Cmd::none(),
        }
    }

    /// Sync TextInput value back to input_buffer (used by save logic).
    fn sync_text_input_to_buffer(&mut self) {
        self.input_buffer = self.text_input.value().to_string();
    }

    fn save_current_edit_field(&mut self) {
        let buf = self.input_buffer.clone();
        if let InputMode::Editing { ref tension_id, ref field } = self.input_mode.clone() {
            match field {
                EditField::Desire => {
                    let _ = self.engine.update_desired(tension_id, &buf);
                }
                EditField::Reality => {
                    let _ = self.engine.update_actual(tension_id, &buf);
                }
                EditField::Horizon => {
                    if buf.is_empty() {
                        let _ = self.engine.update_horizon(tension_id, None);
                    } else if let Ok(h) = crate::horizon::parse_horizon(&buf) {
                        let _ = self.engine.update_horizon(tension_id, Some(h));
                    }
                }
            }
        }
    }

    fn reload_after_edit(&mut self) {
        self.load_siblings();
        if let Some(ref gaze) = self.gaze {
            let id = self.siblings.get(gaze.index).map(|e| e.id.clone());
            if let Some(id) = id {
                self.gaze_data = self.compute_gaze(&id);
            }
        }
    }

    fn update_annotating(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char(c) => {
                self.input_buffer.push(c);
                Cmd::none()
            }
            Msg::Backspace => {
                self.input_buffer.pop();
                Cmd::none()
            }
            Msg::Submit => {
                let buf = self.input_buffer.clone();
                if !buf.is_empty() {
                    if let InputMode::Annotating { ref tension_id } = self.input_mode.clone() {
                        let _ = self.engine.store().record_mutation(
                            &sd_core::Mutation::new(
                                tension_id.clone(),
                                chrono::Utc::now(),
                                "note".to_owned(),
                                None,
                                buf,
                            ),
                        );
                        self.set_transient("note added");
                        self.load_siblings();
                    }
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Cancel => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn update_confirming(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char('y') | Msg::Submit => {
                match self.input_mode.clone() {
                    InputMode::Confirming(ConfirmKind::Resolve { tension_id, desired }) => {
                        let _ = self.engine.resolve(&tension_id);
                        self.set_transient(format!("resolved: {}", truncate_str(&desired, 30)));
                        self.load_siblings();
                    }
                    InputMode::Confirming(ConfirmKind::Release { tension_id, desired }) => {
                        let _ = self.engine.release(&tension_id);
                        self.set_transient(format!("released: {}", truncate_str(&desired, 30)));
                        self.load_siblings();
                    }
                    _ => {}
                }
                self.input_mode = InputMode::Normal;
                Cmd::none()
            }
            Msg::Char('n') | Msg::Cancel => {
                self.input_mode = InputMode::Normal;
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    fn update_searching(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char(c) => {
                self.input_buffer.push(c);
                self.refresh_search_results(false);
                Cmd::none()
            }
            Msg::Backspace => {
                self.input_buffer.pop();
                self.refresh_search_results(false);
                Cmd::none()
            }
            Msg::Up => {
                if let Some(ref mut s) = self.search_state {
                    if s.cursor > 0 {
                        s.cursor -= 1;
                    }
                }
                Cmd::none()
            }
            Msg::Down => {
                if let Some(ref mut s) = self.search_state {
                    if s.cursor + 1 < s.results.len() {
                        s.cursor += 1;
                    }
                }
                Cmd::none()
            }
            Msg::Submit => {
                // Navigate to selected result
                if let Some(ref s) = self.search_state {
                    if let Some(result) = s.selected().cloned() {
                        // Navigate to the tension's parent level, cursor on the tension
                        let parent_id = if let Ok(Some(t)) = self.engine.store().get_tension(&result.id) {
                            t.parent_id.clone()
                        } else {
                            None
                        };
                        self.parent_id = parent_id;
                        self.load_siblings();
                        if let Some(idx) = self.siblings.iter().position(|s| s.id == result.id) {
                            self.vlist.cursor = idx;
                        }
                    }
                }
                self.input_mode = InputMode::Normal;
                self.search_state = None;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Cancel => {
                self.input_mode = InputMode::Normal;
                self.search_state = None;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn update_moving(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char(c) => {
                self.input_buffer.push(c);
                self.refresh_search_results(true);
                Cmd::none()
            }
            Msg::Backspace => {
                self.input_buffer.pop();
                self.refresh_search_results(true);
                Cmd::none()
            }
            Msg::Up => {
                if let Some(ref mut s) = self.search_state {
                    if s.cursor > 0 { s.cursor -= 1; }
                }
                Cmd::none()
            }
            Msg::Down => {
                if let Some(ref mut s) = self.search_state {
                    if s.cursor + 1 < s.results.len() { s.cursor += 1; }
                }
                Cmd::none()
            }
            Msg::Submit => {
                let tension_id = match &self.input_mode {
                    InputMode::Moving { tension_id } => tension_id.clone(),
                    _ => return Cmd::none(),
                };
                if let Some(ref s) = self.search_state {
                    if let Some(result) = s.selected().cloned() {
                        let new_parent = if result.is_root_entry {
                            None
                        } else {
                            Some(result.id.as_str())
                        };
                        let _ = self.engine.update_parent(&tension_id, new_parent);
                        self.set_transient(format!("moved to {}", if result.is_root_entry { "root" } else { &result.desired }));
                        self.load_siblings();
                    }
                }
                self.input_mode = InputMode::Normal;
                self.search_state = None;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Cancel => {
                self.input_mode = InputMode::Normal;
                self.search_state = None;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    // -----------------------------------------------------------------------
    // Agent prompt
    // -----------------------------------------------------------------------

    fn update_agent_prompt(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char(c) => {
                // Check for @! clipboard handoff
                if c == '!' {
                    if let InputMode::AgentPrompt { ref tension_id } = self.input_mode.clone() {
                        let context = self.build_agent_context_for_clipboard(tension_id);
                        let prompt = if self.input_buffer.is_empty() {
                            "Analyze this tension and suggest what to do next.".to_string()
                        } else {
                            self.input_buffer.clone()
                        };
                        let short = &tension_id[..12.min(tension_id.len())];
                        let full = format!("{}\n\nUser request: {}\n\nYou can query more info and make changes directly via the werk CLI:\n\n  werk show {}          # full tension details + history\n  werk context {}       # JSON with dynamics\n  werk notes {}         # notes for this tension\n  werk tree             # forest overview\n  werk list             # all tensions\n  werk help             # all commands\n\n  werk reality {} \"new reality\"\n  werk desire {} \"new desire\"\n  werk note {} \"your note\"\n  werk resolve {}\n  werk release --reason \"reason\" {}\n  werk horizon {} \"2w\"\n  werk add -p {} \"child desired\" \"child actual\"",
                            context, prompt,
                            short, short, short,
                            short, short, short, short, short, short, short,
                        );
                        let _ = self.copy_to_clipboard(&full);
                        self.set_transient("copied to clipboard \u{2014} paste into your agent");
                    }
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                    return Cmd::none();
                }
                self.input_buffer.push(c);
                Cmd::none()
            }
            Msg::Backspace => {
                self.input_buffer.pop();
                Cmd::none()
            }
            Msg::Submit => {
                let prompt = self.input_buffer.clone();
                if prompt.is_empty() {
                    return Cmd::none();
                }
                let tension_id = match &self.input_mode {
                    InputMode::AgentPrompt { tension_id } => tension_id.clone(),
                    _ => return Cmd::none(),
                };

                // Build the full prompt with context
                let context = self.build_agent_context(&tension_id);

                let mut full_prompt = context.clone();

                // If there's a prior exchange, include both the user's question and the agent's response
                if let (Some(ref prev_q), Some(ref prev_a)) = (&self.agent_last_user_message, &self.agent_response_text) {
                    full_prompt.push_str("\n\n--- Previous exchange ---\n");
                    full_prompt.push_str(&format!("User: {}\n\n", prev_q));
                    full_prompt.push_str(&format!("Assistant: {}\n", prev_a));
                    full_prompt.push_str("--- End previous exchange ---\n");
                } else if let Some(ref prev_a) = self.agent_response_text {
                    full_prompt.push_str("\n\n--- Previous agent response ---\n");
                    full_prompt.push_str(prev_a);
                    full_prompt.push_str("\n--- End previous response ---\n");
                }
                full_prompt.push_str(&format!("\nUser: {}", prompt));

                // Store this message for next follow-up
                self.agent_last_user_message = Some(prompt);

                // Get agent command from config
                let agent_cmd = self.get_agent_command();

                self.agent_tension_id = Some(tension_id);
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.set_transient("running agent...");

                if let Some(cmd) = agent_cmd {
                    // Run agent in background via Cmd::task
                    let cmd_clone = cmd.clone();
                    Cmd::task_named("agent", move || {
                        let result = crate::agent::execute_agent_oneshot(&cmd_clone, &full_prompt);
                        Msg::AgentResponse(result)
                    })
                } else {
                    self.set_transient("no agent configured \u{2014} run: werk config set agent.command <cmd>");
                    Cmd::none()
                }
            }
            Msg::Cancel => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn update_mutation_review(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            // Follow-up: @ while viewing response opens prompt overlay on this screen
            Msg::Char('@') => {
                if let Some(ref tid) = self.agent_tension_id.clone() {
                    // Switch to AgentPrompt but keep the response visible
                    // (AgentPrompt rendering will overlay on the response view)
                    self.input_mode = InputMode::AgentPrompt { tension_id: tid.clone() };
                    self.input_buffer.clear();
                }
                return Cmd::none();
            }
            Msg::Up | Msg::Char('k') => {
                if self.agent_mutation_cursor > 0 {
                    self.agent_mutation_cursor -= 1;
                }
                Cmd::none()
            }
            Msg::Down | Msg::Char('j') => {
                if self.agent_mutation_cursor + 1 < self.agent_mutations.len() {
                    self.agent_mutation_cursor += 1;
                }
                Cmd::none()
            }
            Msg::Char(' ') => {
                // Toggle selection
                if let Some(sel) = self.agent_mutation_selected.get_mut(self.agent_mutation_cursor) {
                    *sel = !*sel;
                }
                Cmd::none()
            }
            Msg::Char('a') | Msg::Submit => {
                // Apply selected mutations
                self.apply_selected_mutations();
                self.input_mode = InputMode::Normal;
                Cmd::none()
            }
            Msg::Cancel => {
                self.agent_mutations.clear();
                self.agent_mutation_selected.clear();
                self.agent_response_text = None;
                self.input_mode = InputMode::Normal;
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn update_insight_review(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char(' ') => {
                // Toggle expanded view for current insight
                if let Some(insight) = self.pending_insights.get_mut(self.insight_cursor) {
                    insight.expanded = !insight.expanded;
                }
                Cmd::none()
            }
            Msg::Up | Msg::Char('k') => {
                if self.insight_cursor > 0 {
                    self.insight_cursor -= 1;
                }
                Cmd::none()
            }
            Msg::Down | Msg::Char('j') => {
                if self.insight_cursor + 1 < self.pending_insights.len() {
                    self.insight_cursor += 1;
                }
                Cmd::none()
            }
            Msg::Char('a') | Msg::Submit => {
                // Accept/apply: mark reviewed, add note mutation
                self.accept_current_insight();
                Cmd::none()
            }
            Msg::Char('d') => {
                // Dismiss: mark reviewed without action
                self.dismiss_current_insight();
                Cmd::none()
            }
            Msg::Cancel => {
                // Close insight review without marking anything
                self.pending_insights.clear();
                self.input_mode = InputMode::Normal;
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn refresh_search_results(&mut self, include_root: bool) {
        let results = if include_root {
            crate::search::search_all_with_root(&self.input_buffer, &self.engine)
        } else {
            crate::search::search_all(&self.input_buffer, &self.engine)
        };
        if let Some(ref mut s) = self.search_state {
            s.query = self.input_buffer.clone();
            s.results = results;
            s.cursor = 0;
        }
    }

    // -----------------------------------------------------------------------
    // Gaze helpers
    // -----------------------------------------------------------------------

    fn toggle_gaze(&mut self) {
        if let Some(gaze) = self.gaze.clone() {
            if gaze.index == self.vlist.cursor {
                self.close_gaze();
                return;
            }
        }
        let idx = self.vlist.cursor;
        let entry_id = self.siblings.get(idx).map(|e| e.id.clone());
        if let Some(id) = entry_id {
            let gaze_data = self.compute_gaze(&id);
            self.gaze = Some(GazeState {
                index: idx,
                full: false,
            });
            self.gaze_data = gaze_data;
            let height = self.quick_gaze_height();
            self.vlist.reset_heights();
            self.vlist.set_height(idx, height);
        }
    }

    fn close_gaze(&mut self) {
        if self.gaze.is_some() {
            self.gaze = None;
            self.gaze_data = None;
            self.full_gaze_data = None;
            self.vlist.reset_heights();
        }
    }

    fn quick_gaze_height(&self) -> usize {
        // Panel (2 border lines) + children + reality
        let mut h = 2; // panel top + bottom border
        if let Some(ref data) = self.gaze_data {
            h += data.children.len().max(1); // at least "no children" line
            if !data.actual.is_empty() {
                h += 2; // separator + at least 1 reality line
            }
        } else {
            h += 1; // "no children" line
        }
        h
    }

    // -----------------------------------------------------------------------
    // Data operations
    // -----------------------------------------------------------------------

    fn create_tension(&mut self, name: &str, desire: &str, reality: &str) {
        let desired = if desire.is_empty() { name } else { desire };
        let actual = reality;
        let parent = self.parent_id.clone();

        let result = self
            .engine
            .create_tension_with_parent(desired, actual, parent);

        if let Ok(tension) = result {
            self.set_transient(format!("created: {}", truncate_str(&tension.desired, 30)));
            self.load_siblings();
            if let Some(idx) = self.siblings.iter().position(|s| s.id == tension.id) {
                self.vlist.cursor = idx;
            }
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
