//! MCP tool definitions for werk.
//!
//! Each tool maps directly to a CLI gesture. Read tools return structured JSON.
//! Gesture tools mutate the tension structure and return confirmation JSON.
//! All tools operate on the same workspace discovery and store as the CLI.

use chrono::{Datelike, DateTime, NaiveDate, Utc};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use sd_core::{
    compute_frontier, compute_structural_signals, compute_temporal_signals, compute_urgency,
    detect_horizon_drift, extract_mutation_pattern, gap_magnitude, project_field, project_tension,
    Engine, Forest, Horizon, HorizonDriftType, Mutation, ProjectionHorizon, ProjectionThresholds,
    TensionStatus,
};
use serde::{Deserialize, Serialize};
use werk_shared::{Config, HookEvent, HookRunner, PrefixResolver, WerkError, Workspace};

// ── Server ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WerkServer {
    tool_router: ToolRouter<Self>,
}

// ── Parameter structs ───────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IdParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ShowParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// Include ancestors, siblings, and engagement metrics.
    #[serde(default)]
    pub full: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TreeParam {
    /// Tension ID or prefix (omit for full forest).
    #[serde(default)]
    pub id: Option<String>,
    /// Filter: "active" (default), "all", "resolved", or "released".
    #[serde(default = "default_active")]
    pub filter: String,
}

fn default_active() -> String {
    "active".to_string()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct ListParam {
    /// Filter: "all", "urgent", "neglected", "stagnant", or omit for active.
    #[serde(default)]
    pub filter: Option<String>,
    /// Sort by: "urgency" (default), "name", or "deadline".
    #[serde(default = "default_urgency")]
    pub sort: String,
    /// Only overdue tensions.
    #[serde(default)]
    pub overdue: Option<bool>,
    /// Only tensions approaching deadline within N days.
    #[serde(default)]
    pub approaching: Option<i64>,
    /// Only tensions with no mutations in N days.
    #[serde(default)]
    pub stale: Option<i64>,
    /// Only held (unpositioned) tensions.
    #[serde(default)]
    pub held: Option<bool>,
    /// Only positioned tensions.
    #[serde(default)]
    pub positioned: Option<bool>,
    /// Only root tensions.
    #[serde(default)]
    pub root: Option<bool>,
    /// Only children of this tension.
    #[serde(default)]
    pub parent: Option<String>,
    /// Only tensions with deadlines.
    #[serde(default)]
    pub has_deadline: Option<bool>,
    /// Show tensions changed since (e.g., "today", "yesterday", "3d").
    #[serde(default)]
    pub changed_since: Option<String>,
}

fn default_urgency() -> String {
    "urgency".to_string()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParam {
    /// Search query — natural language or keywords. Ranks tensions by relevance using FrankenSearch hybrid retrieval.
    pub query: String,
    /// Maximum results to return (default: 20).
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SurveyParam {
    /// Temporal frame in days (default: 14).
    #[serde(default = "default_14")]
    pub days: i64,
}

fn default_14() -> i64 {
    14
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GroundParam {
    /// Lookback window in days (default: 7).
    #[serde(default = "default_7")]
    pub days: i64,
}

fn default_7() -> i64 {
    7
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DiffParam {
    /// Show changes since date (e.g., "today", "yesterday", "2026-03-10").
    #[serde(default = "default_today")]
    pub since: String,
}

fn default_today() -> String {
    "today".to_string()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ContextParam {
    /// Tension ID or prefix (omit for all active).
    #[serde(default)]
    pub id: Option<String>,
    /// Mode: "single" (default if id given), "all", or "urgent".
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TrajectoryParam {
    /// Tension ID or prefix (omit for field-wide).
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InsightsParam {
    /// Analysis window in days (default: 30).
    #[serde(default = "default_30")]
    pub days: i64,
}

fn default_30() -> i64 {
    30
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StatsParam {
    /// Sections to include. Array of: "temporal", "attention", "changes", "trajectory", "engagement", "drift", "health". Omit or pass "all" for everything. Default: vitals only.
    #[serde(default)]
    pub sections: Option<Vec<String>>,
    /// Time window in days for windowed sections (default: 7).
    #[serde(default = "default_7")]
    pub days: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddParam {
    /// The desired outcome.
    pub desired: String,
    /// The current reality.
    pub actual: String,
    /// Parent tension ID (creates child tension).
    #[serde(default)]
    pub parent: Option<String>,
    /// Temporal horizon (e.g., "2026", "2026-05", "2026-05-15").
    #[serde(default)]
    pub horizon: Option<String>,
    /// Optional palette response action key. If creating a child with horizon triggers a containment violation, this action is applied automatically.
    #[serde(default)]
    pub palette_response: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ComposeParam {
    /// Desired outcome for the new parent.
    pub desired: String,
    /// Current reality for the new parent.
    pub actual: String,
    /// IDs of existing tensions to become children.
    pub children: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RealityParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// New current reality text.
    pub value: String,
    /// Skip epoch creation (for minor corrections).
    #[serde(default)]
    pub no_epoch: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DesireParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// New desired state text.
    pub value: String,
    /// Skip epoch creation (for minor corrections).
    #[serde(default)]
    pub no_epoch: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResolveParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// When resolution actually happened (e.g., "yesterday", "2026-03-20").
    #[serde(default)]
    pub actual_at: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReopenParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// Reason for reopening (optional).
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReleaseParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// Reason for releasing.
    pub reason: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// New parent ID (omit to make root).
    #[serde(default)]
    pub parent: Option<String>,
    /// Optional palette response action key. If the move creates a containment violation, this action is applied automatically.
    #[serde(default)]
    pub palette_response: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PositionParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// Position number (1-based).
    pub position: i32,
    /// Optional palette response action key (e.g., "swap_positions", "move_before", "hold_tension"). If sequencing pressure is detected, this action is applied automatically.
    #[serde(default)]
    pub palette_response: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HorizonParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// New horizon value, "none" to clear, or omit to display current.
    #[serde(default)]
    pub value: Option<String>,
    /// Optional palette response action key (e.g., "clip_child", "extend_parent", "promote_child", "remove_child_deadline"). If a containment violation is detected, this action is applied automatically instead of returning signals for separate handling.
    #[serde(default)]
    pub palette_response: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoteAddParam {
    /// Note text.
    pub text: String,
    /// Tension ID (omit for workspace note).
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoteRmParam {
    /// Note number (1-based) to retract.
    pub index: usize,
    /// Tension ID (omit for workspace note).
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoteListParam {
    /// Tension ID (omit for workspace notes).
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SnoozeParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// Date to snooze until (+3d, +2w, +1m, or YYYY-MM-DD).
    #[serde(default)]
    pub date: Option<String>,
    /// Clear the snooze.
    #[serde(default)]
    pub clear: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecurParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// Recurrence interval (+1d, +1w, +2w, +1m).
    #[serde(default)]
    pub interval: Option<String>,
    /// Clear the recurrence.
    #[serde(default)]
    pub clear: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EpochParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EpochListParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EpochShowParam {
    /// Tension ID, short code, or ULID prefix.
    pub id: String,
    /// Epoch number to show.
    pub epoch: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BatchParam {
    /// YAML content containing mutation list.
    pub yaml: String,
    /// Validate only, don't apply.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LogParam {
    /// Tension ID, short code, or address. Omit for cross-tension timeline.
    #[serde(default)]
    pub id: Option<String>,
    /// Text search across epoch snapshots.
    #[serde(default)]
    pub search: Option<String>,
    /// Show epochs since (YYYY-MM-DD, YYYY-MM, Nd, Nw).
    #[serde(default)]
    pub since: Option<String>,
    /// Show desire-reality evolution (ghost geometry).
    #[serde(default)]
    pub compare: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SplitParam {
    /// Source tension ID or short code.
    pub id: String,
    /// Desired states for new tensions (at least 2).
    pub desires: Vec<String>,
    /// Child assignments: ["30=1", "31=2"] — child short code = target number.
    #[serde(default)]
    pub assign: Vec<String>,
    /// Float all children to source's parent.
    #[serde(default)]
    pub children_to_parent: bool,
    /// Move all children to successor N (1-based).
    #[serde(default)]
    pub children_to: Option<usize>,
    /// Keep source active (default: resolve).
    #[serde(default)]
    pub keep: bool,
    /// Preview without making changes.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MergeParam {
    /// First tension ID.
    pub id1: String,
    /// Second tension ID.
    pub id2: String,
    /// Asymmetric: surviving tension ID (must be id1 or id2).
    #[serde(default)]
    pub into: Option<String>,
    /// Symmetric: desire for the new merged tension.
    #[serde(default)]
    pub as_desire: Option<String>,
    /// Update survivor's desire (asymmetric mode).
    #[serde(default)]
    pub desire: Option<String>,
    /// Float absorbed tension's children to its parent.
    #[serde(default)]
    pub children_to_parent: bool,
    /// Preview without making changes.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EdgesParam {
    /// Tension ID to get edges for. Omit for all edges.
    #[serde(default)]
    pub id: Option<String>,
    /// Filter by edge type (contains, split_from, merged_into).
    #[serde(default)]
    pub edge_type: Option<String>,
}

// ── Helpers ─────────────────────────────────────────────────────────

const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";

fn err(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

fn werk_err(e: WerkError) -> McpError {
    err(e.to_string())
}

fn json_result(value: &impl Serialize) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value).map_err(|e| err(e.to_string()))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn open_store() -> Result<(Workspace, sd_core::Store), McpError> {
    let workspace = Workspace::discover().map_err(werk_err)?;
    let store = workspace.open_store().map_err(werk_err)?;
    Ok((workspace, store))
}

fn resolve_id(
    tensions: &[sd_core::Tension],
    id: &str,
) -> Result<sd_core::Tension, McpError> {
    let resolver = PrefixResolver::new(tensions.to_vec());
    resolver.resolve(id).map(|t| t.clone()).map_err(werk_err)
}

fn autoflush(workspace: &Workspace) {
    let Ok(config) = Config::load(workspace) else {
        return;
    };
    if config.get("flush.auto").map(|v| v.as_str()) != Some("true") {
        return;
    }
    // Silently flush — same as CLI autoflush
    let Ok(store) = workspace.open_store() else {
        return;
    };
    let Ok(tensions) = store.list_tensions() else {
        return;
    };
    let now = Utc::now();
    let mut sorted = tensions;
    sorted.sort_by(|a, b| match (a.short_code, b.short_code) {
        (Some(sa), Some(sb)) => sa.cmp(&sb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    #[derive(Serialize)]
    struct FlushTension {
        actual: String,
        created_at: String,
        desired: String,
        horizon: Option<String>,
        id: String,
        parent_id: Option<String>,
        position: Option<i32>,
        short_code: Option<i32>,
        status: String,
    }

    #[derive(Serialize)]
    struct FlushSummary {
        active: usize,
        released: usize,
        resolved: usize,
        total: usize,
    }

    #[derive(Serialize)]
    struct FlushState {
        flushed_at: String,
        summary: FlushSummary,
        tensions: Vec<FlushTension>,
    }

    let active = sorted
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved = sorted
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released = sorted
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();

    let flush_tensions: Vec<FlushTension> = sorted
        .iter()
        .map(|t| FlushTension {
            actual: t.actual.clone(),
            created_at: t.created_at.to_rfc3339(),
            desired: t.desired.clone(),
            horizon: t.horizon.as_ref().map(|h| h.to_string()),
            id: t.id.clone(),
            parent_id: t.parent_id.clone(),
            position: t.position,
            short_code: t.short_code,
            status: t.status.to_string(),
        })
        .collect();

    let state = FlushState {
        flushed_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        summary: FlushSummary {
            active,
            released,
            resolved,
            total: sorted.len(),
        },
        tensions: flush_tensions,
    };

    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let path = workspace.root().join("tensions.json");
        let _ = std::fs::write(&path, format!("{}\n", json));
    }
}

fn parse_mcp_timespec(s: &str) -> Result<DateTime<Utc>, String> {
    use chrono::NaiveDate;
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }
    if s.len() == 7 && s.chars().nth(4) == Some('-') {
        let with_day = format!("{}-01", s);
        if let Ok(date) = NaiveDate::parse_from_str(&with_day, "%Y-%m-%d") {
            return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
        }
    }
    let now = Utc::now();
    match s {
        "today" => Ok(now),
        "yesterday" => Ok(now - chrono::Duration::days(1)),
        _ => {
            if let Some(n_str) = s.strip_suffix('d') {
                let n: i64 = n_str.parse().map_err(|_| format!("invalid timespec: '{}'", s))?;
                Ok(now - chrono::Duration::days(n))
            } else if let Some(n_str) = s.strip_suffix('w') {
                let n: i64 = n_str.parse().map_err(|_| format!("invalid timespec: '{}'", s))?;
                Ok(now - chrono::Duration::weeks(n))
            } else {
                Err(format!("invalid timespec: '{}'. Use YYYY-MM-DD, YYYY-MM, Nd, Nw", s))
            }
        }
    }
}

fn build_mcp_provenance(
    edges: &[sd_core::Edge],
    tension_id: &str,
    tensions: &[sd_core::Tension],
) -> serde_json::Value {
    let find_ref = |id: &str| -> serde_json::Value {
        let t = tensions.iter().find(|t| t.id == id);
        serde_json::json!({
            "id": id,
            "short_code": t.and_then(|t| t.short_code),
            "desired": t.map(|t| t.desired.as_str()).unwrap_or(""),
        })
    };

    let split_from: Vec<_> = edges.iter()
        .filter(|e| e.from_id == tension_id && e.edge_type == sd_core::EDGE_SPLIT_FROM)
        .map(|e| find_ref(&e.to_id)).collect();
    let merged_into: Vec<_> = edges.iter()
        .filter(|e| e.from_id == tension_id && e.edge_type == sd_core::EDGE_MERGED_INTO)
        .map(|e| find_ref(&e.to_id)).collect();
    let split_children: Vec<_> = edges.iter()
        .filter(|e| e.to_id == tension_id && e.edge_type == sd_core::EDGE_SPLIT_FROM)
        .map(|e| find_ref(&e.from_id)).collect();
    let merge_sources: Vec<_> = edges.iter()
        .filter(|e| e.to_id == tension_id && e.edge_type == sd_core::EDGE_MERGED_INTO)
        .map(|e| find_ref(&e.from_id)).collect();

    if split_from.is_empty() && merged_into.is_empty() && split_children.is_empty() && merge_sources.is_empty() {
        return serde_json::Value::Null;
    }

    serde_json::json!({
        "split_from": split_from,
        "merged_into": merged_into,
        "split_children": split_children,
        "merge_sources": merge_sources,
    })
}

fn load_hooks(workspace: &Workspace) -> HookRunner {
    Config::load(workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop())
}

/// Detect containment palettes after a horizon change, optionally applying a pre-selected response.
///
/// Returns (palette_json_array, applied_action_description).
fn mcp_check_containment(
    store: &mut sd_core::Store,
    tension_id: &str,
    palette_response: Option<&str>,
) -> Result<(Vec<serde_json::Value>, Option<String>), McpError> {
    let detected = werk_shared::detect_containment_palettes(store, tension_id)
        .map_err(werk_err)?;

    let mut palette_json = Vec::new();
    let mut applied_desc = None;

    for (palette, ctx) in detected {
        let palette_val = serde_json::to_value(&palette).unwrap_or(serde_json::Value::Null);
        palette_json.push(palette_val);

        // If a palette_response was provided, find the matching action and apply it
        if let Some(response_action) = palette_response {
            if let Some(idx) = palette.options.iter().position(|o| o.action == response_action) {
                let choice = if response_action == "dismiss" {
                    werk_shared::PaletteChoice::Dismissed
                } else {
                    werk_shared::PaletteChoice::Selected(idx)
                };
                if let Ok(Some(desc)) = werk_shared::apply_choice(store, &ctx, &choice) {
                    applied_desc = Some(desc);
                }
            }
        }
    }

    Ok((palette_json, applied_desc))
}

/// Detect sequencing palettes after a position change, optionally applying a pre-selected response.
fn mcp_check_sequencing(
    store: &mut sd_core::Store,
    tension_id: &str,
    palette_response: Option<&str>,
) -> Result<(Vec<serde_json::Value>, Option<String>), McpError> {
    let detected = werk_shared::detect_sequencing_palettes(store, tension_id)
        .map_err(werk_err)?;

    let mut palette_json = Vec::new();
    let mut applied_desc = None;

    for (palette, ctx) in detected {
        let palette_val = serde_json::to_value(&palette).unwrap_or(serde_json::Value::Null);
        palette_json.push(palette_val);

        if let Some(response_action) = palette_response {
            if let Some(idx) = palette.options.iter().position(|o| o.action == response_action) {
                let choice = if response_action == "dismiss" {
                    werk_shared::PaletteChoice::Dismissed
                } else {
                    werk_shared::PaletteChoice::Selected(idx)
                };
                if let Ok(Some(desc)) = werk_shared::apply_choice(store, &ctx, &choice) {
                    applied_desc = Some(desc);
                }
            }
        }
    }

    Ok((palette_json, applied_desc))
}

fn parse_actual_at(value: &str) -> Result<DateTime<Utc>, McpError> {
    let v = value.trim().to_lowercase();
    let now = Utc::now();
    if v == "yesterday" {
        return Ok(now - chrono::Duration::days(1));
    }
    if let Some(rest) = v.strip_suffix(" days ago") {
        let n: i64 = rest
            .trim()
            .parse()
            .map_err(|_| err(format!("invalid number in '{}': expected 'N days ago'", value)))?;
        return Ok(now - chrono::Duration::days(n));
    }
    if let Ok(date) = NaiveDate::parse_from_str(&v, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(12, 0, 0).unwrap().and_utc()); // ubs:ignore 12:00:00 is always valid
    }
    Err(err(format!(
        "cannot parse '{}' as a date. Try: 'yesterday', '3 days ago', or '2026-03-20'",
        value
    )))
}

fn parse_since(value: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, McpError> {
    let v = value.trim().to_lowercase();

    if v == "today" {
        return Ok(start_of_day(now));
    }
    if v == "yesterday" || v == "1 day ago" {
        return Ok(start_of_day(now - chrono::Duration::days(1)));
    }
    if let Some(rest) = v.strip_suffix(" days ago") {
        let n: i64 = rest.trim().parse().map_err(|_| err(format!("invalid number in '{}'", value)))?;
        return Ok(start_of_day(now - chrono::Duration::days(n)));
    }
    // Weekday names
    let weekdays = [
        ("monday", 0), ("mon", 0), ("tuesday", 1), ("tue", 1),
        ("wednesday", 2), ("wed", 2), ("thursday", 3), ("thu", 3),
        ("friday", 4), ("fri", 4), ("saturday", 5), ("sat", 5),
        ("sunday", 6), ("sun", 6),
    ];
    for (name, target) in &weekdays {
        if v == *name {
            let from = now.weekday().num_days_from_monday();
            let days_back = if from >= *target { from - target } else { 7 - (target - from) };
            return Ok(start_of_day(now - chrono::Duration::days(days_back as i64)));
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(&v, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc()); // ubs:ignore 00:00:00 is always valid
    }
    Err(err(format!("unrecognized date: '{}'. Try 'today', 'yesterday', '3 days ago', or 'YYYY-MM-DD'.", value)))
}

fn start_of_day(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive().and_hms_opt(0, 0, 0).map(|n| n.and_utc()).unwrap_or(dt)
}

fn validate_batch_mutation(engine: &Engine, mutation: &werk_shared::BatchMutation) -> Result<(), McpError> {
    use werk_shared::BatchMutation;
    let tensions = engine.store().list_tensions().map_err(|e| err(e.to_string()))?;
    let exists = |id: &str| tensions.iter().any(|t| t.id == id);

    match mutation {
        BatchMutation::UpdateActual { tension_id, .. }
        | BatchMutation::AddNote { tension_id, .. }
        | BatchMutation::UpdateStatus { tension_id, .. }
        | BatchMutation::UpdateDesired { tension_id, .. } => {
            if !exists(tension_id) { return Err(err(format!("tension '{}' not found", tension_id))); }
        }
        BatchMutation::CreateChild { parent_id, .. } => {
            if !exists(parent_id) { return Err(err(format!("parent '{}' not found", parent_id))); }
        }
        BatchMutation::SetHorizon { tension_id, .. }
        | BatchMutation::MoveTension { tension_id, .. } => {
            if !exists(tension_id) { return Err(err(format!("tension '{}' not found", tension_id))); }
        }
        BatchMutation::CreateParent { child_id, .. } => {
            if !exists(child_id) { return Err(err(format!("child '{}' not found", child_id))); }
        }
    }

    if let BatchMutation::UpdateStatus { new_status, .. } = mutation {
        match new_status.to_lowercase().as_str() {
            "resolved" | "released" | "active" => {}
            other => return Err(err(format!("unknown status: '{}'", other))),
        }
    }
    Ok(())
}

fn apply_batch_mutation(engine: &mut Engine, mutation: &werk_shared::BatchMutation) -> Result<(), McpError> {
    use werk_shared::BatchMutation;
    match mutation {
        BatchMutation::UpdateActual { tension_id, new_value, .. } => {
            engine.store().update_actual(tension_id, new_value).map_err(|e| err(e.to_string()))?;
        }
        BatchMutation::CreateChild { parent_id, desired, actual, .. } => {
            engine.store().create_tension_with_parent(desired, actual, Some(parent_id.clone()))
                .map_err(|e| err(e.to_string()))?;
        }
        BatchMutation::AddNote { tension_id, text, .. } => {
            engine.store().record_mutation(&Mutation::new(
                tension_id.clone(), Utc::now(), "note".to_owned(), None, text.clone(),
            )).map_err(|e| err(e.to_string()))?;
        }
        BatchMutation::UpdateStatus { tension_id, new_status, .. } => {
            let status = match new_status.to_lowercase().as_str() {
                "resolved" => TensionStatus::Resolved,
                "released" => TensionStatus::Released,
                "active" => TensionStatus::Active,
                other => return Err(err(format!("unknown status: '{}'", other))),
            };
            engine.store().update_status(tension_id, status).map_err(|e| err(e.to_string()))?;
        }
        BatchMutation::UpdateDesired { tension_id, new_value, .. } => {
            engine.store().update_desired(tension_id, new_value).map_err(|e| err(e.to_string()))?;
        }
        BatchMutation::SetHorizon { tension_id, horizon, .. } => {
            if let Ok(h) = Horizon::parse(horizon) {
                engine.update_horizon(tension_id, Some(h)).map_err(|e| err(e.to_string()))?;
            }
        }
        BatchMutation::MoveTension { tension_id, new_parent_id, .. } => {
            engine.update_parent(tension_id, new_parent_id.as_deref()).map_err(|e| err(e.to_string()))?;
        }
        BatchMutation::CreateParent { child_id, desired, actual, .. } => {
            let current_parent = engine.store().get_tension(child_id)
                .ok().flatten().and_then(|t| t.parent_id.clone());
            let parent = engine.create_tension_with_parent(desired, actual, current_parent)
                .map_err(|e| err(e.to_string()))?;
            engine.update_parent(child_id, Some(&parent.id)).map_err(|e| err(e.to_string()))?;
        }
    }
    Ok(())
}

// ── Tool implementations ────────────────────────────────────────────

#[tool_router]
impl WerkServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    // ── Read tools ──────────────────────────────────────────────

    #[tool(description = "Show tension details including desired/actual state, status, frontier, temporal signals, children, and recent mutations. Pass full=true to include ancestors, siblings, and engagement metrics (replaces context tool).")]
    async fn show(
        &self,
        Parameters(p): Parameters<ShowParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;

        let mutations = store
            .get_mutations(&tension.id)
            .map_err(|e| err(e.to_string()))?;
        let forest = Forest::from_tensions(tensions.clone()).map_err(|e| err(e.to_string()))?;
        let now = Utc::now();

        let raw_children = forest.children(&tension.id).unwrap_or_default();
        let child_mutations: Vec<(String, Vec<Mutation>)> = raw_children
            .iter()
            .filter_map(|c| {
                let muts = store.get_mutations(&c.id()).ok()?;
                Some((c.id().to_string(), muts))
            })
            .collect();

        let epochs = store
            .get_epochs(&tension.id)
            .map_err(|e| err(e.to_string()))?;
        let frontier = compute_frontier(&forest, &tension.id, now, &epochs, &child_mutations);
        let urgency = compute_urgency(&tension, now);
        let overdue = tension.status == TensionStatus::Active
            && tension
                .horizon
                .as_ref()
                .map(|h| h.is_past(now))
                .unwrap_or(false);
        let temporal = compute_temporal_signals(&forest, &tension.id, now);
        let field_structural = compute_structural_signals(&forest);
        let structural = field_structural.signals.get(&tension.id).cloned().unwrap_or_default();

        let mutation_infos: Vec<serde_json::Value> = mutations
            .iter()
            .rev()
            .take(10)
            .rev()
            .map(|m| {
                serde_json::json!({
                    "timestamp": m.timestamp().to_rfc3339(),
                    "field": m.field(),
                    "old_value": m.old_value(),
                    "new_value": m.new_value(),
                })
            })
            .collect();

        let children_info: Vec<serde_json::Value> = raw_children
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id(),
                    "short_code": c.tension.short_code,
                    "desired": c.tension.desired,
                    "status": c.tension.status.to_string(),
                    "position": c.tension.position,
                })
            })
            .collect();

        let epoch_infos: Vec<serde_json::Value> = epochs
            .iter()
            .enumerate()
            .map(|(i, e)| {
                serde_json::json!({
                    "number": i + 1,
                    "timestamp": e.timestamp.to_rfc3339(),
                    "desire_snapshot": e.desire_snapshot,
                    "reality_snapshot": e.reality_snapshot,
                    "trigger_gesture_id": e.trigger_gesture_id,
                })
            })
            .collect();

        let mut result = serde_json::json!({
            "id": tension.id,
            "short_code": tension.short_code,
            "desired": tension.desired,
            "actual": tension.actual,
            "status": tension.status.to_string(),
            "parent_id": tension.parent_id,
            "created_at": tension.created_at.to_rfc3339(),
            "horizon": tension.horizon.as_ref().map(|h| h.to_string()),
            "urgency": urgency.as_ref().map(|u| u.value),
            "overdue": overdue,
            "frontier": frontier,
            "temporal": temporal,
            "structural": structural,
            "mutations": mutation_infos,
            "children": children_info,
            "epochs": epoch_infos,
        });

        // Include context data when full=true (absorbs context tool)
        if p.full.unwrap_or(false) {
            let thresholds = ProjectionThresholds::default();

            let ancestors: Vec<serde_json::Value> = forest
                .ancestors(&tension.id)
                .unwrap_or_default()
                .into_iter()
                .map(|n| serde_json::json!({
                    "id": n.id(), "short_code": n.tension.short_code,
                    "desired": n.tension.desired, "status": n.tension.status.to_string(),
                }))
                .collect();

            let siblings: Vec<serde_json::Value> = forest
                .siblings(&tension.id)
                .unwrap_or_default()
                .into_iter()
                .map(|n| serde_json::json!({
                    "id": n.id(), "short_code": n.tension.short_code,
                    "desired": n.tension.desired, "status": n.tension.status.to_string(),
                }))
                .collect();

            let pattern = extract_mutation_pattern(&tension, &mutations, thresholds.pattern_window_seconds, now);
            let engagement = serde_json::json!({
                "current_gap": gap_magnitude(&tension.desired, &tension.actual),
                "mutation_count": pattern.mutation_count,
                "frequency_per_day": pattern.frequency_per_day,
                "frequency_trend": pattern.frequency_trend,
                "gap_trend": pattern.gap_trend,
                "gap_samples": pattern.gap_samples,
                "mean_interval_seconds": pattern.mean_interval_seconds,
            });

            if let Some(obj) = result.as_object_mut() {
                obj.insert("ancestors".to_string(), serde_json::json!(ancestors));
                obj.insert("siblings".to_string(), serde_json::json!(siblings));
                obj.insert("engagement".to_string(), engagement);
            }
        }

        json_result(&result)
    }

    #[tool(description = "Display the tension forest as a tree. Shows hierarchy, closure progress, and temporal signals. Pass an ID to show a subtree.")]
    async fn tree(
        &self,
        Parameters(p): Parameters<TreeParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let forest = Forest::from_tensions(tensions.clone()).map_err(|e| err(e.to_string()))?;
        let now = Utc::now();

        // If an ID is provided, resolve and show subtree
        let (forest, tensions) = if let Some(ref id_str) = p.id {
            let root = resolve_id(&tensions, id_str)?;
            let sub = forest
                .subtree(&root.id)
                .ok_or_else(|| err(format!("no subtree found for {}", root.id)))?;
            let sub_tensions: Vec<_> = tensions
                .into_iter()
                .filter(|t| sub.find(&t.id).is_some())
                .collect();
            (sub, sub_tensions)
        } else {
            (forest, tensions)
        };

        let filter_status = match p.filter.as_str() {
            "all" => None,
            "resolved" => Some(TensionStatus::Resolved),
            "released" => Some(TensionStatus::Released),
            _ => Some(TensionStatus::Active),
        };

        let filtered: Vec<serde_json::Value> = tensions
            .iter()
            .filter(|t| match filter_status {
                Some(ref s) => t.status == *s,
                None => true,
            })
            .map(|t| {
                let children = forest.children(&t.id).unwrap_or_default();
                let resolved = children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Resolved)
                    .count();
                let overdue = t.status == TensionStatus::Active
                    && t.horizon
                        .as_ref()
                        .map(|h| h.is_past(now))
                        .unwrap_or(false);
                serde_json::json!({
                    "id": t.id,
                    "short_code": t.short_code,
                    "desired": t.desired,
                    "actual": t.actual,
                    "status": t.status.to_string(),
                    "parent_id": t.parent_id,
                    "horizon": t.horizon.as_ref().map(|h| h.to_string()),
                    "overdue": overdue,
                    "closure_resolved": resolved,
                    "closure_total": children.len(),
                })
            })
            .collect();

        let summary = serde_json::json!({
            "total": tensions.len(),
            "active": tensions.iter().filter(|t| t.status == TensionStatus::Active).count(),
            "resolved": tensions.iter().filter(|t| t.status == TensionStatus::Resolved).count(),
            "released": tensions.iter().filter(|t| t.status == TensionStatus::Released).count(),
        });

        json_result(&serde_json::json!({
            "tensions": filtered,
            "summary": summary,
        }))
    }

    #[tool(description = "List tensions with rich filtering and sorting. Filter by status (all/urgent/neglected/stagnant), overdue, approaching deadline, stale, held, positioned, root, parent, has_deadline, or changed_since. Sort by urgency (default), name, or deadline.")]
    async fn list(
        &self,
        Parameters(p): Parameters<ListParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let now = Utc::now();

        let mut items: Vec<serde_json::Value> = tensions
            .iter()
            .filter(|t| match p.filter.as_deref() {
                Some("all") => true,
                Some("urgent") => {
                    t.status == TensionStatus::Active
                        && compute_urgency(t, now)
                            .map(|u| u.value >= 0.7)
                            .unwrap_or(false)
                }
                Some("neglected") => {
                    t.status == TensionStatus::Active
                        && store
                            .get_mutations(&t.id)
                            .ok()
                            .and_then(|m| m.last().map(|l| {
                                (now - l.timestamp()).num_days() > 7
                            }))
                            .unwrap_or(false)
                }
                Some("stagnant") => {
                    t.status == TensionStatus::Active
                        && t.horizon
                            .as_ref()
                            .map(|h| h.is_past(now))
                            .unwrap_or(false)
                        && store
                            .get_mutations(&t.id)
                            .ok()
                            .and_then(|m| m.last().map(|l| {
                                (now - l.timestamp()).num_days() > 3
                            }))
                            .unwrap_or(false)
                }
                _ => t.status == TensionStatus::Active,
            })
            .map(|t| {
                let urgency = compute_urgency(t, now);
                let overdue = t.status == TensionStatus::Active
                    && t.horizon
                        .as_ref()
                        .map(|h| h.is_past(now))
                        .unwrap_or(false);
                serde_json::json!({
                    "id": t.id,
                    "short_code": t.short_code,
                    "desired": t.desired,
                    "actual": t.actual,
                    "status": t.status.to_string(),
                    "urgency": urgency.as_ref().map(|u| u.value),
                    "horizon": t.horizon.as_ref().map(|h| h.to_string()),
                    "overdue": overdue,
                })
            })
            .collect();

        // Sort
        match p.sort.as_str() {
            "name" => items.sort_by(|a, b| {
                a["desired"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["desired"].as_str().unwrap_or(""))
            }),
            "deadline" => items.sort_by(|a, b| {
                a["horizon"]
                    .as_str()
                    .unwrap_or("zzzz")
                    .cmp(b["horizon"].as_str().unwrap_or("zzzz"))
            }),
            _ => items.sort_by(|a, b| {
                let ua = a["urgency"].as_f64().unwrap_or(-1.0);
                let ub = b["urgency"].as_f64().unwrap_or(-1.0);
                ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
            }),
        }

        json_result(&serde_json::json!({ "tensions": items }))
    }

    #[tool(description = "Search tensions by content using FrankenSearch hybrid retrieval. Returns results ranked by relevance — finds tensions by meaning, not just exact keywords. Use for natural language queries like 'tensions about revenue' or 'anything related to temporal signals'.")]
    async fn search(
        &self,
        Parameters(p): Parameters<SearchParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let limit = p.limit.unwrap_or(20);

        let index = sd_core::SearchIndex::build(&store)
            .ok_or_else(|| err("failed to build search index (empty store or no workspace path)"))?;

        let hits = index.search(&p.query, limit);
        let now = Utc::now();

        let results: Vec<serde_json::Value> = hits.iter().filter_map(|hit| {
            let t = store.get_tension(&hit.doc_id).ok()??;
            let urgency = compute_urgency(&t, now).map(|u| u.value);
            let display_id = t.short_code
                .map(|c| format!("#{c}"))
                .unwrap_or_else(|| t.id[..8].to_string());
            Some(serde_json::json!({
                "id": t.id,
                "display_id": display_id,
                "short_code": t.short_code,
                "desired": t.desired,
                "actual": t.actual,
                "status": format!("{:?}", t.status),
                "relevance_score": hit.score,
                "urgency": urgency,
                "horizon": t.horizon.as_ref().map(|h| h.to_string()),
                "parent_id": t.parent_id,
            }))
        }).collect();

        json_result(&serde_json::json!({
            "query": p.query,
            "results": results,
            "count": results.len(),
        }))
    }

    #[tool(description = "Show system health summary — structural statistics, temporal alerts, and field-wide signals.")]
    async fn health(&self) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let now = Utc::now();

        let active: Vec<_> = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .collect();
        let total = active.len();

        let mut with_children = 0usize;
        let mut leaf_count = 0usize;
        let mut total_children = 0usize;
        let mut resolved_children = 0usize;

        for t in &active {
            let children: Vec<_> = tensions
                .iter()
                .filter(|c| c.parent_id.as_deref() == Some(&t.id))
                .collect();
            if children.is_empty() {
                leaf_count += 1;
            } else {
                with_children += 1;
                total_children += children.len();
                resolved_children += children
                    .iter()
                    .filter(|c| c.status == TensionStatus::Resolved)
                    .count();
            }
        }

        let mut urgent = 0usize;
        let mut overdue = 0usize;
        for t in &active {
            if let Some(u) = compute_urgency(t, now) {
                if u.value > 1.0 {
                    overdue += 1;
                } else if u.value > 0.75 {
                    urgent += 1;
                }
            }
        }

        json_result(&serde_json::json!({
            "active_count": total,
            "with_children": with_children,
            "leaf_count": leaf_count,
            "closure": {
                "total_children": total_children,
                "resolved_children": resolved_children,
            },
            "alerts": { "urgent": urgent, "overdue": overdue },
        }))
    }

    #[tool(description = "The Napoleonic field survey — all tensions organized by temporal urgency across the entire field.")]
    async fn survey(
        &self,
        Parameters(p): Parameters<SurveyParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let now = Utc::now();
        let frame_end = now + chrono::Duration::days(p.days);

        let parent_lookup: std::collections::HashMap<String, (Option<i32>, String)> = tensions
            .iter()
            .map(|t| (t.id.clone(), (t.short_code, t.desired.clone())))
            .collect();

        let mut overdue_items = Vec::new();
        let mut due_soon = Vec::new();
        let mut active_items = Vec::new();
        let mut held_items = Vec::new();
        let mut recently_resolved = Vec::new();

        for t in &tensions {
            let urgency_val = compute_urgency(t, now).map(|u| u.value);
            let (parent_sc, parent_desired) = t
                .parent_id
                .as_ref()
                .and_then(|pid| parent_lookup.get(pid))
                .map(|(sc, d)| (*sc, Some(d.clone())))
                .unwrap_or((None, None));

            let item = serde_json::json!({
                "id": t.id,
                "short_code": t.short_code,
                "desired": t.desired,
                "parent_id": t.parent_id,
                "parent_short_code": parent_sc,
                "parent_desired": parent_desired,
                "deadline": t.horizon.as_ref().map(|h| h.to_string()),
                "urgency": urgency_val,
                "position": t.position,
            });

            match t.status {
                TensionStatus::Resolved | TensionStatus::Released => {
                    let mutations = store
                        .get_mutations(&t.id)
                        .map_err(|e| err(e.to_string()))?;
                    let resolved_recently = mutations.iter().any(|m| {
                        m.field() == "status"
                            && m.new_value().contains("Resolved")
                            && (now - m.timestamp()).num_days() <= p.days
                    });
                    if resolved_recently {
                        recently_resolved.push(item);
                    }
                }
                TensionStatus::Active => {
                    let is_overdue = t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false);
                    let is_due_soon = !is_overdue
                        && t.horizon
                            .as_ref()
                            .map(|h| h.range_end() <= frame_end)
                            .unwrap_or(false);
                    let is_held = t.position.is_none();

                    if is_overdue {
                        overdue_items.push(item);
                    } else if is_due_soon {
                        due_soon.push(item);
                    } else if is_held {
                        held_items.push(item);
                    } else {
                        active_items.push(item);
                    }
                }
            }
        }

        // Sort overdue/due_soon by urgency descending
        let sort_by_urgency = |a: &serde_json::Value, b: &serde_json::Value| {
            let ua = a["urgency"].as_f64().unwrap_or(-1.0);
            let ub = b["urgency"].as_f64().unwrap_or(-1.0);
            ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
        };
        overdue_items.sort_by(sort_by_urgency);
        due_soon.sort_by(sort_by_urgency);

        json_result(&serde_json::json!({
            "overdue": overdue_items,
            "due_soon": due_soon,
            "active": active_items,
            "held": held_items,
            "recently_resolved": recently_resolved,
        }))
    }

    #[tool(description = "Ground mode — debrief and study surface. Field statistics, epoch history, recent gestures.")]
    async fn ground(
        &self,
        Parameters(p): Parameters<GroundParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let now = Utc::now();
        let cutoff = now - chrono::Duration::days(p.days);

        let active = tensions.iter().filter(|t| t.status == TensionStatus::Active).count();
        let resolved = tensions.iter().filter(|t| t.status == TensionStatus::Resolved).count();
        let released = tensions.iter().filter(|t| t.status == TensionStatus::Released).count();
        let with_deadlines = tensions.iter().filter(|t| t.horizon.is_some()).count();
        let overdue_count = tensions.iter().filter(|t| {
            t.status == TensionStatus::Active
                && t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false)
        }).count();
        let held_count = tensions.iter().filter(|t| t.status == TensionStatus::Active && t.position.is_none()).count();
        let positioned_count = tensions.iter().filter(|t| t.status == TensionStatus::Active && t.position.is_some()).count();

        let mut total_mutations = 0usize;
        let mut recent_mutations = 0usize;
        let mut recent_gestures: Vec<serde_json::Value> = Vec::new();
        let mut epochs: Vec<serde_json::Value> = Vec::new();

        for t in &tensions {
            let mutations = store.get_mutations(&t.id).map_err(|e| err(e.to_string()))?;
            total_mutations += mutations.len();

            for m in &mutations {
                if m.timestamp() >= cutoff {
                    recent_mutations += 1;
                    let field = m.field();
                    if field == "actual" || field == "desired" || field == "status" || field == "note" {
                        let diff = now - m.timestamp();
                        let minutes = diff.num_minutes();
                        let age = if minutes < 60 {
                            format!("{} min ago", minutes)
                        } else if minutes < 1440 {
                            format!("{} hr ago", minutes / 60)
                        } else {
                            format!("{} days ago", minutes / 1440)
                        };
                        recent_gestures.push(serde_json::json!({
                            "tension_id": t.id,
                            "tension_short_code": t.short_code,
                            "field": field,
                            "timestamp": m.timestamp().to_rfc3339(),
                            "age": age,
                        }));
                    }
                }
            }

            let epoch_list = store.get_epochs(&t.id).map_err(|e| err(e.to_string()))?;
            if !epoch_list.is_empty() {
                epochs.push(serde_json::json!({
                    "tension_id": t.id,
                    "tension_short_code": t.short_code,
                    "tension_desired": t.desired,
                    "epoch_count": epoch_list.len(),
                }));
            }
        }

        recent_gestures.sort_by(|a, b| {
            b["timestamp"].as_str().unwrap_or("").cmp(a["timestamp"].as_str().unwrap_or(""))
        });
        recent_gestures.truncate(15);
        epochs.sort_by(|a, b| {
            b["epoch_count"].as_u64().unwrap_or(0).cmp(&a["epoch_count"].as_u64().unwrap_or(0))
        });

        json_result(&serde_json::json!({
            "stats": {
                "total_tensions": tensions.len(),
                "active": active,
                "resolved": resolved,
                "released": released,
                "with_deadlines": with_deadlines,
                "overdue": overdue_count,
                "held": held_count,
                "positioned": positioned_count,
                "total_mutations": total_mutations,
                "recent_mutations": recent_mutations,
            },
            "epochs": epochs,
            "recent_gestures": recent_gestures,
        }))
    }

    #[tool(description = "Show what changed in a time window. Accepts 'today', 'yesterday', 'N days ago', or 'YYYY-MM-DD'.")]
    async fn diff(
        &self,
        Parameters(p): Parameters<DiffParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let now = Utc::now();
        let since_dt = parse_since(&p.since, now)?;

        let mutations = store
            .mutations_between(since_dt, now)
            .map_err(|e| err(e.to_string()))?;

        let all_tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension_map: std::collections::HashMap<String, &sd_core::Tension> = all_tensions
            .iter()
            .map(|t| (t.id.clone(), t))
            .collect();

        let mut grouped: std::collections::BTreeMap<String, Vec<&sd_core::Mutation>> =
            std::collections::BTreeMap::new();
        for m in &mutations {
            grouped.entry(m.tension_id().to_owned()).or_default().push(m);
        }

        let mut changes: Vec<serde_json::Value> = Vec::new();
        let mut created_count = 0usize;
        let mut resolved_count = 0usize;
        let mut updated_count = 0usize;

        for (tid, muts) in &grouped {
            let desired = tension_map
                .get(tid)
                .map(|t| t.desired.clone())
                .unwrap_or_else(|| "(deleted)".to_string());

            let mut is_created = false;
            let mut is_resolved = false;

            let mutation_infos: Vec<serde_json::Value> = muts
                .iter()
                .map(|m| {
                    if m.field() == "created" { is_created = true; }
                    if m.field() == "status" && m.new_value() == "Resolved" { is_resolved = true; }
                    serde_json::json!({
                        "timestamp": m.timestamp().to_rfc3339(),
                        "field": m.field(),
                        "old_value": m.old_value(),
                        "new_value": m.new_value(),
                    })
                })
                .collect();

            if is_created { created_count += 1; }
            else if is_resolved { resolved_count += 1; }
            else { updated_count += 1; }

            changes.push(serde_json::json!({
                "tension_id": tid,
                "tension_desired": desired,
                "mutations": mutation_infos,
            }));
        }

        json_result(&serde_json::json!({
            "since": since_dt.to_rfc3339(),
            "changes": changes,
            "summary": {
                "updated": updated_count,
                "created": created_count,
                "resolved": resolved_count,
            },
        }))
    }

    #[tool(description = "Output structural context (JSON) for a tension, all active tensions, or urgent tensions. Rich context for agent consumption.")]
    async fn context(
        &self,
        Parameters(p): Parameters<ContextParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let forest = Forest::from_tensions(tensions.clone()).map_err(|e| err(e.to_string()))?;
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();

        let build_context = |t: &sd_core::Tension| -> Result<serde_json::Value, McpError> {
            let mutations = store.get_mutations(&t.id).map_err(|e| err(e.to_string()))?;
            let urgency = compute_urgency(t, now);

            let ancestors: Vec<serde_json::Value> = forest
                .ancestors(&t.id)
                .unwrap_or_default()
                .into_iter()
                .map(|n| serde_json::json!({
                    "id": n.id(), "short_code": n.tension.short_code,
                    "desired": n.tension.desired, "status": n.tension.status.to_string(),
                }))
                .collect();

            let siblings: Vec<serde_json::Value> = forest
                .siblings(&t.id)
                .unwrap_or_default()
                .into_iter()
                .map(|n| serde_json::json!({
                    "id": n.id(), "short_code": n.tension.short_code,
                    "desired": n.tension.desired, "status": n.tension.status.to_string(),
                }))
                .collect();

            let children: Vec<serde_json::Value> = forest
                .children(&t.id)
                .unwrap_or_default()
                .into_iter()
                .map(|n| serde_json::json!({
                    "id": n.id(), "short_code": n.tension.short_code,
                    "desired": n.tension.desired, "status": n.tension.status.to_string(),
                    "position": n.tension.position,
                }))
                .collect();

            // Engagement metrics: raw facts anchored to user actions.
            // Standard of Measurement: no classification, no instrument-originated thresholds.
            // Classification lives in the trajectory tool (analytical/practice layer).
            let pattern = extract_mutation_pattern(t, &mutations, thresholds.pattern_window_seconds, now);
            let engagement = serde_json::json!({
                "current_gap": gap_magnitude(&t.desired, &t.actual),
                "mutation_count": pattern.mutation_count,
                "frequency_per_day": pattern.frequency_per_day,
                "frequency_trend": pattern.frequency_trend,
                "gap_trend": pattern.gap_trend,
                "gap_samples": pattern.gap_samples,
                "mean_interval_seconds": pattern.mean_interval_seconds,
            });

            let mutation_infos: Vec<serde_json::Value> = mutations.iter().map(|m| {
                serde_json::json!({
                    "timestamp": m.timestamp().to_rfc3339(),
                    "field": m.field(),
                    "old_value": m.old_value(),
                    "new_value": m.new_value(),
                })
            }).collect();

            Ok(serde_json::json!({
                "tension": {
                    "id": t.id, "short_code": t.short_code,
                    "desired": t.desired, "actual": t.actual,
                    "status": t.status.to_string(), "parent_id": t.parent_id,
                    "horizon": t.horizon.as_ref().map(|h| h.to_string()),
                    "urgency": urgency.as_ref().map(|u| u.value),
                },
                "ancestors": ancestors,
                "siblings": siblings,
                "children": children,
                "mutations": mutation_infos,
                "engagement": engagement,
            }))
        };

        match (p.id, p.mode.as_deref()) {
            (Some(id), _) => {
                let t = resolve_id(&tensions, &id)?;
                let result = build_context(&t)?;
                json_result(&result)
            }
            (None, Some("urgent")) => {
                let results: Vec<serde_json::Value> = tensions
                    .iter()
                    .filter(|t| {
                        t.status == TensionStatus::Active
                            && compute_urgency(t, now).map(|u| u.value > 0.75).unwrap_or(false)
                    })
                    .map(|t| build_context(t))
                    .collect::<Result<_, _>>()?;
                json_result(&results)
            }
            _ => {
                let results: Vec<serde_json::Value> = tensions
                    .iter()
                    .filter(|t| t.status == TensionStatus::Active)
                    .map(|t| build_context(t))
                    .collect::<Result<_, _>>()?;
                json_result(&results)
            }
        }
    }

    #[tool(description = "Practice-layer analysis: trajectory classification, gap projections, and risk flags. These use instrument-originated thresholds — they are interpretive readings of the engagement metrics, not facts anchored to user-supplied standards. Use for study, debrief, or triage — not as authoritative signals. Field-wide funnel or per-tension. Optionally show urgency collision windows.")]
    async fn trajectory(
        &self,
        Parameters(p): Parameters<TrajectoryParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let now = Utc::now();
        let thresholds = ProjectionThresholds::default();

        if let Some(ref id) = p.id {
            // Per-tension projection
            let tension = resolve_id(&tensions, id)?;
            let mutations = store.get_mutations(&tension.id).map_err(|e| err(e.to_string()))?;
            let projections = project_tension(&tension, &mutations, &thresholds, now);

            let find_gap = |h: ProjectionHorizon| -> f64 {
                projections.iter().find(|p| p.horizon == h).map(|p| p.projected_gap).unwrap_or(0.0)
            };
            let current_gap = projections.first().map(|p| p.current_gap).unwrap_or(0.0);
            let trajectory = projections.first().map(|p| format!("{:?}", p.trajectory)).unwrap_or_else(|| "Stalling".to_string());
            let ttr = projections.first().and_then(|p| p.time_to_resolution);

            let pattern = extract_mutation_pattern(&tension, &mutations, thresholds.pattern_window_seconds, now);
            let engagement = if pattern.frequency_trend > 0.1 { "accelerating" }
                else if pattern.frequency_trend < -0.1 { "declining" }
                else { "steady" };

            let mut risks = Vec::new();
            for proj in &projections {
                if proj.oscillation_risk && !risks.contains(&"oscillation") { risks.push("oscillation"); }
                if proj.neglect_risk && !risks.contains(&"neglect") { risks.push("neglect"); }
            }

            json_result(&serde_json::json!({
                "tension_id": tension.id,
                "desired": tension.desired,
                "trajectory": trajectory,
                "current_gap": current_gap,
                "gap_1w": find_gap(ProjectionHorizon::OneWeek),
                "gap_1m": find_gap(ProjectionHorizon::OneMonth),
                "gap_3m": find_gap(ProjectionHorizon::ThreeMonths),
                "time_to_resolution": ttr,
                "engagement": engagement,
                "risks": risks,
            }))
        } else {
            // Field-wide projection
            let mut all_mutations = Vec::new();
            for t in &tensions {
                let muts = store.get_mutations(&t.id).map_err(|e| err(e.to_string()))?;
                all_mutations.extend(muts);
            }
            let field = project_field(&tensions, &all_mutations, &thresholds, now);

            let get_buckets = |h: ProjectionHorizon| -> serde_json::Value {
                field.funnel.get(&h).map(|b| serde_json::json!({
                    "resolving": b.resolving, "stalling": b.stalling,
                    "drifting": b.drifting, "oscillating": b.oscillating, "total": b.total,
                })).unwrap_or(serde_json::json!({}))
            };

            let collisions: Vec<serde_json::Value> = field.urgency_collisions.iter().map(|c| {
                serde_json::json!({
                    "window_start": c.window_start.to_rfc3339(),
                    "window_end": c.window_end.to_rfc3339(),
                    "tension_ids": c.tension_ids,
                    "peak_combined_urgency": c.peak_combined_urgency,
                })
            }).collect();

            json_result(&serde_json::json!({
                "computed_at": now.to_rfc3339(),
                "funnel": {
                    "week_1": get_buckets(ProjectionHorizon::OneWeek),
                    "month_1": get_buckets(ProjectionHorizon::OneMonth),
                    "month_3": get_buckets(ProjectionHorizon::ThreeMonths),
                },
                "collisions": collisions,
            }))
        }
    }

    #[tool(description = "Show behavioral pattern insights from mutation history.")]
    async fn insights(
        &self,
        Parameters(p): Parameters<InsightsParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let now = Utc::now();
        let since = now - chrono::Duration::days(p.days);

        let all_mutations = store.all_mutations().map_err(|e| err(e.to_string()))?;
        let recent: Vec<_> = all_mutations.iter().filter(|m| m.timestamp() >= since).collect();
        let recent_count = recent.len();

        let mut per_tension: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let mut day_counts = [0usize; 7];

        for m in &recent {
            *per_tension.entry(m.tension_id().to_string()).or_insert(0) += 1;
            let wd = m.timestamp().weekday().num_days_from_monday() as usize;
            day_counts[wd] += 1;
        }

        let tension_map: std::collections::HashMap<&str, &sd_core::Tension> =
            tensions.iter().map(|t| (t.id.as_str(), t)).collect();

        let mut attention: Vec<serde_json::Value> = per_tension
            .iter()
            .map(|(id, &count)| {
                let desired = tension_map
                    .get(id.as_str())
                    .map(|t| t.desired.clone())
                    .unwrap_or_else(|| id.clone());
                serde_json::json!({
                    "tension_id": id,
                    "desired": desired,
                    "mutation_count": count,
                })
            })
            .collect();
        attention.sort_by(|a, b| {
            b["mutation_count"].as_u64().unwrap_or(0).cmp(&a["mutation_count"].as_u64().unwrap_or(0))
        });

        let mut postponed_count = 0usize;
        let mut overdue_count = 0usize;
        for t in &tensions {
            if t.status != TensionStatus::Active { continue; }
            let t_mutations = store.get_mutations(&t.id).map_err(|e| err(e.to_string()))?;
            let drift = detect_horizon_drift(&t.id, &t_mutations);
            match drift.drift_type {
                HorizonDriftType::Postponement | HorizonDriftType::RepeatedPostponement => {
                    postponed_count += 1;
                }
                _ => {}
            }
            if let Some(h) = &t.horizon {
                if h.is_past(now) { overdue_count += 1; }
            }
        }

        let mut activity: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (i, &count) in day_counts.iter().enumerate() {
            activity.insert(day_names[i].to_string(), count);
        }

        json_result(&serde_json::json!({
            "days": p.days,
            "mutation_count": recent_count,
            "attention": attention,
            "postponed_count": postponed_count,
            "overdue_count": overdue_count,
            "activity_by_day": activity,
        }))
    }

    #[tool(description = "Field-level summaries, aggregates, and analysis. Default: vitals only. Pass sections array to include: temporal, attention, changes, trajectory, engagement, drift, health, or 'all'. Replaces ground, health, insights, trajectory for field-wide queries.")]
    async fn stats(
        &self,
        Parameters(p): Parameters<StatsParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let all_mutations = store.all_mutations().map_err(|e| err(e.to_string()))?;
        let now = Utc::now();
        let cutoff = now - chrono::Duration::days(p.days);

        let sections = p.sections.unwrap_or_default();
        let show_all = sections.iter().any(|s| s == "all");
        let has = |name: &str| show_all || sections.iter().any(|s| s == name);

        // Vitals (always)
        let active = tensions.iter().filter(|t| t.status == TensionStatus::Active).count();
        let resolved = tensions.iter().filter(|t| t.status == TensionStatus::Resolved).count();
        let released = tensions.iter().filter(|t| t.status == TensionStatus::Released).count();
        let deadlined = tensions.iter().filter(|t| t.horizon.is_some()).count();
        let overdue_count = tensions.iter().filter(|t| {
            t.status == TensionStatus::Active && t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false)
        }).count();
        let positioned = tensions.iter().filter(|t| t.status == TensionStatus::Active && t.position.is_some()).count();
        let held = tensions.iter().filter(|t| t.status == TensionStatus::Active && t.position.is_none()).count();

        let recent: Vec<&Mutation> = all_mutations.iter().filter(|m| m.timestamp() >= cutoff).collect();
        let mut touched = std::collections::HashSet::new();
        for m in &recent { touched.insert(m.tension_id()); }
        let avg = if p.days > 0 { recent.len() as f64 / p.days as f64 } else { 0.0 };

        let mut result = serde_json::json!({
            "vitals": {
                "active": active, "resolved": resolved, "released": released,
                "deadlined": deadlined, "overdue": overdue_count,
                "positioned": positioned, "held": held,
                "mutations": recent.len(), "tensions_touched": touched.len(),
                "avg_per_day": (avg * 10.0).round() / 10.0, "period_days": p.days,
            }
        });

        if has("temporal") {
            let forest = Forest::from_tensions(tensions.to_vec()).map_err(|e| err(e.to_string()))?;
            let frame_end = now + chrono::Duration::days(14);

            let mut approaching: Vec<serde_json::Value> = Vec::new();
            for t in tensions.iter().filter(|t| t.status == TensionStatus::Active) {
                if let Some(u) = compute_urgency(t, now) {
                    let close = u.value > 0.5 || t.horizon.as_ref().map(|h| h.range_end() <= frame_end).unwrap_or(false);
                    let past = t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false);
                    if close || past {
                        approaching.push(serde_json::json!({
                            "short_code": t.short_code, "desired": t.desired,
                            "deadline": t.horizon.as_ref().map(|h| h.to_string()),
                            "urgency": u.value,
                        }));
                    }
                }
            }
            approaching.sort_by(|a, b| b["urgency"].as_f64().unwrap_or(0.0).partial_cmp(&a["urgency"].as_f64().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
            approaching.truncate(10);

            let root_ids: Vec<String> = tensions.iter().filter(|t| t.parent_id.is_none() && t.status == TensionStatus::Active).map(|t| t.id.clone()).collect();
            let tension_map: std::collections::HashMap<String, &sd_core::Tension> = tensions.iter().map(|t| (t.id.clone(), t)).collect();

            let mut critical_path: Vec<serde_json::Value> = Vec::new();
            for rid in &root_ids {
                for cp in sd_core::detect_critical_path_recursive(&forest, rid, now) {
                    let child = tension_map.get(&cp.tension_id);
                    let parent = tension_map.get(&cp.parent_id);
                    critical_path.push(serde_json::json!({
                        "parent_short_code": parent.and_then(|t| t.short_code),
                        "child_short_code": child.and_then(|t| t.short_code),
                        "child_desired": child.map(|t| t.desired.as_str()).unwrap_or(""),
                        "slack_days": cp.slack_seconds / 86400,
                    }));
                }
            }

            if let Some(obj) = result.as_object_mut() {
                obj.insert("temporal".to_string(), serde_json::json!({
                    "approaching": approaching,
                    "critical_path": critical_path,
                }));
            }
        }

        if has("attention") {
            let roots: Vec<&sd_core::Tension> = tensions.iter().filter(|t| t.parent_id.is_none() && t.status == TensionStatus::Active).collect();
            let mut root_data: Vec<serde_json::Value> = Vec::new();

            for root in &roots {
                let mut total = 0usize;
                let mut desc_touched = 0usize;
                let children: Vec<&sd_core::Tension> = tensions.iter().filter(|t| t.parent_id.as_deref() == Some(&root.id) && t.status == TensionStatus::Active).collect();
                let mut branches: Vec<serde_json::Value> = Vec::new();

                for child in &children {
                    let desc_ids = store.get_descendant_ids(&child.id).map_err(|e| err(e.to_string()))?;
                    let mut bm = 0usize;
                    let mut bt = 0usize;
                    let child_muts = store.get_mutations(&child.id).map_err(|e| err(e.to_string()))?;
                    let cr = child_muts.iter().filter(|m| m.timestamp() >= cutoff).count();
                    if cr > 0 { bm += cr; bt += 1; }
                    for did in &desc_ids {
                        let dm = store.get_mutations(did).map_err(|e| err(e.to_string()))?;
                        let dr = dm.iter().filter(|m| m.timestamp() >= cutoff).count();
                        if dr > 0 { bm += dr; bt += 1; }
                    }
                    total += bm; desc_touched += bt;
                    branches.push(serde_json::json!({
                        "short_code": child.short_code, "desired": child.desired,
                        "mutations": bm, "tensions_touched": bt,
                    }));
                }
                branches.sort_by(|a, b| b["mutations"].as_u64().unwrap_or(0).cmp(&a["mutations"].as_u64().unwrap_or(0)));

                root_data.push(serde_json::json!({
                    "short_code": root.short_code, "desired": root.desired,
                    "total_mutations": total, "descendants_touched": desc_touched,
                    "branches": branches,
                }));
            }
            root_data.sort_by(|a, b| b["total_mutations"].as_u64().unwrap_or(0).cmp(&a["total_mutations"].as_u64().unwrap_or(0)));

            if let Some(obj) = result.as_object_mut() {
                obj.insert("attention".to_string(), serde_json::json!({ "roots": root_data }));
            }
        }

        if has("trajectory") {
            let thresholds = ProjectionThresholds::default();
            let field = project_field(&tensions, &all_mutations, &thresholds, now);

            let buckets = field.funnel.get(&ProjectionHorizon::OneWeek);
            let collisions: Vec<serde_json::Value> = field.urgency_collisions.iter().map(|c| {
                serde_json::json!({
                    "tension_ids": c.tension_ids,
                    "window": format!("{} to {}", c.window_start.format("%Y-%m-%d"), c.window_end.format("%Y-%m-%d")),
                    "peak_urgency": c.peak_combined_urgency,
                })
            }).collect();

            if let Some(obj) = result.as_object_mut() {
                obj.insert("trajectory".to_string(), serde_json::json!({
                    "distribution": buckets.map(|b| serde_json::json!({
                        "resolving": b.resolving, "drifting": b.drifting,
                        "stalling": b.stalling, "oscillating": b.oscillating,
                    })),
                    "collisions": collisions,
                }));
            }
        }

        if has("engagement") {
            let mut per_tension: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for m in &recent { *per_tension.entry(m.tension_id().to_string()).or_default() += 1; }
            let tension_map: std::collections::HashMap<String, &sd_core::Tension> = tensions.iter().map(|t| (t.id.clone(), t)).collect();

            let most = per_tension.iter().max_by_key(|(_, c)| *c);
            let least = tensions.iter()
                .filter(|t| t.status == TensionStatus::Active && t.horizon.is_some())
                .min_by_key(|t| per_tension.get(&t.id).copied().unwrap_or(0));

            if let Some(obj) = result.as_object_mut() {
                obj.insert("engagement".to_string(), serde_json::json!({
                    "field_frequency": (avg * 10.0).round() / 10.0,
                    "most_engaged": most.and_then(|(id, c)| tension_map.get(id).map(|t| serde_json::json!({
                        "short_code": t.short_code, "desired": t.desired,
                        "frequency": *c as f64 / p.days.max(1) as f64,
                    }))),
                    "least_engaged_with_deadline": least.map(|t| serde_json::json!({
                        "short_code": t.short_code, "desired": t.desired,
                        "frequency": per_tension.get(&t.id).copied().unwrap_or(0) as f64 / p.days.max(1) as f64,
                        "deadline": t.horizon.as_ref().map(|h| h.to_string()),
                    })),
                }));
            }
        }

        if has("drift") {
            let mut drifts: Vec<serde_json::Value> = Vec::new();
            for t in tensions.iter().filter(|t| t.status == TensionStatus::Active && t.horizon.is_some()) {
                let muts = store.get_mutations(&t.id).map_err(|e| err(e.to_string()))?;
                let drift = detect_horizon_drift(&t.id, &muts);
                let dtype = format!("{:?}", drift.drift_type);
                if dtype != "Stable" && drift.change_count > 0 {
                    drifts.push(serde_json::json!({
                        "short_code": t.short_code, "desired": t.desired,
                        "drift_type": dtype, "changes": drift.change_count,
                        "net_shift_days": drift.net_shift_seconds / 86400,
                    }));
                }
            }
            if let Some(obj) = result.as_object_mut() {
                obj.insert("drift".to_string(), serde_json::json!(drifts));
            }
        }

        if has("health") {
            let noop = store.count_noop_mutations().map_err(|e| err(e.to_string()))?;
            if let Some(obj) = result.as_object_mut() {
                obj.insert("health".to_string(), serde_json::json!({ "noop_mutations": noop }));
            }
        }

        if has("changes") {
            let recent_mutations = store.mutations_between(cutoff, now).map_err(|e| err(e.to_string()))?;
            let tension_map: std::collections::HashMap<String, &sd_core::Tension> = tensions.iter().map(|t| (t.id.clone(), t)).collect();

            let mut epochs_list: Vec<serde_json::Value> = Vec::new();
            for t in &tensions {
                let eps = store.get_epochs(&t.id).map_err(|e| err(e.to_string()))?;
                for ep in &eps {
                    if ep.timestamp >= cutoff {
                        epochs_list.push(serde_json::json!({"short_code": t.short_code, "desired": t.desired}));
                    }
                }
            }

            let mut resolutions = Vec::new();
            let mut new_tensions = Vec::new();
            let mut seen_r = std::collections::HashSet::new();
            let mut seen_c = std::collections::HashSet::new();

            for m in &recent_mutations {
                let t = tension_map.get(m.tension_id());
                let sc = t.and_then(|t| t.short_code);
                let des = t.map(|t| t.desired.as_str()).unwrap_or("(deleted)");

                if m.field() == "status" && (m.new_value() == "Resolved" || m.new_value() == "Released") && seen_r.insert(m.tension_id().to_string()) {
                    resolutions.push(serde_json::json!({"short_code": sc, "desired": des}));
                }
                if m.field() == "created" && seen_c.insert(m.tension_id().to_string()) {
                    new_tensions.push(serde_json::json!({"short_code": sc, "desired": des}));
                }
            }

            if let Some(obj) = result.as_object_mut() {
                obj.insert("changes".to_string(), serde_json::json!({
                    "epochs": epochs_list, "resolutions": resolutions, "new_tensions": new_tensions,
                }));
            }
        }

        json_result(&result)
    }

    // ── Gesture tools (mutating) ────────────────────────────────

    #[tool(description = "Create a new tension with desired outcome and current reality. Optionally set parent and horizon.")]
    async fn add(
        &self,
        Parameters(p): Parameters<AddParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.desired.is_empty() {
            return Err(err("desired state cannot be empty"));
        }
        if p.actual.is_empty() {
            return Err(err("actual state cannot be empty"));
        }

        let horizon_parsed = p
            .horizon
            .as_ref()
            .map(|h| Horizon::parse(h).map_err(|e| err(format!("invalid horizon '{}': {}", h, e))))
            .transpose()?;

        let (workspace, mut store) = open_store()?;

        let parent_id = if let Some(ref parent_prefix) = p.parent {
            let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
            let parent = resolve_id(&tensions, parent_prefix)?;
            Some(parent.id.clone())
        } else {
            None
        };

        let _ = store.begin_gesture(Some("create tension"));
        let tension = store
            .create_tension_full(&p.desired, &p.actual, parent_id.clone(), horizon_parsed.clone())
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        // Fire post_create hook
        let hooks = load_hooks(&workspace);
        let create_event = HookEvent::create(&tension.id, &tension.desired, Some(&tension.actual), tension.parent_id.as_deref());
        hooks.post_create(&create_event);

        // Detect containment palettes if child created with horizon under a parent
        let (signals, applied) = if horizon_parsed.is_some() && tension.parent_id.is_some() {
            mcp_check_containment(&mut store, &tension.id, p.palette_response.as_deref())?
        } else {
            (vec![], None)
        };

        autoflush(&workspace);

        let mut result = serde_json::json!({
            "id": tension.id,
            "short_code": tension.short_code,
            "desired": tension.desired,
            "actual": tension.actual,
            "status": tension.status.to_string(),
            "parent_id": parent_id,
            "horizon": tension.horizon.as_ref().map(|h| h.to_string()),
        });
        if !signals.is_empty() {
            result["signals"] = serde_json::Value::Array(signals);
        }
        if let Some(desc) = applied {
            result["palette_applied"] = serde_json::Value::String(desc);
        }
        json_result(&result)
    }

    #[tool(description = "Compose up: create a parent for existing tensions. Reveals implicit coherence by composing structure upward.")]
    async fn compose(
        &self,
        Parameters(p): Parameters<ComposeParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.children.is_empty() {
            return Err(err("at least one child ID is required"));
        }

        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;

        // Resolve all child IDs
        let mut child_ids = Vec::new();
        for child_ref in &p.children {
            let child = resolve_id(&tensions, child_ref)?;
            child_ids.push(child.id.clone());
        }

        // Verify all children share the same parent
        let first_parent = tensions
            .iter()
            .find(|t| t.id == child_ids[0])
            .and_then(|t| t.parent_id.clone());
        for cid in &child_ids[1..] {
            let parent = tensions
                .iter()
                .find(|t| &t.id == cid)
                .and_then(|t| t.parent_id.clone());
            if parent != first_parent {
                return Err(err(
                    "all children must share the same current parent (or all be roots)",
                ));
            }
        }

        let _ = store.begin_gesture(Some("compose up"));
        let parent = store
            .create_tension_full(&p.desired, &p.actual, first_parent, None)
            .map_err(|e| err(e.to_string()))?;

        // Reparent children
        let mut engine = Engine::with_store(store);
        for cid in &child_ids {
            engine
                .update_parent(cid, Some(&parent.id))
                .map_err(|e| err(e.to_string()))?;
        }
        engine.store_mut().end_gesture();

        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": parent.id,
            "short_code": parent.short_code,
            "desired": parent.desired,
            "actual": parent.actual,
            "children": child_ids,
        }))
    }

    #[tool(description = "Update the current reality of a tension. Reality updates are epoch boundaries by default.")]
    async fn reality(
        &self,
        Parameters(p): Parameters<RealityParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.value.is_empty() {
            return Err(err("actual state cannot be empty"));
        }

        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let old_actual = tension.actual.clone();
        let tension_id = tension.id.clone();

        let hooks = load_hooks(&workspace);
        let event =
            HookEvent::mutation(&tension_id, &tension.desired, Some(&old_actual), tension.parent_id.as_deref(), "actual", Some(&old_actual), &p.value);
        if !hooks.pre_mutation(&event) {
            return Err(err("blocked by pre_mutation hook"));
        }

        let _ = store.begin_gesture(Some(&format!("update reality {}", &tension_id)));

        let epoch_id = if !p.no_epoch {
            let children = store
                .get_children(&tension_id)
                .map_err(|e| err(e.to_string()))?;
            let children_snapshot: Vec<serde_json::Value> = children
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "desired": c.desired,
                        "actual": c.actual,
                        "status": c.status.to_string(),
                        "position": c.position,
                    })
                })
                .collect();
            let children_json =
                serde_json::to_string(&children_snapshot).map_err(|e| err(e.to_string()))?;
            let eid = store
                .create_epoch(
                    &tension_id,
                    &tension.desired,
                    &old_actual,
                    Some(&children_json),
                    store.active_gesture().as_deref(),
                )
                .map_err(|e| err(e.to_string()))?;
            Some(eid)
        } else {
            None
        };

        store
            .update_actual(&tension_id, &p.value)
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        hooks.post_mutation(&event);
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "actual": p.value,
            "old_actual": old_actual,
            "epoch_id": epoch_id,
        }))
    }

    #[tool(description = "Update the desired state of a tension. Desire updates are epoch boundaries by default.")]
    async fn desire(
        &self,
        Parameters(p): Parameters<DesireParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.value.is_empty() {
            return Err(err("desired state cannot be empty"));
        }

        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let old_desired = tension.desired.clone();
        let tension_id = tension.id.clone();

        let hooks = load_hooks(&workspace);
        let event = HookEvent::mutation(
            &tension_id,
            &old_desired,
            Some(&tension.actual),
            tension.parent_id.as_deref(),
            "desired",
            Some(&old_desired),
            &p.value,
        );
        if !hooks.pre_mutation(&event) {
            return Err(err("blocked by pre_mutation hook"));
        }

        let _ = store.begin_gesture(Some(&format!("update desire {}", &tension_id)));

        let epoch_id = if !p.no_epoch {
            let children = store
                .get_children(&tension_id)
                .map_err(|e| err(e.to_string()))?;
            let children_snapshot: Vec<serde_json::Value> = children
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "desired": c.desired,
                        "actual": c.actual,
                        "status": c.status.to_string(),
                        "position": c.position,
                    })
                })
                .collect();
            let children_json =
                serde_json::to_string(&children_snapshot).map_err(|e| err(e.to_string()))?;
            let eid = store
                .create_epoch(
                    &tension_id,
                    &old_desired,
                    &tension.actual,
                    Some(&children_json),
                    store.active_gesture().as_deref(),
                )
                .map_err(|e| err(e.to_string()))?;
            Some(eid)
        } else {
            None
        };

        store
            .update_desired(&tension_id, &p.value)
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        hooks.post_mutation(&event);
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "desired": p.value,
            "old_desired": old_desired,
            "epoch_id": epoch_id,
        }))
    }

    #[tool(description = "Mark a tension as resolved. Optionally specify when resolution actually happened.")]
    async fn resolve(
        &self,
        Parameters(p): Parameters<ResolveParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        if tension.status != TensionStatus::Active {
            return Err(err(format!(
                "cannot resolve tension with status {} (must be Active)",
                tension.status
            )));
        }

        let hooks = load_hooks(&workspace);
        let event = HookEvent::status_change(&tension_id, &tension.desired, Some(&tension.actual), tension.parent_id.as_deref(), "Resolved");
        if !hooks.pre_mutation(&event) {
            return Err(err("blocked by pre_mutation hook"));
        }

        let _ = store.begin_gesture(Some(&format!("resolve {}", &tension_id)));

        if let Some(ref at) = p.actual_at {
            let dt = parse_actual_at(at)?;
            store.set_actual_at(dt);
        }

        store
            .update_status(&tension_id, TensionStatus::Resolved)
            .map_err(|e| err(e.to_string()))?;

        store.clear_actual_at();
        store.end_gesture();

        hooks.post_mutation(&event);
        hooks.post_resolve(&event);
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "status": "Resolved",
            "actual_at": p.actual_at,
        }))
    }

    #[tool(description = "Release a tension — abandon the desired state. Requires a reason.")]
    async fn release(
        &self,
        Parameters(p): Parameters<ReleaseParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        if tension.status != TensionStatus::Active {
            return Err(err(format!(
                "cannot release tension with status {} (must be Active)",
                tension.status
            )));
        }

        let hooks = load_hooks(&workspace);
        let event = HookEvent::status_change(&tension_id, &tension.desired, Some(&tension.actual), tension.parent_id.as_deref(), "Released");
        if !hooks.pre_mutation(&event) {
            return Err(err("blocked by pre_mutation hook"));
        }

        let _ = store.begin_gesture(Some(&format!("release {}", &tension_id)));

        // Record the reason as a note before changing status
        store
            .record_mutation(&Mutation::new(
                tension_id.clone(),
                Utc::now(),
                "note".to_owned(),
                None,
                format!("Released: {}", &p.reason),
            ))
            .map_err(|e| err(e.to_string()))?;

        store
            .update_status(&tension_id, TensionStatus::Released)
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        hooks.post_mutation(&event);
        hooks.post_release(&event);
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "status": "Released",
            "reason": p.reason,
        }))
    }

    #[tool(description = "Reopen a resolved or released tension (set status back to Active).")]
    async fn reopen(
        &self,
        Parameters(p): Parameters<ReopenParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();
        let old_status = tension.status;

        if old_status == TensionStatus::Active {
            return Err(err("tension is already Active"));
        }

        let hooks = load_hooks(&workspace);
        let event = HookEvent::status_change(&tension_id, &tension.desired, Some(&tension.actual), tension.parent_id.as_deref(), "Active");
        if !hooks.pre_mutation(&event) {
            return Err(err("blocked by pre_mutation hook"));
        }

        let _ = store.begin_gesture(Some(&format!("reopen {}", &tension_id)));
        store
            .update_status(&tension_id, TensionStatus::Active)
            .map_err(|e| err(e.to_string()))?;

        if let Some(ref reason) = p.reason {
            store
                .record_mutation(&Mutation::new(
                    tension_id.clone(),
                    Utc::now(),
                    "reopen_reason".to_owned(),
                    None,
                    reason.clone(),
                ))
                .map_err(|e| err(e.to_string()))?;
        }
        store.end_gesture();

        hooks.post_mutation(&event);
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "status": "Active",
            "old_status": old_status.to_string(),
            "reason": p.reason,
        }))
    }

    #[tool(description = "Reparent a tension. Omit parent to make it a root tension.", name = "move")]
    async fn move_tension(
        &self,
        Parameters(p): Parameters<MoveParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        let new_parent_id = if let Some(ref parent_prefix) = p.parent {
            let parent = resolve_id(&tensions, parent_prefix)?;
            Some(parent.id.clone())
        } else {
            None
        };

        let _ = store.begin_gesture(Some(&format!("move {}", &tension_id)));
        let mut engine = Engine::with_store(store);
        engine
            .update_parent(&tension_id, new_parent_id.as_deref())
            .map_err(|e| err(e.to_string()))?;
        engine.store_mut().end_gesture();

        // Detect containment palettes after reparenting
        let (signals, applied) = if new_parent_id.is_some() {
            mcp_check_containment(engine.store_mut(), &tension_id, p.palette_response.as_deref())?
        } else {
            (vec![], None)
        };

        autoflush(&workspace);

        let mut result = serde_json::json!({
            "id": tension_id,
            "parent_id": new_parent_id,
        });
        if !signals.is_empty() {
            result["signals"] = serde_json::Value::Array(signals);
        }
        if let Some(desc) = applied {
            result["palette_applied"] = serde_json::Value::String(desc);
        }
        json_result(&result)
    }

    #[tool(description = "Remove a tension from the sequence (set to held/unpositioned).")]
    async fn hold(
        &self,
        Parameters(p): Parameters<IdParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        let _ = store.begin_gesture(Some(&format!("hold {}", &tension_id)));
        store
            .update_position(&tension_id, None)
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "position": "held",
        }))
    }

    #[tool(description = "Set position in the order of operations (1-based, higher = earlier in sequence).")]
    async fn position(
        &self,
        Parameters(p): Parameters<PositionParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();
        let old_pos = tension.position;

        let _ = store.begin_gesture(Some(&format!("position {}", &tension_id)));
        store
            .update_position(&tension_id, Some(p.position))
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        // Detect sequencing palettes, optionally apply pre-selected response
        let (signals, applied) = mcp_check_sequencing(
            &mut store,
            &tension_id,
            p.palette_response.as_deref(),
        )?;

        autoflush(&workspace);

        let mut result = serde_json::json!({
            "id": tension_id,
            "position": p.position,
            "old_position": old_pos,
        });
        if !signals.is_empty() {
            result["signals"] = serde_json::Value::Array(signals);
        }
        if let Some(desc) = applied {
            result["palette_applied"] = serde_json::Value::String(desc);
        }
        json_result(&result)
    }

    #[tool(description = "Set or clear the deadline (horizon) of a tension. Omit value to display current. Use 'none' to clear.")]
    async fn horizon(
        &self,
        Parameters(p): Parameters<HorizonParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        match p.value.as_deref() {
            None => {
                // Display current
                let now = Utc::now();
                let urgency = compute_urgency(&tension, now);
                json_result(&serde_json::json!({
                    "id": tension_id,
                    "horizon": tension.horizon.as_ref().map(|h| h.to_string()),
                    "urgency": urgency.as_ref().map(|u| u.value),
                }))
            }
            Some("none") => {
                let _ = store.begin_gesture(Some(&format!("clear horizon {}", &tension_id)));
                let mut engine = Engine::with_store(store);
                engine
                    .update_horizon(&tension_id, None)
                    .map_err(|e| err(e.to_string()))?;
                engine.store_mut().end_gesture();
                autoflush(&workspace);
                json_result(&serde_json::json!({
                    "id": tension_id,
                    "horizon": null,
                }))
            }
            Some(val) => {
                let h = Horizon::parse(val).map_err(|e| err(format!("invalid horizon: {}", e)))?;
                let _ = store.begin_gesture(Some(&format!("set horizon {}", &tension_id)));
                let mut engine = Engine::with_store(store);
                engine
                    .update_horizon(&tension_id, Some(h))
                    .map_err(|e| err(e.to_string()))?;
                engine.store_mut().end_gesture();

                // Detect containment palettes, optionally apply pre-selected response
                let (signals, applied) = mcp_check_containment(
                    engine.store_mut(),
                    &tension_id,
                    p.palette_response.as_deref(),
                )?;

                autoflush(&workspace);
                let mut result = serde_json::json!({
                    "id": tension_id,
                    "horizon": val,
                });
                if !signals.is_empty() {
                    result["signals"] = serde_json::Value::Array(signals);
                }
                if let Some(desc) = applied {
                    result["palette_applied"] = serde_json::Value::String(desc);
                }
                json_result(&result)
            }
        }
    }

    #[tool(description = "Delete a tension (reparents children to grandparent).")]
    async fn rm(
        &self,
        Parameters(p): Parameters<IdParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();
        let desired = tension.desired.clone();
        let grandparent = tension.parent_id.clone();

        // Reparent children to grandparent
        let children = store
            .get_children(&tension_id)
            .map_err(|e| err(e.to_string()))?;

        let _ = store.begin_gesture(Some(&format!("rm {}", &tension_id)));

        let mut engine = Engine::with_store(store);
        for child in &children {
            engine
                .update_parent(&child.id, grandparent.as_deref())
                .map_err(|e| err(e.to_string()))?;
        }

        // Delete the tension
        engine
            .store()
            .delete_tension(&tension_id)
            .map_err(|e| err(e.to_string()))?;
        engine.store_mut().end_gesture();

        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "deleted": true,
            "desired": desired,
            "children_reparented": children.len(),
        }))
    }

    // ── Note tools ──────────────────────────────────────────────

    #[tool(description = "Add a note (observational testimony) to a tension or workspace.")]
    async fn note_add(
        &self,
        Parameters(p): Parameters<NoteAddParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.text.is_empty() {
            return Err(err("note text cannot be empty"));
        }

        let (workspace, mut store) = open_store()?;
        let hooks = load_hooks(&workspace);

        let (tension_id, display, t_actual, t_parent) = if let Some(ref id_prefix) = p.id {
            let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
            let tension = resolve_id(&tensions, id_prefix)?;
            (tension.id.clone(), Some(tension.desired.clone()), Some(tension.actual.clone()), tension.parent_id.clone())
        } else {
            (WORKSPACE_NOTE_TENSION_ID.to_string(), None, None, None)
        };

        let event = HookEvent::mutation(
            &tension_id,
            display.as_deref().unwrap_or("workspace"),
            t_actual.as_deref(),
            t_parent.as_deref(),
            "note",
            None,
            &p.text,
        );
        if !hooks.pre_mutation(&event) {
            return Err(err("blocked by pre_mutation hook"));
        }

        let _ = store.begin_gesture(Some(&format!("note {}", &tension_id)));
        store
            .record_mutation(&Mutation::new(
                tension_id.clone(),
                Utc::now(),
                "note".to_owned(),
                None,
                p.text.clone(),
            ))
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        hooks.post_mutation(&event);
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": if tension_id == WORKSPACE_NOTE_TENSION_ID { None } else { Some(&tension_id) },
            "note": p.text,
        }))
    }

    #[tool(description = "Retract a note by number (1-based index).")]
    async fn note_rm(
        &self,
        Parameters(p): Parameters<NoteRmParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.index == 0 {
            return Err(err("note number must be 1 or greater"));
        }

        let (workspace, mut store) = open_store()?;
        let hooks = load_hooks(&workspace);

        let (tension_id, t_actual, t_parent) = if let Some(ref id_prefix) = p.id {
            let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
            let tension = resolve_id(&tensions, id_prefix)?;
            (tension.id.clone(), Some(tension.actual.clone()), tension.parent_id.clone())
        } else {
            (WORKSPACE_NOTE_TENSION_ID.to_string(), None, None)
        };

        let mutations = store
            .get_mutations(&tension_id)
            .map_err(|e| err(e.to_string()))?;

        let retracted: std::collections::HashSet<String> = mutations
            .iter()
            .filter(|m| m.field() == "note_retracted")
            .map(|m| m.new_value().to_owned())
            .collect();

        let active_notes: Vec<&Mutation> = mutations
            .iter()
            .filter(|m| {
                m.field() == "note" && !retracted.contains(&m.timestamp().to_rfc3339())
            })
            .collect();

        if p.index > active_notes.len() {
            return Err(err(format!(
                "note #{} does not exist ({} active notes)",
                p.index,
                active_notes.len()
            )));
        }

        let target = active_notes[p.index - 1];
        let note_text = target.new_value().to_owned();
        let note_ts = target.timestamp().to_rfc3339();

        let event = HookEvent::mutation(
            &tension_id,
            "note",
            t_actual.as_deref(),
            t_parent.as_deref(),
            "note_retracted",
            Some(&note_text),
            &note_ts,
        );
        if !hooks.pre_mutation(&event) {
            return Err(err("blocked by pre_mutation hook"));
        }

        let _ = store.begin_gesture(Some(&format!("retract note {}", &tension_id)));
        store
            .record_mutation(&Mutation::new(
                tension_id.clone(),
                Utc::now(),
                "note_retracted".to_owned(),
                Some(note_text.clone()),
                note_ts,
            ))
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        hooks.post_mutation(&event);
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": if tension_id == WORKSPACE_NOTE_TENSION_ID { None } else { Some(&tension_id) },
            "retracted_note": note_text,
            "note_number": p.index,
        }))
    }

    #[tool(description = "List active notes for a tension or workspace.")]
    async fn note_list(
        &self,
        Parameters(p): Parameters<NoteListParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;

        let tension_id = if let Some(ref id_prefix) = p.id {
            let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
            let tension = resolve_id(&tensions, id_prefix)?;
            tension.id.clone()
        } else {
            WORKSPACE_NOTE_TENSION_ID.to_string()
        };

        let mutations = store
            .get_mutations(&tension_id)
            .map_err(|e| err(e.to_string()))?;

        let retracted: std::collections::HashSet<String> = mutations
            .iter()
            .filter(|m| m.field() == "note_retracted")
            .map(|m| m.new_value().to_owned())
            .collect();

        let notes: Vec<serde_json::Value> = mutations
            .iter()
            .filter(|m| {
                m.field() == "note" && !retracted.contains(&m.timestamp().to_rfc3339())
            })
            .enumerate()
            .map(|(i, m)| {
                serde_json::json!({
                    "number": i + 1,
                    "timestamp": m.timestamp().to_rfc3339(),
                    "text": m.new_value(),
                })
            })
            .collect();

        json_result(&serde_json::json!({
            "tension_id": if tension_id == WORKSPACE_NOTE_TENSION_ID { None } else { Some(&tension_id) },
            "notes": notes,
        }))
    }

    // ── Snooze / Recur ──────────────────────────────────────────

    #[tool(description = "Snooze a tension until a future date (+3d, +2w, +1m, or YYYY-MM-DD). Use clear=true to unsnooze.")]
    async fn snooze(
        &self,
        Parameters(p): Parameters<SnoozeParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        let _ = store.begin_gesture(Some(&format!("snooze {}", &tension_id)));

        if p.clear {
            store
                .record_mutation(&Mutation::new(
                    tension_id.clone(),
                    Utc::now(),
                    "snooze_cleared".to_owned(),
                    None,
                    "cleared".to_owned(),
                ))
                .map_err(|e| err(e.to_string()))?;
            store.end_gesture();
            autoflush(&workspace);
            return json_result(&serde_json::json!({
                "id": tension_id,
                "snoozed_until": null,
            }));
        }

        let date = p.date.as_deref().ok_or_else(|| {
            err("date is required when not clearing snooze")
        })?;

        store
            .record_mutation(&Mutation::new(
                tension_id.clone(),
                Utc::now(),
                "snoozed_until".to_owned(),
                None,
                date.to_owned(),
            ))
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "snoozed_until": date,
        }))
    }

    #[tool(description = "Set or clear a recurrence interval (+1d, +1w, +2w, +1m). Use clear=true to remove recurrence.")]
    async fn recur(
        &self,
        Parameters(p): Parameters<RecurParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        let _ = store.begin_gesture(Some(&format!("recur {}", &tension_id)));

        if p.clear {
            store
                .record_mutation(&Mutation::new(
                    tension_id.clone(),
                    Utc::now(),
                    "recurrence_cleared".to_owned(),
                    None,
                    "cleared".to_owned(),
                ))
                .map_err(|e| err(e.to_string()))?;
            store.end_gesture();
            autoflush(&workspace);
            return json_result(&serde_json::json!({
                "id": tension_id,
                "recurrence": null,
            }));
        }

        let interval = p.interval.as_deref().ok_or_else(|| {
            err("interval is required when not clearing recurrence")
        })?;

        store
            .record_mutation(&Mutation::new(
                tension_id.clone(),
                Utc::now(),
                "recurrence".to_owned(),
                None,
                interval.to_owned(),
            ))
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        autoflush(&workspace);

        json_result(&serde_json::json!({
            "id": tension_id,
            "recurrence": interval,
        }))
    }

    // ── Epoch tools ─────────────────────────────────────────────

    #[tool(description = "Mark an epoch boundary for a tension — snapshot the current delta.")]
    async fn epoch(
        &self,
        Parameters(p): Parameters<EpochParam>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;
        let tension_id = tension.id.clone();

        let children = store
            .get_children(&tension_id)
            .map_err(|e| err(e.to_string()))?;
        let children_snapshot: Vec<serde_json::Value> = children
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "desired": c.desired,
                    "actual": c.actual,
                    "status": c.status.to_string(),
                    "position": c.position,
                })
            })
            .collect();
        let children_json =
            serde_json::to_string(&children_snapshot).map_err(|e| err(e.to_string()))?;

        let _ = store.begin_gesture(Some(&format!("epoch {}", &tension_id)));
        let epoch_id = store
            .create_epoch(
                &tension_id,
                &tension.desired,
                &tension.actual,
                Some(&children_json),
                store.active_gesture().as_deref(),
            )
            .map_err(|e| err(e.to_string()))?;
        store.end_gesture();

        autoflush(&workspace);

        let epoch_count = store
            .get_epochs(&tension_id)
            .map_err(|e| err(e.to_string()))?
            .len();

        json_result(&serde_json::json!({
            "id": tension_id,
            "epoch_id": epoch_id,
            "epoch_number": epoch_count,
        }))
    }

    #[tool(description = "List all epochs for a tension.")]
    async fn epoch_list(
        &self,
        Parameters(p): Parameters<EpochListParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;

        let epochs = store
            .get_epochs(&tension.id)
            .map_err(|e| err(e.to_string()))?;

        let epoch_list: Vec<serde_json::Value> = epochs
            .iter()
            .enumerate()
            .map(|(i, e)| {
                serde_json::json!({
                    "number": i + 1,
                    "epoch_id": e.id,
                    "desired": e.desire_snapshot,
                    "actual": e.reality_snapshot,
                    "timestamp": e.timestamp.to_rfc3339(),
                })
            })
            .collect();

        json_result(&serde_json::json!({
            "id": tension.id,
            "epochs": epoch_list,
        }))
    }

    #[tool(description = "Show what happened during a specific epoch (mutations on tension + descendants).")]
    async fn epoch_show(
        &self,
        Parameters(p): Parameters<EpochShowParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.epoch == 0 {
            return Err(err("epoch number must be 1 or greater"));
        }

        let (_ws, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let tension = resolve_id(&tensions, &p.id)?;

        let epochs = store.get_epochs(&tension.id).map_err(|e| err(e.to_string()))?;
        if p.epoch > epochs.len() {
            return Err(err(format!(
                "epoch #{} does not exist ({} epochs)",
                p.epoch, epochs.len()
            )));
        }

        let epoch = &epochs[p.epoch - 1];
        let span_start = if p.epoch == 1 {
            tension.created_at
        } else {
            epochs[p.epoch - 2].timestamp
        };
        let span_end = epoch.timestamp;

        let mutations = store
            .get_epoch_mutations(&tension.id, span_start, span_end)
            .map_err(|e| err(e.to_string()))?;

        let mutation_entries: Vec<serde_json::Value> = mutations
            .iter()
            .map(|m| serde_json::json!({
                "tension_id": m.tension_id(),
                "timestamp": m.timestamp().to_rfc3339(),
                "field": m.field(),
                "old_value": m.old_value(),
                "new_value": m.new_value(),
            }))
            .collect();

        json_result(&serde_json::json!({
            "tension_id": tension.id,
            "epoch_number": p.epoch,
            "desire_snapshot": epoch.desire_snapshot,
            "reality_snapshot": epoch.reality_snapshot,
            "span_start": span_start.to_rfc3339(),
            "span_end": span_end.to_rfc3339(),
            "mutations": mutation_entries,
        }))
    }

    // ── Logbase query ──────────────────────────────────────────

    #[tool(description = "Query the logbase — the searchable substrate of all prior epochs. Returns epoch history for a tension (with provenance), cross-tension timeline, or filtered results. Accepts addresses (#42~e3, #42@2026-03). Use compare=true for ghost geometry (desire-reality evolution).")]
    async fn query_logbase(
        &self,
        Parameters(p): Parameters<LogParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_workspace, store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;

        if p.id.is_none() {
            // Cross-tension timeline
            let cutoff = match &p.since {
                Some(s) => parse_mcp_timespec(s).map_err(|e| err(e))?,
                None => Utc::now() - chrono::Duration::days(7),
            };
            let mut entries: Vec<serde_json::Value> = Vec::new();
            for tension in &tensions {
                let epochs = store.get_epochs(&tension.id).map_err(|e| err(e.to_string()))?;
                for (i, epoch) in epochs.iter().enumerate() {
                    if epoch.timestamp >= cutoff {
                        entries.push(serde_json::json!({
                            "timestamp": epoch.timestamp.to_rfc3339(),
                            "tension_id": tension.id,
                            "short_code": tension.short_code,
                            "desired": tension.desired,
                            "epoch_number": i + 1,
                            "epoch_type": epoch.epoch_type,
                        }));
                    }
                }
            }
            entries.sort_by(|a, b| b["timestamp"].as_str().cmp(&a["timestamp"].as_str()));
            return json_result(&serde_json::json!({ "timeline": entries }));
        }

        let id_str = p.id.unwrap();
        let resolver = PrefixResolver::new(tensions.clone());
        let tension = resolver.resolve(&id_str).map_err(|e| err(e.to_string()))?;

        let mut epochs = store.get_epochs(&tension.id).map_err(|e| err(e.to_string()))?;

        if let Some(ref since) = p.since {
            let cutoff = parse_mcp_timespec(since).map_err(|e| err(e))?;
            epochs.retain(|e| e.timestamp >= cutoff);
        }
        if let Some(ref search) = p.search {
            let term = search.to_lowercase();
            epochs.retain(|e| {
                e.desire_snapshot.to_lowercase().contains(&term)
                    || e.reality_snapshot.to_lowercase().contains(&term)
            });
        }

        // Provenance
        let edges = store.get_edges_for_tension(&tension.id).map_err(|e| err(e.to_string()))?;
        let provenance = build_mcp_provenance(&edges, &tension.id, &tensions);

        if p.compare {
            let entries: Vec<serde_json::Value> = epochs.iter().enumerate().map(|(i, e)| {
                serde_json::json!({
                    "number": i + 1,
                    "timestamp": e.timestamp.to_rfc3339(),
                    "desire_snapshot": e.desire_snapshot,
                    "reality_snapshot": e.reality_snapshot,
                })
            }).collect();
            return json_result(&serde_json::json!({
                "tension_id": tension.id,
                "short_code": tension.short_code,
                "compare": entries,
                "current_desire": tension.desired,
                "current_reality": tension.actual,
            }));
        }

        let epoch_entries: Vec<serde_json::Value> = epochs.iter().enumerate().map(|(i, e)| {
            serde_json::json!({
                "number": i + 1,
                "id": e.id,
                "timestamp": e.timestamp.to_rfc3339(),
                "desire_snapshot": e.desire_snapshot,
                "reality_snapshot": e.reality_snapshot,
                "epoch_type": e.epoch_type,
            })
        }).collect();

        json_result(&serde_json::json!({
            "tension_id": tension.id,
            "short_code": tension.short_code,
            "desired": tension.desired,
            "epochs": epoch_entries,
            "provenance": provenance,
        }))
    }

    // ── Split ─────────────────────────────────────────────────────

    #[tool(description = "Split a tension into N new tensions with provenance. Creates split_from edges and cross-tension epochs. Source is resolved by default (set keep=true to keep active). If source has children, provide child assignment via assign array, children_to_parent, or children_to.")]
    async fn split(
        &self,
        Parameters(p): Parameters<SplitParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.desires.len() < 2 {
            return Err(err("split requires at least 2 desires"));
        }

        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let resolver = PrefixResolver::new(tensions.clone());
        let source = resolver.resolve(&p.id).map_err(|e| err(e.to_string()))?;

        if source.status != TensionStatus::Active {
            return Err(err(format!("cannot split {} tension", source.status)));
        }

        let children = store.get_children(&source.id).map_err(|e| err(e.to_string()))?;

        // Parse assignments
        let mut assignments: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        for a in &p.assign {
            let parts: Vec<&str> = a.split('=').collect();
            if parts.len() != 2 { return Err(err(format!("invalid assign: '{}'", a))); }
            let cc: i32 = parts[0].trim_start_matches('#').parse().map_err(|_| err(format!("invalid child: '{}'", parts[0])))?;
            let target: usize = parts[1].parse().map_err(|_| err(format!("invalid target: '{}'", parts[1])))?;
            assignments.insert(cc, target);
        }

        if !children.is_empty() && assignments.is_empty() && !p.children_to_parent && p.children_to.is_none() {
            let child_list: Vec<String> = children.iter().map(|c| format!("#{}", c.short_code.unwrap_or(0))).collect();
            return Err(err(format!("source has children that need assignment: {}. Use assign, children_to_parent, or children_to.", child_list.join(", "))));
        }

        if p.dry_run {
            return json_result(&serde_json::json!({
                "dry_run": true,
                "source_id": source.id,
                "source_short_code": source.short_code,
                "desires": p.desires,
                "children_count": children.len(),
            }));
        }

        let gesture_id = store.begin_gesture(Some("split")).map_err(|e| err(e.to_string()))?;

        store.create_epoch_typed(&source.id, &source.desired, &source.actual, None, Some(&gesture_id), Some("split_source"))
            .map_err(|e| err(e.to_string()))?;

        let mut new_tensions = Vec::new();
        for desire in &p.desires {
            let t = store.create_tension_with_parent(desire, "", source.parent_id.clone())
                .map_err(|e| err(e.to_string()))?;
            store.create_edge(&t.id, &source.id, sd_core::EDGE_SPLIT_FROM).map_err(|e| err(e.to_string()))?;
            store.create_epoch_typed(&t.id, desire, "", None, Some(&gesture_id), Some("split_target"))
                .map_err(|e| err(e.to_string()))?;
            new_tensions.push(t);
        }

        for child in &children {
            let cc = child.short_code.unwrap_or(0);
            let target_idx = if let Some(&t) = assignments.get(&cc) { Some(t - 1) }
                else if p.children_to_parent { None }
                else if let Some(t) = p.children_to { Some(t - 1) }
                else { None };
            let new_parent = match target_idx {
                Some(idx) => Some(new_tensions[idx].id.as_str()),
                None => source.parent_id.as_deref(),
            };
            store.update_parent(&child.id, new_parent).map_err(|e| err(e.to_string()))?;
        }

        if !p.keep {
            store.update_status(&source.id, TensionStatus::Resolved).map_err(|e| err(e.to_string()))?;
        }

        store.end_gesture();
        autoflush(&workspace);

        let result: Vec<serde_json::Value> = new_tensions.iter().map(|t| {
            serde_json::json!({ "id": t.id, "short_code": t.short_code, "desired": t.desired })
        }).collect();

        json_result(&serde_json::json!({
            "source_id": source.id,
            "source_short_code": source.short_code,
            "source_status": if p.keep { "active" } else { "resolved" },
            "new_tensions": result,
        }))
    }

    // ── Merge ─────────────────────────────────────────────────────

    #[tool(description = "Merge tensions with provenance. Asymmetric (into=id, one survives) or symmetric (as_desire='text', both absorbed into new). Creates merged_into edges and cross-tension epochs.")]
    async fn merge(
        &self,
        Parameters(p): Parameters<MergeParam>,
    ) -> Result<CallToolResult, McpError> {
        if p.into.is_none() && p.as_desire.is_none() {
            return Err(err("merge requires either 'into' (asymmetric) or 'as_desire' (symmetric)"));
        }

        let (workspace, mut store) = open_store()?;
        let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
        let resolver = PrefixResolver::new(tensions.clone());

        let t1 = resolver.resolve(&p.id1).map_err(|e| err(e.to_string()))?;
        let t2 = resolver.resolve(&p.id2).map_err(|e| err(e.to_string()))?;

        if t1.id == t2.id { return Err(err("cannot merge a tension with itself")); }
        for t in [&t1, &t2] {
            if t.status != TensionStatus::Active {
                return Err(err(format!("cannot merge {} tension #{}", t.status, t.short_code.unwrap_or(0))));
            }
        }

        if p.dry_run {
            return json_result(&serde_json::json!({
                "dry_run": true,
                "id1": t1.id, "id2": t2.id,
                "mode": if p.into.is_some() { "asymmetric" } else { "symmetric" },
            }));
        }

        let gesture_id = store.begin_gesture(Some("merge")).map_err(|e| err(e.to_string()))?;

        if let Some(ref as_desire) = p.as_desire {
            // Symmetric
            for t in [&t1, &t2] {
                store.create_epoch_typed(&t.id, &t.desired, &t.actual, None, Some(&gesture_id), Some("merge_source"))
                    .map_err(|e| err(e.to_string()))?;
            }
            let new_t = store.create_tension_with_parent(as_desire, "", t1.parent_id.clone())
                .map_err(|e| err(e.to_string()))?;
            store.create_epoch_typed(&new_t.id, as_desire, "", None, Some(&gesture_id), Some("merge_target"))
                .map_err(|e| err(e.to_string()))?;
            for t in [&t1, &t2] {
                store.create_edge(&t.id, &new_t.id, sd_core::EDGE_MERGED_INTO).map_err(|e| err(e.to_string()))?;
                store.update_status(&t.id, TensionStatus::Resolved).map_err(|e| err(e.to_string()))?;
            }
            store.end_gesture();
            autoflush(&workspace);
            return json_result(&serde_json::json!({
                "mode": "symmetric",
                "new_tension": { "id": new_t.id, "short_code": new_t.short_code, "desired": new_t.desired },
                "absorbed": [
                    { "id": t1.id, "short_code": t1.short_code },
                    { "id": t2.id, "short_code": t2.short_code },
                ],
            }));
        }

        // Asymmetric
        let into_id = p.into.unwrap();
        let into_resolved = resolver.resolve(&into_id).map_err(|e| err(e.to_string()))?;
        if into_resolved.id != t1.id && into_resolved.id != t2.id {
            return Err(err("--into must be one of the merge arguments"));
        }
        let (survivor, absorbed) = if into_resolved.id == t1.id { (&t1, &t2) } else { (&t2, &t1) };

        store.create_epoch_typed(&absorbed.id, &absorbed.desired, &absorbed.actual, None, Some(&gesture_id), Some("merge_source"))
            .map_err(|e| err(e.to_string()))?;
        store.create_epoch_typed(&survivor.id, &survivor.desired, &survivor.actual, None, Some(&gesture_id), Some("merge_target"))
            .map_err(|e| err(e.to_string()))?;
        store.create_edge(&absorbed.id, &survivor.id, sd_core::EDGE_MERGED_INTO).map_err(|e| err(e.to_string()))?;

        if let Some(ref d) = p.desire {
            store.update_desired(&survivor.id, d).map_err(|e| err(e.to_string()))?;
        }

        // Reparent absorbed children
        let absorbed_children = store.get_children(&absorbed.id).map_err(|e| err(e.to_string()))?;
        for child in &absorbed_children {
            let new_parent = if p.children_to_parent { absorbed.parent_id.as_deref() } else { Some(survivor.id.as_str()) };
            store.update_parent(&child.id, new_parent).map_err(|e| err(e.to_string()))?;
        }

        store.update_status(&absorbed.id, TensionStatus::Resolved).map_err(|e| err(e.to_string()))?;
        store.end_gesture();
        autoflush(&workspace);

        json_result(&serde_json::json!({
            "mode": "asymmetric",
            "survivor": { "id": survivor.id, "short_code": survivor.short_code },
            "absorbed": { "id": absorbed.id, "short_code": absorbed.short_code, "status": "resolved" },
            "children_reparented": absorbed_children.len(),
        }))
    }

    // ── Edge queries ──────────────────────────────────────────────

    #[tool(description = "Query typed edges (structural relationships). Returns edges for a tension or all edges of a given type. Edge types: contains (parent-child), split_from (provenance), merged_into (provenance).")]
    async fn edges(
        &self,
        Parameters(p): Parameters<EdgesParam>,
    ) -> Result<CallToolResult, McpError> {
        let (_workspace, store) = open_store()?;

        let edges = if let Some(ref id) = p.id {
            let tensions = store.list_tensions().map_err(|e| err(e.to_string()))?;
            let resolver = PrefixResolver::new(tensions);
            let t = resolver.resolve(id).map_err(|e| err(e.to_string()))?;
            store.get_edges_for_tension(&t.id).map_err(|e| err(e.to_string()))?
        } else if let Some(ref edge_type) = p.edge_type {
            store.get_edges_by_type(edge_type).map_err(|e| err(e.to_string()))?
        } else {
            store.get_all_edges().map_err(|e| err(e.to_string()))?
        };

        let mut filtered = edges;
        if let Some(ref et) = p.edge_type {
            if p.id.is_some() {
                filtered.retain(|e| &e.edge_type == et);
            }
        }

        let entries: Vec<serde_json::Value> = filtered.iter().map(|e| {
            serde_json::json!({
                "id": e.id,
                "from_id": e.from_id,
                "to_id": e.to_id,
                "edge_type": e.edge_type,
                "created_at": e.created_at.to_rfc3339(),
                "gesture_id": e.gesture_id,
            })
        }).collect();

        json_result(&serde_json::json!({ "edges": entries, "count": entries.len() }))
    }

    // ── Batch tool ──────────────────────────────────────────────

    #[tool(description = "Apply batch mutations from YAML. Supports create_child, update_actual, update_desired, update_status, add_note, set_horizon, move_tension, create_parent.")]
    async fn batch(
        &self,
        Parameters(p): Parameters<BatchParam>,
    ) -> Result<CallToolResult, McpError> {
        use werk_shared::BatchMutation;

        if p.yaml.trim().is_empty() {
            return Err(err("input is empty"));
        }

        let mutations: Vec<BatchMutation> = serde_yaml::from_str(&p.yaml)
            .map_err(|e| err(format!("could not parse YAML: {}", e)))?;

        if mutations.is_empty() {
            return json_result(&serde_json::json!({
                "applied": 0, "failed": 0, "dry_run": p.dry_run, "mutations": [],
            }));
        }

        let (workspace, store) = open_store()?;
        let mut engine = Engine::with_store(store);

        if !p.dry_run {
            let _ = engine.store_mut().begin_gesture(
                Some(&format!("batch apply ({} mutations)", mutations.len())),
            );
        }

        let mut applied = 0usize;
        let mut failed = 0usize;
        let mut results: Vec<serde_json::Value> = Vec::new();

        for (i, mutation) in mutations.iter().enumerate() {
            let summary = mutation.summary();

            // Validate tension exists
            let validation = validate_batch_mutation(&engine, &mutation);
            match validation {
                Ok(()) => {
                    if p.dry_run {
                        results.push(serde_json::json!({
                            "index": i, "status": "valid", "summary": summary,
                        }));
                        applied += 1;
                    } else {
                        match apply_batch_mutation(&mut engine, &mutation) {
                            Ok(()) => {
                                results.push(serde_json::json!({
                                    "index": i, "status": "applied", "summary": summary,
                                }));
                                applied += 1;
                            }
                            Err(e) => {
                                results.push(serde_json::json!({
                                    "index": i, "status": "failed", "summary": summary,
                                    "error": e.to_string(),
                                }));
                                failed += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "index": i, "status": "failed", "summary": summary,
                        "error": e.to_string(),
                    }));
                    failed += 1;
                }
            }
        }

        if !p.dry_run {
            engine.store_mut().end_gesture();
        }

        autoflush(&workspace);

        json_result(&serde_json::json!({
            "applied": applied,
            "failed": failed,
            "dry_run": p.dry_run,
            "mutations": results,
        }))
    }
}

// ── ServerHandler impl ──────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for WerkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "werk is an operative instrument for structural dynamics practice. \
                 It manages tensions (desire-reality pairs) organized in a tree structure. \
                 Use read tools (show, tree, list, survey, health) to understand the current state, \
                 and gesture tools (add, resolve, reality, desire, etc.) to mutate it."
                    .to_string(),
            )
    }
}

