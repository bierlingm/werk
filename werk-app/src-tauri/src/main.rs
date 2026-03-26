#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

//! werk-app: Tauri 2.0 desktop app for the werk structural dynamics instrument.
//!
//! sd-core's Store is !Send (fsqlite uses Rc internally). We handle this by
//! running all store operations on a dedicated OS thread, communicating via
//! std::sync::mpsc channels. This is the same pattern used by werk-web.

use sd_core::{Horizon, Tension, TensionStatus};
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Mutex};
use tauri::State;

// ─── Store Actor ────────────────────────────────────────────────────

type StoreResult<T> = Result<T, String>;

enum StoreCmd {
    ListTensions {
        reply: mpsc::SyncSender<StoreResult<Vec<Tension>>>,
    },
    CreateTension {
        desired: String,
        actual: String,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
        reply: mpsc::SyncSender<StoreResult<Tension>>,
    },
    UpdateDesired {
        id: String,
        value: String,
        reply: mpsc::SyncSender<StoreResult<()>>,
    },
    UpdateReality {
        id: String,
        value: String,
        reply: mpsc::SyncSender<StoreResult<()>>,
    },
    UpdateStatus {
        id: String,
        status: TensionStatus,
        reply: mpsc::SyncSender<StoreResult<()>>,
    },
    GetTension {
        id: String,
        reply: mpsc::SyncSender<StoreResult<Option<Tension>>>,
    },
    UpdatePosition {
        id: String,
        position: Option<i32>,
        reply: mpsc::SyncSender<StoreResult<bool>>,
    },
}

struct StoreHandle {
    tx: Mutex<mpsc::Sender<StoreCmd>>,
}

impl StoreHandle {
    fn spawn(store_path: std::path::PathBuf) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<StoreCmd>();

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
                            let _ = store.begin_gesture(Some("app: create tension"));
                            let result = store
                                .create_tension_full(&desired, &actual, parent_id, horizon)
                                .map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateDesired { id, value, reply } => {
                            let _ = store.begin_gesture(Some("app: update desired"));
                            let result = store.update_desired(&id, &value).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateReality { id, value, reply } => {
                            let _ = store.begin_gesture(Some("app: update reality"));
                            let result = store.update_actual(&id, &value).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                        StoreCmd::UpdateStatus { id, status, reply } => {
                            let label = match status {
                                TensionStatus::Active => "app: reopen",
                                TensionStatus::Resolved => "app: resolve",
                                TensionStatus::Released => "app: release",
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
                        StoreCmd::UpdatePosition { id, position, reply } => {
                            let _ = store.begin_gesture(Some("app: reposition"));
                            let result = store.update_position(&id, position).map_err(|e| e.to_string());
                            store.end_gesture();
                            let _ = reply.send(result);
                        }
                    }
                }
            })
            .map_err(|e| format!("failed to spawn store thread: {}", e))?;

        Ok(Self { tx: Mutex::new(tx) })
    }

    fn list_tensions(&self) -> StoreResult<Vec<Tension>> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx.lock().unwrap()
            .send(StoreCmd::ListTensions { reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn create_tension(
        &self,
        desired: String,
        actual: String,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> StoreResult<Tension> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx.lock().unwrap()
            .send(StoreCmd::CreateTension { desired, actual, parent_id, horizon, reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn update_desired(&self, id: String, value: String) -> StoreResult<()> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx.lock().unwrap()
            .send(StoreCmd::UpdateDesired { id, value, reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn update_reality(&self, id: String, value: String) -> StoreResult<()> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx.lock().unwrap()
            .send(StoreCmd::UpdateReality { id, value, reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn update_status(&self, id: String, status: TensionStatus) -> StoreResult<()> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx.lock().unwrap()
            .send(StoreCmd::UpdateStatus { id, status, reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn get_tension(&self, id: String) -> StoreResult<Option<Tension>> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx.lock().unwrap()
            .send(StoreCmd::GetTension { id, reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn update_position(&self, id: String, position: Option<i32>) -> StoreResult<bool> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx.lock().unwrap()
            .send(StoreCmd::UpdatePosition { id, position, reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn resolve_id(&self, input: &str) -> StoreResult<String> {
        if let Ok(code) = input.parse::<i32>() {
            let all = self.list_tensions()?;
            if let Some(t) = all.iter().find(|t| t.short_code == Some(code)) {
                return Ok(t.id.clone());
            }
        }
        let tension = self.get_tension(input.to_string())?;
        match tension {
            Some(t) => Ok(t.id),
            None => Err(format!("tension not found: {}", input)),
        }
    }
}

// ─── JSON Types ─────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct TensionJson {
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
            (Some(h), TensionStatus::Active) => h.is_past(chrono::Utc::now()),
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

#[derive(Serialize)]
struct SummaryJson {
    active: usize,
    resolved: usize,
    released: usize,
    total: usize,
}

#[derive(Serialize)]
struct TreeResponse {
    tensions: Vec<TensionJson>,
    summary: SummaryJson,
}

// ─── Tauri Commands ─────────────────────────────────────────────────

#[tauri::command]
fn get_tree(store: State<'_, StoreHandle>) -> Result<TreeResponse, String> {
    let all = store.list_tensions()?;
    let summary = SummaryJson {
        active: all.iter().filter(|t| t.status == TensionStatus::Active).count(),
        resolved: all.iter().filter(|t| t.status == TensionStatus::Resolved).count(),
        released: all.iter().filter(|t| t.status == TensionStatus::Released).count(),
        total: all.len(),
    };
    let tension_jsons: Vec<TensionJson> = all.iter().map(TensionJson::from_tension).collect();
    Ok(TreeResponse { tensions: tension_jsons, summary })
}

#[derive(Deserialize)]
struct CreateTensionArgs {
    desired: String,
    actual: Option<String>,
    parent_id: Option<String>,
    horizon: Option<String>,
}

#[tauri::command]
fn create_tension(store: State<'_, StoreHandle>, args: CreateTensionArgs) -> Result<TensionJson, String> {
    let horizon = if let Some(ref h) = args.horizon {
        if h.is_empty() {
            None
        } else {
            Some(Horizon::parse(h).map_err(|e| format!("invalid horizon: {}", e))?)
        }
    } else {
        None
    };

    let actual = args.actual.unwrap_or_else(|| "Not yet started".to_string());

    let parent_id = if let Some(ref pid) = args.parent_id {
        Some(store.resolve_id(pid)?)
    } else {
        None
    };

    let t = store.create_tension(args.desired, actual, parent_id, horizon)?;
    Ok(TensionJson::from_tension(&t))
}

#[tauri::command]
fn update_reality(store: State<'_, StoreHandle>, id: String, new_reality: String) -> Result<(), String> {
    let tension_id = store.resolve_id(&id)?;
    store.update_reality(tension_id, new_reality)
}

#[tauri::command]
fn update_desired(store: State<'_, StoreHandle>, id: String, new_desired: String) -> Result<(), String> {
    let tension_id = store.resolve_id(&id)?;
    store.update_desired(tension_id, new_desired)
}

#[tauri::command]
fn resolve_tension(store: State<'_, StoreHandle>, id: String) -> Result<(), String> {
    let tension_id = store.resolve_id(&id)?;
    store.update_status(tension_id, TensionStatus::Resolved)
}

#[tauri::command]
fn reopen_tension(store: State<'_, StoreHandle>, id: String) -> Result<(), String> {
    let tension_id = store.resolve_id(&id)?;
    store.update_status(tension_id, TensionStatus::Active)
}

#[tauri::command]
fn get_tension(store: State<'_, StoreHandle>, id: String) -> Result<Option<TensionJson>, String> {
    let tension_id = store.resolve_id(&id)?;
    let t = store.get_tension(tension_id)?;
    Ok(t.as_ref().map(TensionJson::from_tension))
}

#[tauri::command]
fn position_tension(store: State<'_, StoreHandle>, id: String, position: Option<i32>) -> Result<bool, String> {
    let tension_id = store.resolve_id(&id)?;
    store.update_position(tension_id, position)
}

// ─── Store Path Discovery ───────────────────────────────────────────

fn discover_store_path() -> std::path::PathBuf {
    // WERK_STORE env var takes priority
    if let Ok(p) = std::env::var("WERK_STORE") {
        let path = std::path::PathBuf::from(p);
        if let Some(parent) = path.parent() {
            return parent.to_path_buf();
        }
    }

    // Walk up from CWD looking for .werk/
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd.as_path();
        loop {
            if dir.join(".werk").is_dir() {
                return dir.to_path_buf();
            }
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
        }
    }

    // Fall back to home directory
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}

// ─── Main ───────────────────────────────────────────────────────────

fn main() {
    let store_path = discover_store_path();

    let store_handle = StoreHandle::spawn(store_path)
        .expect("failed to initialize store");

    tauri::Builder::default()
        .manage(store_handle)
        .invoke_handler(tauri::generate_handler![
            get_tree,
            create_tension,
            update_reality,
            update_desired,
            resolve_tension,
            reopen_tension,
            get_tension,
            position_tension,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
