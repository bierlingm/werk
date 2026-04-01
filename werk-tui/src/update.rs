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
        // Tick toast queue on timer events — re-schedule if toasts still active
        if matches!(msg, Msg::Tick) {
            self.toasts.tick(std::time::Duration::from_millis(100));
            return if self.toasts.is_active() {
                Cmd::Tick(std::time::Duration::from_millis(100))
            } else {
                Cmd::none()
            };
        }

        // Handle resize — update layout regime before anything else.
        if let Msg::Resize { width, height } = msg {
            self.layout.update_regime(width, height);
            return Cmd::none();
        }

        // Check for external DB changes on user input (not reorder — preserves drag state)
        // Skip in logbase view — logbase has its own data, and load_siblings is irrelevant.
        if !matches!(msg, Msg::Tick | Msg::Noop)
            && !matches!(self.input_mode, InputMode::Reordering { .. })
            && self.view_orientation != crate::state::ViewOrientation::Logbase
            && self.db_has_changed()
        {
            self.load_siblings();
        }

        // Toggle inspector overlay on Ctrl+Shift+I
        if matches!(msg, Msg::InspectorToggle) {
            self.show_inspector = !self.show_inspector;
            return Cmd::none();
        }

        // Open palette on Ctrl+K (works from any mode in Normal)
        if matches!(msg, Msg::OpenPalette) {
            // Build combined items (actions only — tensions populate on typing)
            let items = crate::palette::build_combined_items(
                "",
                Some(&self.palette_feedback),
                self.search_index.as_ref(),
                self.engine.store(),
            );
            self.command_palette.replace_actions(items);
            self.command_palette.open();
            return Cmd::none();
        }

        // Route events to palette when visible — it consumes them
        if self.command_palette.is_visible() {
            if let Some(action) = self.handle_palette_event(&msg) {
                return action;
            }
            return Cmd::none(); // palette consumed the event
        }

        // Survey orientation intercepts Normal-mode input.
        // Non-Normal modes (confirming, editing, annotating) use their own
        // handlers so gestures work from the survey.
        let in_survey = self.view_orientation == crate::state::ViewOrientation::Survey;
        let in_logbase = self.view_orientation == crate::state::ViewOrientation::Logbase;
        let was_not_normal = !matches!(self.input_mode, InputMode::Normal);

        let cmd = if in_logbase && matches!(self.input_mode, InputMode::Normal) {
            self.update_logbase(msg)
        } else if in_survey && matches!(self.input_mode, InputMode::Normal) {
            self.update_survey(msg)
        } else {
            match &self.input_mode {
                InputMode::Normal => self.update_normal(msg),
                InputMode::Help => self.update_help(msg),
                InputMode::Adding(_) => self.update_adding(msg),
                InputMode::Editing { .. } => self.update_editing(msg),
                InputMode::Annotating { .. } => self.update_annotating(msg),
                InputMode::Confirming(_) => self.update_confirming(msg),
                InputMode::Moving { .. } => self.update_moving(msg),
                InputMode::Reordering { .. } => self.update_reordering(msg),
                InputMode::Pathway => self.update_pathway(msg),
            }
        };

        // Reload survey when returning to Normal after a gesture completes.
        if in_survey && was_not_normal && matches!(self.input_mode, InputMode::Normal) {
            self.load_survey_items();
        }

        // Schedule toast tick if toasts are active (starts the dismiss timer chain)
        if self.toasts.is_active() {
            return match cmd {
                Cmd::None => Cmd::Tick(std::time::Duration::from_millis(100)),
                other => Cmd::Batch(vec![other, Cmd::Tick(std::time::Duration::from_millis(100))]),
            };
        }

        cmd
    }

    fn view(&self, frame: &mut Frame<'_>) {
        frame.set_cursor_visible(false);
        frame.set_cursor(None);

        let area = Rect::new(0, 0, frame.width(), frame.height());

        // Clear the ENTIRE visible area to black/dim before rendering anything.
        // This prevents stale Cell::default() (WHITE fg) from bleeding through
        // on margins, blank lines, hints rows, or any cell the widgets skip.
        crate::helpers::clear_area_styled(frame, area, self.styles.clr_dim);
        let show_hints = area.height >= 6;

        // Outer frame: content + lever + hints
        let mut constraints = vec![Constraint::Fill, Constraint::Fixed(1)];
        if show_hints {
            constraints.push(Constraint::Fixed(1));
        }
        let outer = Flex::vertical().constraints(constraints).split(area);
        let content_area = outer[0];
        let lever_area = outer[1];
        let hints_area = if show_hints { outer[2] } else { Rect::default() };

        // Survey orientation — render survey as background, then fall through
        // to overlay rendering so gestures (confirm, edit, note) work in survey.
        let in_survey = self.view_orientation == crate::state::ViewOrientation::Survey;
        let in_logbase = self.view_orientation == crate::state::ViewOrientation::Logbase;

        if in_logbase {
            self.render_logbase(&content_area, frame);
            self.render_logbase_bar(&lever_area, frame);
            // Don't return — fall through so overlays (palette, toasts) render on top.
        } else if in_survey {
            self.render_survey(&content_area, frame);
            self.render_survey_bar(&lever_area, frame);
            // Don't return — fall through so overlays render on top.
        }

        // Full-screen modes bypass the three-pane layout.
        if !in_survey && !in_logbase {
            if matches!(self.input_mode, InputMode::Help) {
                self.render_help(&content_area, frame);
            } else if matches!(self.input_mode, InputMode::Moving { .. }) {
                self.render_search(&content_area, frame);
            } else if self.siblings.is_empty() && self.parent_id.is_none()
                && !matches!(self.input_mode, InputMode::Adding(_))
            {
                self.render_empty(&content_area, frame);
            } else {
                // === Three-pane spatial model: desire / field / reality ===
                let content = self.layout.content_area(content_area);
                let w = content.width as usize;

                if w >= 20 && content.height >= 8 {
                    let cols = self.compute_cols(w);
                    let (desire_h, desire_lines) = self.desire_zone_height(w, &cols);
                    let reality_h = self.reality_zone_height(w);
                    let panes = self.layout.split(content, desire_h, reality_h);

                    // Desire and reality anchors
                    let frontier = self.current_frontier();
                    self.render_anchors(frame, panes.desire, panes.reality,
                        w, &cols, &desire_lines, &frontier);

                    // Field of action
                    self.render_deck(&panes.field, &cols, frame);
                }
            }
        }

        // Render inline overlays on top of the background (works for both deck and survey)
        match &self.input_mode {
            InputMode::Adding(step) => {
                self.render_add_prompt(step, &content_area, frame);
            }
            InputMode::Confirming(kind) => {
                self.render_confirm(kind, &content_area, frame);
            }
            InputMode::Editing { field, .. } => {
                self.render_edit_prompt(field, &content_area, frame);
            }
            InputMode::Annotating { .. } => {
                self.render_note_prompt(&content_area, frame);
            }
            InputMode::Pathway => {
                self.render_pathway(&content_area, frame);
            }
            _ => {}
        }

        // Bottom bar (survey/logbase bars already rendered above, hidden when palette is open)
        if !in_survey && !in_logbase && !self.command_palette.is_visible() {
            self.render_deck_bar(&lever_area, frame);
        }

        // Command palette — unified search/command/navigation surface (above modals, below toasts)
        if self.command_palette.is_visible() {
            // Backdrop dims the field behind the palette
            crate::modal::render_backdrop(frame, content_area, &self.styles);
            // The widget internally sizes to 60% of the area width and centers
            // itself. We pass the full content area — upstream issue #58 tracks
            // making this configurable.
            let palette_height = (content_area.height / 2).max(8).min(content_area.height.saturating_sub(1));
            let palette_area = ftui::layout::Rect::new(
                content_area.x,
                content_area.y,
                content_area.width,
                palette_height,
            );
            use ftui::widgets::Widget;
            self.command_palette.render(palette_area, frame);
        }

        // Inspector overlay (above content, below toasts)
        crate::inspector::render_inspector(self, frame, content_area);

        // Toast notifications (topmost layer)
        self.toasts.render(&area, frame);

        // Hints
        if show_hints {
            if self.command_palette.is_visible() {
                self.render_input_hints("Enter select  \u{2191}/\u{2193} navigate  Esc dismiss", &hints_area, frame);
            } else {
                match &self.input_mode {
                    InputMode::Adding(_) => self.render_input_hints("Enter create  Tab more fields  Esc cancel  Bksp back", &hints_area, frame),
                    InputMode::Confirming(_) => self.render_input_hints("y confirm  n cancel", &hints_area, frame),
                    InputMode::Editing { .. } => self.render_input_hints("Enter save  Tab more fields  Esc cancel", &hints_area, frame),
                    InputMode::Annotating { .. } => self.render_input_hints("Enter save  Esc cancel", &hints_area, frame),
                    InputMode::Moving { .. } => self.render_input_hints("Enter place here  \u{2191}/\u{2193} navigate  Esc cancel", &hints_area, frame),
                    InputMode::Reordering { .. } => self.render_input_hints("Shift+J/K move  Enter drop  Esc cancel", &hints_area, frame),
                    _ => if in_logbase {
                        self.render_input_hints("j/k events  J/K epochs  Enter expand  L return  Esc return", &hints_area, frame);
                    } else if !in_survey {
                        self.render_hints(&hints_area, frame);
                    },
                }
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
    /// Route an event to the command palette. Returns Some(Cmd) if the palette
    /// produced an action, None if it just consumed the event silently.
    fn handle_palette_event(&mut self, msg: &Msg) -> Option<Cmd<Msg>> {
        use ftui::widgets::command_palette::PaletteAction;

        // Convert Msg to ftui Event for the palette's handle_event
        let event = match msg {
            Msg::Char(c) => ftui::Event::Key(ftui::KeyEvent {
                code: ftui::KeyCode::Char(*c),
                modifiers: ftui::Modifiers::NONE,
                kind: ftui::KeyEventKind::Press,
            }),
            Msg::Up => ftui::Event::Key(ftui::KeyEvent {
                code: ftui::KeyCode::Up,
                modifiers: ftui::Modifiers::NONE,
                kind: ftui::KeyEventKind::Press,
            }),
            Msg::Down => ftui::Event::Key(ftui::KeyEvent {
                code: ftui::KeyCode::Down,
                modifiers: ftui::Modifiers::NONE,
                kind: ftui::KeyEventKind::Press,
            }),
            Msg::Submit => ftui::Event::Key(ftui::KeyEvent {
                code: ftui::KeyCode::Enter,
                modifiers: ftui::Modifiers::NONE,
                kind: ftui::KeyEventKind::Press,
            }),
            Msg::Cancel => ftui::Event::Key(ftui::KeyEvent {
                code: ftui::KeyCode::Escape,
                modifiers: ftui::Modifiers::NONE,
                kind: ftui::KeyEventKind::Press,
            }),
            Msg::Backspace => ftui::Event::Key(ftui::KeyEvent {
                code: ftui::KeyCode::Backspace,
                modifiers: ftui::Modifiers::NONE,
                kind: ftui::KeyEventKind::Press,
            }),
            Msg::RawEvent(e) => e.clone(),
            _ => return None, // Don't consume system messages
        };

        // Track query before event to detect changes
        let query_before = self.command_palette.query().to_string();

        match self.command_palette.handle_event(&event) {
            Some(PaletteAction::Execute(id)) => {
                Some(self.dispatch_palette_action(&id))
            }
            Some(PaletteAction::Dismiss) => Some(Cmd::none()),
            None => {
                // Query changed — refresh combined items with tension results
                let query_after = self.command_palette.query().to_string();
                if query_after != query_before {
                    let items = crate::palette::build_combined_items(
                        &query_after,
                        Some(&self.palette_feedback),
                        self.search_index.as_ref(),
                        self.engine.store(),
                    );
                    self.command_palette.replace_actions(items);
                    // Restore query that was cleared by replace_actions
                    self.command_palette.set_query(&query_after);
                }
                None
            }
        }
    }

    /// Dispatch a palette action to the same code path as direct keybindings.
    fn dispatch_palette_action(&mut self, action_id: &str) -> Cmd<Msg> {
        // Handle tension navigation — IDs prefixed with "t:"
        if let Some(tension_id) = action_id.strip_prefix(crate::palette::TENSION_ID_PREFIX) {
            // Navigate to the tension's parent level with cursor on the tension
            let parent_id = if let Ok(Some(t)) = self.engine.store().get_tension(tension_id) {
                t.parent_id.clone()
            } else {
                None
            };
            self.parent_id = parent_id;
            self.load_siblings();
            if let Some(idx) = self.siblings.iter().position(|s| s.id == tension_id) {
                self.deck_cursor_to_sibling(idx);
            }
            return Cmd::none();
        }

        // Record selection for cross-session learning
        crate::palette::record_action_selected(&mut self.palette_feedback, action_id);
        match action_id {
            "add" => {
                self.input_buffer.clear();
                self.input_mode = InputMode::Adding(AddStep::Desire);
            }
            "resolve" => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active {
                        self.input_mode = InputMode::Confirming(ConfirmKind::Resolve {
                            tension_id: entry.id.clone(),
                            desired: entry.desired.clone(),
                        });
                    }
                }
            }
            "release" => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active {
                        self.input_mode = InputMode::Confirming(ConfirmKind::Release {
                            tension_id: entry.id.clone(),
                            desired: entry.desired.clone(),
                        });
                    }
                }
            }
            "edit_desire" => {
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
            }
            "edit_reality" => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active {
                        self.input_buffer = entry.actual.clone();
                        self.text_input.set_value(&entry.actual);
                        self.text_input.set_focused(true);
                        self.text_input.select_all();
                        self.input_mode = InputMode::Editing {
                            tension_id: entry.id,
                            field: EditField::Reality,
                        };
                    }
                }
            }
            "edit_horizon" => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active {
                        let horizon_text = entry.horizon_label.as_deref().unwrap_or("");
                        self.input_buffer = horizon_text.to_string();
                        self.text_input.set_value(horizon_text);
                        self.text_input.set_focused(true);
                        self.text_input.select_all();
                        self.input_mode = InputMode::Editing {
                            tension_id: entry.id,
                            field: EditField::Horizon,
                        };
                    }
                }
            }
            "note" => {
                if let Some(entry) = self.action_target().cloned() {
                    self.input_buffer.clear();
                    self.text_input.set_value("");
                    self.text_input.set_focused(true);
                    self.input_mode = InputMode::Annotating {
                        tension_id: entry.id,
                    };
                }
            }
            "move" => {
                if let Some(entry) = self.action_target().cloned() {
                    self.input_mode = InputMode::Moving { tension_id: entry.id.clone() };
                    self.input_buffer.clear();
                    self.search_state = Some(crate::search::SearchState::new());
                }
            }
            "ascend" => {
                self.ascend();
            }
            "descend" => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.has_children {
                        self.descend(&entry.id);
                    }
                }
            }
            "undo" => {
                let current = self.capture_snapshot();
                if let Some((desc, snapshot)) = self.gesture_history.undo(current) {
                    self.restore_snapshot(snapshot);
                    self.toasts.push_undo(&format!("undone: {}", desc));
                } else {
                    self.global_undo_redo(false);
                }
            }
            "redo" => {
                let current = self.capture_snapshot();
                if let Some((desc, snapshot)) = self.gesture_history.redo(current) {
                    self.restore_snapshot(snapshot);
                    self.toasts.push_success(&format!("redone: {}", desc));
                } else {
                    self.global_undo_redo(true);
                }
            }
            "help" => {
                self.input_mode = InputMode::Help;
            }
            "survey" => {
                // Save stream position and switch to survey
                self.pre_survey_state = Some((
                    self.parent_id.clone(),
                    self.focus_state.active,
                ));
                self.load_survey_items();
                self.view_orientation = crate::state::ViewOrientation::Survey;
            }
            "logbase" => {
                let target = self.focus_state.cursor_target();
                let tension_id = match target {
                    crate::deck::CursorTarget::Desire | crate::deck::CursorTarget::Reality => {
                        self.parent_id.clone()
                    }
                    _ => {
                        self.action_target().map(|e| e.id.clone())
                            .or_else(|| self.parent_id.clone())
                    }
                };
                if let Some(tid) = tension_id {
                    self.enter_logbase(&tid);
                }
            }
            "hold" => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status == sd_core::TensionStatus::Active && entry.position.is_some() {
                        self.begin_gesture(&format!("hold {}", &entry.id[..8.min(entry.id.len())]));
                        let _ = self.engine.store().update_position(&entry.id, None);
                        self.end_gesture();
                        self.load_siblings();
                        self.set_transient("held");
                    }
                }
            }
            "quit" => {
                return self.save_and_quit();
            }
            _ => {}
        }
        Cmd::none()
    }

    fn update_normal(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            // Navigation
            Msg::Char('k') | Msg::Up => {
                if self.deck_zoom.has_detail() {
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                    self.focused_note = None;
                }
                self.deck_pitch_up();
                Cmd::none()
            }
            Msg::Char('j') | Msg::Down => {
                if self.deck_zoom.has_detail() {
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                    self.focused_note = None;
                }
                // j below Reality → enter logbase for the parent
                let before = self.focus_state.active;
                let target = self.focus_state.cursor_target();
                if matches!(target, crate::deck::CursorTarget::Reality) {
                    self.deck_pitch_down();
                    if self.focus_state.active == before {
                        // Nowhere to go — we're at the bottom. Enter logbase.
                        if let Some(pid) = self.parent_id.clone() {
                            self.enter_logbase(&pid);
                        }
                    }
                } else {
                    self.deck_pitch_down();
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
                if let Some(entry) = self.action_target().cloned() {
                    self.descend(&entry.id);
                }
                Cmd::none()
            }
            Msg::Submit => {
                // Enter: if already in Focus, dismiss. If in Peek, upgrade to Focus. Otherwise enter Focus.
                if self.deck_zoom == crate::deck::ZoomLevel::Focus {
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                    self.focused_note = None;
                } else if self.deck_zoom == crate::deck::ZoomLevel::Peek {
                    // Upgrade peek to full focus — reload with reality + notes
                    if let Some(entry) = self.action_target().cloned() {
                        let detail = self.load_focus_detail(&entry);
                        self.focused_detail = Some(detail);
                        self.deck_zoom = crate::deck::ZoomLevel::Focus;
                    }
                } else if self.toggle_summary_expansion() {
                    // Enter on a summary line toggles expansion — handled
                } else if let Some(note_focus) = self.try_focus_note() {
                    self.focused_note = Some(note_focus);
                    self.focused_detail = None;
                    self.deck_zoom = crate::deck::ZoomLevel::Focus;
                } else if let Some(entry) = self.action_target().cloned() {
                    let detail = self.load_focus_detail(&entry);
                    self.focused_detail = Some(detail);
                    self.focused_note = None;
                    self.deck_zoom = crate::deck::ZoomLevel::Focus;
                }
                Cmd::none()
            }
            // Shift+Enter — orient zoom (V9 placeholder)
            Msg::ShiftSubmit => {
                self.set_transient("orient zoom: coming soon");
                Cmd::none()
            }

            Msg::Char('h') | Msg::Backspace | Msg::Ascend => {
                if self.parent_id.is_some() {
                    self.ascend();
                }
                Cmd::none()
            }

            Msg::Char('g') | Msg::JumpTop => {
                self.deck_zoom = crate::deck::ZoomLevel::Normal;
                self.focused_detail = None;
                self.focused_note = None;
                // Navigate to first focus node
                if let Some(&(id, _)) = self.focus_state.targets_ref().first() {
                    self.focus_state.active = id;
                }
                Cmd::none()
            }
            Msg::Char('G') | Msg::JumpBottom => {
                self.deck_zoom = crate::deck::ZoomLevel::Normal;
                self.focused_detail = None;
                self.focused_note = None;
                // Navigate to last focus node
                if let Some(&(id, _)) = self.focus_state.targets_ref().last() {
                    self.focus_state.active = id;
                }
                Cmd::none()
            }

            // Space: peek — lighter detail card. Switches between densities.
            Msg::Char(' ') | Msg::ToggleGaze => {
                if self.deck_zoom == crate::deck::ZoomLevel::Peek {
                    // Already peeking — dismiss
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                    self.focused_note = None;
                } else if self.deck_zoom == crate::deck::ZoomLevel::Focus {
                    // Downgrade focus to peek — strip reality + notes
                    if let Some(ref mut detail) = self.focused_detail {
                        detail.actual = String::new();
                        detail.recent_notes = Vec::new();
                    }
                    self.deck_zoom = crate::deck::ZoomLevel::Peek;
                } else if let Some(entry) = self.action_target().cloned() {
                    let mut detail = self.load_focus_detail(&entry);
                    detail.actual = String::new();       // peek = no reality
                    detail.recent_notes = Vec::new();    // peek = no notes
                    self.focused_detail = Some(detail);
                    self.deck_zoom = crate::deck::ZoomLevel::Peek;
                }
                Cmd::none()
            }
            // Tab — pivot yaw: open survey centered on the current tension.
            // Saves stream state so Shift+Tab can return without pivoting.
            Msg::Tab | Msg::ExpandGaze => {
                // Save stream position for Shift+Tab return.
                self.pre_survey_state = Some((
                    self.parent_id.clone(),
                    self.focus_state.active,
                ));
                let focused_id = self.action_target().map(|e| e.id.clone());
                self.load_survey_items();
                // Position survey cursor on the focused tension.
                if let Some(ref id) = focused_id {
                    if let Some(idx) = self.survey_items.iter().position(|i| &i.tension_id == id) {
                        self.survey_cursor = idx;
                    } else {
                        self.survey_cursor = 0;
                    }
                }
                self.view_orientation = crate::state::ViewOrientation::Survey;
                Cmd::none()
            }
            // Shift+Tab — return yaw: reopen survey at the cursor position
            // you left it at. No pivot, no reload — just flip back.
            Msg::BackTab => {
                if !self.survey_items.is_empty() {
                    // Save current stream position so Tab from survey can return here.
                    self.pre_survey_state = Some((
                        self.parent_id.clone(),
                        self.focus_state.active,
                    ));
                    self.view_orientation = crate::state::ViewOrientation::Survey;
                } else {
                    // No survey data yet — do a full Tab instead.
                    return self.update_normal(Msg::Tab);
                }
                Cmd::none()
            }

            // L — open logbase for focused tension (or parent if on anchor/summary)
            Msg::Char('L') => {
                let target = self.focus_state.cursor_target();
                let tension_id = match target {
                    crate::deck::CursorTarget::Desire | crate::deck::CursorTarget::Reality => {
                        // On desire/reality anchor → logbase for the parent
                        self.parent_id.clone()
                    }
                    _ => {
                        // On a child → logbase for that child
                        self.action_target().map(|e| e.id.clone())
                            .or_else(|| self.parent_id.clone())
                    }
                };
                if let Some(tid) = tension_id {
                    self.enter_logbase(&tid);
                }
                Cmd::none()
            }

            // Acts
            Msg::Char('a') | Msg::StartAdd => {
                self.input_mode = InputMode::Adding(AddStep::Desire);
                self.input_buffer.clear();
                Cmd::none()
            }
            Msg::Char('e') | Msg::StartEdit => {
                if self.enter_anchor_edit() {
                    // Cursor was on desire/reality anchor — edit opened
                } else if let Some(entry) = self.action_target().cloned() {
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
                    self.text_input.set_value("");
                    self.text_input.set_focused(true);
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

            // Undo (u / Ctrl+Z) — gesture history first, falls back to DB mutation reversal
            Msg::Char('u') | Msg::Undo => {
                let current = self.capture_snapshot();
                if let Some((desc, snapshot)) = self.gesture_history.undo(current) {
                    self.restore_snapshot(snapshot);
                    self.toasts.push_undo(&format!("undone: {}", desc));
                } else {
                    // Fall back to legacy single-mutation undo
                    self.global_undo_redo(false);
                }
                Cmd::none()
            }

            // Redo (U / Ctrl+Shift+Z) — gesture history first, falls back to DB mutation reversal
            Msg::Char('U') | Msg::Redo => {
                let current = self.capture_snapshot();
                if let Some((desc, snapshot)) = self.gesture_history.redo(current) {
                    self.restore_snapshot(snapshot);
                    self.toasts.push_success(&format!("redone: {}", desc));
                } else {
                    self.global_undo_redo(true);
                }
                Cmd::none()
            }

            // Yank — y copies short code, Y copies full context for agent handoff
            Msg::Char('y') => {
                if let Some(entry) = self.action_target().cloned() {
                    let label = entry.short_code
                        .map(|c| format!("#{}", c))
                        .unwrap_or_else(|| entry.id[..12.min(entry.id.len())].to_string());
                    let _ = self.copy_to_clipboard(&label);
                    self.set_transient(format!("copied: {}", label));
                }
                Cmd::none()
            }
            Msg::Char('Y') => {
                if let Some(entry) = self.action_target().cloned() {
                    let ctx = self.build_clipboard_context(&entry.id);
                    let _ = self.copy_to_clipboard(&ctx);
                    let label = entry.short_code
                        .map(|c| format!("#{}", c))
                        .unwrap_or_else(|| "tension".to_string());
                    self.set_transient(format!("copied {} context", label));
                }
                Cmd::none()
            }

            // Reopen (for resolved/released)
            Msg::Char('o') => {
                if let Some(entry) = self.action_target().cloned() {
                    if entry.status != sd_core::TensionStatus::Active {
                        self.begin_gesture(&format!("reopen {}", &entry.id[..8.min(entry.id.len())]));
                        let _ = self.engine.store().update_status(
                            &entry.id,
                            sd_core::TensionStatus::Active,
                        );
                        self.end_gesture();
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
                        self.begin_gesture(&format!("hold {}", &entry_id[..8.min(entry_id.len())]));
                        let _ = self.engine.update_position(&entry_id, None);
                        self.end_gesture();
                        self.set_transient("held");
                        self.load_siblings();
                        if let Some(idx) = self.siblings.iter().position(|s| s.id == entry_id) {
                            self.deck_cursor_to_sibling(idx);
                        }
                    } else {
                        // Held → position at 1 (bottom of sequence = next to act on)
                        self.begin_gesture(&format!("position {}", &entry_id[..8.min(entry_id.len())]));
                        let _ = self.engine.update_position(&entry_id, Some(1));
                        self.end_gesture();
                        self.set_transient("positioned");
                        self.load_siblings();
                        if let Some(idx) = self.siblings.iter().position(|s| s.id == entry_id) {
                            self.deck_cursor_to_sibling(idx);
                        }
                        self.check_sequencing_palette(&entry_id);
                    }
                }
                Cmd::none()
            }

            // Search — opens unified palette (same as Ctrl+K)
            Msg::Char('/') | Msg::Search => {
                let items = crate::palette::build_combined_items(
                    "",
                    Some(&self.palette_feedback),
                    self.search_index.as_ref(),
                    self.engine.store(),
                );
                self.command_palette.replace_actions(items);
                self.command_palette.open();
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

            // Toggle trajectory mode (Q30: resolved stay in-place on route)
            Msg::Char('T') => {
                self.trajectory_mode = !self.trajectory_mode;
                self.set_transient(if self.trajectory_mode { "trajectory view" } else { "frontier view" });
                self.deck_cursor_reset();
                Cmd::none()
            }

            // ? = edit parent reality (V4 quick-edit), help at root
            Msg::Char('?') | Msg::ToggleHelp => {
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
                } else {
                    self.input_mode = InputMode::Help;
                }
                Cmd::none()
            }

            // ! = edit parent desire (V4 quick-edit)
            Msg::Char('!') => {
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
            Msg::Char('q') | Msg::Quit => self.save_and_quit(),
            Msg::Cancel => {
                if self.deck_zoom.has_detail() {
                    self.deck_zoom = crate::deck::ZoomLevel::Normal;
                    self.focused_detail = None;
                    self.focused_note = None;
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
                for _ in 0..self.siblings.len() {
                    let prev = self.reorder_grabbed_index();
                    self.reorder_move_up();
                    if self.reorder_grabbed_index() == prev { break; }
                }
                Cmd::none()
            }
            Msg::Char('G') => {
                for _ in 0..self.siblings.len() {
                    let prev = self.reorder_grabbed_index();
                    self.reorder_move_down();
                    if self.reorder_grabbed_index() == prev { break; }
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
            Msg::Char('q') | Msg::Quit => self.save_and_quit(),
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
            Msg::Quit => self.save_and_quit(),
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

    pub fn update_survey(&mut self, msg: Msg) -> Cmd<Msg> {
        use crate::state::ViewOrientation;
        match msg {
            Msg::Char('j') | Msg::Down => {
                if self.survey_cursor + 1 < self.survey_items.len() {
                    self.survey_cursor += 1;
                }
                Cmd::none()
            }
            Msg::Char('k') | Msg::Up => {
                self.survey_cursor = self.survey_cursor.saturating_sub(1);
                Cmd::none()
            }
            Msg::Char('g') | Msg::JumpTop => {
                self.survey_cursor = 0;
                Cmd::none()
            }
            Msg::Char('G') | Msg::JumpBottom => {
                if !self.survey_items.is_empty() {
                    self.survey_cursor = self.survey_items.len() - 1;
                }
                Cmd::none()
            }

            // Tab — pivot yaw: navigate to the selected tension's parent deck,
            // with cursor on that tension. Changes your structural position.
            Msg::Tab => {
                if let Some(item) = self.survey_items.get(self.survey_cursor).cloned() {
                    self.view_orientation = ViewOrientation::Stream;
                    // Navigate to the tension's parent context.
                    let target_parent = self.engine.store()
                        .get_tension(&item.tension_id).ok().flatten()
                        .and_then(|t| t.parent_id);
                    if target_parent != self.parent_id {
                        // Navigate to a different structural context.
                        match &target_parent {
                            Some(pid) => {
                                let pid = pid.clone();
                                self.parent_id = Some(pid);
                                self.load_siblings();
                            }
                            None => {
                                self.parent_id = None;
                                self.load_siblings();
                            }
                        }
                    }
                    self.recompute_frontier();
                    // Position cursor on the tension.
                    if let Some(sib_idx) = self.siblings.iter().position(|s| s.id == item.tension_id) {
                        self.deck_cursor_to_sibling(sib_idx);
                    }
                }
                Cmd::none()
            }

            // Shift+Tab / Esc — return yaw: go back to exactly where you were
            // before Tab, without changing structural position.
            Msg::BackTab | Msg::Cancel => {
                self.view_orientation = ViewOrientation::Stream;
                if let Some((saved_parent, saved_focus)) = self.pre_survey_state.take() {
                    if saved_parent != self.parent_id {
                        self.parent_id = saved_parent;
                        self.load_siblings();
                    }
                    self.recompute_frontier();
                    // Restore focus — clamp_active already ran, but try to
                    // restore the exact node if it still exists.
                    if self.focus_state.targets_ref().iter().any(|(id, _)| *id == saved_focus) {
                        self.focus_state.active = saved_focus;
                    }
                }
                Cmd::none()
            }

            // Space — toggle band collapse/expand for the current item's band.
            Msg::Char(' ') => {
                if let Some(item) = self.survey_items.get(self.survey_cursor) {
                    let band = item.band;
                    let expanded = self.survey_tree_state.toggle_band(band);
                    let label = if expanded { "expanded" } else { "collapsed" };
                    self.set_transient(&format!("{} {}", band.label(), label));
                }
                Cmd::none()
            }

            // h/l — reserved for temporal navigation (pan along time axis).
            // No-op until temporal depth is implemented.
            Msg::Char('h') | Msg::Ascend | Msg::Char('l') | Msg::Descend => Cmd::none(),

            // Enter — descend into the selected tension (show its children in stream).
            Msg::Submit => {
                if let Some(item) = self.survey_items.get(self.survey_cursor).cloned() {
                    self.view_orientation = ViewOrientation::Stream;
                    self.descend(&item.tension_id);
                }
                Cmd::none()
            }

            // --- Gestures on the selected tension (same as stream) ---

            // r — resolve
            Msg::Char('r') | Msg::StartResolve => {
                if let Some(item) = self.survey_items.get(self.survey_cursor) {
                    self.input_mode = InputMode::Confirming(ConfirmKind::Resolve {
                        tension_id: item.tension_id.clone(),
                        desired: item.desired.clone(),
                    });
                }
                Cmd::none()
            }
            // x — release
            Msg::Char('x') | Msg::StartRelease => {
                if let Some(item) = self.survey_items.get(self.survey_cursor) {
                    self.input_mode = InputMode::Confirming(ConfirmKind::Release {
                        tension_id: item.tension_id.clone(),
                        desired: item.desired.clone(),
                    });
                }
                Cmd::none()
            }
            // e — edit desire
            Msg::Char('e') | Msg::StartEdit => {
                if let Some(item) = self.survey_items.get(self.survey_cursor) {
                    self.input_buffer = item.desired.clone();
                    self.text_input.set_value(&item.desired);
                    self.text_input.set_focused(true);
                    self.text_input.select_all();
                    self.input_mode = InputMode::Editing {
                        tension_id: item.tension_id.clone(),
                        field: EditField::Desire,
                    };
                }
                Cmd::none()
            }
            // n — note
            Msg::Char('n') | Msg::StartNote => {
                if let Some(item) = self.survey_items.get(self.survey_cursor) {
                    self.input_buffer.clear();
                    self.text_input.set_value("");
                    self.text_input.set_focused(true);
                    self.input_mode = InputMode::Annotating {
                        tension_id: item.tension_id.clone(),
                    };
                }
                Cmd::none()
            }

            // L — open logbase for the focused tension
            Msg::Char('L') => {
                if let Some(item) = self.survey_items.get(self.survey_cursor) {
                    let tid = item.tension_id.clone();
                    self.enter_logbase(&tid);
                }
                Cmd::none()
            }

            Msg::Char('?') | Msg::ToggleHelp => {
                self.view_orientation = ViewOrientation::Stream;
                self.input_mode = InputMode::Help;
                Cmd::none()
            }
            Msg::Char('q') | Msg::Quit => self.save_and_quit(),
            _ => Cmd::none(),
        }
    }

    /// Update handler for logbase view — epoch stream navigation.
    pub fn update_logbase(&mut self, msg: Msg) -> Cmd<Msg> {
        let total = self.logbase_events.len();
        if total == 0 {
            // Empty logbase — any key returns
            match msg {
                Msg::Char('L') | Msg::Cancel | Msg::BackTab => {
                    self.exit_logbase();
                }
                Msg::Char('q') | Msg::Quit => return self.save_and_quit(),
                _ => {}
            }
            return Cmd::none();
        }

        match msg {
            // Event-level navigation
            Msg::Char('j') | Msg::Down => {
                if self.logbase_cursor + 1 < total {
                    self.logbase_cursor += 1;
                    self.logbase_focused_epoch = self.logbase_events[self.logbase_cursor].epoch_index();
                }
                Cmd::none()
            }
            Msg::Char('k') | Msg::Up => {
                self.logbase_cursor = self.logbase_cursor.saturating_sub(1);
                self.logbase_focused_epoch = self.logbase_events[self.logbase_cursor].epoch_index();
                Cmd::none()
            }

            // Epoch-level navigation (Shift+J/K)
            Msg::Char('J') | Msg::MoveDown => {
                // Jump to next epoch boundary below current position
                let current = self.logbase_cursor;
                for i in (current + 1)..total {
                    if self.logbase_events[i].is_boundary() {
                        self.logbase_cursor = i;
                        self.logbase_focused_epoch = self.logbase_events[i].epoch_index();
                        break;
                    }
                }
                Cmd::none()
            }
            Msg::Char('K') | Msg::MoveUp => {
                // Jump to previous epoch boundary above current position
                let current = self.logbase_cursor;
                for i in (0..current).rev() {
                    if self.logbase_events[i].is_boundary() {
                        self.logbase_cursor = i;
                        self.logbase_focused_epoch = self.logbase_events[i].epoch_index();
                        break;
                    }
                }
                Cmd::none()
            }

            // Jump to top/bottom
            Msg::Char('g') | Msg::JumpTop => {
                self.logbase_cursor = 0;
                self.logbase_focused_epoch = self.logbase_events[0].epoch_index();
                Cmd::none()
            }
            Msg::Char('G') | Msg::JumpBottom => {
                self.logbase_cursor = total - 1;
                self.logbase_focused_epoch = self.logbase_events[total - 1].epoch_index();
                Cmd::none()
            }

            // L / Esc / BackTab — return to originating view
            Msg::Char('L') | Msg::Cancel | Msg::BackTab => {
                self.exit_logbase();
                Cmd::none()
            }

            Msg::Char('q') | Msg::Quit => self.save_and_quit(),
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
                        InputMode::Adding(AddStep::Desire) => {
                            self.input_mode = InputMode::Normal;
                        }
                        InputMode::Adding(AddStep::Reality { desire }) => {
                            self.input_buffer = desire.clone();
                            self.input_mode = InputMode::Adding(AddStep::Desire);
                        }
                        InputMode::Adding(AddStep::Horizon { desire, reality }) => {
                            let (d, r) = (desire.clone(), reality.clone());
                            self.input_buffer = r;
                            self.input_mode = InputMode::Adding(AddStep::Reality { desire: d });
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
                    InputMode::Adding(AddStep::Desire) => {
                        if buf.is_empty() { return Cmd::none(); } // desire is required
                        let reality = self.parent_id.as_ref().and_then(|pid| {
                            self.engine.store().get_tension(pid).ok().flatten()
                                .map(|p| p.actual)
                                .filter(|a| !a.is_empty())
                        }).unwrap_or_else(|| "(not yet assessed)".to_string());
                        self.create_tension(&buf, &reality);
                    }
                    InputMode::Adding(AddStep::Reality { desire }) => {
                        let reality = if buf.is_empty() {
                            self.parent_id.as_ref().and_then(|pid| {
                                self.engine.store().get_tension(pid).ok().flatten()
                                    .map(|p| p.actual)
                                    .filter(|a| !a.is_empty())
                            }).unwrap_or_else(|| "(not yet assessed)".to_string())
                        } else { buf };
                        self.create_tension(&desire, &reality);
                    }
                    InputMode::Adding(AddStep::Horizon { desire, reality }) => {
                        if buf.is_empty() {
                            self.create_tension(&desire, &reality);
                        } else {
                            self.create_tension_with_horizon(&desire, &reality, &buf);
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
                    InputMode::Adding(AddStep::Desire) => {
                        if buf.is_empty() { return Cmd::none(); } // desire required
                        let prefill = self.parent_id.as_ref().and_then(|pid| {
                            self.engine.store().get_tension(pid).ok().flatten().map(|p| p.actual)
                        }).unwrap_or_default();
                        self.input_buffer = prefill;
                        self.input_mode = InputMode::Adding(AddStep::Reality { desire: buf });
                    }
                    InputMode::Adding(AddStep::Reality { desire }) => {
                        let reality = if buf.is_empty() {
                            self.parent_id.as_ref().and_then(|pid| {
                                self.engine.store().get_tension(pid).ok().flatten()
                                    .map(|p| p.actual)
                                    .filter(|a| !a.is_empty())
                            }).unwrap_or_else(|| "(not yet assessed)".to_string())
                        } else { buf };
                        self.input_buffer.clear();
                        self.input_mode = InputMode::Adding(AddStep::Horizon { desire, reality });
                    }
                    InputMode::Adding(AddStep::Horizon { .. }) => {
                        // Already on last field — Tab wraps to commit (same as Enter)
                        return self.update_adding(Msg::Submit);
                    }
                    _ => {}
                }
                Cmd::none()
            }
            Msg::Quit => self.save_and_quit(),
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
            Msg::Quit => self.save_and_quit(),

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
            let gesture_desc = format!("edit {} #{}", field.label(), tension_id.get(..8).unwrap_or(&tension_id));
            self.begin_gesture(&gesture_desc);

            match field {
                EditField::Desire => {
                    let _ = self.engine.update_desired(tension_id, &buf);
                    if self.parent_id.as_deref() == Some(tension_id) {
                        self.close_epoch(tension_id);
                    }
                }
                EditField::Reality => {
                    let _ = self.engine.update_actual(tension_id, &buf);
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

            self.end_gesture();
        }
    }

    /// Close the current epoch for a tension by creating an epoch snapshot.
    /// Captures current desire/reality and children state. Uses the active
    /// gesture (if any) as the trigger — call within a gesture boundary.
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
                            "actual": c.actual,
                            "status": format!("{:?}", c.status),
                            "position": c.position,
                        })
                    }).collect();
                    serde_json::json!({"children": summaries}).to_string()
                });

            let gesture_id = self.engine.active_gesture().map(|s| s.to_owned());
            let _ = self.engine.store().create_epoch(
                tension_id,
                &t.desired,
                &t.actual,
                children_json.as_deref(),
                gesture_id.as_deref(),
            );
        }
    }

    fn reload_after_edit(&mut self) {
        self.load_siblings();
    }

    fn update_annotating(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Submit => {
                self.sync_text_input_to_buffer();
                let buf = self.input_buffer.clone();
                if !buf.is_empty() {
                    if let InputMode::Annotating { ref tension_id } = self.input_mode.clone() {
                        self.begin_gesture(&format!("add note {}", &tension_id[..8.min(tension_id.len())]));
                        let _ = self.engine.store().record_mutation(
                            &sd_core::Mutation::new(
                                tension_id.clone(),
                                chrono::Utc::now(),
                                "note".to_owned(),
                                None,
                                buf,
                            ),
                        );
                        self.end_gesture();
                        self.set_transient("note added");
                        self.load_siblings();
                        // Reload detail and enter Focus so the note is immediately visible
                        if let Some(entry) = self.action_target().cloned() {
                            let detail = self.load_focus_detail(&entry);
                            self.focused_detail = Some(detail);
                            self.deck_zoom = crate::deck::ZoomLevel::Focus;
                        }
                    }
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.text_input.set_focused(false);
                Cmd::none()
            }
            Msg::Cancel => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.text_input.set_focused(false);
                Cmd::none()
            }
            Msg::Quit => self.save_and_quit(),

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

    fn update_confirming(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Char('y') | Msg::Submit => {
                match self.input_mode.clone() {
                    InputMode::Confirming(ConfirmKind::Resolve { tension_id, desired }) => {
                        self.begin_gesture(&format!("resolve '{}'", truncate_str(&desired, 30)));
                        let _ = self.engine.resolve(&tension_id);
                        self.end_gesture();
                        self.set_transient(format!("resolved: {}", truncate_str(&desired, 30)));
                        self.load_siblings();
                    }
                    InputMode::Confirming(ConfirmKind::Release { tension_id, desired }) => {
                        self.begin_gesture(&format!("release '{}'", truncate_str(&desired, 30)));
                        let _ = self.engine.release(&tension_id);
                        self.end_gesture();
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
            Msg::Quit => self.save_and_quit(),
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
                        let dest = if result.is_root_entry { "root" } else { &result.desired };
                        self.begin_gesture(&format!("move {} to {}", &tension_id[..8.min(tension_id.len())], truncate_str(dest, 20)));
                        let _ = self.engine.update_parent(&tension_id, new_parent);
                        self.end_gesture();
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
            Msg::Quit => self.save_and_quit(),
            _ => Cmd::none(),
        }
    }

    fn refresh_search_results(&mut self, include_root: bool) {
        let idx = self.search_index.as_ref();
        let results = if include_root {
            crate::search::search_all_with_root(&self.input_buffer, self.engine.store(), idx)
        } else {
            crate::search::search_all(&self.input_buffer, self.engine.store(), idx)
        };
        if let Some(ref mut s) = self.search_state {
            s.query = self.input_buffer.clone();
            s.results = results;
            s.cursor = 0;
        }
    }

    // -----------------------------------------------------------------------
    // Data operations
    // -----------------------------------------------------------------------

    fn create_tension(&mut self, desired: &str, actual: &str) {
        let parent = self.parent_id.clone();

        self.begin_gesture(&format!("create tension '{}'", truncate_str(desired, 40)));
        let result = self
            .engine
            .create_tension_with_parent(desired, actual, parent);
        self.end_gesture();

        match result {
            Ok(tension) => {
                self.set_transient(format!("created: {}", truncate_str(&tension.desired, 30)));
                self.load_siblings();
                // Position cursor on the new item
                if let Some(idx) = self.siblings.iter().position(|s| s.id == tension.id) {
                    self.deck_cursor_to_sibling(idx);
                }
            }
            Err(e) => {
                self.set_transient(format!("create failed: {e}"));
            }
        }
    }

    /// Load detail card data for a focused tension.
    fn load_focus_detail(&self, entry: &crate::state::FieldEntry) -> crate::deck::FocusedDetail {
        let now = chrono::Utc::now();
        let id = &entry.id;

        // Temporal facts from mutations
        let mutations = self.engine.store().get_mutations(id).unwrap_or_default();

        let last_reality = mutations.iter().rev()
            .find(|m| m.field() == "actual" || m.field() == "created")
            .map(|m| m.timestamp())
            .unwrap_or(now);

        let last_desire = mutations.iter().rev()
            .find(|m| m.field() == "desired" || m.field() == "created")
            .map(|m| m.timestamp())
            .unwrap_or(now);

        let created_at = mutations.iter()
            .find(|m| m.field() == "created")
            .map(|m| m.timestamp())
            .unwrap_or(now);

        // Recent notes (most recent first, rendering decides how many fit)
        let recent_notes: Vec<(String, String)> = mutations.iter().rev()
            .filter(|m| m.field() == "note")
            .map(|m| {
                let age = crate::glyphs::relative_time(m.timestamp(), now);
                (age, m.new_value().to_string())
            })
            .collect();

        // Child breakdown
        let children = self.engine.store().get_children(id).unwrap_or_default();
        let child_count = children.len();
        let child_active = children.iter().filter(|c| c.status == sd_core::TensionStatus::Active).count();
        let child_resolved = children.iter().filter(|c| c.status == sd_core::TensionStatus::Resolved).count();
        let child_released = children.iter().filter(|c| c.status == sd_core::TensionStatus::Released).count();
        let child_held = children.iter().filter(|c| c.status == sd_core::TensionStatus::Active && c.position.is_none()).count();

        crate::deck::FocusedDetail {
            sibling_index: self.deck_selected_sibling_index().unwrap_or(0),
            desired: entry.desired.clone(),
            actual: entry.actual.clone(),
            short_code: entry.short_code,
            deadline_label: entry.horizon_label.clone(),
            created_age: crate::glyphs::relative_time(created_at, now),
            last_reality_age: crate::glyphs::relative_time(last_reality, now),
            last_desire_age: crate::glyphs::relative_time(last_desire, now),
            temporal_urgency: entry.temporal_urgency,
            child_count,
            child_active,
            child_resolved,
            child_released,
            child_held,
            recent_notes,
        }
    }

    /// Try to focus a note if the cursor is on a NoteItem. Returns the FocusedNote or None.
    fn try_focus_note(&self) -> Option<crate::deck::FocusedNote> {
        let frontier = &self.frontier;
        let target = self.focus_state.cursor_target();
        if let crate::deck::CursorTarget::NoteItem(acc_idx) = target {
            if let Some(crate::deck::AccumulatedItem::Note { text, age, .. }) = frontier.accumulated.get(acc_idx) {
                return Some(crate::deck::FocusedNote {
                    acc_index: acc_idx,
                    text: text.clone(),
                    age: age.clone(),
                });
            }
        }
        None
    }

    /// Enter edit mode for the desire/reality anchor under the cursor.
    /// Returns true if the cursor was on an anchor and edit was entered.
    fn enter_anchor_edit(&mut self) -> bool {
        let _frontier = &self.frontier;
        let target = self.focus_state.cursor_target();
        let pid = match self.parent_id.clone() {
            Some(pid) => pid,
            None => return false,
        };
        match target {
            crate::deck::CursorTarget::Desire => {
                let desired = self.parent_tension.as_ref()
                    .map(|t| t.desired.clone()).unwrap_or_default();
                self.input_buffer = desired.clone();
                self.text_input.set_value(&desired);
                self.text_input.set_focused(true);
                self.text_input.select_all();
                self.input_mode = InputMode::Editing {
                    tension_id: pid,
                    field: EditField::Desire,
                };
                true
            }
            crate::deck::CursorTarget::Reality => {
                let actual = self.parent_tension.as_ref()
                    .map(|t| t.actual.clone()).unwrap_or_default();
                self.input_buffer = actual.clone();
                self.text_input.set_value(&actual);
                self.text_input.set_focused(true);
                self.text_input.select_all();
                self.input_mode = InputMode::Editing {
                    tension_id: pid,
                    field: EditField::Reality,
                };
                true
            }
            _ => false,
        }
    }

    /// Toggle expansion of the summary zone under the cursor.
    /// Returns true if the cursor was on a summary and the toggle was applied.
    fn toggle_summary_expansion(&mut self) -> bool {
        let _frontier = &self.frontier;
        let target = self.focus_state.cursor_target();
        match target {
            crate::deck::CursorTarget::RouteSummary => {
                self.route_expanded = !self.route_expanded;
                self.recompute_frontier();
                true
            }
            crate::deck::CursorTarget::Held => {
                self.held_expanded = !self.held_expanded;
                self.recompute_frontier();
                true
            }
            crate::deck::CursorTarget::Accumulated => {
                self.accumulated_expanded = !self.accumulated_expanded;
                self.recompute_frontier();
                true
            }
            _ => false,
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
