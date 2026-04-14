//! Mutation echo — compact post-mutation tension summary.
//!
//! After a write gesture (reality, desire, resolve, release, note)
//! completes, users benefit from a brief reminder of what the tension
//! now looks like. GitButler calls this pattern `--status-after`: the
//! mutation succeeds and the program prints a tiny slice of the new
//! state so the human doesn't have to run a separate `show` command
//! to verify the edit landed.
//!
//! ## Behavior
//!
//! In **human mode** the echo is always printed below the success
//! message. It is three or four lines: a bold ID, the desire, the
//! reality, and a dim metadata line with horizon and closure progress.
//!
//! In **JSON mode** the echo is *opt-in* via `--show-after`. When
//! enabled, an additive `"show"` key is merged into the mutation's
//! result JSON without restructuring the existing top-level fields.
//! Agents that depend on `json.id` / `json.actual` see no breaking
//! change; agents that want the echo just add the flag.
//!
//! ## Why not call `cmd_show`?
//!
//! `cmd_show` is 300+ lines of field-level rendering with closure
//! frontiers, structural signals, epochs, engagement statistics, and
//! the optional `--full` expansion. The echo is intentionally much
//! smaller: it answers "did my edit land?" not "tell me everything
//! about this tension". A dedicated helper keeps the surface area
//! tiny and lets the mutation commands call it from a single line.

use crate::error::WerkError;
use werk_core::{Store, TensionStatus};
use werk_shared::cli_display::{Palette, glyphs};

/// Render a compact human-readable echo of a tension's current state.
///
/// Prints directly to stdout using the supplied palette. Intended to be
/// called by mutation commands immediately after their success message.
/// Silently becomes a no-op if the tension cannot be re-read from the
/// store (shouldn't happen in practice, but mutations should never fail
/// because their echo failed).
pub fn print_human_echo(
    store: &Store,
    palette: &Palette,
    tension_id: &str,
) -> Result<(), WerkError> {
    let tension = match store
        .get_tension(tension_id)
        .map_err(WerkError::StoreError)?
    {
        Some(t) => t,
        None => return Ok(()),
    };

    let id_display = werk_shared::display_id(tension.short_code, &tension.id);
    let status_glyph = match tension.status {
        TensionStatus::Resolved => format!(" {}", palette.resolved(glyphs::STATUS_RESOLVED)),
        TensionStatus::Released => format!(" {}", palette.chrome(glyphs::STATUS_RELEASED)),
        TensionStatus::Active => String::new(),
    };

    println!();
    println!(
        "  {}{}  {}",
        palette.bold(&id_display),
        status_glyph,
        &tension.desired
    );
    println!("  {} {}", palette.chrome("Reality:"), &tension.actual);

    // Compact metadata line: horizon · closure · position
    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(h) = &tension.horizon {
        meta_parts.push(palette.chrome(&format!("[{}]", h)));
    }
    let children = store
        .get_children(&tension.id)
        .map_err(WerkError::StoreError)?;
    if !children.is_empty() {
        let resolved_count = children
            .iter()
            .filter(|c| c.status == TensionStatus::Resolved)
            .count();
        let released_count = children
            .iter()
            .filter(|c| c.status == TensionStatus::Released)
            .count();
        let active_count = children.len() - released_count;
        let closure = if released_count > 0 {
            format!(
                "[{}/{}] ({} released)",
                resolved_count, active_count, released_count
            )
        } else {
            format!("[{}/{}]", resolved_count, active_count)
        };
        meta_parts.push(palette.chrome(&closure));
    }
    if let Some(pos) = tension.position {
        meta_parts.push(palette.chrome(&format!("{}{}", glyphs::STATUS_POSITION, pos)));
    }

    if !meta_parts.is_empty() {
        println!("  {}", meta_parts.join(&palette.chrome(" · ")));
    }

    Ok(())
}

/// Build a minimal JSON view of a tension for the additive `show` key.
///
/// The returned object is intentionally a tiny subset of [`cmd_show`]'s
/// full JSON response: it carries the core facts a caller needs to
/// verify that a mutation landed, without dragging in the expensive
/// structural/temporal analysis fields that show computes.
///
/// Callers merge this into their mutation result via:
///
/// ```ignore
/// let mut val = serde_json::to_value(&result).unwrap();
/// if show_after {
///     val["show"] = build_json_echo(&store, &tension_id)?;
/// }
/// ```
pub fn build_json_echo(store: &Store, tension_id: &str) -> Result<serde_json::Value, WerkError> {
    let tension = store
        .get_tension(tension_id)
        .map_err(WerkError::StoreError)?
        .ok_or_else(|| WerkError::InvalidInput(format!("tension {} not found", tension_id)))?;

    let children = store
        .get_children(&tension.id)
        .map_err(WerkError::StoreError)?;
    let resolved_count = children
        .iter()
        .filter(|c| c.status == TensionStatus::Resolved)
        .count();
    let released_count = children
        .iter()
        .filter(|c| c.status == TensionStatus::Released)
        .count();

    Ok(serde_json::json!({
        "id": tension.id,
        "short_code": tension.short_code,
        "desired": tension.desired,
        "actual": tension.actual,
        "status": tension.status.to_string(),
        "parent_id": tension.parent_id,
        "horizon": tension.horizon.as_ref().map(|h| h.to_string()),
        "position": tension.position,
        "closure": {
            "resolved": resolved_count,
            "active": children.len() - released_count,
            "released": released_count,
        },
    }))
}
