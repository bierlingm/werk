//! Update logic for the Operative Instrument.

use ftui::{Cmd, Event, Frame, Model};
use ftui::layout::{Constraint, Flex, Rect};
use ftui::runtime::subscription::Subscription;

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

        // Check for external DB changes on user input (not reorder — preserves drag state)
        if !matches!(msg, Msg::Tick | Msg::Noop)
            && !matches!(self.input_mode, InputMode::Reordering { .. })
            && self.db_has_changed()
        {
            self.load_siblings();
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
            InputMode::Reordering { .. } => self.update_reordering(msg),
            InputMode::Pathway => self.update_pathway(msg),
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
        } else if matches!(self.input_mode, InputMode::Searching) {
            crate::helpers::clear_area(frame, rects[0]);
            self.render_search(&rects[0], frame);
        } else if matches!(self.input_mode, InputMode::Moving { .. }) {
            crate::helpers::clear_area(frame, rects[0]);
            self.render_search(&rects[0], frame);
        } else if self.siblings.is_empty() && self.parent_id.is_none()
            && !matches!(self.input_mode, InputMode::Adding(_))
        {
            self.render_empty(&rects[0], frame);
        } else if self.use_deck && self.parent_id.is_some() {
            // New deck rendering (V1+) for descended views
            self.render_deck(&rects[0], frame);
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
            InputMode::Pathway => {
                self.render_pathway(&rects[0], frame);
            }
            // Full-screen modes already rendered above
            _ => {}
        }

        // Bottom bar: deck bar in deck mode, old lever otherwise
        if self.use_deck && self.parent_id.is_some() {
            self.render_deck_bar(&rects[1], frame);
        } else {
            self.render_lever(&rects[1], frame);
        }

        // Hints — show contextual hints for input modes
        if show_hints {
            match &self.input_mode {
                InputMode::Adding(_) => self.render_input_hints("Enter create  Tab more fields  Esc cancel  Bksp back", &rects[2], frame),
                InputMode::Confirming(_) => self.render_input_hints("y confirm  n cancel", &rects[2], frame),
                InputMode::Editing { .. } => self.render_input_hints("Enter save  Tab more fields  Esc cancel", &rects[2], frame),
                InputMode::Annotating { .. } => self.render_input_hints("Enter save  Esc cancel", &rects[2], frame),
                InputMode::Searching => self.render_input_hints("Enter jump  j/k navigate  Esc cancel", &rects[2], frame),
                InputMode::Moving { .. } => self.render_input_hints("Enter place here  \u{2191}/\u{2193} navigate  Esc cancel", &rects[2], frame),
                InputMode::Reordering { .. } => self.render_input_hints("Shift+J/K move  Enter drop  Esc cancel", &rects[2], frame),
                _ => self.render_hints(&rects[2], frame),
            }
        }
    }

    fn subscriptions(&self) -> Vec<Box<dyn Subscription<Msg>>> {
        // No recurring tick — we schedule one-shot ticks on demand to avoid
        // unnecessary redraws that cause terminal flicker.
        vec![]
    }
}

impl InstrumentApp {
    fn update_normal(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            // Navigation
            Msg::Char('k') | Msg::Up => {
                if self.use_deck && self.parent_id.is_some() {
                    // Clear focus on cursor move
                    if self.deck_zoom == crate::deck::ZoomLevel::Focus {
                        self.deck_zoom = crate::deck::ZoomLevel::Normal;
                        self.focused_detail = None;
                    }
                    self.deck_pitch_up();
                } else {
                    self.vlist.up();
                    self.close_gaze();
                }
                Cmd::none()
            }
            Msg::Char('j') | Msg::Down => {
                if self.use_deck && self.parent_id.is_some() {
                    // Clear focus on cursor move
                    if self.deck_zoom == crate::deck::ZoomLevel::Focus {
                        self.deck_zoom = crate::deck::ZoomLevel::Normal;
                        self.focused_detail = None;
                    }
                    self.deck_pitch_down();
                } else {
                    self.vlist.down();
                    self.close_gaze();
                }
                Cmd::none()
            }

            // Reorder: Shift+J/K enters grab mode and does first move
            Msg::Char('K') | Msg::MoveUp => {
                if self.enter_reorder() {
                    self.reorder_move_up();
                } else {
                    self.session_log.record(crate::session_log::Category::Reorder,
                        "enter_reorder failed (cursor not on reorderable item)");
                }
                Cmd::none()
            }
            Msg::Char('J') | Msg::MoveDown => {
                if self.enter_reorder() {
                    self.reorder_move_down();
                } else {
                    self.session_log.record(crate::session_log::Category::Reorder,
                        "enter_reorder failed (cursor not on reorderable item)");
                }
                Cmd::none()
            }

            Msg::Char('l') | Msg::Descend => {
                // Roll right: descend into selected tension
                if self.use_deck && self.parent_id.is_some() {
                    if let Some(idx) = self.deck_selected_sibling_index() {
                        let id = self.siblings[idx].id.clone();
                        self.descend(&id);
                    }
                } else if let Some(entry) = self.action_target().cloned() {
                    self.descend(&entry.id);
                }
                Cmd::none()
            }
            Msg::Submit => {
                if self.use_deck && self.parent_id.is_some() {
                    // V7: Enter toggles focus zoom on the selected element
                    if self.deck_zoom == crate::deck::ZoomLevel::Focus {
                        // Already focused — unfocus
                        self.deck_zoom = crate::deck::ZoomLevel::Normal;
                        self.focused_detail = None;
                    } else if let Some(entry) = self.action_target().cloned() {
                        // Load focus detail with full FieldEntry children
                        let now = chrono::Utc::now();
                        let raw_children = self.engine.store()
                            .get_children(&entry.id)
                            .unwrap_or_default();
                        let child_ids: Vec<&str> = raw_children.iter().map(|c| c.id.as_str()).collect();
                        let child_counts = self.engine.store()
                            .count_children_by_parent(&child_ids)
                            .unwrap_or_default();
                        let children: Vec<crate::state::FieldEntry> = raw_children.iter().map(|c| {
                            let cc = child_counts.get(&c.id).copied().unwrap_or(0);
                            crate::state::FieldEntry::from_tension(c, c.created_at, cc, c.created_at, now)
                        }).collect();
                        let deadline_label = entry.horizon_label.clone();
                        let sibling_idx = self.deck_selected_sibling_index().unwrap_or(0);
                        self.focused_detail = Some(crate::deck::FocusedDetail {
                            sibling_index: sibling_idx,
                            desired: entry.desired.clone(),
                            actual: entry.actual.clone(),
                            children,
                            short_code: entry.short_code,
                            deadline_label,
                        });
                        self.deck_zoom = crate::deck::ZoomLevel::Focus;
                    }
                } else if let Some(entry) = self.action_target().cloned() {
                    self.descend(&entry.id);
                }
                Cmd::none()
            }
            // Shift+Enter — orient zoom (V9 placeholder)
            Msg::ShiftSubmit => {
                if self.use_deck && self.parent_id.is_some() {
                    self.set_transient("orient zoom: coming soon");
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
                if self.use_deck && self.parent_id.is_some() {
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                    self.deck_cursor.index = 0;
                } else {
                    self.vlist.top();
                    self.close_gaze();
                }
                Cmd::none()
            }
            Msg::Char('G') | Msg::JumpBottom => {
                if self.use_deck && self.parent_id.is_some() {
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                    let frontier = crate::deck::Frontier::compute(&self.siblings, self.trajectory_mode, self.epoch_boundary);
                    self.deck_cursor.index = frontier.selectable_count().saturating_sub(1);
                } else {
                    self.vlist.bottom();
                    self.close_gaze();
                }
                Cmd::none()
            }

            // Space: peek in deck mode, gaze in field mode
            Msg::Char(' ') | Msg::ToggleGaze => {
                if self.use_deck && self.parent_id.is_some() {
                    // V8: peek — inline children preview, lighter than focus
                    if self.deck_zoom == crate::deck::ZoomLevel::Focus {
                        // Already peeking/focused — close
                        self.deck_zoom = crate::deck::ZoomLevel::Normal;
                        self.focused_detail = None;
                    } else if let Some(entry) = self.action_target().cloned() {
                        if entry.child_count > 0 {
                            // Load children only (no deep reality for peek)
                            let now = chrono::Utc::now();
                            let raw_children = self.engine.store()
                                .get_children(&entry.id)
                                .unwrap_or_default();
                            let child_ids: Vec<&str> = raw_children.iter().map(|c| c.id.as_str()).collect();
                            let child_counts = self.engine.store()
                                .count_children_by_parent(&child_ids)
                                .unwrap_or_default();
                            let children: Vec<crate::state::FieldEntry> = raw_children.iter().map(|c| {
                                let cc = child_counts.get(&c.id).copied().unwrap_or(0);
                                crate::state::FieldEntry::from_tension(c, c.created_at, cc, c.created_at, now)
                            }).collect();
                            let sibling_idx = self.deck_selected_sibling_index().unwrap_or(0);
                            self.focused_detail = Some(crate::deck::FocusedDetail {
                                sibling_index: sibling_idx,
                                desired: entry.desired.clone(),
                                actual: String::new(), // peek = no reality
                                children,
                                short_code: entry.short_code,
                                deadline_label: entry.horizon_label.clone(),
                            });
                            self.deck_zoom = crate::deck::ZoomLevel::Focus;
                        }
                    }
                } else {
                    self.toggle_gaze();
                }
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

            // Position toggle: held → position (next step), positioned → hold
            Msg::Char('p') => {
                if let Some(entry) = self.action_target().cloned() {
                    let entry_id = entry.id.clone();
                    if entry.status != sd_core::TensionStatus::Active {
                        self.set_transient("only active steps can be positioned");
                    } else if entry.position.is_some() {
                        // Positioned → hold
                        let _ = self.engine.update_position(&entry_id, None);
                        self.set_transient("held");
                        self.load_siblings();
                        // Track cursor to the moved item
                        if let Some(idx) = self.siblings.iter().position(|s| s.id == entry_id) {
                            self.deck_cursor_to_sibling(idx);
                        }
                    } else {
                        // Held → position at 1 (bottom of sequence = next to act on)
                        let _ = self.engine.update_position(&entry_id, Some(1));
                        self.set_transient("positioned");
                        self.load_siblings();
                        // Track cursor to the moved item
                        if let Some(idx) = self.siblings.iter().position(|s| s.id == entry_id) {
                            self.deck_cursor_to_sibling(idx);
                        }
                        self.check_sequencing_palette(&entry_id);
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

            // Toggle deck view (new V1 rendering)
            Msg::Char('D') => {
                self.use_deck = !self.use_deck;
                self.set_transient(if self.use_deck { "deck view" } else { "field view" });
                Cmd::none()
            }

            // Toggle trajectory mode (Q30: resolved stay in-place on route)
            Msg::Char('T') => {
                if self.use_deck && self.parent_id.is_some() {
                    self.trajectory_mode = !self.trajectory_mode;
                    self.set_transient(if self.trajectory_mode { "trajectory view" } else { "frontier view" });
                    self.deck_cursor_reset();
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

            // In deck mode: ? = edit parent reality (V4 quick-edit)
            // In field mode: ? = help
            Msg::Char('?') | Msg::ToggleHelp => {
                if self.use_deck && self.parent_id.is_some() {
                    if let Some(ref pid) = self.parent_id.clone() {
                        let actual = self.parent_tension.as_ref()
                            .map(|t| t.actual.clone()).unwrap_or_default();
                        self.input_buffer = actual.clone();
                        self.text_input.set_value(&actual);
                        self.text_input.set_focused(true);
                        self.text_input.select_all();
                        self.input_mode = InputMode::Editing {
                            tension_id: pid.clone(),
                            field: EditField::Reality,
                        };
                    }
                } else {
                    self.input_mode = InputMode::Help;
                }
                Cmd::none()
            }

            // In deck mode: ! = edit parent desire (V4 quick-edit)
            Msg::Char('!') => {
                if self.use_deck && self.parent_id.is_some() {
                    if let Some(ref pid) = self.parent_id.clone() {
                        let desired = self.parent_tension.as_ref()
                            .map(|t| t.desired.clone()).unwrap_or_default();
                        self.input_buffer = desired.clone();
                        self.text_input.set_value(&desired);
                        self.text_input.set_focused(true);
                        self.text_input.select_all();
                        self.input_mode = InputMode::Editing {
                            tension_id: pid.clone(),
                            field: EditField::Desire,
                        };
                    }
                }
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
                        crate::state::AlertKind::MultipleRoots { .. } => {
                            self.set_transient("create a parent tension or reparent siblings");
                        }
                    }
                }
                Cmd::none()
            }

            // Dump session log
            Msg::Char('`') => {
                self.dump_session_log();
                Cmd::none()
            }

            // Quit
            Msg::Char('q') | Msg::Quit => Cmd::quit(),
            Msg::Cancel => {
                if self.deck_zoom == crate::deck::ZoomLevel::Focus {
                    // V7: Esc exits focus zoom
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                } else if self.gaze.is_some() {
                    self.close_gaze();
                }
                Cmd::none()
            }

            Msg::Tick => Cmd::none(),

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
            // J/K/Shift+J/K/arrows all move in reorder mode
            Msg::Char('K') | Msg::Char('k') | Msg::MoveUp | Msg::Up => {
                self.reorder_move_up();
                Cmd::none()
            }
            Msg::Char('J') | Msg::Char('j') | Msg::MoveDown | Msg::Down => {
                self.reorder_move_down();
                Cmd::none()
            }
            // g/G: jump to top/bottom of active list
            Msg::Char('g') => {
                while self.vlist.cursor > 0 {
                    self.reorder_move_up();
                }
                Cmd::none()
            }
            Msg::Char('G') => {
                while self.vlist.cursor < self.siblings.len().saturating_sub(1) {
                    let prev = self.vlist.cursor;
                    self.reorder_move_down();
                    if self.vlist.cursor == prev { break; } // hit bottom
                }
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
            // Quit always works (Ctrl+C or q)
            Msg::Char('q') | Msg::Quit => Cmd::quit(),
            // Everything else: ignore (stay in reorder mode)
            _ => Cmd::none(),
        }
    }

    fn update_pathway(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Noop | Msg::Tick => Cmd::none(),
            Msg::Char('j') | Msg::Down => {
                if let Some(ref mut pw) = self.pathway_state {
                    if pw.cursor + 1 < pw.palette.options.len() {
                        pw.cursor += 1;
                    }
                }
                Cmd::none()
            }
            Msg::Char('k') | Msg::Up => {
                if let Some(ref mut pw) = self.pathway_state {
                    if pw.cursor > 0 {
                        pw.cursor -= 1;
                    }
                }
                Cmd::none()
            }
            // Number keys select directly (1-9)
            Msg::Char(c @ '1'..='9') => {
                let idx = (c as usize) - ('1' as usize);
                if let Some(ref pw) = self.pathway_state {
                    if idx < pw.palette.options.len() {
                        self.apply_pathway_choice(idx);
                    }
                }
                Cmd::none()
            }
            Msg::Submit => {
                if let Some(ref pw) = self.pathway_state {
                    let idx = pw.cursor;
                    self.apply_pathway_choice(idx);
                }
                Cmd::none()
            }
            Msg::Cancel | Msg::Char('q') => {
                // Dismiss = option index 0 (keep as-is)
                self.apply_pathway_choice(0);
                Cmd::none()
            }
            Msg::Quit => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn apply_pathway_choice(&mut self, option_index: usize) {
        if let Some(pw) = self.pathway_state.take() {
            let choice = if pw.palette.options.get(option_index)
                .map(|o| o.action == "dismiss")
                .unwrap_or(true)
            {
                werk_shared::palette::PaletteChoice::Dismissed
            } else {
                werk_shared::palette::PaletteChoice::Selected(option_index)
            };

            match werk_shared::palette::apply_choice(
                self.engine.store_mut(),
                &pw.context,
                &choice,
            ) {
                Ok(Some(msg)) => self.set_transient(msg),
                Ok(None) => self.set_transient("dismissed"),
                Err(e) => self.set_transient(format!("palette error: {}", e)),
            }

            self.load_siblings();
        }
        self.input_mode = InputMode::Normal;
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
                // Enter = create now with what I have, intelligent defaults for the rest.
                let buf = self.input_buffer.clone();
                match self.input_mode.clone() {
                    InputMode::Adding(AddStep::Name) => {
                        if buf.is_empty() { return Cmd::none(); } // name is required
                        // Name becomes desire, reality from parent or empty
                        let reality = self.parent_id.as_ref().and_then(|pid| {
                            self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                        }).unwrap_or_default();
                        self.create_tension(&buf, "", &reality);
                    }
                    InputMode::Adding(AddStep::Desire { name }) => {
                        let desire = if buf.is_empty() { String::new() } else { buf };
                        let reality = self.parent_id.as_ref().and_then(|pid| {
                            self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                        }).unwrap_or_default();
                        self.create_tension(&name, &desire, &reality);
                    }
                    InputMode::Adding(AddStep::Reality { name, desire }) => {
                        let reality = if buf.is_empty() {
                            self.parent_id.as_ref().and_then(|pid| {
                                self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                            }).unwrap_or_default()
                        } else { buf };
                        self.create_tension(&name, &desire, &reality);
                    }
                    InputMode::Adding(AddStep::Horizon { name, desire, reality }) => {
                        if buf.is_empty() {
                            self.create_tension(&name, &desire, &reality);
                        } else {
                            self.create_tension_with_horizon(&name, &desire, &reality, &buf);
                        }
                    }
                    _ => {}
                }
                if !matches!(self.input_mode, InputMode::Pathway) {
                    self.input_mode = InputMode::Normal;
                }
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Cancel => {
                // Esc = always cancel, abandon entirely
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Tab => {
                // Tab = advance to next field (I want to fill more detail)
                let buf = self.input_buffer.clone();
                match self.input_mode.clone() {
                    InputMode::Adding(AddStep::Name) => {
                        if buf.is_empty() { return Cmd::none(); } // name required
                        self.input_buffer.clear();
                        self.input_mode = InputMode::Adding(AddStep::Desire { name: buf });
                    }
                    InputMode::Adding(AddStep::Desire { name }) => {
                        let prefill = self.parent_id.as_ref().and_then(|pid| {
                            self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                        }).unwrap_or_default();
                        self.input_buffer = prefill;
                        self.input_mode = InputMode::Adding(AddStep::Reality { name, desire: buf });
                    }
                    InputMode::Adding(AddStep::Reality { name, desire }) => {
                        let reality = if buf.is_empty() {
                            self.parent_id.as_ref().and_then(|pid| {
                                self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                            }).unwrap_or_default()
                        } else { buf };
                        self.input_buffer.clear();
                        self.input_mode = InputMode::Adding(AddStep::Horizon { name, desire, reality });
                    }
                    InputMode::Adding(AddStep::Horizon { .. }) => {
                        // Already on last field — Tab wraps to commit (same as Enter)
                        return self.update_adding(Msg::Submit);
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
                self.reload_after_edit();
                self.input_buffer.clear();
                self.text_input.set_focused(false);
                // If save triggered a pathway palette, don't override to Normal
                if !matches!(self.input_mode, InputMode::Pathway) {
                    self.set_transient("saved");
                    self.input_mode = InputMode::Normal;
                }
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
                    // V5: desire change on the parent tension closes the current epoch
                    if self.parent_id.as_deref() == Some(tension_id) {
                        self.close_epoch(tension_id);
                    }
                }
                EditField::Reality => {
                    let _ = self.engine.update_actual(tension_id, &buf);
                    // V5: reality change on the parent tension closes the current epoch
                    if self.parent_id.as_deref() == Some(tension_id) {
                        self.close_epoch(tension_id);
                    }
                }
                EditField::Horizon => {
                    if buf.is_empty() {
                        let _ = self.engine.update_horizon(tension_id, None);
                    } else {
                        match crate::horizon::parse_horizon(&buf) {
                            Ok(h) => {
                                let _ = self.engine.update_horizon(tension_id, Some(h));
                                self.check_containment_palette(tension_id);
                            }
                            Err(_) => { self.set_transient(format!("horizon not recognized: {}", buf)); }
                        }
                    }
                }
            }
        }
    }

    /// V5: Close the current epoch for a tension by creating an epoch snapshot.
    /// This captures the current desire/reality and children state.
    fn close_epoch(&mut self, tension_id: &str) {
        if let Ok(Some(t)) = self.engine.store().get_tension(tension_id) {
            let children_json = self.engine.store()
                .get_children(tension_id)
                .ok()
                .map(|children| {
                    let summaries: Vec<serde_json::Value> = children.iter().map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "desired": c.desired,
                            "status": format!("{:?}", c.status),
                        })
                    }).collect();
                    serde_json::json!({"children": summaries}).to_string()
                });

            let _ = self.engine.store().create_epoch(
                tension_id,
                &t.desired,
                &t.actual,
                children_json.as_deref(),
                None, // gesture_id — not tracked in TUI yet
            );
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
                        // Check for containment and sequencing after reparent
                        self.check_containment_palette(&tension_id);
                        if !matches!(self.input_mode, InputMode::Pathway) {
                            self.check_sequencing_palette(&tension_id);
                        }
                    }
                }
                if !matches!(self.input_mode, InputMode::Pathway) {
                    self.input_mode = InputMode::Normal;
                }
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

    fn refresh_search_results(&mut self, include_root: bool) {
        let results = if include_root {
            crate::search::search_all_with_root(&self.input_buffer, self.engine.store())
        } else {
            crate::search::search_all(&self.input_buffer, self.engine.store())
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
