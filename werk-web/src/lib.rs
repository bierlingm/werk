//! werk-web: Axum-based web server for the werk structural dynamics instrument.
//!
//! Serves an interactive HTML frontend and exposes a REST API that calls sd-core directly.
//! Designed to migrate cleanly into Tauri commands later.
//!
//! # Architecture
//!
//! sd-core's `Store` is `!Send` (fsqlite uses Rc internally). We handle this by
//! running all store operations on a dedicated OS thread, communicating via channels.
//! This pattern maps directly to Tauri's command model later.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response, Sse},
    routing::{get, patch, post},
};
use sd_core::{Forest, Horizon, Tension, TensionStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

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
                let mut store = match sd_core::Store::init(&store_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("failed to open store: {}", e);
                        return;
                    }
                };

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        StoreCmd::ListTensions { reply } => {
                            let _ = reply.send(
                                store.list_tensions().map_err(|e| e.to_string()),
                            );
                        }
                        StoreCmd::CreateTension { desired, actual, parent_id, horizon, reply } => {
                            let _ = store.begin_gesture(Some("web: create tension"));
                            let result = store
                                .create_tension_full(&desired, &actual, parent_id, horizon)
                                .map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateDesired { id, value, reply } => {
                            let _ = store.begin_gesture(Some("web: update desired"));
                            let result = store.update_desired(&id, &value).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateReality { id, value, reply } => {
                            let _ = store.begin_gesture(Some("web: update reality"));
                            let result = store.update_actual(&id, &value).map_err(|e| e.to_string());
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
                            let result = store.update_status(&id, status).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::GetTension { id, reply } => {
                            let _ = reply.send(
                                store.get_tension(&id).map_err(|e| e.to_string()),
                            );
                        }
                    }
                }
            })
            .map_err(|e| format!("failed to spawn store thread: {}", e))?;

        Ok(Self { tx })
    }

    async fn list_tensions(&self) -> StoreResult<Vec<Tension>> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(StoreCmd::ListTensions { reply }).map_err(|e| e.to_string())?;
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
            .send(StoreCmd::CreateTension { desired, actual, parent_id, horizon, reply })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn update_desired(&self, id: String, value: String) -> StoreResult<()> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(StoreCmd::UpdateDesired { id, value, reply }).map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn update_reality(&self, id: String, value: String) -> StoreResult<()> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(StoreCmd::UpdateReality { id, value, reply }).map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn update_status(&self, id: String, status: TensionStatus) -> StoreResult<()> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(StoreCmd::UpdateStatus { id, status, reply }).map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }

    async fn get_tension(&self, id: String) -> StoreResult<Option<Tension>> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(StoreCmd::GetTension { id, reply }).map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())?
    }
}

// ─── Shared App State ──────────────────────────────────────────────

/// Shared application state (Send + Sync).
pub struct AppState {
    store: StoreHandle,
    tx: broadcast::Sender<SseEvent>,
}

#[derive(Clone, Serialize, Debug)]
struct SseEvent {
    kind: String,
}

// ─── JSON Types ────────────────────────────────────────────────────

/// A JSON-serializable tension for the API.
#[derive(Serialize, Clone)]
pub struct TensionJson {
    id: String,
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: String,
    parent_id: Option<String>,
    horizon: Option<String>,
    position: Option<i32>,
    created_at: String,
    overdue: bool,
}

impl TensionJson {
    fn from_tension(t: &Tension) -> Self {
        let overdue = match (&t.horizon, &t.status) {
            (Some(h), TensionStatus::Active) => {
                let now = chrono::Utc::now();
                h.is_past(now)
            }
            _ => false,
        };
        Self {
            id: t.id.clone(),
            short_code: t.short_code,
            desired: t.desired.clone(),
            actual: t.actual.clone(),
            status: t.status.to_string(),
            parent_id: t.parent_id.clone(),
            horizon: t.horizon.as_ref().map(|h| h.to_string()),
            position: t.position,
            created_at: t.created_at.to_rfc3339(),
            overdue,
        }
    }
}

/// Tree node for structured response.
#[derive(Serialize)]
struct TreeNodeJson {
    tension: TensionJson,
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
    tensions: Vec<TensionJson>,
    roots: Vec<TreeNodeJson>,
    summary: SummaryJson,
}

#[derive(Serialize)]
struct SummaryJson {
    active: usize,
    resolved: usize,
    released: usize,
    total: usize,
}

#[derive(Deserialize)]
pub struct CreateTensionRequest {
    desired: String,
    actual: Option<String>,
    parent_id: Option<String>,
    horizon: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateFieldRequest {
    value: String,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

fn err_response(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ApiError { error: msg.into() })).into_response()
}

// ─── Router ────────────────────────────────────────────────────────

/// Build the Axum router. Takes the workspace root path for store discovery.
pub fn build_router(store_path: std::path::PathBuf) -> Result<Router, String> {
    let store = StoreHandle::spawn(store_path)?;
    let (tx, _) = broadcast::channel::<SseEvent>(64);
    let state = Arc::new(AppState { store, tx });

    Ok(Router::new()
        .route("/", get(serve_frontend))
        .route("/api/tensions", get(get_tensions))
        .route("/api/tensions", post(create_tension))
        .route("/api/tensions/{id}/desired", patch(update_desired))
        .route("/api/tensions/{id}/reality", patch(update_reality))
        .route("/api/tensions/{id}/resolve", post(resolve_tension))
        .route("/api/tensions/{id}/release", post(release_tension))
        .route("/api/tensions/{id}/reopen", post(reopen_tension))
        .route("/api/events", get(sse_handler))
        .layer(CorsLayer::permissive())
        .with_state(state))
}

/// Start the server on the given port.
pub async fn serve(store_path: std::path::PathBuf, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_router(store_path)?;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("werk web → http://localhost:{}", port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
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

    let summary = SummaryJson {
        active: all.iter().filter(|t| t.status == TensionStatus::Active).count(),
        resolved: all.iter().filter(|t| t.status == TensionStatus::Resolved).count(),
        released: all.iter().filter(|t| t.status == TensionStatus::Released).count(),
        total: all.len(),
    };

    let tension_jsons: Vec<TensionJson> = all.iter().map(TensionJson::from_tension).collect();

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

fn build_tree_node(forest: &Forest, node: &sd_core::tree::Node) -> TreeNodeJson {
    let children: Vec<TreeNodeJson> = node
        .children
        .iter()
        .filter_map(|id| forest.find(id))
        .map(|child| build_tree_node(forest, child))
        .collect();

    let (resolved, total) = count_descendants(forest, &node.tension.id);

    TreeNodeJson {
        tension: TensionJson::from_tension(&node.tension),
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
                    return err_response(StatusCode::BAD_REQUEST, format!("invalid horizon: {}", e))
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

    match state.store.create_tension(req.desired, actual, parent_id, horizon).await {
        Ok(t) => {
            let _ = state.tx.send(SseEvent { kind: "tension_created".into() });
            (StatusCode::CREATED, Json(TensionJson::from_tension(&t))).into_response()
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
            let _ = state.tx.send(SseEvent { kind: "tension_updated".into() });
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
            let _ = state.tx.send(SseEvent { kind: "tension_updated".into() });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn resolve_tension(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state.store.update_status(tension_id, TensionStatus::Resolved).await {
        Ok(()) => {
            let _ = state.tx.send(SseEvent { kind: "tension_resolved".into() });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn release_tension(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state.store.update_status(tension_id, TensionStatus::Released).await {
        Ok(()) => {
            let _ = state.tx.send(SseEvent { kind: "tension_released".into() });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn reopen_tension(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let tension_id = match resolve_id(&state.store, &id).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match state.store.update_status(tension_id, TensionStatus::Active).await {
        Ok(()) => {
            let _ = state.tx.send(SseEvent { kind: "tension_reopened".into() });
            StatusCode::OK.into_response()
        }
        Err(e) => err_response(StatusCode::BAD_REQUEST, e),
    }
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
{
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
