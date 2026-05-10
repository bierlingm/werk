//! werk-web: Axum-based web server for the werk structural dynamics instrument.
//!
//! Serves an interactive HTML frontend and exposes a REST API that calls werk-core directly.
//! Designed to migrate cleanly into Tauri commands later.
//!
//! # Architecture
//!
//! werk-core's `Store` is `!Send` (fsqlite uses Rc internally). We handle this by
//! running all store operations on a dedicated OS thread, communicating via channels.
//! This pattern maps directly to Tauri's command model later.

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Html, IntoResponse, Response, Sse},
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, oneshot};
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;
use werk_core::{Address, Forest, Horizon, Tension, TensionStatus, compute_urgency, parse_address};
use werk_shared::PrefixResolver;
use werk_shared::aggregate::{
    AttentionItem, DEFAULT_HELD_PER_SPACE, DEFAULT_NEXT_UP_PER_SPACE, SkippedSpace, SpaceRef,
    SpaceVitals, VitalsTotals, compute_attention_for_store, compute_vitals_for_store,
    enumerate_spaces,
};
use werk_shared::dto::{
    ApiError, CreateTensionRequest, SummaryDto, TensionDto, UpdateFieldRequest,
};
use werk_sigil::{
    Ctx, Engine, Logic, Scope, SigilError, cache_path, derive_seed, load_preset, scope_canonical,
    start_hot_reload, werk_state_revision,
};

const FRONTEND_HTML: &str = include_str!("../index.html");

// ─── Store Thread ──────────────────────────────────────────────────

type StoreResult<T> = Result<T, String>;

/// A command sent to the dedicated store thread.
enum StoreCmd {
    ListTensions {
        reply: oneshot::Sender<StoreResult<Vec<Tension>>>,
    },
    CreateTension {
        desired: String,
        actual: String,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
        reply: oneshot::Sender<StoreResult<Tension>>,
    },
    UpdateDesired {
        id: String,
        value: String,
        reply: oneshot::Sender<StoreResult<()>>,
    },
    UpdateReality {
        id: String,
        value: String,
        reply: oneshot::Sender<StoreResult<()>>,
    },
    UpdateStatus {
        id: String,
        status: TensionStatus,
        reply: oneshot::Sender<StoreResult<()>>,
    },
    GetTension {
        id: String,
        reply: oneshot::Sender<StoreResult<Option<Tension>>>,
    },
    /// Compute vitals for this store, tagged with the given space.
    ComputeVitals {
        space: SpaceRef,
        now: chrono::DateTime<chrono::Utc>,
        reply: oneshot::Sender<StoreResult<SpaceVitals>>,
    },
    /// Compute attention bands for this store, tagged with the given space.
    ComputeAttention {
        space: SpaceRef,
        now: chrono::DateTime<chrono::Utc>,
        next_up_per_space: usize,
        held_per_space: usize,
        reply: oneshot::Sender<
            StoreResult<(Vec<AttentionItem>, Vec<AttentionItem>, Vec<AttentionItem>)>,
        >,
    },
    /// List mutations recorded in a time window (used by /api/views/epoch).
    MutationsBetween {
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        reply: oneshot::Sender<StoreResult<Vec<werk_core::mutation::Mutation>>>,
    },
    ResolveSigil {
        scope: String,
        logic: Logic,
        seed: Option<u64>,
        workspace_name: String,
        reply: oneshot::Sender<StoreResult<SigilScopeInfo>>,
    },
    RenderSigil {
        scope: Scope,
        logic: Logic,
        seed: Option<u64>,
        workspace_name: String,
        reply: oneshot::Sender<StoreResult<SigilRenderResult>>,
    },
}

#[derive(Clone)]
struct SigilScopeInfo {
    scope: Scope,
    scope_canonical: String,
    seed: u64,
    revision: String,
}

struct SigilRenderResult {
    svg: Vec<u8>,
}

/// Handle to communicate with the store thread.
#[derive(Clone)]
struct StoreHandle {
    tx: std::sync::mpsc::Sender<StoreCmd>,
}

impl StoreHandle {
    /// Spawn a dedicated OS thread that owns the Store.
    fn spawn(store_path: std::path::PathBuf) -> Result<Self, String> {
        let (tx, rx) = std::sync::mpsc::channel::<StoreCmd>();

        std::thread::Builder::new()
            .name("werk-store".into())
            .spawn(move || {
                let mut store = match werk_core::Store::init(&store_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("failed to open store: {}", e);
                        return;
                    }
                };

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        StoreCmd::ListTensions { reply } => {
                            let _ = reply.send(store.list_tensions().map_err(|e| e.to_string()));
                        }
                        StoreCmd::CreateTension {
                            desired,
                            actual,
                            parent_id,
                            horizon,
                            reply,
                        } => {
                            let _ = store.begin_gesture(Some("web: create tension"));
                            let result = store
                                .create_tension_full(&desired, &actual, parent_id, horizon)
                                .map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateDesired { id, value, reply } => {
                            let _ = store.begin_gesture(Some("web: update desired"));
                            let result =
                                store.update_desired(&id, &value).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateReality { id, value, reply } => {
                            let _ = store.begin_gesture(Some("web: update reality"));
                            let result =
                                store.update_actual(&id, &value).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateStatus { id, status, reply } => {
                            let label = match status {
                                TensionStatus::Active => "web: reopen",
                                TensionStatus::Resolved => "web: resolve",
                                TensionStatus::Released => "web: release",
                            };
                            let _ = store.begin_gesture(Some(label));
                            let result =
                                store.update_status(&id, status).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::GetTension { id, reply } => {
                            let _ = reply.send(store.get_tension(&id).map_err(|e| e.to_string()));
                        }
                        StoreCmd::ComputeVitals { space, now, reply } => {
                            let _ = reply.send(
                                compute_vitals_for_store(space, &store, now)
                                    .map_err(|e| e.to_string()),
                            );
                        }
                        StoreCmd::MutationsBetween { start, end, reply } => {
                            let _ = reply.send(
                                store
                                    .mutations_between(start, end)
                                    .map_err(|e| e.to_string()),
                            );
                        }
                        StoreCmd::ComputeAttention {
                            space,
                            now,
                            next_up_per_space,
                            held_per_space,
                            reply,
                        } => {
                            let _ = reply.send(
                                compute_attention_for_store(
                                    &space,
                                    &store,
                                    now,
                                    next_up_per_space,
                                    held_per_space,
                                )
                                .map_err(|e| e.to_string()),
                            );
                        }
                        StoreCmd::ResolveSigil {
                            scope,
                            logic,
                            seed,
                            workspace_name,
                            reply,
                        } => {
                            let result =
                                resolve_sigil_scope(&store, &logic, &scope).and_then(|resolved| {
                                    let mut ctx =
                                        Ctx::new(chrono::Utc::now(), &store, workspace_name, 0);
                                    let compiled =
                                        Engine::compile(logic).map_err(|e| e.to_string())?;
                                    let resolved_scope = compiled
                                        .selector
                                        .select(resolved.clone(), &mut ctx)
                                        .map_err(|e| e.to_string())?;
                                    let scope_canonical = scope_canonical(&resolved_scope);
                                    let seed_value = seed.unwrap_or_else(|| {
                                        derive_seed(&compiled.logic, &scope_canonical)
                                    });
                                    let revision =
                                        werk_state_revision(&store, &resolved_scope.tensions)
                                            .map_err(|e| e.to_string())?;
                                    Ok(SigilScopeInfo {
                                        scope: resolved,
                                        scope_canonical,
                                        seed: seed_value,
                                        revision,
                                    })
                                });
                            let _ = reply.send(result);
                        }
                        StoreCmd::RenderSigil {
                            scope,
                            logic,
                            seed,
                            workspace_name,
                            reply,
                        } => {
                            let mut ctx = Ctx::new(chrono::Utc::now(), &store, workspace_name, 0);
                            let result = Engine::render_with_seed(scope, logic, &mut ctx, seed)
                                .map(|sigil| SigilRenderResult { svg: sigil.svg.0 })
                                .map_err(|e| e.to_string());
                            let _ = reply.send(result);
                        }
                    }
                }
            })
            .map_err(|e| format!("failed to spawn store thread: {}", e))?;

        Ok(Self { tx })
    }

    async fn list_tensions(&self) -> StoreResult<Vec<Tension>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::ListTensions { reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn create_tension(
        &self,
        desired: String,
        actual: String,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> StoreResult<Tension> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::CreateTension {
                desired,
                actual,
                parent_id,
                horizon,
                reply,
            })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn update_desired(&self, id: String, value: String) -> StoreResult<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::UpdateDesired { id, value, reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn update_reality(&self, id: String, value: String) -> StoreResult<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::UpdateReality { id, value, reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn update_status(&self, id: String, status: TensionStatus) -> StoreResult<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::UpdateStatus { id, status, reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn get_tension(&self, id: String) -> StoreResult<Option<Tension>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::GetTension { id, reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn compute_vitals(
        &self,
        space: SpaceRef,
        now: chrono::DateTime<chrono::Utc>,
    ) -> StoreResult<SpaceVitals> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::ComputeVitals { space, now, reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn mutations_between(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> StoreResult<Vec<werk_core::mutation::Mutation>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::MutationsBetween { start, end, reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn compute_attention(
        &self,
        space: SpaceRef,
        now: chrono::DateTime<chrono::Utc>,
        next_up_per_space: usize,
        held_per_space: usize,
    ) -> StoreResult<(Vec<AttentionItem>, Vec<AttentionItem>, Vec<AttentionItem>)> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::ComputeAttention {
                space,
                now,
                next_up_per_space,
                held_per_space,
                reply,
            })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn resolve_sigil(
        &self,
        scope: String,
        logic: Logic,
        seed: Option<u64>,
        workspace_name: String,
    ) -> StoreResult<SigilScopeInfo> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::ResolveSigil {
                scope,
                logic,
                seed,
                workspace_name,
                reply,
            })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn render_sigil(
        &self,
        scope: Scope,
        logic: Logic,
        seed: Option<u64>,
        workspace_name: String,
    ) -> StoreResult<SigilRenderResult> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(StoreCmd::RenderSigil {
                scope,
                logic,
                seed,
                workspace_name,
                reply,
            })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }
}

// ─── Shared App State ──────────────────────────────────────────────

/// Shared application state (Send + Sync).
pub struct AppState {
    store: StoreHandle,
    tx: broadcast::Sender<SseEvent>,
    workspace_root: std::path::PathBuf,
    /// Lazy pool of StoreHandles keyed by absolute workspace path, used by
    /// `/api/field/*` endpoints to fan out reads across every registered
    /// space. Handles spawn on first access and persist for the lifetime of
    /// the server — at the scale of registered spaces (dozens, not thousands)
    /// a persistent thread-per-store is the simplest shape that respects the
    /// `!Send` constraint on `werk_core::Store`.
    field_pool: Arc<RwLock<HashMap<PathBuf, StoreHandle>>>,
}

impl AppState {
    /// Get-or-spawn a StoreHandle for the given space path. Returns `Err` on
    /// spawn failure; the caller should treat that as a skipped space rather
    /// than failing the whole aggregate.
    async fn handle_for(&self, path: &std::path::Path) -> Result<StoreHandle, String> {
        {
            let pool = self.field_pool.read().await;
            if let Some(h) = pool.get(path) {
                return Ok(h.clone());
            }
        }
        let mut pool = self.field_pool.write().await;
        if let Some(h) = pool.get(path) {
            return Ok(h.clone());
        }
        let handle = StoreHandle::spawn(path.to_path_buf())?;
        pool.insert(path.to_path_buf(), handle.clone());
        Ok(handle)
    }
}

#[derive(Serialize)]
struct WorkspaceJson {
    path: String,
    name: String,
    is_global: bool,
}

impl WorkspaceJson {
    fn from_entry(e: &werk_shared::daemon_workspaces::WorkspaceEntry) -> Self {
        Self {
            path: e.path.display().to_string(),
            name: e.name.clone(),
            is_global: e.is_global,
        }
    }
}

#[derive(Serialize)]
struct WorkspacesResponse {
    current: WorkspaceJson,
    available: Vec<WorkspaceJson>,
}

#[derive(Deserialize)]
pub struct SelectWorkspaceRequest {
    path: String,
}

#[derive(Deserialize)]
struct SigilQuery {
    scope: Option<String>,
    logic: Option<String>,
    seed: Option<u64>,
}

#[derive(Clone, Serialize, Debug)]
struct SseEvent {
    kind: String,
}

fn sigil_preset_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../werk-sigil/presets")
}

fn sigil_hot_reload_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let preset_dir = sigil_preset_dir();
    if preset_dir.exists() {
        paths.push(preset_dir);
    }
    if let Ok(extra) = env::var("WERK_SIGIL_WATCH_PATHS") {
        for part in extra.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            let path = PathBuf::from(trimmed);
            if path.exists() {
                paths.push(path);
            }
        }
    }
    paths
}

fn start_sigil_hot_reload(tx: broadcast::Sender<SseEvent>) {
    let paths = sigil_hot_reload_paths();
    if paths.is_empty() {
        return;
    }
    match start_hot_reload(paths) {
        Ok(watcher) => {
            watcher.spawn_listener(move |_| {
                let _ = tx.send(SseEvent {
                    kind: "invalidate".into(),
                });
            });
        }
        Err(err) => {
            eprintln!("sigil hot reload disabled: {err}");
        }
    }
}

// ─── JSON Types ────────────────────────────────────────────────────
//
// TensionDto, SummaryDto, CreateTensionRequest, UpdateFieldRequest, ApiError
// are defined in `werk_shared::dto` so the Web, Tauri and CLI surfaces share
// the same wire format.  Only web-specific envelope types live here.

/// Tree node for structured response.
#[derive(Serialize)]
struct TreeNodeJson {
    tension: TensionDto,
    children: Vec<TreeNodeJson>,
    closure: ClosureJson,
}

#[derive(Serialize)]
struct ClosureJson {
    resolved: usize,
    total: usize,
}

#[derive(Serialize)]
struct TreeResponse {
    tensions: Vec<TensionDto>,
    roots: Vec<TreeNodeJson>,
    summary: SummaryDto,
}

fn err_response(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ApiError::new(msg))).into_response()
}

// ─── Router ────────────────────────────────────────────────────────

/// Build the Axum router. Takes the workspace root path for store discovery.
pub fn build_router(store_path: std::path::PathBuf) -> Result<Router, String> {
    let store = StoreHandle::spawn(store_path.clone())?;
    let (tx, _) = broadcast::channel::<SseEvent>(64);
    let mut pool: HashMap<PathBuf, StoreHandle> = HashMap::new();
    // Seed the pool with the active workspace's handle so `/api/field/*`
    // reuses the already-spawned thread when it enumerates the active space.
    pool.insert(store_path.clone(), store.clone());
    let state = Arc::new(AppState {
        store,
        tx,
        workspace_root: store_path,
        field_pool: Arc::new(RwLock::new(pool)),
    });
    start_sigil_hot_reload(state.tx.clone());

    Ok(Router::new()
        .route("/", get(serve_frontend))
        .route("/api/tensions", get(get_tensions))
        .route("/api/tensions", post(create_tension))
        .route("/api/tensions/{id}/desired", patch(update_desired))
        .route("/api/tensions/{id}/reality", patch(update_reality))
        .route("/api/tensions/{id}/resolve", post(resolve_tension))
        .route("/api/tensions/{id}/release", post(release_tension))
        .route("/api/tensions/{id}/reopen", post(reopen_tension))
        .route("/api/workspace", get(get_workspace))
        .route("/api/workspaces", get(get_workspaces))
        .route("/api/workspace/select", post(select_workspace))
        .route("/api/field/vitals", get(get_field_vitals))
        .route("/api/field/attention", get(get_field_attention))
        .route("/api/views/focus", get(get_view_focus))
        .route("/api/views/horizon", get(get_view_horizon))
        .route("/api/views/deadlines", get(get_view_deadlines))
        .route("/api/views/epoch", get(get_view_epoch))
        .route("/api/views/tree", get(get_view_tree))
        .route("/api/sigil", get(get_sigil))
        .route("/api/sigil/stream", get(sse_sigil_handler))
        .route("/api/events", get(sse_handler))
        .layer(CorsLayer::permissive())
        .with_state(state))
}

async fn get_workspace(State(state): State<Arc<AppState>>) -> Response {
    let entry =
        werk_shared::daemon_workspaces::WorkspaceEntry::from_path(state.workspace_root.clone());
    Json(WorkspaceJson::from_entry(&entry)).into_response()
}

async fn get_workspaces(State(state): State<Arc<AppState>>) -> Response {
    let current =
        werk_shared::daemon_workspaces::WorkspaceEntry::from_path(state.workspace_root.clone());
    let available = match werk_shared::daemon_workspaces::list() {
        Ok((_, list)) => list,
        Err(_) => vec![current.clone()],
    };
    Json(WorkspacesResponse {
        current: WorkspaceJson::from_entry(&current),
        available: available.iter().map(WorkspaceJson::from_entry).collect(),
    })
    .into_response()
}

async fn select_workspace(Json(req): Json<SelectWorkspaceRequest>) -> Response {
    let path = std::path::PathBuf::from(&req.path);
    if !path.join(".werk").exists() {
        return err_response(
            StatusCode::BAD_REQUEST,
            format!("{} is not a werk workspace", path.display()),
        );
    }
    if let Err(e) = werk_shared::daemon_workspaces::set_active(&path) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    // Exit so the supervisor (launchd / systemd) restarts us against the new
    // workspace. Sleep briefly to let the response flush before tearing down.
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        std::process::exit(0);
    });
    let entry = werk_shared::daemon_workspaces::WorkspaceEntry::from_path(path);
    (
        StatusCode::ACCEPTED,
        Json(WorkspaceJson::from_entry(&entry)),
    )
        .into_response()
}

/// Start the server on the given host and port.
pub async fn serve(
    store_path: std::path::PathBuf,
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_router(store_path)?;
    let ip: std::net::IpAddr = host
        .parse()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("invalid host '{host}': {e}")))?;
    let addr = std::net::SocketAddr::new(ip, port);
    let display_host = if host == "127.0.0.1" || host == "0.0.0.0" {
        "localhost".to_string()
    } else {
        host
    };
    eprintln!("werk web → http://{display_host}:{port}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Start the server on an already-bound listener.
///
/// Use this when the caller needs to pick the port itself (e.g. scanning a
/// range for a free port and writing the chosen port to disk before handing
/// control over).
pub async fn serve_on(
    store_path: std::path::PathBuf,
    listener: tokio::net::TcpListener,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_router(store_path)?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ─── Handlers ──────────────────────────────────────────────────────

async fn serve_frontend() -> Html<&'static str> {
    Html(FRONTEND_HTML)
}

async fn get_tensions(State(state): State<Arc<AppState>>) -> Response {
    let all = match state.store.list_tensions().await {
        Ok(t) => t,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    let summary = SummaryDto::from_tensions(&all);

    let tension_jsons: Vec<TensionDto> = all.iter().map(TensionDto::from_tension).collect();

    // Build tree
    let forest = match Forest::from_tensions(all) {
        Ok(f) => f,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let roots = forest
        .root_ids()
        .iter()
        .filter_map(|id| forest.find(id))
        .map(|node| build_tree_node(&forest, node))
        .collect();

    Json(TreeResponse {
        tensions: tension_jsons,
        roots,
        summary,
    })
    .into_response()
}

fn build_tree_node(forest: &Forest, node: &werk_core::tree::Node) -> TreeNodeJson {
    let children: Vec<TreeNodeJson> = node
        .children
        .iter()
        .filter_map(|id| forest.find(id))
        .map(|child| build_tree_node(forest, child))
        .collect();

    let (resolved, total) = count_descendants(forest, &node.tension.id);

    TreeNodeJson {
        tension: TensionDto::from_tension(&node.tension),
        closure: ClosureJson { resolved, total },
        children,
    }
}

fn count_descendants(forest: &Forest, id: &str) -> (usize, usize) {
    let node = match forest.find(id) {
        Some(n) => n,
        None => return (0, 0),
    };
    let mut resolved = 0usize;
    let mut total = 0usize;
    for child_id in &node.children {
        if let Some(child) = forest.find(child_id) {
            total += 1;
            if child.tension.status == TensionStatus::Resolved {
                resolved += 1;
            }
            let (cr, ct) = count_descendants(forest, child_id);
            resolved += cr;
            total += ct;
        }
    }
    (resolved, total)
}

async fn create_tension(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTensionRequest>,
) -> Response {
    let horizon = if let Some(ref h) = req.horizon {
        if h.is_empty() {
            None
        } else {
            match Horizon::parse(h) {
                Ok(h) => Some(h),
                Err(e) => {
                    return err_response(
                        StatusCode::BAD_REQUEST,
                        format!("invalid horizon: {}", e),
                    );
                }
            }
        }
    } else {
        None
    };

    let actual = req.actual.unwrap_or_else(|| "Not yet started".to_string());

    // If parent_id is a short code, resolve it first
    let parent_id = if let Some(ref pid) = req.parent_id {
        match resolve_id(&state.store, pid).await {
            Ok(id) => Some(id),
            Err(r) => return r,
        }
    } else {
        None
    };

    match state
        .store
        .create_tension(req.desired, actual, parent_id, horizon)
        .await
    {
        Ok(t) => {
            let _ = state.tx.send(SseEvent {
                kind: "tension_created".into(),
            });
            let _ = state.tx.send(SseEvent {
                kind: "invalidate".into(),
            });
            (StatusCode::CREATED, Json(TensionDto::from_tension(&t))).into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn update_desired(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateFieldRequest>,
) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state.store.update_desired(tension_id, req.value).await {
        Ok(()) => {
            let _ = state.tx.send(SseEvent {
                kind: "tension_updated".into(),
            });
            let _ = state.tx.send(SseEvent {
                kind: "invalidate".into(),
            });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn update_reality(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateFieldRequest>,
) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state.store.update_reality(tension_id, req.value).await {
        Ok(()) => {
            let _ = state.tx.send(SseEvent {
                kind: "tension_updated".into(),
            });
            let _ = state.tx.send(SseEvent {
                kind: "invalidate".into(),
            });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn resolve_tension(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state
        .store
        .update_status(tension_id, TensionStatus::Resolved)
        .await
    {
        Ok(()) => {
            let _ = state.tx.send(SseEvent {
                kind: "tension_resolved".into(),
            });
            let _ = state.tx.send(SseEvent {
                kind: "invalidate".into(),
            });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn release_tension(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state
        .store
        .update_status(tension_id, TensionStatus::Released)
        .await
    {
        Ok(()) => {
            let _ = state.tx.send(SseEvent {
                kind: "tension_released".into(),
            });
            let _ = state.tx.send(SseEvent {
                kind: "invalidate".into(),
            });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn reopen_tension(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state
        .store
        .update_status(tension_id, TensionStatus::Active)
        .await
    {
        Ok(()) => {
            let _ = state.tx.send(SseEvent {
                kind: "tension_reopened".into(),
            });
            let _ = state.tx.send(SseEvent {
                kind: "invalidate".into(),
            });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn get_sigil(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SigilQuery>,
) -> Response {
    let scope = match query.scope.as_deref().map(str::trim) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return err_response(StatusCode::BAD_REQUEST, "missing required scope parameter"),
    };

    let logic = match load_logic(query.logic) {
        Ok(l) => l,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let workspace_name = workspace_name_from_root(&state.workspace_root);
    let scope_info = match state
        .store
        .resolve_sigil(scope, logic.clone(), query.seed, workspace_name.clone())
        .await
    {
        Ok(info) => info,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, e),
    };

    if let Err(e) = werk_sigil::cleanup_cache(7) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    let cache = cache_path(
        &scope_info.scope_canonical,
        &logic.cache_key(),
        scope_info.seed,
        &scope_info.revision,
    );
    if let Ok(bytes) = std::fs::read(&cache) {
        return svg_response(bytes);
    }

    if let Some(parent) = cache.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    let rendered = match state
        .store
        .render_sigil(scope_info.scope, logic, query.seed, workspace_name)
        .await
    {
        Ok(result) => result,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, e),
    };

    if let Err(e) = std::fs::write(&cache, &rendered.svg) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    svg_response(rendered.svg)
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<
    impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
> {
    let rx = state.tx.subscribe();
    let stream =
        tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(event) => Some(Ok::<_, std::convert::Infallible>(
                axum::response::sse::Event::default()
                    .event(&event.kind)
                    .data("{}"),
            )),
            Err(_) => None,
        });
    Sse::new(stream)
}

async fn sse_sigil_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<
    impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
> {
    let rx = state.tx.subscribe();
    let stream =
        tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(event) if event.kind == "invalidate" => Some(Ok::<_, std::convert::Infallible>(
                axum::response::sse::Event::default()
                    .event("invalidate")
                    .data("{}"),
            )),
            _ => None,
        });
    Sse::new(stream)
}

// ─── Field (aggregate) endpoints ────────────────────────────────────

/// JSON shape of the aggregate vitals response. Mirrors
/// `werk_shared::aggregate::AggregateVitals` but uses API-friendly
/// Strings for paths.
#[derive(Serialize)]
struct FieldVitalsJson {
    computed_at: String,
    spaces: Vec<FieldSpaceVitalsJson>,
    totals: FieldTotalsJson,
    skipped: Vec<FieldSkippedJson>,
}

#[derive(Serialize)]
struct FieldSpaceVitalsJson {
    name: String,
    path: String,
    is_global: bool,
    active: usize,
    resolved: usize,
    released: usize,
    deadlined: usize,
    overdue: usize,
    positioned: usize,
    held: usize,
    last_activity: Option<String>,
}

#[derive(Serialize, Default)]
struct FieldTotalsJson {
    active: usize,
    resolved: usize,
    released: usize,
    deadlined: usize,
    overdue: usize,
    positioned: usize,
    held: usize,
}

#[derive(Serialize)]
struct FieldSkippedJson {
    name: String,
    path: String,
    reason: String,
}

#[derive(Serialize)]
struct FieldAttentionJson {
    computed_at: String,
    overdue: Vec<FieldAttentionItemJson>,
    next_up: Vec<FieldAttentionItemJson>,
    held: Vec<FieldAttentionItemJson>,
    skipped: Vec<FieldSkippedJson>,
}

#[derive(Serialize)]
struct FieldAttentionItemJson {
    space_name: String,
    short_code: Option<i32>,
    desired: String,
    horizon: Option<String>,
    urgency: Option<f64>,
    position: Option<i32>,
}

async fn get_field_vitals(State(state): State<Arc<AppState>>) -> Response {
    let (spaces, mut skipped) = match enumerate_spaces() {
        Ok(pair) => pair,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let now = chrono::Utc::now();
    let mut per_space: Vec<SpaceVitals> = Vec::new();

    for space in spaces {
        match state.handle_for(&space.path).await {
            Ok(handle) => match handle.compute_vitals(space.clone(), now).await {
                Ok(v) => per_space.push(v),
                Err(e) => skipped.push(SkippedSpace {
                    name: space.name,
                    path: space.path,
                    reason: format!("read failed: {e}"),
                }),
            },
            Err(e) => skipped.push(SkippedSpace {
                name: space.name,
                path: space.path,
                reason: format!("spawn failed: {e}"),
            }),
        }
    }

    let totals = per_space
        .iter()
        .fold(VitalsTotals::default(), |mut acc, v| {
            acc.active += v.active;
            acc.resolved += v.resolved;
            acc.released += v.released;
            acc.deadlined += v.deadlined;
            acc.overdue += v.overdue;
            acc.positioned += v.positioned;
            acc.held += v.held;
            acc
        });

    Json(FieldVitalsJson {
        computed_at: now.to_rfc3339(),
        spaces: per_space.into_iter().map(to_space_vitals_json).collect(),
        totals: FieldTotalsJson {
            active: totals.active,
            resolved: totals.resolved,
            released: totals.released,
            deadlined: totals.deadlined,
            overdue: totals.overdue,
            positioned: totals.positioned,
            held: totals.held,
        },
        skipped: skipped.into_iter().map(to_skipped_json).collect(),
    })
    .into_response()
}

async fn get_field_attention(State(state): State<Arc<AppState>>) -> Response {
    let (spaces, mut skipped) = match enumerate_spaces() {
        Ok(pair) => pair,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let now = chrono::Utc::now();
    let mut overdue: Vec<AttentionItem> = Vec::new();
    let mut next_up: Vec<AttentionItem> = Vec::new();
    let mut held: Vec<AttentionItem> = Vec::new();

    for space in &spaces {
        match state.handle_for(&space.path).await {
            Ok(handle) => match handle
                .compute_attention(
                    space.clone(),
                    now,
                    DEFAULT_NEXT_UP_PER_SPACE,
                    DEFAULT_HELD_PER_SPACE,
                )
                .await
            {
                Ok((o, n, h)) => {
                    overdue.extend(o);
                    next_up.extend(n);
                    held.extend(h);
                }
                Err(e) => skipped.push(SkippedSpace {
                    name: space.name.clone(),
                    path: space.path.clone(),
                    reason: format!("read failed: {e}"),
                }),
            },
            Err(e) => skipped.push(SkippedSpace {
                name: space.name.clone(),
                path: space.path.clone(),
                reason: format!("spawn failed: {e}"),
            }),
        }
    }

    // Locality-safe pooled ordering: overdue by urgency desc, next_up by
    // position ascending (tie-broken by space for determinism), held by urgency.
    overdue.sort_by(|a, b| {
        b.urgency
            .unwrap_or(0.0)
            .partial_cmp(&a.urgency.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    next_up.sort_by(|a, b| {
        a.position
            .unwrap_or(i32::MAX)
            .cmp(&b.position.unwrap_or(i32::MAX))
            .then_with(|| a.space_name.cmp(&b.space_name))
    });
    held.sort_by(|a, b| {
        b.urgency
            .unwrap_or(0.0)
            .partial_cmp(&a.urgency.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Json(FieldAttentionJson {
        computed_at: now.to_rfc3339(),
        overdue: overdue.into_iter().map(to_attention_item_json).collect(),
        next_up: next_up.into_iter().map(to_attention_item_json).collect(),
        held: held.into_iter().map(to_attention_item_json).collect(),
        skipped: skipped.into_iter().map(to_skipped_json).collect(),
    })
    .into_response()
}

fn to_space_vitals_json(v: SpaceVitals) -> FieldSpaceVitalsJson {
    FieldSpaceVitalsJson {
        name: v.space.name,
        path: v.space.path.display().to_string(),
        is_global: v.space.is_global,
        active: v.active,
        resolved: v.resolved,
        released: v.released,
        deadlined: v.deadlined,
        overdue: v.overdue,
        positioned: v.positioned,
        held: v.held,
        last_activity: v.last_activity.map(|t| t.to_rfc3339()),
    }
}

fn to_attention_item_json(item: AttentionItem) -> FieldAttentionItemJson {
    FieldAttentionItemJson {
        space_name: item.space_name,
        short_code: item.short_code,
        desired: item.desired,
        horizon: item.horizon,
        urgency: item.urgency,
        position: item.position,
    }
}

fn to_skipped_json(s: SkippedSpace) -> FieldSkippedJson {
    FieldSkippedJson {
        name: s.name,
        path: s.path.display().to_string(),
        reason: s.reason,
    }
}

// ─── Views (consumer-agnostic projections) ─────────────────────────
//
// `/api/views/*` endpoints emit werk-native shapes for downstream
// consumers (TRMNL, watch faces, status bars). They do not encode
// device dimensions, character budgets, or glyphs — that lives in
// each consumer's adapter.

#[derive(Serialize)]
struct FocusViewJson {
    view: &'static str,
    generated_at: String,
    workspace: WorkspaceJson,
    selection_reason: &'static str,
    tension: Option<FocusTensionJson>,
}

#[derive(Serialize)]
struct FocusTensionJson {
    id: String,
    short_code: Option<i32>,
    desired: String,
    reality: String,
    status: String,
    horizon: Option<String>,
    overdue: bool,
    urgency: Option<f64>,
    age_days: i64,
    parent: Option<FocusParentJson>,
}

#[derive(Serialize)]
struct FocusParentJson {
    short_code: Option<i32>,
    desired: String,
}

async fn get_view_focus(State(state): State<Arc<AppState>>) -> Response {
    let all = match state.store.list_tensions().await {
        Ok(t) => t,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    let now = chrono::Utc::now();
    let workspace = WorkspaceJson::from_entry(
        &werk_shared::daemon_workspaces::WorkspaceEntry::from_path(state.workspace_root.clone()),
    );

    // Selection: among Active tensions, prefer the one with highest urgency
    // (deadline-relative pressure). Fall back to most recently created Active
    // tension if no one has a horizon. Returns None only when nothing is Active.
    let active: Vec<&Tension> = all
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .collect();

    let (chosen, reason): (Option<&Tension>, &'static str) = active
        .iter()
        .filter_map(|t| compute_urgency(t, now).map(|u| (*t, u.value)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(t, _)| (Some(t), "most-urgent"))
        .unwrap_or_else(|| {
            let fallback = active.iter().max_by_key(|t| t.created_at).copied();
            (
                fallback,
                if fallback.is_some() {
                    "newest-active-no-horizon"
                } else {
                    "no-active-tensions"
                },
            )
        });

    let tension = chosen.map(|t| {
        let parent = t
            .parent_id
            .as_ref()
            .and_then(|pid| all.iter().find(|x| &x.id == pid))
            .map(|p| FocusParentJson {
                short_code: p.short_code,
                desired: p.desired.clone(),
            });
        let urgency = compute_urgency(t, now).map(|u| u.value);
        let overdue = matches!(&t.horizon, Some(h) if h.is_past(now));
        let age_days = (now - t.created_at).num_days();
        FocusTensionJson {
            id: t.id.clone(),
            short_code: t.short_code,
            desired: t.desired.clone(),
            reality: t.actual.clone(),
            status: format!("{:?}", t.status).to_lowercase(),
            horizon: t.horizon.as_ref().map(|h| h.to_string()),
            overdue,
            urgency,
            age_days,
            parent,
        }
    });

    Json(FocusViewJson {
        view: "focus",
        generated_at: now.to_rfc3339(),
        workspace,
        selection_reason: reason,
        tension,
    })
    .into_response()
}

// ─── /api/views/horizon ─────────────────────────────────────────
//
// Top-N most-pressing active tensions, ordered by urgency (deadline-
// relative pressure). Tie-broken by deadline range_end ascending then
// position. Items without urgency (no horizon) sort last.

const HORIZON_LIMIT: usize = 5;

#[derive(Serialize)]
struct HorizonViewJson {
    view: &'static str,
    generated_at: String,
    workspace: WorkspaceJson,
    items: Vec<HorizonItem>,
}

#[derive(Serialize)]
struct HorizonItem {
    id: String,
    short_code: Option<i32>,
    desired: String,
    horizon: Option<String>,
    overdue: bool,
    urgency: Option<f64>,
    parent_short_code: Option<i32>,
}

async fn get_view_horizon(State(state): State<Arc<AppState>>) -> Response {
    let all = match state.store.list_tensions().await {
        Ok(t) => t,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let now = chrono::Utc::now();
    let workspace = WorkspaceJson::from_entry(
        &werk_shared::daemon_workspaces::WorkspaceEntry::from_path(state.workspace_root.clone()),
    );

    let mut active: Vec<&Tension> = all
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .collect();
    active.sort_by(|a, b| {
        let ua = compute_urgency(a, now).map(|u| u.value);
        let ub = compute_urgency(b, now).map(|u| u.value);
        match (ua, ub) {
            (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    active.truncate(HORIZON_LIMIT);

    let parent_short = |pid: &Option<String>| {
        pid.as_ref()
            .and_then(|id| all.iter().find(|t| &t.id == id))
            .and_then(|p| p.short_code)
    };

    let items: Vec<HorizonItem> = active
        .iter()
        .map(|t| HorizonItem {
            id: t.id.clone(),
            short_code: t.short_code,
            desired: t.desired.clone(),
            horizon: t.horizon.as_ref().map(|h| h.to_string()),
            overdue: matches!(&t.horizon, Some(h) if h.is_past(now)),
            urgency: compute_urgency(t, now).map(|u| u.value),
            parent_short_code: parent_short(&t.parent_id),
        })
        .collect();

    Json(HorizonViewJson {
        view: "horizon",
        generated_at: now.to_rfc3339(),
        workspace,
        items,
    })
    .into_response()
}

// ─── /api/views/deadlines ───────────────────────────────────────
//
// Active tensions whose horizon range_end is within the next 14 days,
// sorted ascending by due date. The downstream adapter decides how
// many days to actually show (typically 7).

const DEADLINES_WINDOW_DAYS: i64 = 14;

#[derive(Serialize)]
struct DeadlinesViewJson {
    view: &'static str,
    generated_at: String,
    workspace: WorkspaceJson,
    horizon_window_days: i64,
    items: Vec<DeadlineItem>,
}

#[derive(Serialize)]
struct DeadlineItem {
    id: String,
    short_code: Option<i32>,
    desired: String,
    horizon: String,
    due_at: String,
    days_until: i64,
    urgency: Option<f64>,
    overdue: bool,
}

async fn get_view_deadlines(State(state): State<Arc<AppState>>) -> Response {
    let all = match state.store.list_tensions().await {
        Ok(t) => t,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let now = chrono::Utc::now();
    let window_end = now + chrono::Duration::days(DEADLINES_WINDOW_DAYS);
    let workspace = WorkspaceJson::from_entry(
        &werk_shared::daemon_workspaces::WorkspaceEntry::from_path(state.workspace_root.clone()),
    );

    let mut items: Vec<DeadlineItem> = all
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .filter_map(|t| {
            let h = t.horizon.as_ref()?;
            let due = h.range_end();
            // Include overdue items (negative days_until) too.
            if due > window_end {
                return None;
            }
            let days_until = (due - now).num_days();
            Some(DeadlineItem {
                id: t.id.clone(),
                short_code: t.short_code,
                desired: t.desired.clone(),
                horizon: h.to_string(),
                due_at: due.to_rfc3339(),
                days_until,
                urgency: compute_urgency(t, now).map(|u| u.value),
                overdue: h.is_past(now),
            })
        })
        .collect();
    items.sort_by_key(|i| i.days_until);

    Json(DeadlinesViewJson {
        view: "deadlines",
        generated_at: now.to_rfc3339(),
        workspace,
        horizon_window_days: DEADLINES_WINDOW_DAYS,
        items,
    })
    .into_response()
}

// ─── /api/views/epoch ───────────────────────────────────────────
//
// Recent meaningful gestures since (now - 24h). One row per mutation
// with a human-readable summary. The downstream adapter chooses how
// many to show.

const EPOCH_LOOKBACK_HOURS: i64 = 24;
const EPOCH_LIMIT: usize = 30;

#[derive(Serialize)]
struct EpochViewJson {
    view: &'static str,
    generated_at: String,
    workspace: WorkspaceJson,
    since: String,
    items: Vec<EpochItem>,
}

#[derive(Serialize)]
struct EpochItem {
    ts: String,
    tension_id: String,
    tension_short_code: Option<i32>,
    field: String,
    summary: String,
}

async fn get_view_epoch(State(state): State<Arc<AppState>>) -> Response {
    let now = chrono::Utc::now();
    let since = now - chrono::Duration::hours(EPOCH_LOOKBACK_HOURS);
    let mutations = match state.store.mutations_between(since, now).await {
        Ok(m) => m,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let all = match state.store.list_tensions().await {
        Ok(t) => t,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let workspace = WorkspaceJson::from_entry(
        &werk_shared::daemon_workspaces::WorkspaceEntry::from_path(state.workspace_root.clone()),
    );

    let short_for = |id: &str| all.iter().find(|t| t.id == id).and_then(|t| t.short_code);

    let mut items: Vec<EpochItem> = mutations
        .iter()
        .map(|m| EpochItem {
            ts: m.timestamp().to_rfc3339(),
            tension_id: m.tension_id().to_string(),
            tension_short_code: short_for(m.tension_id()),
            field: m.field().to_string(),
            summary: summarize_mutation(m.field()),
        })
        .collect();
    items.sort_by(|a, b| b.ts.cmp(&a.ts));
    items.truncate(EPOCH_LIMIT);

    Json(EpochViewJson {
        view: "epoch",
        generated_at: now.to_rfc3339(),
        workspace,
        since: since.to_rfc3339(),
        items,
    })
    .into_response()
}

fn summarize_mutation(field: &str) -> String {
    match field {
        "created" => "created".to_string(),
        "desired" => "desired updated".to_string(),
        "actual" | "reality" => "reality updated".to_string(),
        "status" => "status changed".to_string(),
        "horizon" => "deadline set".to_string(),
        "position" => "repositioned".to_string(),
        "note" => "note added".to_string(),
        "parent_id" => "moved".to_string(),
        other => other.to_string(),
    }
}

fn load_logic(arg: Option<String>) -> Result<Logic, SigilError> {
    let logic_name = arg.unwrap_or_else(|| "contemplative".to_string());
    let path = logic_path(&logic_name)?;
    load_preset(path).map(|preset| preset.logic)
}

fn logic_path(logic_name: &str) -> Result<PathBuf, SigilError> {
    let candidate = PathBuf::from(logic_name);
    if candidate.extension().is_some() || logic_name.contains('/') || logic_name.contains('\\') {
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(SigilError::io(format!(
            "logic file not found: {}",
            candidate.display()
        )));
    }
    let preset_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../werk-sigil/presets");
    Ok(preset_dir.join(format!("{logic_name}.toml")))
}

fn workspace_name_from_root(root: &std::path::Path) -> String {
    root.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "werk".to_string())
}

fn svg_response(bytes: Vec<u8>) -> Response {
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, "image/svg+xml".parse().unwrap());
    response
}

// ─── /api/views/tree ────────────────────────────────────────────
//
// Forest summary limited to depth 2, listing root tensions and their
// direct children. Each item carries depth, parent short_code, and a
// closure ratio (open vs total descendants). Adapters decide how
// many to render.

const TREE_MAX_DEPTH: usize = 2;
const TREE_LIMIT: usize = 30;

#[derive(Serialize)]
struct TreeViewJson {
    view: &'static str,
    generated_at: String,
    workspace: WorkspaceJson,
    items: Vec<TreeItem>,
    totals: TreeTotalsJson,
}

#[derive(Serialize)]
struct TreeItem {
    id: String,
    short_code: Option<i32>,
    desired: String,
    depth: usize,
    parent_short_code: Option<i32>,
    direct_children: usize,
    descendants_total: usize,
    descendants_resolved: usize,
}

#[derive(Serialize)]
struct TreeTotalsJson {
    active: usize,
    resolved: usize,
    released: usize,
}

async fn get_view_tree(State(state): State<Arc<AppState>>) -> Response {
    let all = match state.store.list_tensions().await {
        Ok(t) => t,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let now = chrono::Utc::now();
    let workspace = WorkspaceJson::from_entry(
        &werk_shared::daemon_workspaces::WorkspaceEntry::from_path(state.workspace_root.clone()),
    );

    let totals = TreeTotalsJson {
        active: all
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .count(),
        resolved: all
            .iter()
            .filter(|t| t.status == TensionStatus::Resolved)
            .count(),
        released: all
            .iter()
            .filter(|t| t.status == TensionStatus::Released)
            .count(),
    };

    let forest = match Forest::from_tensions(all.clone()) {
        Ok(f) => f,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let mut items: Vec<TreeItem> = Vec::new();
    let parent_short = |pid: &Option<String>| {
        pid.as_ref()
            .and_then(|id| all.iter().find(|t| &t.id == id))
            .and_then(|p| p.short_code)
    };

    fn walk<'a>(
        forest: &'a Forest,
        node: &'a werk_core::tree::Node,
        depth: usize,
        max_depth: usize,
        all: &[Tension],
        parent_short_lookup: &dyn Fn(&Option<String>) -> Option<i32>,
        out: &mut Vec<TreeItem>,
    ) {
        let (resolved, total) = {
            let mut r = 0usize;
            let mut t = 0usize;
            fn rec(forest: &Forest, id: &str, r: &mut usize, t: &mut usize) {
                if let Some(n) = forest.find(id) {
                    for child_id in &n.children {
                        if let Some(c) = forest.find(child_id) {
                            *t += 1;
                            if c.tension.status == TensionStatus::Resolved {
                                *r += 1;
                            }
                            rec(forest, child_id, r, t);
                        }
                    }
                }
            }
            rec(forest, &node.tension.id, &mut r, &mut t);
            (r, t)
        };

        out.push(TreeItem {
            id: node.tension.id.clone(),
            short_code: node.tension.short_code,
            desired: node.tension.desired.clone(),
            depth,
            parent_short_code: parent_short_lookup(&node.tension.parent_id),
            direct_children: node.children.len(),
            descendants_total: total,
            descendants_resolved: resolved,
        });

        if depth >= max_depth {
            return;
        }
        for child_id in &node.children {
            if let Some(child) = forest.find(child_id) {
                if child.tension.status == TensionStatus::Active {
                    walk(
                        forest,
                        child,
                        depth + 1,
                        max_depth,
                        all,
                        parent_short_lookup,
                        out,
                    );
                }
            }
        }
    }

    for root_id in forest.root_ids() {
        if let Some(root) = forest.find(&root_id) {
            if root.tension.status == TensionStatus::Active {
                walk(
                    &forest,
                    root,
                    0,
                    TREE_MAX_DEPTH,
                    &all,
                    &parent_short,
                    &mut items,
                );
            }
        }
    }
    items.truncate(TREE_LIMIT);

    let _ = now; // reserved for future per-item urgency annotations
    Json(TreeViewJson {
        view: "tree",
        generated_at: chrono::Utc::now().to_rfc3339(),
        workspace,
        items,
        totals,
    })
    .into_response()
}

fn resolve_sigil_scope(
    store: &werk_core::Store,
    logic: &Logic,
    input: &str,
) -> Result<Scope, String> {
    if input.trim().is_empty() {
        return Err("scope is required".to_string());
    }
    if let Some(at) = logic.scope_at.as_deref()
        && at != "now"
    {
        return Err("historical scope is not supported for sigils in v1".to_string());
    }

    if let Ok(addr) = parse_address(input) {
        match addr {
            Address::Tension(n) => return resolve_scope_by_prefix(store, logic, &n.to_string()),
            Address::Epoch { .. } | Address::Note { .. } | Address::TensionAt { .. } => {
                return Err("historical or sub-address scopes are not supported".to_string());
            }
            Address::Sigil(_) => {
                return Err("sigil short codes cannot be used as render scopes".to_string());
            }
            Address::Gesture(_) | Address::Session(_) | Address::CrossSpace { .. } => {
                return Err("unsupported scope address for sigil rendering".to_string());
            }
        }
    }

    resolve_scope_by_prefix(store, logic, input)
}

fn resolve_scope_by_prefix(
    store: &werk_core::Store,
    logic: &Logic,
    input: &str,
) -> Result<Scope, String> {
    let tensions = store.list_tensions().map_err(|e| e.to_string())?;
    let resolver = PrefixResolver::new(tensions);
    let tension = resolver.resolve(input).map_err(|e| e.to_string())?;
    Ok(logic
        .scope_default
        .clone()
        .into_scope(Some(tension.id.clone()), None))
}

/// Resolve an ID that might be a short_code number or a full ULID.
async fn resolve_id(store: &StoreHandle, input: &str) -> Result<String, Response> {
    // Try as short_code first
    if let Ok(code) = input.parse::<i32>() {
        let all = store
            .list_tensions()
            .await
            .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        if let Some(t) = all.iter().find(|t| t.short_code == Some(code)) {
            return Ok(t.id.clone());
        }
    }

    // Try as full ID
    let tension = store
        .get_tension(input.to_string())
        .await
        .map_err(|e| err_response(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    match tension {
        Some(t) => Ok(t.id),
        None => Err(err_response(
            StatusCode::NOT_FOUND,
            format!("tension not found: {}", input),
        )),
    }
}
