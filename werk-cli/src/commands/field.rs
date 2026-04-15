//! `werk field` — the aggregate command center across every registered space.
//!
//! The field-scope counterpart to `werk stats`: instead of one workspace's
//! vitals, it shows every registered space's vitals pooled together, with
//! `--attention` adding the three exception bands (overdue / next-up / held)
//! tagged by space.
//!
//! Locality is preserved by API shape. We sum per-space counts and tag items
//! by space; we never infer cross-space rankings or cross-space blockedness.
//! Every number shown is the workspace's own standard — the field view is
//! an honest read of the federation, not a new interpretive layer.

use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use werk_shared::aggregate::{
    AggregateAttention, AggregateVitals, AttentionItem, DEFAULT_HELD_PER_SPACE,
    DEFAULT_NEXT_UP_PER_SPACE, SkippedSpace, SpaceVitals, VitalsTotals,
    compute_aggregate_attention, compute_aggregate_vitals,
};
use werk_shared::cli_display::Palette;
use werk_shared::{relative_time, truncate};

#[derive(Serialize)]
struct FieldJson {
    vitals: AggregateVitals,
    #[serde(skip_serializing_if = "Option::is_none")]
    attention: Option<AggregateAttention>,
}

pub fn cmd_field(output: &Output, attention: bool) -> Result<(), WerkError> {
    let now = chrono::Utc::now();
    let vitals = compute_aggregate_vitals(now)?;
    let attention_data = if attention {
        Some(compute_aggregate_attention(
            now,
            DEFAULT_NEXT_UP_PER_SPACE,
            DEFAULT_HELD_PER_SPACE,
        )?)
    } else {
        None
    };

    if output.is_structured() {
        let result = FieldJson {
            vitals,
            attention: attention_data,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
        return Ok(());
    }

    let palette = output.palette();
    print_vitals_table(&vitals, &palette, now);
    if let Some(att) = attention_data {
        println!();
        print_attention(&att, &palette);
    }

    if !vitals.skipped.is_empty() {
        print_skipped_warning(&vitals.skipped);
    }

    println!();
    let hint = if !attention {
        "`werk field --attention` for pooled overdue / next-up / held"
    } else {
        "`werk -w <name> show <id>` to drill into a specific item"
    };
    crate::hints::print_hint(&palette, hint);

    Ok(())
}

fn print_vitals_table(vitals: &AggregateVitals, palette: &Palette, now: chrono::DateTime<chrono::Utc>) {
    // Fixed-width column layout. Space name + six numeric columns + last-act.
    let name_w = vitals
        .spaces
        .iter()
        .map(|s| s.space.name.chars().count())
        .max()
        .unwrap_or(5)
        .max("SPACE".len());

    let header = format!(
        "{:<name_w$}  {:>6}  {:>7}  {:>10}  {:>4}  {}",
        "SPACE",
        "ACTIVE",
        "OVERDUE",
        "POSITIONED",
        "HELD",
        "LAST ACT",
        name_w = name_w,
    );
    println!("{}", palette.chrome(&header));

    for sv in &vitals.spaces {
        print_space_row(sv, name_w, palette, now);
    }

    // Separator + totals.
    let rule = "─".repeat(name_w + 2 + 6 + 2 + 7 + 2 + 10 + 2 + 4 + 2 + 12);
    println!("{}", palette.chrome(&rule));
    print_totals_row(&vitals.totals, name_w, palette);
}

fn print_space_row(
    sv: &SpaceVitals,
    name_w: usize,
    palette: &Palette,
    now: chrono::DateTime<chrono::Utc>,
) {
    let last_act = sv
        .last_activity
        .map(|t| relative_time(t, now))
        .unwrap_or_else(|| "—".to_string());

    // Only color OVERDUE when it's non-zero — silence-by-default.
    let overdue_cell = if sv.overdue > 0 {
        palette.danger(&format!("{:>7}", sv.overdue))
    } else {
        format!("{:>7}", sv.overdue)
    };

    println!(
        "{:<name_w$}  {:>6}  {}  {:>10}  {:>4}  {}",
        sv.space.name,
        sv.active,
        overdue_cell,
        sv.positioned,
        sv.held,
        last_act,
        name_w = name_w,
    );
}

fn print_totals_row(totals: &VitalsTotals, name_w: usize, palette: &Palette) {
    let overdue_cell = if totals.overdue > 0 {
        palette.danger(&format!("{:>7}", totals.overdue))
    } else {
        format!("{:>7}", totals.overdue)
    };
    let label = palette.chrome("TOTAL");
    println!(
        "{:<name_w$}  {:>6}  {}  {:>10}  {:>4}",
        label,
        totals.active,
        overdue_cell,
        totals.positioned,
        totals.held,
        name_w = name_w,
    );
}

fn print_attention(att: &AggregateAttention, palette: &Palette) {
    print_band("Overdue", &att.overdue, palette, true);
    print_band("Next up", &att.next_up, palette, false);
    print_band("Held", &att.held, palette, false);
}

fn print_band(label: &str, items: &[AttentionItem], palette: &Palette, danger: bool) {
    if items.is_empty() {
        return;
    }
    let heading = format!("{} ({})", label, items.len());
    if danger {
        println!("{}", palette.danger(&heading));
    } else {
        println!("{}", palette.chrome(&heading));
    }

    // Widest tag = `[space:#N]`. Compute once for consistent alignment.
    let tag_w = items.iter().map(|i| item_tag(i).chars().count()).max().unwrap_or(0);

    for item in items {
        let tag = item_tag(item);
        let tag_padded = format!("{:<tag_w$}", tag, tag_w = tag_w);
        let tag_styled = if danger {
            palette.danger(&tag_padded)
        } else {
            palette.chrome(&tag_padded)
        };
        let desired = truncate(&item.desired, 60);
        let suffix = item_suffix(item);
        if suffix.is_empty() {
            println!("  {}  {}", tag_styled, desired);
        } else {
            println!(
                "  {}  {}  {}",
                tag_styled,
                desired,
                palette.chrome(&suffix)
            );
        }
    }
    println!();
}

fn item_tag(item: &AttentionItem) -> String {
    let id = match item.short_code {
        Some(c) => format!("#{}", c),
        None => "?".to_string(),
    };
    format!("[{}:{}]", item.space_name, id)
}

fn item_suffix(item: &AttentionItem) -> String {
    match (&item.horizon, item.position) {
        (Some(h), _) => format!("due {}", h),
        (None, Some(p)) => format!("pos {}", p),
        (None, None) => String::new(),
    }
}

fn print_skipped_warning(skipped: &[SkippedSpace]) {
    let count = skipped.len();
    let mut msg = format!(
        "{} space{} skipped: ",
        count,
        if count == 1 { "" } else { "s" }
    );
    msg.push_str(
        &skipped
            .iter()
            .map(|s| format!("{} ({})", s.name, s.reason))
            .collect::<Vec<_>>()
            .join(", "),
    );
    eprintln!("{}", msg);
}
