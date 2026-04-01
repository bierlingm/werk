//! Logbase view — epoch stream for a single tension.
//!
//! History-first orientation: the settled past as a structural resource.
//! Epochs are displayed most-recent-first with fisheye expansion — the
//! focused epoch shows all events, adjacent epochs show desire/reality
//! + summary, distant epochs compress to one-line summaries.
//!
//! Navigation:
//!   j/k — event-level (individual mutations within expanded epochs)
//!   J/K — epoch-level (jump between epoch boundaries)
//!   L   — return to originating view (Deck or Survey)
//!   Esc — same as L

use chrono::{DateTime, Utc};
use ftui::Frame;
use ftui::layout::Rect;
use ftui::style::Style;
use ftui::text::{Line, Span, Text};
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use sd_core::{EpochRecord, Tension};

use crate::app::InstrumentApp;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single event in the logbase stream — either an epoch boundary or a mutation.
#[derive(Debug, Clone)]
pub enum LogbaseEvent {
    /// An epoch boundary — marks the transition between epochs.
    EpochBoundary {
        /// Index into the epochs vec (0 = oldest).
        epoch_index: usize,
        /// What triggered this boundary.
        boundary_trigger: BoundaryTrigger,
    },
    /// A mutation within an epoch span.
    Mutation {
        /// Which epoch this mutation belongs to.
        epoch_index: usize,
        /// The mutation data.
        field: String,
        old_value: Option<String>,
        new_value: String,
        timestamp: DateTime<Utc>,
        /// If this mutation is on a child tension, its short code.
        child_short_code: Option<i32>,
        /// If this mutation is on a child tension, its ID.
        child_tension_id: Option<String>,
    },
}

impl LogbaseEvent {
    /// Get the epoch index this event belongs to.
    pub fn epoch_index(&self) -> usize {
        match self {
            LogbaseEvent::EpochBoundary { epoch_index, .. } => *epoch_index,
            LogbaseEvent::Mutation { epoch_index, .. } => *epoch_index,
        }
    }

    /// Is this an epoch boundary event?
    pub fn is_boundary(&self) -> bool {
        matches!(self, LogbaseEvent::EpochBoundary { .. })
    }
}

/// What triggered an epoch boundary.
#[derive(Debug, Clone)]
pub enum BoundaryTrigger {
    /// Desire changed (the desire text differs from the previous epoch).
    DesireChanged,
    /// Reality changed.
    RealityChanged,
    /// Both desire and reality changed.
    BothChanged,
    /// Structural event (split, merge) — from epoch_type field.
    Structural(String),
    /// Unknown (older epoch without gesture tracking).
    Unknown,
}

/// Provenance information for the logbase header.
#[derive(Debug, Clone, Default)]
pub struct LogbaseProvenance {
    /// Tensions this was split from.
    pub split_from: Vec<ProvenanceRef>,
    /// Tensions this was split into.
    pub split_into: Vec<ProvenanceRef>,
    /// Tensions that were merged into this.
    pub merged_from: Vec<ProvenanceRef>,
    /// Tensions this was merged into.
    pub merged_into: Vec<ProvenanceRef>,
}

/// A reference to another tension in provenance display.
#[derive(Debug, Clone)]
pub struct ProvenanceRef {
    pub id: String,
    pub short_code: Option<i32>,
    pub desired: String,
}

// ---------------------------------------------------------------------------
// Event stream construction
// ---------------------------------------------------------------------------

/// Build the flat event stream from epochs and mutations.
///
/// Returns events ordered most-recent-first. Each epoch produces a boundary
/// event followed by its mutations (also most-recent-first within the epoch).
pub fn build_event_stream(
    tension: &Tension,
    epochs: &[EpochRecord],
    store: &sd_core::Store,
) -> Vec<LogbaseEvent> {
    let mut events = Vec::new();

    // Process epochs in reverse (most recent first)
    for (epoch_idx, epoch) in epochs.iter().enumerate().rev() {
        // Determine what triggered this boundary
        let trigger = compute_boundary_trigger(epoch, epochs, epoch_idx, store);

        events.push(LogbaseEvent::EpochBoundary {
            epoch_index: epoch_idx,
            boundary_trigger: trigger,
        });

        // Load mutations for this epoch span
        let span_start = if epoch_idx == 0 {
            tension.created_at
        } else {
            epochs[epoch_idx - 1].timestamp
        };

        if let Ok(mutations) = store.get_epoch_mutations(&tension.id, span_start, epoch.timestamp) {
            // Get all tensions for short code lookup
            let all_tensions = store.list_tensions().unwrap_or_default();

            // Filter out desire/reality mutations on the tension itself —
            // those are the epoch boundary events, already shown as snapshots.
            for m in mutations.iter().rev() {
                let is_self = m.tension_id() == tension.id;
                let is_boundary_field = m.field() == "desired" || m.field() == "actual";

                // Skip the tension's own desire/reality changes (they ARE the boundary)
                if is_self && is_boundary_field {
                    continue;
                }

                // Skip status=created mutations (redundant with "created" events)
                if m.field() == "created" {
                    continue;
                }

                let child_info = if !is_self {
                    all_tensions.iter()
                        .find(|t| t.id == m.tension_id())
                        .map(|t| (t.short_code, t.id.clone()))
                } else {
                    None
                };

                events.push(LogbaseEvent::Mutation {
                    epoch_index: epoch_idx,
                    field: m.field().to_owned(),
                    old_value: m.old_value().map(|s| s.to_owned()),
                    new_value: m.new_value().to_owned(),
                    timestamp: m.timestamp(),
                    child_short_code: child_info.as_ref().and_then(|(sc, _)| *sc),
                    child_tension_id: child_info.map(|(_, id)| id),
                });
            }
        }
    }

    events
}

/// Determine what triggered an epoch boundary.
///
/// Strategy:
/// 1. If epoch_type is set → structural event
/// 2. If trigger_gesture_id exists → look up gesture mutations for field types
/// 3. Fallback → compare snapshots against previous epoch
fn compute_boundary_trigger(
    epoch: &EpochRecord,
    epochs: &[EpochRecord],
    epoch_idx: usize,
    store: &sd_core::Store,
) -> BoundaryTrigger {
    // Structural events from epoch_type
    if let Some(ref etype) = epoch.epoch_type {
        return BoundaryTrigger::Structural(etype.clone());
    }

    // Try trigger gesture
    if let Some(ref gesture_id) = epoch.trigger_gesture_id {
        if let Ok(all_mutations) = store.all_mutations() {
            let gesture_fields: Vec<&str> = all_mutations.iter()
                .filter(|m| m.gesture_id() == Some(gesture_id.as_str()))
                .filter(|m| m.tension_id() == epoch.tension_id)
                .map(|m| m.field())
                .collect();

            let has_desire = gesture_fields.iter().any(|f| *f == "desired");
            let has_reality = gesture_fields.iter().any(|f| *f == "actual");

            return match (has_desire, has_reality) {
                (true, true) => BoundaryTrigger::BothChanged,
                (true, false) => BoundaryTrigger::DesireChanged,
                (false, true) => BoundaryTrigger::RealityChanged,
                (false, false) => BoundaryTrigger::Unknown,
            };
        }
    }

    // Fallback: compare against previous epoch
    if epoch_idx > 0 {
        let prev = &epochs[epoch_idx - 1];
        let desire_changed = epoch.desire_snapshot != prev.desire_snapshot;
        let reality_changed = epoch.reality_snapshot != prev.reality_snapshot;
        match (desire_changed, reality_changed) {
            (true, true) => BoundaryTrigger::BothChanged,
            (true, false) => BoundaryTrigger::DesireChanged,
            (false, true) => BoundaryTrigger::RealityChanged,
            (false, false) => BoundaryTrigger::Unknown,
        }
    } else {
        // First epoch — by definition it's the initial state
        BoundaryTrigger::BothChanged
    }
}

/// Build provenance from edges.
pub fn build_provenance(
    store: &sd_core::Store,
    tension_id: &str,
) -> LogbaseProvenance {
    let edges = store.get_edges_for_tension(tension_id).unwrap_or_default();
    let all_tensions = store.list_tensions().unwrap_or_default();

    let mut prov = LogbaseProvenance::default();

    for edge in &edges {
        let other_id = if edge.from_id == tension_id {
            &edge.to_id
        } else {
            &edge.from_id
        };

        let other_ref = all_tensions.iter()
            .find(|t| t.id == *other_id)
            .map(|t| ProvenanceRef {
                id: t.id.clone(),
                short_code: t.short_code,
                desired: t.desired.clone(),
            });

        if let Some(r) = other_ref {
            match edge.edge_type.as_str() {
                sd_core::EDGE_SPLIT_FROM => {
                    if edge.to_id == tension_id {
                        // This tension was split FROM the other
                        prov.split_from.push(r);
                    } else {
                        // This tension was split INTO the other
                        prov.split_into.push(r);
                    }
                }
                sd_core::EDGE_MERGED_INTO => {
                    if edge.from_id == tension_id {
                        // This tension was merged INTO the other
                        prov.merged_into.push(r);
                    } else {
                        // The other was merged INTO this tension
                        prov.merged_from.push(r);
                    }
                }
                _ => {} // EDGE_CONTAINS handled elsewhere
            }
        }
    }

    prov
}

impl LogbaseProvenance {
    pub fn has_any(&self) -> bool {
        !self.split_from.is_empty()
            || !self.split_into.is_empty()
            || !self.merged_from.is_empty()
            || !self.merged_into.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

impl InstrumentApp {
    /// Load logbase data for a tension and switch to logbase view.
    pub fn enter_logbase(&mut self, tension_id: &str) {
        // Save originating state for return
        self.pre_logbase_state = Some((
            self.view_orientation,
            self.parent_id.clone(),
            self.focus_state.active,
        ));

        // Load tension
        let tension = match self.engine.store().get_tension(tension_id) {
            Ok(Some(t)) => t,
            _ => return,
        };

        // Load epochs
        let epochs = self.engine.store()
            .get_epochs(tension_id)
            .unwrap_or_default();

        // Build event stream
        let events = build_event_stream(&tension, &epochs, self.engine.store());

        // Build provenance
        let provenance = build_provenance(self.engine.store(), tension_id);

        // Find the first epoch boundary (most recent epoch) for cursor start
        let initial_cursor = 0; // First event = most recent epoch boundary

        self.logbase_tension_id = Some(tension_id.to_owned());
        self.logbase_tension = Some(tension);
        self.logbase_epochs = epochs;
        self.logbase_events = events;
        self.logbase_provenance = provenance;
        self.logbase_cursor = initial_cursor;
        self.logbase_focused_epoch = if !self.logbase_epochs.is_empty() {
            self.logbase_epochs.len() - 1 // Most recent epoch index
        } else {
            0
        };

        self.view_orientation = crate::state::ViewOrientation::Logbase;
    }

    /// Return from logbase to the originating view.
    pub fn exit_logbase(&mut self) {
        if let Some((orientation, parent_id, focus_id)) = self.pre_logbase_state.take() {
            self.view_orientation = orientation;
            // Restore deck state if returning to Stream
            if orientation == crate::state::ViewOrientation::Stream {
                self.parent_id = parent_id;
                self.load_siblings();
                self.focus_state.active = focus_id;
            }
            // Survey state is still intact — just switch orientation back
        } else {
            self.view_orientation = crate::state::ViewOrientation::Stream;
        }

        // Clear logbase state
        self.logbase_tension_id = None;
        self.logbase_tension = None;
        self.logbase_epochs.clear();
        self.logbase_events.clear();
        self.logbase_provenance = LogbaseProvenance::default();
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

impl InstrumentApp {
    /// Render the logbase view.
    pub fn render_logbase(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.layout.content_area(*area);
        let w = area.width as usize;

        let tension = match &self.logbase_tension {
            Some(t) => t,
            None => {
                let line = Line::from_spans([
                    Span::styled("  No tension loaded.", self.styles.dim),
                ]);
                Paragraph::new(Text::from_lines(vec![line])).render(area, frame);
                return;
            }
        };

        // === Header ===
        let mut header_lines: Vec<Line> = Vec::new();

        // Parent ref
        if let Some(ref pid) = tension.parent_id {
            if let Ok(Some(parent)) = self.engine.store().get_tension(pid) {
                let display = werk_shared::display_id(parent.short_code, &parent.id);
                let desired_trunc = werk_shared::truncate(&parent.desired, w.saturating_sub(6));
                header_lines.push(Line::from_spans([
                    Span::styled(format!("  \u{2190} {} {}", display, desired_trunc), self.styles.dim),
                ]));
            }
        }

        // Identity: desire
        let display = werk_shared::display_id(tension.short_code, &tension.id);
        let desire_text = format!("  \u{25C6} {} {}", display, tension.desired);
        let desire_wrapped = word_wrap(&desire_text, w);
        for line_text in &desire_wrapped {
            header_lines.push(Line::from_spans([
                Span::styled(line_text.clone(), self.styles.text),
            ]));
        }

        // Frontier summary between desire and reality (if tension has children)
        if let Ok(children) = self.engine.store().get_children(&tension.id) {
            if !children.is_empty() {
                let resolved = children.iter().filter(|c| c.status == sd_core::TensionStatus::Resolved).count();
                let released = children.iter().filter(|c| c.status == sd_core::TensionStatus::Released).count();
                let held = children.iter().filter(|c| c.status == sd_core::TensionStatus::Active && c.position.is_none()).count();
                let done = resolved + released;
                let total = children.len();
                let mut parts = vec![format!("[{}/{}]", done, total)];
                if held > 0 {
                    parts.push(format!("{} held", held));
                }
                let summary = parts.join(" \u{00b7} ");
                header_lines.push(Line::from_spans([
                    Span::styled(format!("    {}", summary), self.styles.dim),
                ]));
            }
        }

        // Reality
        if !tension.actual.is_empty() {
            let reality_text = format!("  \u{25C7} {}", tension.actual);
            let reality_wrapped = word_wrap(&reality_text, w);
            for line_text in &reality_wrapped {
                header_lines.push(Line::from_spans([
                    Span::styled(line_text.clone(), self.styles.subdued),
                ]));
            }
        }

        // Provenance
        if self.logbase_provenance.has_any() {
            for r in &self.logbase_provenance.split_from {
                let display = werk_shared::display_id(r.short_code, &r.id);
                let text = format!("  \u{2919} split from {} {}", display, werk_shared::truncate(&r.desired, w.saturating_sub(20)));
                header_lines.push(Line::from_spans([
                    Span::styled(text, self.styles.dim),
                ]));
            }
            for r in &self.logbase_provenance.split_into {
                let display = werk_shared::display_id(r.short_code, &r.id);
                let text = format!("  \u{291A} split into {} {}", display, werk_shared::truncate(&r.desired, w.saturating_sub(20)));
                header_lines.push(Line::from_spans([
                    Span::styled(text, self.styles.dim),
                ]));
            }
            for r in &self.logbase_provenance.merged_from {
                let display = werk_shared::display_id(r.short_code, &r.id);
                let text = format!("  \u{291B} merged from {} {}", display, werk_shared::truncate(&r.desired, w.saturating_sub(20)));
                header_lines.push(Line::from_spans([
                    Span::styled(text, self.styles.dim),
                ]));
            }
            for r in &self.logbase_provenance.merged_into {
                let display = werk_shared::display_id(r.short_code, &r.id);
                let text = format!("  \u{291B} merged into {} {}", display, werk_shared::truncate(&r.desired, w.saturating_sub(20)));
                header_lines.push(Line::from_spans([
                    Span::styled(text, self.styles.dim),
                ]));
            }
        }

        // Summary line
        let epoch_count = self.logbase_epochs.len();
        let mutation_count: usize = self.logbase_events.iter()
            .filter(|e| matches!(e, LogbaseEvent::Mutation { .. }))
            .count();
        let age = if let Some(first) = self.logbase_epochs.first() {
            format_age(first.timestamp)
        } else {
            "no history".to_owned()
        };
        let summary = format!("  {} epoch{} \u{00b7} {} mutation{} \u{00b7} {}",
            epoch_count, if epoch_count == 1 { "" } else { "s" },
            mutation_count, if mutation_count == 1 { "" } else { "s" },
            age,
        );
        header_lines.push(Line::from_spans([
            Span::styled(summary, self.styles.dim),
        ]));

        let header_height = header_lines.len() as u16;

        // === Layout: header + separator + stream + hints ===
        let stream_height = area.height.saturating_sub(header_height + 1); // +1 for separator

        // Render header
        let header_area = Rect::new(area.x, area.y, area.width, header_height);
        Paragraph::new(Text::from_lines(header_lines)).render(header_area, frame);

        // Separator
        let sep_y = area.y + header_height;
        if sep_y < area.y + area.height {
            let sep_text = format_separator(
                w,
                epoch_count,
                mutation_count,
            );
            render_styled_line(frame, area.x, sep_y, area.width, &sep_text, self.styles.dim);
        }

        // === Event stream ===
        let stream_y = sep_y + 1;
        if stream_height < 2 || self.logbase_events.is_empty() {
            return;
        }

        let stream_area = Rect::new(area.x, stream_y, area.width, stream_height);
        self.render_event_stream(&stream_area, w, frame);
    }

    /// Render the event stream with fisheye expansion.
    fn render_event_stream(&self, area: &Rect, w: usize, frame: &mut Frame<'_>) {
        let available = area.height as usize;
        if available == 0 || self.logbase_events.is_empty() {
            return;
        }

        // Determine visible window centered on cursor
        // For now: simple scroll — cursor is always visible, events above/below
        // are rendered until we run out of space.
        let total = self.logbase_events.len();
        let cursor = self.logbase_cursor.min(total.saturating_sub(1));

        // Render events around the cursor
        let mut lines: Vec<(Line, bool)> = Vec::new(); // (line, is_cursor)

        for (i, event) in self.logbase_events.iter().enumerate() {
            let is_cursor = i == cursor;
            let is_focused_epoch = event.epoch_index() == self.logbase_focused_epoch;

            match event {
                LogbaseEvent::EpochBoundary { epoch_index, boundary_trigger } => {
                    let epoch = &self.logbase_epochs[*epoch_index];
                    let epoch_num = epoch_index + 1;
                    let age_text = format_age(epoch.timestamp);

                    // Boundary trigger label
                    let trigger_label = match boundary_trigger {
                        BoundaryTrigger::DesireChanged => " [\u{25C6} changed]",
                        BoundaryTrigger::RealityChanged => " [\u{25C7} changed]",
                        BoundaryTrigger::BothChanged if *epoch_index > 0 => " [\u{25C6}\u{25C7} both]",
                        BoundaryTrigger::BothChanged => "", // first epoch, don't label
                        BoundaryTrigger::Structural(_) => "",
                        BoundaryTrigger::Unknown => "",
                    };

                    // For structural events, format the label differently
                    let structural_label = if let BoundaryTrigger::Structural(s) = boundary_trigger {
                        format!(" [{}]", s)
                    } else {
                        trigger_label.to_owned()
                    };

                    // Epoch boundary line
                    let label = format!("epoch {}", epoch_num);
                    let right = format!("{}{} ", age_text, structural_label);
                    let rule_w = w.saturating_sub(4 + label.len() + right.len() + 2);
                    let rule = "\u{2500}".repeat(rule_w);
                    let boundary_text = format!(" \u{2500}\u{2500} {} {} {} ", label, rule, right);

                    let style = if is_cursor { self.styles.selected } else { self.styles.dim };
                    lines.push((Line::from_spans([Span::styled(boundary_text, style)]), is_cursor));

                    // Desire/reality snapshots (always shown for focused epoch, summary for others)
                    if is_focused_epoch || is_cursor {
                        // Full desire/reality
                        let desire_trunc = werk_shared::truncate(&epoch.desire_snapshot, w.saturating_sub(6));
                        lines.push((Line::from_spans([
                            Span::styled(format!("    \u{25C6} {}", desire_trunc), self.styles.text),
                        ]), false));

                        let reality_trunc = werk_shared::truncate(&epoch.reality_snapshot, w.saturating_sub(6));
                        lines.push((Line::from_spans([
                            Span::styled(format!("    \u{25C7} {}", reality_trunc), self.styles.subdued),
                        ]), false));

                        // Dotted rule before mutations
                        let mutation_count = self.logbase_events.iter()
                            .filter(|e| matches!(e, LogbaseEvent::Mutation { epoch_index: ei, .. } if *ei == *epoch_index))
                            .count();
                        if mutation_count > 0 {
                            let dots = "\u{2508}".repeat(w.saturating_sub(4));
                            lines.push((Line::from_spans([
                                Span::styled(format!("    {}", dots), self.styles.dim),
                            ]), false));
                        }
                    } else {
                        // Compressed: one-line summary
                        let desire_short = werk_shared::truncate(&epoch.desire_snapshot, 40);
                        let mutation_count = self.logbase_events.iter()
                            .filter(|e| matches!(e, LogbaseEvent::Mutation { epoch_index: ei, .. } if *ei == *epoch_index))
                            .count();
                        let summary = if mutation_count > 0 {
                            format!("    \u{25C6} {} \u{00b7} {} mut", desire_short, mutation_count)
                        } else {
                            format!("    \u{25C6} {}", desire_short)
                        };
                        lines.push((Line::from_spans([
                            Span::styled(summary, self.styles.dim),
                        ]), false));
                    }
                }

                LogbaseEvent::Mutation { epoch_index, field, new_value, timestamp, child_short_code, .. } => {
                    // Only show mutation details for the focused epoch
                    if !(*epoch_index == self.logbase_focused_epoch) {
                        continue;
                    }

                    let ts_display = format_date_short(*timestamp);

                    let (glyph, text) = match field.as_str() {
                        "note" => {
                            let trunc = werk_shared::truncate(new_value, w.saturating_sub(12 + ts_display.len()));
                            ("\u{203B}", format!("{}", trunc)) // ※
                        }
                        "status" if new_value == "Resolved" => {
                            let child_label = child_short_code
                                .map(|sc| format!("#{} ", sc))
                                .unwrap_or_default();
                            ("\u{2713}", format!("{}{}", child_label, werk_shared::truncate(new_value, w.saturating_sub(16)))) // ✓
                        }
                        "status" if new_value == "Released" => {
                            let child_label = child_short_code
                                .map(|sc| format!("#{} ", sc))
                                .unwrap_or_default();
                            ("\u{2717}", format!("{}{}", child_label, "released")) // ✗
                        }
                        "desired" => {
                            let child_label = child_short_code
                                .map(|sc| format!("#{} ", sc))
                                .unwrap_or_default();
                            let trunc = werk_shared::truncate(new_value, w.saturating_sub(16 + child_label.len()));
                            ("\u{25C6}", format!("{}desire: {}", child_label, trunc)) // ◆
                        }
                        "actual" => {
                            let child_label = child_short_code
                                .map(|sc| format!("#{} ", sc))
                                .unwrap_or_default();
                            let trunc = werk_shared::truncate(new_value, w.saturating_sub(18 + child_label.len()));
                            ("\u{25C7}", format!("{}reality: {}", child_label, trunc)) // ◇
                        }
                        "position" => {
                            let child_label = child_short_code
                                .map(|sc| format!("#{} ", sc))
                                .unwrap_or_default();
                            ("\u{2022}", format!("{}positioned", child_label)) // •
                        }
                        _ => {
                            let child_label = child_short_code
                                .map(|sc| format!("#{} ", sc))
                                .unwrap_or_default();
                            let trunc = werk_shared::truncate(new_value, w.saturating_sub(16 + field.len()));
                            ("\u{2022}", format!("{}[{}] {}", child_label, field, trunc)) // •
                        }
                    };

                    // Pad to right-align timestamp
                    let content = format!("    {} {}", glyph, text);
                    let content_w = content.chars().count();
                    let pad = w.saturating_sub(content_w + ts_display.len() + 1);
                    let line_text = format!("{}{}{}", content, " ".repeat(pad), ts_display);

                    let style = if is_cursor { self.styles.selected } else { self.styles.dim };
                    lines.push((Line::from_spans([Span::styled(line_text, style)]), is_cursor));
                }
            }
        }

        // Viewport: find the cursor line and center around it
        let cursor_line_idx = lines.iter().position(|(_, is_c)| *is_c).unwrap_or(0);

        let half = available / 2;
        let start = if cursor_line_idx > half {
            cursor_line_idx - half
        } else {
            0
        };
        let end = (start + available).min(lines.len());
        let start = if end < available { 0 } else { end - available };

        // Render visible lines
        for (i, (line, _)) in lines[start..end].iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }
            // Render using Paragraph for correct span styling
            let line_area = Rect::new(area.x, y, area.width, 1);
            Paragraph::new(Text::from_lines(vec![line.clone()])).render(line_area, frame);
        }

        // Top compression line
        if start > 0 {
            let above_events: usize = lines[..start].iter()
                .filter(|(_, is_c)| !is_c)
                .count();
            if above_events > 0 {
                let comp_text = format!(" \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} \u{25B4} {} earlier events ", above_events);
                render_styled_line(frame, area.x, area.y, area.width, &comp_text, self.styles.dim);
            }
        }

        // Bottom compression line
        if end < lines.len() {
            let below_events = lines[end..].len();
            let comp_y = area.y + area.height - 1;
            let comp_text = format!(" \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} \u{25BE} {} older events ", below_events);
            render_styled_line(frame, area.x, comp_y, area.width, &comp_text, self.styles.dim);
        }
    }

    /// Render the logbase bottom bar.
    pub fn render_logbase_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let content = self.layout.content_area(Rect::new(area.x, area.y, area.width, area.height + 10));
        let bar_area = Rect::new(content.x, area.y, content.width, 1);

        let tension_label = self.logbase_tension.as_ref()
            .map(|t| {
                let display = werk_shared::display_id(t.short_code, &t.id);
                format!("Log {} ", display)
            })
            .unwrap_or_default();

        let epoch_label = if !self.logbase_events.is_empty() {
            let event = &self.logbase_events[self.logbase_cursor.min(self.logbase_events.len().saturating_sub(1))];
            format!("epoch {}", event.epoch_index() + 1)
        } else {
            String::new()
        };

        let bar_text = format!(" {} \u{00b7} {} ", tension_label, epoch_label);
        render_styled_line(frame, bar_area.x, bar_area.y, bar_area.width, &bar_text, self.styles.dim);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_separator(w: usize, epoch_count: usize, mutation_count: usize) -> String {
    let label = format!(" {} epoch{} \u{00b7} {} mutation{} ",
        epoch_count, if epoch_count == 1 { "" } else { "s" },
        mutation_count, if mutation_count == 1 { "" } else { "s" },
    );
    let rule_w = w.saturating_sub(label.len());
    let left = rule_w / 2;
    let right = rule_w - left;
    format!("{}{}{}", "\u{2500}".repeat(left), label, "\u{2500}".repeat(right))
}

fn format_age(timestamp: DateTime<Utc>) -> String {
    let delta = Utc::now().signed_duration_since(timestamp);
    let hours = delta.num_hours();
    let days = delta.num_days();
    if hours < 1 { "just now".to_owned() }
    else if hours < 24 { format!("{}h ago", hours) }
    else if days < 30 { format!("{}d ago", days) }
    else { format!("{}mo ago", days / 30) }
}

fn format_date_short(timestamp: DateTime<Utc>) -> String {
    timestamp.format("%b %d").to_string()
}

/// Render a single styled line at a position.
fn render_styled_line(frame: &mut Frame<'_>, x: u16, y: u16, width: u16, text: &str, style: Style) {
    Paragraph::new(Text::from(Line::from_spans([Span::styled(text.to_owned(), style)])))
        .render(Rect::new(x, y, width, 1), frame);
}

fn word_wrap(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 { return vec![text.to_owned()]; }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_owned();
        } else if current.chars().count() + 1 + word.chars().count() <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = format!("    {}", word); // continuation indent
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
