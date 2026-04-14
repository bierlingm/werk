//! Log command handler — the CLI query surface for the logbase.
//!
//! `werk log <id>` — epoch history for a tension (with provenance traversal)
//! `werk log` — cross-tension timeline of recent epoch transitions
//! `werk log <id> --search <term>` — text search across epoch snapshots
//! `werk log <id> --since <timespec>` — temporal filter
//! `werk log <id> --compare` — ghost geometry (desire-reality bar chart)
//! `werk log <address>` — address-aware (e.g., `#42~e3`, `#42@2026-03`)
//!
//! All modes support `--json` for structured output.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;
use werk_core::address::{Address, parse_address};
use werk_shared::cli_display::glyphs;

// ── JSON output structs ───────────────────────────────────────────

#[derive(Serialize)]
struct LogResult {
    tension_id: String,
    short_code: Option<i32>,
    desired: String,
    epochs: Vec<LogEpochEntry>,
    provenance: Option<ProvenanceInfo>,
}

#[derive(Serialize)]
struct LogEpochEntry {
    number: usize,
    id: String,
    timestamp: String,
    desire_snapshot: String,
    reality_snapshot: String,
    epoch_type: Option<String>,
    mutation_count: usize,
}

#[derive(Serialize)]
struct ProvenanceInfo {
    split_from: Vec<ProvenanceRef>,
    merged_into: Vec<ProvenanceRef>,
    split_children: Vec<ProvenanceRef>,
    merge_sources: Vec<ProvenanceRef>,
}

#[derive(Serialize)]
struct ProvenanceRef {
    id: String,
    short_code: Option<i32>,
    desired: String,
}

#[derive(Serialize)]
struct TimelineResult {
    entries: Vec<TimelineEntry>,
}

#[derive(Serialize)]
struct TimelineEntry {
    timestamp: String,
    tension_id: String,
    short_code: Option<i32>,
    desired: String,
    epoch_number: usize,
    epoch_type: Option<String>,
}

#[derive(Serialize)]
struct CompareResult {
    tension_id: String,
    short_code: Option<i32>,
    epochs: Vec<CompareEntry>,
}

#[derive(Serialize)]
struct CompareEntry {
    number: usize,
    timestamp: String,
    desire_snapshot: String,
    reality_snapshot: String,
}

// ── Main entry point ──────────────────────────────────────────────

pub fn cmd_log(
    output: &Output,
    id: Option<String>,
    search: Option<String>,
    since: Option<String>,
    compare: bool,
    session: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // No ID: cross-tension timeline
    if id.is_none() {
        return cmd_log_timeline(output, &store, since.as_deref());
    }

    let id_str = id.unwrap();

    // Try parsing as an address first
    if let Ok(addr) = parse_address(&id_str) {
        match addr {
            Address::Tension(n) => {
                let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
                let tension = tensions
                    .iter()
                    .find(|t| t.short_code == Some(n))
                    .ok_or_else(|| WerkError::InvalidInput(format!("tension #{} not found", n)))?;
                if compare {
                    return cmd_log_compare(output, &store, tension);
                }
                return cmd_log_tension(
                    output,
                    &store,
                    tension,
                    search.as_deref(),
                    since.as_deref(),
                    session,
                );
            }
            Address::Epoch { tension, epoch_num } => {
                let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
                let t = tensions
                    .iter()
                    .find(|t| t.short_code == Some(tension))
                    .ok_or_else(|| {
                        WerkError::InvalidInput(format!("tension #{} not found", tension))
                    })?;
                return cmd_log_epoch_detail(output, &store, t, epoch_num);
            }
            Address::TensionAt { tension, timespec } => {
                let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
                let t = tensions
                    .iter()
                    .find(|t| t.short_code == Some(tension))
                    .ok_or_else(|| {
                        WerkError::InvalidInput(format!("tension #{} not found", tension))
                    })?;
                return cmd_log_tension(output, &store, t, None, Some(&timespec), session);
            }
            Address::Gesture(gid) => {
                return cmd_log_gesture(output, &store, &gid);
            }
            _ => {}
        }
    }

    // Fall back to prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());
    let tension = resolver.resolve(&id_str)?;

    if compare {
        return cmd_log_compare(output, &store, &tension);
    }

    cmd_log_tension(
        output,
        &store,
        &tension,
        search.as_deref(),
        since.as_deref(),
        session,
    )
}

// ── Tension log ───────────────────────────────────────────────────

fn cmd_log_tension(
    output: &Output,
    store: &werk_core::Store,
    tension: &werk_core::Tension,
    search: Option<&str>,
    since: Option<&str>,
    _session: bool,
) -> Result<(), WerkError> {
    let tension_id = &tension.id;
    let display = werk_shared::display_id(tension.short_code, tension_id);

    let mut epochs = store
        .get_epochs(tension_id)
        .map_err(WerkError::StoreError)?;

    // Temporal filter
    if let Some(since_str) = since {
        let cutoff = parse_timespec(since_str)?;
        epochs.retain(|e| e.timestamp >= cutoff);
    }

    // Text search filter
    if let Some(term) = search {
        let term_lower = term.to_lowercase();
        epochs.retain(|e| {
            e.desire_snapshot.to_lowercase().contains(&term_lower)
                || e.reality_snapshot.to_lowercase().contains(&term_lower)
        });
    }

    // Gather provenance from edges
    let edges = store
        .get_edges_for_tension(tension_id)
        .map_err(WerkError::StoreError)?;
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    let provenance = build_provenance(&edges, tension_id, &all_tensions);

    if output.is_structured() {
        let all_epochs = store
            .get_epochs(tension_id)
            .map_err(WerkError::StoreError)?;
        let result = LogResult {
            tension_id: tension_id.clone(),
            short_code: tension.short_code,
            desired: tension.desired.clone(),
            epochs: epochs
                .iter()
                .map(|e| {
                    let epoch_idx = all_epochs.iter().position(|ae| ae.id == e.id).unwrap_or(0);
                    let mutation_count =
                        count_mutations_in_epoch(store, tension_id, &all_epochs, epoch_idx);
                    LogEpochEntry {
                        number: epoch_idx + 1,
                        id: e.id.clone(),
                        timestamp: e.timestamp.to_rfc3339(),
                        desire_snapshot: e.desire_snapshot.clone(),
                        reality_snapshot: e.reality_snapshot.clone(),
                        epoch_type: e.epoch_type.clone(),
                        mutation_count,
                    }
                })
                .collect(),
            provenance: if has_provenance(&provenance) {
                Some(provenance)
            } else {
                None
            },
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let palette = output.palette();

        if epochs.is_empty() {
            println!("No epochs for {}", display);
            if let Some(term) = search {
                println!("  (filtered by search: \"{}\")", term);
            }
            if let Some(since_str) = since {
                println!("  (filtered by --since {})", since_str);
            }
            return Ok(());
        }

        println!(
            "{} {} — {}",
            palette.bold(&palette.structure("Log for")),
            palette.bold(&display),
            truncate(&tension.desired, 60),
        );

        // Show provenance if any
        print_provenance(&provenance);

        println!();

        let all_epochs = store
            .get_epochs(tension_id)
            .map_err(WerkError::StoreError)?;

        // Render each epoch as a small zone with a left-edge rail.
        // Most-recent first. The rail is `╭` at the top, `│` between
        // lines, `╰` at the bottom — wrapping the epoch in a visual
        // container distinct from tree connectors.
        let rev_epochs: Vec<_> = epochs.iter().rev().collect();
        for epoch in rev_epochs.iter() {
            let epoch_idx = all_epochs
                .iter()
                .position(|ae| ae.id == epoch.id)
                .unwrap_or(0);
            let age = format_age(epoch.timestamp);
            let mutation_count =
                count_mutations_in_epoch(store, tension_id, &all_epochs, epoch_idx);

            let type_label = match &epoch.epoch_type {
                Some(t) => palette.chrome(&format!(" [{}]", t)),
                None => String::new(),
            };

            // Top edge — epoch identity
            println!(
                "  {} {} {}{}",
                palette.structure(glyphs::TREE_ZONE_OPEN),
                palette.bold(&format!("Epoch {}", epoch_idx + 1)),
                palette.chrome(&format!("({})", age)),
                type_label,
            );
            // Desire snapshot — testimony color (this IS the desire as
            // it stood at this point in the past, a kind of testimony).
            println!(
                "  {}   {} {}",
                palette.chrome("│"),
                palette.testimony("◆"),
                truncate(&epoch.desire_snapshot, 72),
            );
            // Reality snapshot — chrome (the past reality is metadata
            // relative to the desire that defined the gap).
            println!(
                "  {}   {} {}",
                palette.chrome("│"),
                palette.chrome("◇"),
                palette.chrome(&truncate(&epoch.reality_snapshot, 72)),
            );
            if mutation_count > 0 {
                println!(
                    "  {}   {}",
                    palette.chrome("│"),
                    palette.chrome(&format!(
                        "{} mutation{}",
                        mutation_count,
                        if mutation_count == 1 { "" } else { "s" }
                    )),
                );
            }
            // Bottom edge
            println!("  {}", palette.structure(glyphs::TREE_ZONE_CLOSE));
        }

        // Footer hint
        let hint_id: String = match tension.short_code {
            Some(c) => c.to_string(),
            None => tension.id[..8.min(tension.id.len())].to_string(),
        };
        crate::hints::print_hint(
            &palette,
            &format!(
                "`werk log {}~e<N>` for one epoch, `werk show {}` for the live state",
                hint_id, hint_id
            ),
        );
    }

    Ok(())
}

// ── Epoch detail ──────────────────────────────────────────────────

fn cmd_log_epoch_detail(
    output: &Output,
    store: &werk_core::Store,
    tension: &werk_core::Tension,
    epoch_num: usize,
) -> Result<(), WerkError> {
    let tension_id = &tension.id;
    let display = werk_shared::display_id(tension.short_code, tension_id);

    let epochs = store
        .get_epochs(tension_id)
        .map_err(WerkError::StoreError)?;

    if epoch_num == 0 || epoch_num > epochs.len() {
        return Err(WerkError::InvalidInput(format!(
            "epoch #{} does not exist ({} has {} epoch{})",
            epoch_num,
            display,
            epochs.len(),
            if epochs.len() == 1 { "" } else { "s" },
        )));
    }

    let epoch = &epochs[epoch_num - 1];
    let span_start = if epoch_num == 1 {
        tension.created_at
    } else {
        epochs[epoch_num - 2].timestamp
    };

    let mutations = store
        .get_epoch_mutations(tension_id, span_start, epoch.timestamp)
        .map_err(WerkError::StoreError)?;

    if output.is_structured() {
        #[derive(Serialize)]
        struct EpochDetail {
            tension_id: String,
            epoch_number: usize,
            desire_snapshot: String,
            reality_snapshot: String,
            epoch_type: Option<String>,
            span_start: String,
            span_end: String,
            mutations: Vec<MutationEntry>,
        }
        #[derive(Serialize)]
        struct MutationEntry {
            tension_id: String,
            timestamp: String,
            field: String,
            old_value: Option<String>,
            new_value: String,
        }
        let result = EpochDetail {
            tension_id: tension_id.clone(),
            epoch_number: epoch_num,
            desire_snapshot: epoch.desire_snapshot.clone(),
            reality_snapshot: epoch.reality_snapshot.clone(),
            epoch_type: epoch.epoch_type.clone(),
            span_start: span_start.to_rfc3339(),
            span_end: epoch.timestamp.to_rfc3339(),
            mutations: mutations
                .iter()
                .map(|m| MutationEntry {
                    tension_id: m.tension_id().to_owned(),
                    timestamp: m.timestamp().to_rfc3339(),
                    field: m.field().to_owned(),
                    old_value: m.old_value().map(|s| s.to_owned()),
                    new_value: m.new_value().to_owned(),
                })
                .collect(),
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        println!("Epoch {} for {}:", epoch_num, display);
        if let Some(ref t) = epoch.epoch_type {
            println!("  Type: {}", t);
        }
        println!();
        println!("  Desire:  {}", truncate(&epoch.desire_snapshot, 72));
        println!("  Reality: {}", truncate(&epoch.reality_snapshot, 72));
        println!();

        let start_str = &span_start.to_rfc3339()[..19].replace('T', " ");
        let end_str = &epoch.timestamp.to_rfc3339()[..19].replace('T', " ");
        println!("  Span: {} to {}", start_str, end_str);

        if mutations.is_empty() {
            println!("\n  No mutations in this epoch span.");
        } else {
            println!("\n  Mutations ({}):", mutations.len());
            for m in &mutations {
                let ts = &m.timestamp().to_rfc3339()[..19].replace('T', " ");
                match m.old_value() {
                    Some(old) => println!(
                        "    {} [{}] {} → {}",
                        ts,
                        m.field(),
                        truncate(old, 35),
                        truncate(m.new_value(), 35),
                    ),
                    None => println!(
                        "    {} [{}] → {}",
                        ts,
                        m.field(),
                        truncate(m.new_value(), 55),
                    ),
                }
            }
        }
    }

    Ok(())
}

// ── Ghost geometry (compare) ──────────────────────────────────────

fn cmd_log_compare(
    output: &Output,
    store: &werk_core::Store,
    tension: &werk_core::Tension,
) -> Result<(), WerkError> {
    let tension_id = &tension.id;
    let display = werk_shared::display_id(tension.short_code, tension_id);

    let epochs = store
        .get_epochs(tension_id)
        .map_err(WerkError::StoreError)?;

    if epochs.is_empty() {
        println!("No epochs for {} — nothing to compare.", display);
        return Ok(());
    }

    if output.is_structured() {
        let result = CompareResult {
            tension_id: tension_id.clone(),
            short_code: tension.short_code,
            epochs: epochs
                .iter()
                .enumerate()
                .map(|(i, e)| CompareEntry {
                    number: i + 1,
                    timestamp: e.timestamp.to_rfc3339(),
                    desire_snapshot: e.desire_snapshot.clone(),
                    reality_snapshot: e.reality_snapshot.clone(),
                })
                .collect(),
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        println!(
            "Ghost geometry for {} — {}",
            display,
            truncate(&tension.desired, 50)
        );
        println!();

        // Render desire-reality evolution with left-rail
        let total = epochs.len();
        for (i, epoch) in epochs.iter().enumerate() {
            let age = format_age(epoch.timestamp);
            let is_last = i == total - 1;
            let (connector, rail) = if is_last {
                ("\u{2514}", " ")
            } else {
                ("\u{251c}", "\u{2502}")
            };

            println!(
                "  {} epoch {} ({:>12})  \u{25c6} {} \u{25c7} {}",
                connector,
                i + 1,
                age,
                truncate(&epoch.desire_snapshot, 35),
                truncate(&epoch.reality_snapshot, 35),
            );
            let _ = rail; // rail available for multi-line expansion
        }

        // Current state
        println!();
        println!(
            "  current               \u{25c6} {} \u{25c7} {}",
            truncate(&tension.desired, 40),
            truncate(&tension.actual, 40)
        );
    }

    Ok(())
}

// ── Cross-tension timeline ────────────────────────────────────────

fn cmd_log_timeline(
    output: &Output,
    store: &werk_core::Store,
    since: Option<&str>,
) -> Result<(), WerkError> {
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    let cutoff = match since {
        Some(s) => parse_timespec(s)?,
        None => Utc::now() - Duration::days(7),
    };

    // Collect all recent epochs across all tensions
    let mut entries: Vec<(
        DateTime<Utc>,
        String,
        Option<i32>,
        String,
        usize,
        Option<String>,
    )> = Vec::new();

    for tension in &tensions {
        let epochs = store
            .get_epochs(&tension.id)
            .map_err(WerkError::StoreError)?;
        for (i, epoch) in epochs.iter().enumerate() {
            if epoch.timestamp >= cutoff {
                entries.push((
                    epoch.timestamp,
                    tension.id.clone(),
                    tension.short_code,
                    tension.desired.clone(),
                    i + 1,
                    epoch.epoch_type.clone(),
                ));
            }
        }
    }

    entries.sort_by(|a, b| b.0.cmp(&a.0)); // most recent first

    if output.is_structured() {
        let result = TimelineResult {
            entries: entries
                .iter()
                .map(|(ts, tid, sc, desired, n, et)| TimelineEntry {
                    timestamp: ts.to_rfc3339(),
                    tension_id: tid.clone(),
                    short_code: *sc,
                    desired: desired.clone(),
                    epoch_number: *n,
                    epoch_type: et.clone(),
                })
                .collect(),
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        if entries.is_empty() {
            println!("No epoch transitions in the last 7 days.");
            return Ok(());
        }

        println!("Recent epoch transitions:");
        println!();

        for (ts, _tid, sc, desired, n, epoch_type) in &entries {
            let display = match sc {
                Some(code) => format!("#{}", code),
                None => "(?)".to_owned(),
            };
            let ts_str = &ts.to_rfc3339()[..16].replace('T', " ");
            let type_label = match epoch_type {
                Some(t) => format!(" [{}]", t),
                None => String::new(),
            };
            println!(
                "  {} {} epoch {}{}  {}",
                ts_str,
                display,
                n,
                type_label,
                truncate(desired, 50),
            );
        }
    }

    Ok(())
}

// ── Gesture query ─────────────────────────────────────────────────

fn cmd_log_gesture(
    output: &Output,
    store: &werk_core::Store,
    gesture_id: &str,
) -> Result<(), WerkError> {
    // Find all mutations with this gesture_id
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;
    let gesture_mutations: Vec<_> = all_mutations
        .iter()
        .filter(|m| m.gesture_id() == Some(gesture_id))
        .collect();

    if gesture_mutations.is_empty() {
        return Err(WerkError::InvalidInput(format!(
            "no mutations found for gesture {}",
            gesture_id
        )));
    }

    if output.is_structured() {
        #[derive(Serialize)]
        struct GestureResult {
            gesture_id: String,
            mutation_count: usize,
            mutations: Vec<GestureMutation>,
        }
        #[derive(Serialize)]
        struct GestureMutation {
            tension_id: String,
            timestamp: String,
            field: String,
            old_value: Option<String>,
            new_value: String,
        }
        let result = GestureResult {
            gesture_id: gesture_id.to_owned(),
            mutation_count: gesture_mutations.len(),
            mutations: gesture_mutations
                .iter()
                .map(|m| GestureMutation {
                    tension_id: m.tension_id().to_owned(),
                    timestamp: m.timestamp().to_rfc3339(),
                    field: m.field().to_owned(),
                    old_value: m.old_value().map(|s| s.to_owned()),
                    new_value: m.new_value().to_owned(),
                })
                .collect(),
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        println!("Gesture {}", &gesture_id[..gesture_id.len().min(12)]);
        println!(
            "  {} mutation{}",
            gesture_mutations.len(),
            if gesture_mutations.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        println!();
        for m in &gesture_mutations {
            let ts = &m.timestamp().to_rfc3339()[..19].replace('T', " ");
            println!(
                "  {} [{}] on {} → {}",
                ts,
                m.field(),
                &m.tension_id()[..8],
                truncate(m.new_value(), 50),
            );
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────

fn build_provenance(
    edges: &[werk_core::Edge],
    tension_id: &str,
    tensions: &[werk_core::Tension],
) -> ProvenanceInfo {
    let find_tension = |id: &str| -> ProvenanceRef {
        let t = tensions.iter().find(|t| t.id == id);
        ProvenanceRef {
            id: id.to_owned(),
            short_code: t.and_then(|t| t.short_code),
            desired: t.map(|t| t.desired.clone()).unwrap_or_default(),
        }
    };

    let split_from: Vec<_> = edges
        .iter()
        .filter(|e| e.from_id == tension_id && e.edge_type == werk_core::EDGE_SPLIT_FROM)
        .map(|e| find_tension(&e.to_id))
        .collect();

    let merged_into: Vec<_> = edges
        .iter()
        .filter(|e| e.from_id == tension_id && e.edge_type == werk_core::EDGE_MERGED_INTO)
        .map(|e| find_tension(&e.to_id))
        .collect();

    let split_children: Vec<_> = edges
        .iter()
        .filter(|e| e.to_id == tension_id && e.edge_type == werk_core::EDGE_SPLIT_FROM)
        .map(|e| find_tension(&e.from_id))
        .collect();

    let merge_sources: Vec<_> = edges
        .iter()
        .filter(|e| e.to_id == tension_id && e.edge_type == werk_core::EDGE_MERGED_INTO)
        .map(|e| find_tension(&e.from_id))
        .collect();

    ProvenanceInfo {
        split_from,
        merged_into,
        split_children,
        merge_sources,
    }
}

fn has_provenance(p: &ProvenanceInfo) -> bool {
    !p.split_from.is_empty()
        || !p.merged_into.is_empty()
        || !p.split_children.is_empty()
        || !p.merge_sources.is_empty()
}

fn print_provenance(p: &ProvenanceInfo) {
    if !p.split_from.is_empty() {
        let refs: Vec<String> = p
            .split_from
            .iter()
            .map(|r| {
                format!(
                    "#{}",
                    r.short_code
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "?".into())
                )
            })
            .collect();
        println!("  Split from: {}", refs.join(", "));
    }
    if !p.merged_into.is_empty() {
        let refs: Vec<String> = p
            .merged_into
            .iter()
            .map(|r| {
                format!(
                    "#{}",
                    r.short_code
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "?".into())
                )
            })
            .collect();
        println!("  Merged into: {}", refs.join(", "));
    }
    if !p.split_children.is_empty() {
        let refs: Vec<String> = p
            .split_children
            .iter()
            .map(|r| {
                format!(
                    "#{}",
                    r.short_code
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "?".into())
                )
            })
            .collect();
        println!("  Split into: {}", refs.join(", "));
    }
    if !p.merge_sources.is_empty() {
        let refs: Vec<String> = p
            .merge_sources
            .iter()
            .map(|r| {
                format!(
                    "#{}",
                    r.short_code
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "?".into())
                )
            })
            .collect();
        println!("  Absorbed: {}", refs.join(", "));
    }
}

fn count_mutations_in_epoch(
    store: &werk_core::Store,
    tension_id: &str,
    epochs: &[werk_core::EpochRecord],
    idx: usize,
) -> usize {
    let epoch = &epochs[idx];
    let start = if idx == 0 {
        // Before first epoch — use a very old date
        DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    } else {
        epochs[idx - 1].timestamp
    };

    store
        .get_epoch_mutations(tension_id, start, epoch.timestamp)
        .map(|m| m.len())
        .unwrap_or(0)
}

fn parse_timespec(s: &str) -> Result<DateTime<Utc>, WerkError> {
    // Try ISO date first
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    // Try partial date (YYYY-MM)
    if s.len() == 7 && s.chars().nth(4) == Some('-') {
        let with_day = format!("{}-01", s);
        if let Ok(date) = NaiveDate::parse_from_str(&with_day, "%Y-%m-%d") {
            return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
        }
    }

    // Relative: "today", "yesterday", "Nd" (days), "Nw" (weeks)
    let now = Utc::now();
    match s {
        "today" => Ok(now - Duration::days(0)),
        "yesterday" => Ok(now - Duration::days(1)),
        _ => {
            if let Some(n_str) = s.strip_suffix('d') {
                let n: i64 = n_str
                    .parse()
                    .map_err(|_| WerkError::InvalidInput(format!("invalid timespec: '{}'", s)))?;
                Ok(now - Duration::days(n))
            } else if let Some(n_str) = s.strip_suffix('w') {
                let n: i64 = n_str
                    .parse()
                    .map_err(|_| WerkError::InvalidInput(format!("invalid timespec: '{}'", s)))?;
                Ok(now - Duration::weeks(n))
            } else {
                Err(WerkError::InvalidInput(format!(
                    "invalid timespec: '{}'. Use YYYY-MM-DD, YYYY-MM, today, yesterday, Nd, or Nw",
                    s
                )))
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

fn format_age(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);
    let seconds = duration.num_seconds();

    if seconds < 60 {
        "just now".to_string()
    } else if seconds < 3600 {
        format!("{} min ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{} hours ago", seconds / 3600)
    } else {
        format!("{} days ago", seconds / 86400)
    }
}
