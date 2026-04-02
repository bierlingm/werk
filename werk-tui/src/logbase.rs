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

/// Style tag for header lines (resolved to actual Style during render).
#[derive(Debug, Clone, Copy)]
pub enum HeaderStyle {
    Dim,
    Text,
    Subdued,
}

/// A pre-built display item for the List widget.
/// Each logbase event becomes one or more LogbaseItems.
#[derive(Debug, Clone)]
pub struct LogbaseItem {
    /// Display text for the list row.
    pub text: String,
    /// Style for this row.
    pub style: Style,
    /// Index into logbase_events that this item maps to (for cursor → event mapping).
    pub event_index: usize,
    /// Whether this is an epoch boundary line (for J/K epoch-level navigation).
    pub is_boundary: bool,
    /// Whether this item is selectable (false for blanks, dotted rules, snapshot lines).
    pub selectable: bool,
    /// Whether to use bright/text style instead of dim (for changed snapshots, boundaries).
    pub bright: bool,
    /// Date label for this item (used for sticky date header).
    pub date: String,
}

// ---------------------------------------------------------------------------
// Event stream construction
// ---------------------------------------------------------------------------

/// Load all logbase data in one pass. Queries the store once for each table,
/// then builds events, provenance, and header from the cached data.
/// This avoids repeated full-table scans that caused 2s entry delay.
pub struct LogbaseData {
    pub events: Vec<LogbaseEvent>,
    pub provenance: LogbaseProvenance,
    pub header: Vec<(String, HeaderStyle)>,
    pub separator_label: String,
    /// ID → short_code lookup for resolving ULIDs in mutation values.
    pub id_lookup: std::collections::HashMap<String, Option<i32>>,
    /// ID → desire text lookup for showing child tension names alongside IDs.
    pub id_to_desire: std::collections::HashMap<String, String>,
}

pub fn load_logbase_data(
    tension: &Tension,
    epochs: &[EpochRecord],
    store: &sd_core::Store,
) -> LogbaseData {
    // Load each table ONCE
    let all_tensions = store.list_tensions().unwrap_or_default();
    let all_mutations = store.all_mutations().unwrap_or_default();
    let edges = store.get_edges_for_tension(&tension.id).unwrap_or_default();

    // Build short code lookup
    let sc_lookup: std::collections::HashMap<&str, Option<i32>> = all_tensions.iter()
        .map(|t| (t.id.as_str(), t.short_code))
        .collect();

    // Build events
    let events = build_event_stream(tension, epochs, &all_mutations, &sc_lookup);

    // Build provenance
    let provenance = build_provenance_from(&edges, &tension.id, &all_tensions);

    // Build header
    let header = build_header_cache(tension, &provenance, store);

    // Build separator label
    let epoch_count = epochs.len();
    let mutation_count = events.iter()
        .filter(|e| matches!(e, LogbaseEvent::Mutation { .. }))
        .count();
    let separator_label = format!(" {} epoch{} \u{00b7} {} mut{} ",
        epoch_count, if epoch_count == 1 { "" } else { "s" },
        mutation_count, if mutation_count == 1 { "" } else { "s" },
    );

    // Build owned ID lookup for list item construction
    let id_lookup: std::collections::HashMap<String, Option<i32>> = all_tensions.iter()
        .map(|t| (t.id.clone(), t.short_code))
        .collect();

    // Build ID → desire text lookup for child tension names
    let id_to_desire: std::collections::HashMap<String, String> = all_tensions.iter()
        .map(|t| (t.id.clone(), t.desired.clone()))
        .collect();

    LogbaseData { events, provenance, header, separator_label, id_lookup, id_to_desire }
}

/// Build the flat event stream from epochs and pre-loaded mutations.
fn build_event_stream(
    tension: &Tension,
    epochs: &[EpochRecord],
    all_mutations: &[sd_core::Mutation],
    sc_lookup: &std::collections::HashMap<&str, Option<i32>>,
) -> Vec<LogbaseEvent> {
    let mut events = Vec::new();

    for (epoch_idx, epoch) in epochs.iter().enumerate().rev() {
        let trigger = compute_boundary_trigger(epoch, epochs, epoch_idx, all_mutations);

        events.push(LogbaseEvent::EpochBoundary {
            epoch_index: epoch_idx,
            boundary_trigger: trigger,
        });

        // Mutations for this epoch span (from pre-loaded data)
        let span_start = if epoch_idx == 0 {
            tension.created_at
        } else {
            epochs[epoch_idx - 1].timestamp
        };

        // Filter mutations by time range and tension subtree
        let descendant_ids: std::collections::HashSet<&str> = all_mutations.iter()
            .filter(|m| m.timestamp() >= span_start && m.timestamp() <= epoch.timestamp)
            .map(|m| m.tension_id())
            .collect();

        for m in all_mutations.iter().rev() {
            if m.timestamp() < span_start || m.timestamp() > epoch.timestamp {
                continue;
            }
            // Only include mutations for this tension or its descendants
            if m.tension_id() != tension.id && !descendant_ids.contains(m.tension_id()) {
                continue;
            }

            let is_self = m.tension_id() == tension.id;
            if is_self && (m.field() == "desired" || m.field() == "actual") {
                continue; // boundary events shown as snapshots
            }
            if m.field() == "created" {
                continue;
            }

            let child_sc = if !is_self {
                sc_lookup.get(m.tension_id()).copied().flatten()
            } else {
                None
            };

            events.push(LogbaseEvent::Mutation {
                epoch_index: epoch_idx,
                field: m.field().to_owned(),
                old_value: m.old_value().map(|s| s.to_owned()),
                new_value: m.new_value().to_owned(),
                timestamp: m.timestamp(),
                child_short_code: child_sc,
                child_tension_id: if !is_self { Some(m.tension_id().to_owned()) } else { None },
            });
        }
    }

    events
}

/// Determine what triggered an epoch boundary using pre-loaded mutations.
fn compute_boundary_trigger(
    epoch: &EpochRecord,
    epochs: &[EpochRecord],
    epoch_idx: usize,
    all_mutations: &[sd_core::Mutation],
) -> BoundaryTrigger {
    if let Some(ref etype) = epoch.epoch_type {
        return BoundaryTrigger::Structural(etype.clone());
    }

    // Try trigger gesture (scan pre-loaded mutations, no DB query)
    if let Some(ref gesture_id) = epoch.trigger_gesture_id {
        let has_desire = all_mutations.iter().any(|m|
            m.gesture_id() == Some(gesture_id.as_str())
            && m.tension_id() == epoch.tension_id
            && m.field() == "desired"
        );
        let has_reality = all_mutations.iter().any(|m|
            m.gesture_id() == Some(gesture_id.as_str())
            && m.tension_id() == epoch.tension_id
            && m.field() == "actual"
        );

        return match (has_desire, has_reality) {
            (true, true) => BoundaryTrigger::BothChanged,
            (true, false) => BoundaryTrigger::DesireChanged,
            (false, true) => BoundaryTrigger::RealityChanged,
            (false, false) => BoundaryTrigger::Unknown,
        };
    }

    // Fallback: compare snapshots
    if epoch_idx > 0 {
        let prev = &epochs[epoch_idx - 1];
        let d = epoch.desire_snapshot != prev.desire_snapshot;
        let r = epoch.reality_snapshot != prev.reality_snapshot;
        match (d, r) {
            (true, true) => BoundaryTrigger::BothChanged,
            (true, false) => BoundaryTrigger::DesireChanged,
            (false, true) => BoundaryTrigger::RealityChanged,
            (false, false) => BoundaryTrigger::Unknown,
        }
    } else {
        BoundaryTrigger::BothChanged
    }
}

/// Build provenance from pre-loaded edges and tensions.
fn build_provenance_from(
    edges: &[sd_core::Edge],
    tension_id: &str,
    all_tensions: &[Tension],
) -> LogbaseProvenance {

    let mut prov = LogbaseProvenance::default();

    for edge in edges {
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
        self.pre_logbase_state = Some((
            self.view_orientation,
            self.parent_id.clone(),
            self.focus_state.active,
        ));

        let tension = match self.engine.store().get_tension(tension_id) {
            Ok(Some(t)) => t,
            _ => return,
        };

        let epochs = self.engine.store()
            .get_epochs(tension_id)
            .unwrap_or_default();

        // Single-pass data load — one query per table, then build everything from cache
        let data = load_logbase_data(&tension, &epochs, self.engine.store());

        let focused_epoch = if !epochs.is_empty() { epochs.len() - 1 } else { 0 };
        let items = build_list_items(&data.events, &epochs, focused_epoch, &data.id_lookup, &data.id_to_desire, None, false);

        self.logbase_tension_id = Some(tension_id.to_owned());
        self.logbase_tension = Some(tension);
        self.logbase_epochs = epochs;
        self.logbase_events = data.events;
        self.logbase_provenance = data.provenance;
        self.logbase_focused_epoch = focused_epoch;
        self.logbase_items = items;
        self.logbase_id_lookup = data.id_lookup;
        self.logbase_id_to_desire = data.id_to_desire;
        self.logbase_header = data.header;
        self.logbase_separator = data.separator_label;

        *self.logbase_list_state.borrow_mut() = ftui::widgets::list::ListState::default();
        self.logbase_list_state.borrow_mut().select(Some(0));

        self.view_orientation = crate::state::ViewOrientation::Logbase;
    }

    /// Rebuild the list items (call when focused epoch changes).
    pub fn rebuild_logbase_items(&mut self) {
        self.logbase_items = build_list_items(
            &self.logbase_events,
            &self.logbase_epochs,
            self.logbase_focused_epoch,
            &self.logbase_id_lookup,
            &self.logbase_id_to_desire,
            self.logbase_expanded,
            self.logbase_show_all,
        );
    }

    /// Return from logbase to the originating view.
    pub fn exit_logbase(&mut self) {
        if let Some((orientation, parent_id, focus_id)) = self.pre_logbase_state.take() {
            self.view_orientation = orientation;
            if orientation == crate::state::ViewOrientation::Stream {
                self.parent_id = parent_id;
                self.load_siblings();
                self.focus_state.active = focus_id;
            }
        } else {
            self.view_orientation = crate::state::ViewOrientation::Stream;
        }

        self.logbase_tension_id = None;
        self.logbase_tension = None;
        self.logbase_epochs.clear();
        self.logbase_events.clear();
        self.logbase_items.clear();
        self.logbase_header.clear();
        self.logbase_separator.clear();
        self.logbase_provenance = LogbaseProvenance::default();
        self.logbase_id_to_desire.clear();
        self.logbase_expanded = None;
        self.logbase_show_all = false;
    }

    /// Get the event index for the currently selected list item.
    pub fn logbase_selected_event(&self) -> Option<usize> {
        self.logbase_list_state.borrow().selected()
            .and_then(|i| self.logbase_items.get(i))
            .map(|item| item.event_index)
    }
}

// ---------------------------------------------------------------------------
// List item construction
// ---------------------------------------------------------------------------

/// Build display items from the event stream.
///
/// Events grouped by date: date header, then indented glyph+text lines.
/// Epoch sections: desire at top, events in middle, reality at bottom.
/// Changed snapshots get HeaderStyle::Text, unchanged get HeaderStyle::Dim.
fn build_list_items(
    events: &[LogbaseEvent],
    epochs: &[EpochRecord],
    focused_epoch: usize,
    id_to_shortcode: &std::collections::HashMap<String, Option<i32>>,
    id_to_desire: &std::collections::HashMap<String, String>,
    expanded_event: Option<usize>,
    show_all: bool,
) -> Vec<LogbaseItem> {
    let mut items = Vec::new();
    let mut current_date = String::new();

    for (i, event) in events.iter().enumerate() {
        let is_focused = event.epoch_index() == focused_epoch;

        match event {
            LogbaseEvent::EpochBoundary { epoch_index, .. } => {
                current_date.clear();
                // Blank separator between epochs (not before first)
                if !items.is_empty() {
                    items.push(LogbaseItem {
                        text: String::new(),
                        style: Style::default(),
                        event_index: i,
                        is_boundary: false,
                        selectable: false,
                    bright: false,
                    date: String::new(),
                    });
                }

                let epoch = &epochs[*epoch_index];
                let epoch_num = epoch_index + 1;
                let age_text = format_age(epoch.timestamp);

                let mutation_count = events.iter()
                    .filter(|e| matches!(e, LogbaseEvent::Mutation { epoch_index: ei, .. } if *ei == *epoch_index))
                    .count();

                // Determine what ACTUALLY changed vs prior epoch (ground truth)
                let prev_epoch = if *epoch_index > 0 { Some(&epochs[*epoch_index - 1]) } else { None };
                let desire_changed = prev_epoch.map_or(true, |p| p.desire_snapshot != epoch.desire_snapshot);
                let reality_changed = prev_epoch.map_or(true, |p| p.reality_snapshot != epoch.reality_snapshot);

                // Trigger label uses actual snapshot comparison, not gesture analysis
                // (gesture may have "changed" a field to the same value)
                let trigger_label = if *epoch_index == 0 {
                    String::new() // First epoch — no comparison
                } else {
                    match (desire_changed, reality_changed) {
                        (true, true) => " [\u{25C6}\u{25C7}]".to_owned(),
                        (true, false) => " [\u{25C6}]".to_owned(),
                        (false, true) => " [\u{25C7}]".to_owned(),
                        (false, false) => {
                            // No actual desire/reality change — show child activity count
                            let child_count = events.iter()
                                .filter(|e| matches!(e, LogbaseEvent::Mutation { epoch_index: ei, child_tension_id: Some(_), .. } if *ei == *epoch_index))
                                .count();
                            if child_count > 0 {
                                format!(" [{}ch]", child_count)
                            } else {
                                String::new()
                            }
                        }
                    }
                };

                let mut right_parts = Vec::new();
                if mutation_count > 0 {
                    right_parts.push(format!("{} mut", mutation_count));
                }
                right_parts.push(age_text);
                if !trigger_label.is_empty() {
                    right_parts.push(trigger_label);
                }

                // Focused epoch: boundary is pinned above the list, not in the list.
                // Non-focused epochs: boundary + summary in the list.
                if is_focused {
                    // No boundary line — it's rendered as the pinned header.
                    // Events follow directly.
                } else {
                    items.push(LogbaseItem {
                        text: format!("\u{2500}\u{2500} epoch {} \u{2500}\u{2500} {}", epoch_num, right_parts.join(" ")),
                        style: Style::default(),
                        event_index: i,
                        is_boundary: true,
                        selectable: true,
                        bright: true,
                        date: String::new(),
                    });
                }

                // Non-focused epoch: one summary line showing what changed.
                if !is_focused {
                    let summary = if desire_changed && reality_changed {
                        format!("  \u{25C6}\u{25C7} {}", werk_shared::truncate(&epoch.desire_snapshot, 120))
                    } else if reality_changed {
                        format!("  \u{25C7} {}", werk_shared::truncate(&epoch.reality_snapshot, 120))
                    } else {
                        format!("  \u{25C6} {}", werk_shared::truncate(&epoch.desire_snapshot, 120))
                    };
                    items.push(LogbaseItem {
                        text: summary,
                        style: Style::default(),
                        event_index: i,
                        is_boundary: false,
                        selectable: false,
                        bright: false,
                        date: String::new(),
                    });
                }
            }

            LogbaseEvent::Mutation { epoch_index, field, old_value, new_value, timestamp, child_short_code, child_tension_id } => {
                if *epoch_index != focused_epoch {
                    continue;
                }

                let item_date = format_date_short(*timestamp);
                let date_col = if item_date != current_date {
                    current_date = item_date.clone();
                    format!("{:<8}", item_date)
                } else {
                    "        ".to_owned()
                };

                // Resolve ULID to "#N" (short code only — detail on expansion)
                let resolve_id = |ulid: &str| -> String {
                    id_to_shortcode.get(ulid)
                        .and_then(|sc| sc.map(|n| format!("#{}", n)))
                        .unwrap_or_else(|| format!("{}…", &ulid[..8.min(ulid.len())]))
                };

                // Child ref: just "#N" for the summary line.
                let child_code = child_short_code.map(|sc| format!("#{}", sc))
                    .or_else(|| child_tension_id.as_ref().map(|cid| format!("{}…", &cid[..8.min(cid.len())])));

                // Summary line: high-level description only. Detail on expansion.
                let cc = child_code.as_deref().unwrap_or("");
                let (glyph, text) = match field.as_str() {
                    "note" if !cc.is_empty() => ("\u{203B}", format!("{} noted", cc)),
                    "note" => ("\u{203B}", werk_shared::truncate(new_value, 80).to_owned()),
                    "status" if new_value.contains("esolved") && !cc.is_empty() =>
                        ("\u{2713}", format!("resolved {}", cc)),
                    "status" if new_value.contains("esolved") =>
                        ("\u{2713}", "resolved".to_owned()),
                    "status" if new_value.contains("eleased") && !cc.is_empty() =>
                        ("\u{223C}", format!("released {}", cc)),
                    "status" if new_value.contains("eleased") =>
                        ("\u{223C}", "released".to_owned()),
                    "status" if new_value.contains("ctive") && !cc.is_empty() =>
                        ("\u{21BB}", format!("reactivated {}", cc)),
                    "status" if new_value.contains("ctive") =>
                        ("\u{21BB}", "reactivated".to_owned()),
                    "status" if !cc.is_empty() =>
                        ("\u{2022}", format!("{} status {}", cc, new_value)),
                    "status" =>
                        ("\u{2022}", format!("status {}", new_value)),
                    "desired" if !cc.is_empty() =>
                        ("\u{25C6}", format!("{} desire changed", cc)),
                    "desired" =>
                        ("\u{25C6}", "desire changed".to_owned()),
                    "actual" if !cc.is_empty() =>
                        ("\u{25C7}", format!("{} reality updated", cc)),
                    "actual" =>
                        ("\u{25C7}", "reality updated".to_owned()),
                    "position" if (new_value.is_empty() || new_value == "null") && !cc.is_empty() =>
                        ("\u{25B8}", format!("held {}", cc)),
                    "position" if new_value.is_empty() || new_value == "null" =>
                        ("\u{25B8}", "held".to_owned()),
                    "position" if !cc.is_empty() =>
                        ("\u{25B8}", format!("positioned {} at {}", cc, new_value)),
                    "position" =>
                        ("\u{25B8}", format!("positioned at {}", new_value)),
                    "horizon" if new_value.is_empty() || new_value == "null" =>
                        ("\u{2298}", "horizon cleared".to_owned()),
                    "horizon" =>
                        ("\u{2298}", format!("horizon {}", new_value)),
                    "parent_id" => {
                        let target = if new_value.is_empty() || new_value == "null" {
                            "root".to_owned()
                        } else {
                            resolve_id(new_value)
                        };
                        if !cc.is_empty() {
                            ("\u{2192}", format!("moved {} to {}", cc, target))
                        } else {
                            ("\u{2192}", format!("moved to {}", target))
                        }
                    }
                    "release_reason" if !cc.is_empty() =>
                        ("\u{223C}", format!("{} release reason set", cc)),
                    "release_reason" =>
                        ("\u{223C}", "release reason set".to_owned()),
                    "deleted" if (new_value.is_empty() || new_value == "true") && !cc.is_empty() =>
                        ("\u{2715}", format!("deleted {}", cc)),
                    "deleted" if new_value.is_empty() || new_value == "true" =>
                        ("\u{2715}", "deleted".to_owned()),
                    "deleted" => {
                        let target = resolve_id(new_value);
                        ("\u{2715}", format!("deleted {}", target))
                    }
                    _ if !cc.is_empty() => {
                        ("\u{2022}", format!("{} [{}] changed", cc, field))
                    }
                    _ => {
                        let display_value = if new_value.len() > 20 && new_value.chars().all(|c| c.is_alphanumeric()) {
                            resolve_id(new_value)
                        } else {
                            werk_shared::truncate(new_value, 60).to_owned()
                        };
                        ("\u{2022}", format!("[{}] {}", field, display_value))
                    }
                };

                let display = format!("{}{} {}", date_col, glyph, text);

                items.push(LogbaseItem {
                    text: display,
                    style: Style::default(),
                    event_index: i,
                    is_boundary: false,
                    selectable: true,
                    bright: true,
                    date: item_date.clone(),
                });

                // Expanded detail (Enter/Space): show child desire, new value, old value.
                // Each detail line is a single list item — must not exceed terminal
                // width or the List widget wraps it visually. Cap at 120 chars.
                if expanded_event == Some(i) {
                    let detail_line = |text: &str| -> String {
                        let prefix = "          ";
                        let max = 120usize.saturating_sub(prefix.len());
                        if text.chars().count() <= max {
                            format!("{}{}", prefix, text)
                        } else {
                            let truncated: String = text.chars().take(max.saturating_sub(1)).collect();
                            format!("{}{}…", prefix, truncated)
                        }
                    };

                    let detail_item = |text: String| -> LogbaseItem {
                        LogbaseItem {
                            text,
                            style: Style::default(),
                            event_index: i,
                            is_boundary: false,
                            selectable: false,
                            bright: false,
                            date: item_date.clone(),
                        }
                    };

                    // Show child tension's desire text if this is a child mutation
                    if let Some(cid) = child_tension_id {
                        if let Some(desire) = id_to_desire.get(cid.as_str()) {
                            items.push(detail_item(detail_line(desire)));
                        }
                    }
                    // Show the new value (for fields where the summary doesn't include it)
                    let show_new = matches!(field.as_str(),
                        "note" | "desired" | "actual" | "release_reason"
                    ) || (field == "status" && !new_value.contains("esolved")
                        && !new_value.contains("eleased") && !new_value.contains("ctive"));
                    if show_new && !new_value.is_empty() {
                        items.push(detail_item(detail_line(new_value)));
                    }
                    // Show old value if present
                    if let Some(old) = old_value {
                        if !old.is_empty() {
                            let old_display = if old.len() > 20 && old.chars().all(|c| c.is_alphanumeric()) {
                                resolve_id(old)
                            } else {
                                old.clone()
                            };
                            items.push(detail_item(detail_line(&format!("was {}", old_display))));
                        }
                    }
                }
            }
        }
    }

    // Batch-collapse consecutive same-type events (e.g., 12 "moved" in a row).
    // Show first 2, a "… and N more" summary, and last 1.
    // Skip when user has expanded the collapsed view.
    if !show_all {
        items = collapse_consecutive_runs(items);
    }

    items
}

/// Collapse consecutive runs of same-type selectable items into summaries.
/// Runs of 5+ items get collapsed: first 2, summary line, last 1.
fn collapse_consecutive_runs(items: Vec<LogbaseItem>) -> Vec<LogbaseItem> {
    if items.len() < 5 {
        return items;
    }

    // Extract a "run key" from item text: the glyph+verb prefix (e.g., "→ moved", "✓ resolved")
    let run_key = |item: &LogbaseItem| -> Option<String> {
        if !item.selectable || item.is_boundary {
            return None;
        }
        // Format is: "    glyph text..." — extract glyph + first word
        let text = item.text.trim();
        let parts: Vec<&str> = text.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            Some(format!("{} {}", parts[0], parts[1]))
        } else {
            None
        }
    };

    let mut result = Vec::with_capacity(items.len());
    let mut i = 0;
    while i < items.len() {
        let key = run_key(&items[i]);
        if key.is_none() {
            result.push(items[i].clone());
            i += 1;
            continue;
        }
        let key = key.unwrap();

        // Count the run length
        let run_start = i;
        while i < items.len() && run_key(&items[i]).as_deref() == Some(&key) {
            i += 1;
        }
        // Also include non-selectable detail lines that follow each item
        let run_end = i; // exclusive

        let run_len = run_end - run_start;
        if run_len < 5 {
            // Short run — keep all
            for j in run_start..run_end {
                result.push(items[j].clone());
            }
        } else {
            // Long run — show first 2, summary, last 1
            result.push(items[run_start].clone());
            result.push(items[run_start + 1].clone());
            let collapsed = run_len - 3;
            result.push(LogbaseItem {
                text: format!("          \u{2026} and {} more {}", collapsed, key),
                style: Style::default(),
                event_index: items[run_start + 2].event_index,
                is_boundary: false,
                selectable: true, // Enter expands the full run
                bright: false,
                date: items[run_start + 2].date.clone(),
            });
            result.push(items[run_end - 1].clone());
        }
    }

    result
}

/// Build cached header lines (called once during enter_logbase).
fn build_header_cache(
    tension: &Tension,
    provenance: &LogbaseProvenance,
    store: &sd_core::Store,
) -> Vec<(String, HeaderStyle)> {
    let mut lines = Vec::new();

    // Parent ref
    if let Some(ref pid) = tension.parent_id {
        if let Ok(Some(parent)) = store.get_tension(pid) {
            let display = werk_shared::display_id(parent.short_code, &parent.id);
            lines.push((format!("  \u{2190} {} {}", display, werk_shared::truncate(&parent.desired, 120)), HeaderStyle::Dim));
        }
    }

    // Desire (capped to 2 lines at ~120 chars each)
    let display = werk_shared::display_id(tension.short_code, &tension.id);
    let desire = format!("  \u{25C6} {} {}", display, tension.desired);
    let wrapped = word_wrap(&desire, 120);
    for line in wrapped.iter().take(2) {
        lines.push((line.clone(), HeaderStyle::Text));
    }

    // Frontier summary
    if let Ok(children) = store.get_children(&tension.id) {
        if !children.is_empty() {
            let done = children.iter().filter(|c| c.status == sd_core::TensionStatus::Resolved || c.status == sd_core::TensionStatus::Released).count();
            let held = children.iter().filter(|c| c.status == sd_core::TensionStatus::Active && c.position.is_none()).count();
            let mut parts = vec![format!("[{}/{}]", done, children.len())];
            if held > 0 { parts.push(format!("{} held", held)); }
            lines.push((format!("    {}", parts.join(" \u{00b7} ")), HeaderStyle::Dim));
        }
    }

    // Reality (capped to 2 lines)
    if !tension.actual.is_empty() {
        let reality = format!("  \u{25C7} {}", tension.actual);
        let wrapped = word_wrap(&reality, 120);
        for line in wrapped.iter().take(2) {
            lines.push((line.clone(), HeaderStyle::Subdued));
        }
    }

    // Provenance
    for r in &provenance.split_from {
        let d = werk_shared::display_id(r.short_code, &r.id);
        lines.push((format!("  \u{2919} split from {} {}", d, werk_shared::truncate(&r.desired, 80)), HeaderStyle::Dim));
    }
    for r in &provenance.split_into {
        let d = werk_shared::display_id(r.short_code, &r.id);
        lines.push((format!("  \u{291A} split into {} {}", d, werk_shared::truncate(&r.desired, 80)), HeaderStyle::Dim));
    }
    for r in &provenance.merged_from {
        let d = werk_shared::display_id(r.short_code, &r.id);
        lines.push((format!("  \u{291B} merged from {} {}", d, werk_shared::truncate(&r.desired, 80)), HeaderStyle::Dim));
    }

    lines
}

// ---------------------------------------------------------------------------
// Rendering (using ftui List widget — pure, no store queries)
// ---------------------------------------------------------------------------

impl InstrumentApp {
    /// Render the logbase view. Pure — reads only cached fields, no store queries.
    pub fn render_logbase(&self, area: &Rect, frame: &mut Frame<'_>) {
        use ftui::widgets::list::{List, ListItem};
        use ftui::widgets::StatefulWidget;

        let area = self.layout.content_area(*area);
        let w = area.width as usize;

        if self.logbase_tension.is_none() {
            Paragraph::new(Text::from_lines(vec![Line::from_spans([
                Span::styled("  No tension loaded.", self.styles.dim),
            ])])).render(area, frame);
            return;
        }

        // === Header from cache ===
        let header_lines: Vec<Line> = self.logbase_header.iter().map(|(text, hstyle)| {
            let style = match hstyle {
                HeaderStyle::Dim => self.styles.dim,
                HeaderStyle::Text => self.styles.text,
                HeaderStyle::Subdued => self.styles.subdued,
            };
            Line::from_spans([Span::styled(text.clone(), style)])
        }).collect();

        let header_height = header_lines.len() as u16;
        let sep_height: u16 = 1;
        let stream_height = area.height.saturating_sub(header_height + sep_height);

        // Render header
        Paragraph::new(Text::from_lines(header_lines))
            .render(Rect::new(area.x, area.y, area.width, header_height), frame);

        // Render separator from cache
        let sep_y = area.y + header_height;
        let rule_w = w.saturating_sub(self.logbase_separator.len());
        let sep_line = format!("{}{}{}", "\u{2500}".repeat(rule_w / 2), self.logbase_separator, "\u{2500}".repeat(rule_w - rule_w / 2));
        Paragraph::new(Text::from(Line::from_spans([Span::styled(sep_line, self.styles.dim)])))
            .render(Rect::new(area.x, sep_y, area.width, 1), frame);

        // === Focused epoch desire/reality anchors (pinned, never scroll away) ===
        let focused = self.logbase_epochs.get(self.logbase_focused_epoch);
        let prev_epoch = if self.logbase_focused_epoch > 0 {
            self.logbase_epochs.get(self.logbase_focused_epoch - 1)
        } else {
            None
        };

        // Focused epoch boundary line (pinned above desire)
        let epoch_line_h: u16 = if focused.is_some() { 1 } else { 0 };

        // Desire anchor (word-wrapped, capped at 3 lines)
        let desire_lines: Vec<String> = if let Some(epoch) = focused {
            let desire_text = format!("  \u{25C6} {}", &epoch.desire_snapshot);
            let wrapped = word_wrap(&desire_text, w.saturating_sub(2));
            wrapped.into_iter().take(3).collect()
        } else {
            Vec::new()
        };
        let desire_h = desire_lines.len() as u16;

        // Reality anchor (word-wrapped, capped at 3 lines)
        let reality_lines: Vec<String> = if let Some(epoch) = focused {
            let reality_text = format!("  \u{25C7} {}", &epoch.reality_snapshot);
            let wrapped = word_wrap(&reality_text, w.saturating_sub(2));
            wrapped.into_iter().take(3).collect()
        } else {
            Vec::new()
        };
        let reality_h = reality_lines.len() as u16;

        let stream_y = sep_y + sep_height;
        let list_height = stream_height.saturating_sub(epoch_line_h + desire_h + reality_h);
        if list_height < 2 || self.logbase_items.is_empty() {
            return;
        }

        // Render focused epoch boundary line (pinned)
        if let Some(epoch) = focused {
            let epoch_num = self.logbase_focused_epoch + 1;
            let age_text = format_age(epoch.timestamp);
            let mutation_count = self.logbase_events.iter()
                .filter(|e| matches!(e, LogbaseEvent::Mutation { epoch_index, .. } if *epoch_index == self.logbase_focused_epoch))
                .count();
            let trigger = if let Some(prev) = prev_epoch {
                let d = epoch.desire_snapshot != prev.desire_snapshot;
                let r = epoch.reality_snapshot != prev.reality_snapshot;
                match (d, r) { (true, true) => " [\u{25C6}\u{25C7}]", (true, false) => " [\u{25C6}]", (false, true) => " [\u{25C7}]", _ => "" }
            } else { "" };
            let mut parts = Vec::new();
            if mutation_count > 0 { parts.push(format!("{} mut", mutation_count)); }
            parts.push(age_text);
            if !trigger.is_empty() { parts.push(trigger.to_owned()); }
            let right = parts.join(" ");
            let label = format!("epoch {}", epoch_num);
            let rule_w = w.saturating_sub(5 + label.len() + right.len() + 2);
            let epoch_text = format!("\u{2500}\u{2500} {} {} {}", label, "\u{2500}".repeat(rule_w), right);
            Paragraph::new(Text::from(Line::from_spans([Span::styled(epoch_text, self.styles.dim)])))
                .render(Rect::new(area.x, stream_y, area.width, 1), frame);
        }

        // Render desire anchor (word-wrapped, below epoch line)
        let desire_y = stream_y + epoch_line_h;
        if let Some(epoch) = focused {
            let desire_changed = prev_epoch.map_or(true, |p| p.desire_snapshot != epoch.desire_snapshot);
            let desire_style = if desire_changed { self.styles.amber } else { self.styles.subdued };
            let lines: Vec<Line> = desire_lines.iter()
                .map(|l| Line::from_spans([Span::styled(l.clone(), desire_style)]))
                .collect();
            Paragraph::new(Text::from_lines(lines))
                .render(Rect::new(area.x, desire_y, area.width, desire_h), frame);
        }

        // List at full height — dates are inline in the event text.
        let list_y = stream_y + epoch_line_h + desire_h;
        self.logbase_list_height.set(list_height);
        let list_area = Rect::new(area.x, list_y, area.width, list_height);

        let list_items: Vec<ListItem> = self.logbase_items.iter()
            .map(|item| {
                ListItem::new(item.text.as_str())
                    .style(self.styles.dim)
                    .marker("  ")
            })
            .collect();

        let list = List::new(list_items)
            .style(self.styles.dim)
            .highlight_style(Style::new().fg(self.styles.clr_dim).bg(self.styles.clr_cyan).bold())
            .highlight_symbol("\u{25B8} ");

        let mut state = self.logbase_list_state.borrow_mut();
        StatefulWidget::render(&list, list_area, frame, &mut state);
        let offset = state.offset;
        drop(state);

        // Sticky date: if the first visible item has a blank date column,
        // overlay the date from the nearest preceding item that has one.
        // Only covers the date column (first 10 chars including highlight marker),
        // so the event text and highlight remain intact.
        if offset > 0 {
            let first_visible_date = self.logbase_items.get(offset)
                .filter(|i| i.date.is_empty())
                .is_some();
            if first_visible_date {
                // Find the date by scanning backward from offset
                let date = self.logbase_items[..offset].iter().rev()
                    .find(|i| !i.date.is_empty())
                    .map(|i| i.date.as_str())
                    .unwrap_or("");
                if !date.is_empty() {
                    // Overlay just the date column area (after the 2-char marker)
                    let date_text = format!("{:<8}", date);
                    Paragraph::new(Text::from(Line::from_spans([
                        Span::styled(date_text, self.styles.dim),
                    ])))
                    .render(Rect::new(area.x + 2, list_y, 8, 1), frame);
                }
            }
        }

        // Render reality anchor (word-wrapped, below list)
        if let Some(epoch) = focused {
            let reality_changed = prev_epoch.map_or(true, |p| p.reality_snapshot != epoch.reality_snapshot);
            let reality_style = if reality_changed { self.styles.amber } else { self.styles.subdued };
            let lines: Vec<Line> = reality_lines.iter()
                .map(|l| Line::from_spans([Span::styled(l.clone(), reality_style)]))
                .collect();
            let reality_y = list_y + list_height;
            Paragraph::new(Text::from_lines(lines))
                .render(Rect::new(area.x, reality_y, area.width, reality_h), frame);
        }
    }

    /// Render the logbase bottom bar with compression counts.
    pub fn render_logbase_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let content = self.layout.content_area(Rect::new(area.x, area.y, area.width, area.height + 10));
        let bar_area = Rect::new(content.x, area.y, content.width, 1);

        let tension_label = self.logbase_tension.as_ref()
            .map(|t| format!("Log {}", werk_shared::display_id(t.short_code, &t.id)))
            .unwrap_or_default();

        let state = self.logbase_list_state.borrow();
        let selected = state.selected();
        let offset = state.offset;
        drop(state);

        let epoch_label = selected
            .and_then(|i| self.logbase_items.get(i))
            .map(|item| {
                let epoch_num = self.logbase_events.get(item.event_index)
                    .map(|e| e.epoch_index() + 1).unwrap_or(0);
                format!("epoch {}/{}", epoch_num, self.logbase_epochs.len())
            })
            .unwrap_or_default();

        // Count focused-epoch selectable items above/below visible area
        let focused_ep = self.logbase_focused_epoch;
        let is_focused_event = |item: &LogbaseItem| -> bool {
            item.selectable && !item.is_boundary && self.logbase_events.get(item.event_index)
                .map(|e| e.epoch_index() == focused_ep)
                .unwrap_or(false)
        };
        let above = self.logbase_items.get(..offset)
            .map(|s| s.iter().filter(|i| is_focused_event(i)).count())
            .unwrap_or(0);
        let visible = self.logbase_list_height.get() as usize;
        let below_start = (offset + visible).min(self.logbase_items.len());
        let below = self.logbase_items.get(below_start..)
            .map(|s| s.iter().filter(|i| is_focused_event(i)).count())
            .unwrap_or(0);

        let mut parts = vec![tension_label, epoch_label];
        if above > 0 {
            parts.push(format!("\u{25B4}{} above", above));
        }
        if below > 0 {
            parts.push(format!("\u{25BE}{} below", below));
        }

        let bar_text = format!(" {} ", parts.join(" \u{00b7} "));
        Paragraph::new(Text::from(Line::from_spans([Span::styled(bar_text, self.styles.dim)])))
            .render(bar_area, frame);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
            current = format!("    {}", word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
