//! fsqlite-backed Store for tensions and mutations.
//!
//! The Store provides persistence for tensions and their mutation history.
//! It uses fsqlite (FrankenSQLite) for storage, supporting both file-based
//! and in-memory databases.
//!
//! # Directory Discovery
//!
//! `Store::open()` walks up from the current working directory looking for
//! a `.werk/` directory containing `sd.db`. If not found, it falls back to
//! `~/.werk/sd.db`.
//!
//! # Schema
//!
//! ```sql
//! CREATE TABLE tensions (
//!     id TEXT PRIMARY KEY,
//!     desired TEXT NOT NULL,
//!     actual TEXT NOT NULL,
//!     parent_id TEXT,
//!     created_at TEXT NOT NULL,
//!     status TEXT NOT NULL,
//!     horizon TEXT,
//!     position INTEGER,
//!     parent_desired_snapshot TEXT,
//!     parent_actual_snapshot TEXT
//! );
//!
//! CREATE TABLE mutations (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     tension_id TEXT NOT NULL,
//!     timestamp TEXT NOT NULL,
//!     field TEXT NOT NULL,
//!     old_value TEXT,
//!     new_value TEXT
//! );
//! ```

use chrono::{DateTime, Utc};
use fsqlite::Connection;
use fsqlite_types::value::SqliteValue;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::events::{Event, EventBuilder, EventBus};
use crate::horizon::Horizon;
use crate::mutation::Mutation;
use crate::tension::{SdError, Tension, TensionStatus};

/// Errors specific to store operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StoreError {
    /// Failed to open or create the database.
    #[error("database error: {0}")]
    DatabaseError(String),

    /// Failed to discover .werk/ directory.
    #[error("failed to discover .werk directory")]
    DiscoveryError,

    /// Tension not found.
    #[error("tension not found: {0}")]
    TensionNotFound(String),

    /// Permission denied.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Disk full or I/O error.
    #[error("I/O error: {0}")]
    IoError(String),

    /// Transaction failed and was rolled back.
    #[error("transaction rolled back: {0}")]
    TransactionRolledBack(String),
}

/// Convert StoreError to SdError for use in operations that return SdError.
impl From<StoreError> for SdError {
    fn from(e: StoreError) -> Self {
        SdError::ValidationError(e.to_string())
    }
}

/// The persistent store for tensions and mutations.
///
/// Uses fsqlite for storage. Note: fsqlite's Connection uses Rc internally,
/// so Store cannot be sent between threads. For concurrent access, open
/// multiple Store instances to the same file-based database.
///
/// # Events
///
/// The store can optionally emit events to an attached EventBus.
/// Use `set_event_bus()` to attach a bus, then all successful operations
/// will emit corresponding events.
pub struct Store {
    conn: Rc<RefCell<Connection>>,
    path: Option<PathBuf>,
    event_bus: Option<EventBus>,
}

impl Store {
    /// Initialize a new store at the given path.
    ///
    /// Creates `.werk/sd.db` with the correct schema. Idempotent —
    /// opening an existing database preserves data.
    pub fn init(path: &std::path::Path) -> Result<Self, StoreError> {
        let werk_dir = path.join(".werk");
        std::fs::create_dir_all(&werk_dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                StoreError::PermissionDenied(format!("{}", werk_dir.display()))
            } else {
                StoreError::IoError(format!("failed to create .werk directory: {}", e))
            }
        })?;

        let db_path = werk_dir.join("sd.db");
        let db_path_str = db_path.to_string_lossy().into_owned();
        let conn = Connection::open(db_path_str)
            .map_err(|e| StoreError::DatabaseError(format!("failed to open database: {:?}", e)))?;

        let store = Self {
            conn: Rc::new(RefCell::new(conn)),
            path: Some(db_path),
            event_bus: None,
        };
        store.create_schema()?;
        Ok(store)
    }

    /// Open an existing store, discovering .werk/ by walking up from CWD.
    ///
    /// Falls back to ~/.werk/sd.db if no local .werk/ found.
    pub fn open() -> Result<Self, StoreError> {
        let path = Self::discover_werk_dir()?;
        Self::init(&path)
    }

    /// Create an in-memory store for testing.
    ///
    /// Each in-memory store is isolated from others.
    pub fn new_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open(":memory:").map_err(|e| {
            StoreError::DatabaseError(format!("failed to create in-memory db: {:?}", e))
        })?;
        let store = Self {
            conn: Rc::new(RefCell::new(conn)),
            path: None,
            event_bus: None,
        };
        store.create_schema()?;
        Ok(store)
    }

    fn discover_werk_dir() -> Result<PathBuf, StoreError> {
        let cwd = std::env::current_dir()
            .map_err(|e| StoreError::IoError(format!("failed to get CWD: {}", e)))?;

        let mut current = cwd.as_path();
        loop {
            let werk_dir = current.join(".werk");
            if werk_dir.exists() {
                return Ok(current.to_path_buf());
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        // Fall back to ~/.werk/
        let home = dirs::home_dir().ok_or(StoreError::DiscoveryError)?;
        Ok(home.join(".werk"))
    }

    fn create_schema(&self) -> Result<(), StoreError> {
        let conn = self.conn.borrow();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tensions (
                id TEXT PRIMARY KEY,
                desired TEXT NOT NULL,
                actual TEXT NOT NULL,
                parent_id TEXT,
                created_at TEXT NOT NULL,
                status TEXT NOT NULL,
                horizon TEXT,
                position INTEGER,
                parent_desired_snapshot TEXT,
                parent_actual_snapshot TEXT
            )",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create tensions table: {:?}", e))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS mutations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tension_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                field TEXT NOT NULL,
                old_value TEXT,
                new_value TEXT
            )",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create mutations table: {:?}", e))
        })?;

        // Migration: Add horizon column to existing databases
        // Check if the column exists, and if not, add it
        let columns: Vec<fsqlite::Row> =
            conn.query("PRAGMA table_info(tensions)").map_err(|e| {
                StoreError::DatabaseError(format!("failed to query table schema: {:?}", e))
            })?;

        let has_horizon = columns.iter().any(|row| {
            // PRAGMA table_info returns: cid, name, type, notnull, dflt_value, pk
            // Column 1 is the name
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                s == "horizon"
            } else {
                false
            }
        });

        if !has_horizon {
            conn.execute("ALTER TABLE tensions ADD COLUMN horizon TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add horizon column: {:?}", e))
                })?;
        }

        let has_position = columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                s == "position"
            } else {
                false
            }
        });

        if !has_position {
            conn.execute("ALTER TABLE tensions ADD COLUMN position INTEGER")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add position column: {:?}", e))
                })?;
        }

        let has_parent_desired_snapshot = columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                s == "parent_desired_snapshot"
            } else {
                false
            }
        });

        if !has_parent_desired_snapshot {
            conn.execute("ALTER TABLE tensions ADD COLUMN parent_desired_snapshot TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add parent_desired_snapshot column: {:?}", e))
                })?;
            conn.execute("ALTER TABLE tensions ADD COLUMN parent_actual_snapshot TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add parent_actual_snapshot column: {:?}", e))
                })?;
        }

        Ok(())
    }

    /// Create a new tension and persist it.
    ///
    /// Generates a ULID id, persists the tension, and records a "created" mutation.
    /// The horizon defaults to None.
    pub fn create_tension(&self, desired: &str, actual: &str) -> Result<Tension, SdError> {
        self.create_tension_with_parent(desired, actual, None)
    }

    /// Create a new tension with a parent reference.
    ///
    /// The horizon defaults to None.
    pub fn create_tension_with_parent(
        &self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
    ) -> Result<Tension, SdError> {
        self.create_tension_full(desired, actual, parent_id, None)
    }

    /// Create a new tension with all optional fields including horizon.
    ///
    /// Generates a ULID id, persists the tension, and records a "created" mutation.
    /// The creation mutation includes horizon if present.
    pub fn create_tension_full(
        &self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> Result<Tension, SdError> {
        let tension = Tension::new_full(desired, actual, parent_id, horizon)?;
        self.persist_tension(&tension)?;

        // Build creation mutation value with optional horizon
        let creation_value = match &tension.horizon {
            Some(h) => format!(
                "desired='{}';actual='{}';horizon='{}'",
                tension.desired, tension.actual, h
            ),
            None => format!("desired='{}';actual='{}'", tension.desired, tension.actual),
        };

        self.record_mutation(&Mutation::new(
            tension.id.clone(),
            tension.created_at,
            "created".to_owned(),
            None,
            creation_value,
        ))?;

        // Emit TensionCreated event
        self.emit_event(&EventBuilder::tension_created(
            tension.id.clone(),
            tension.desired.clone(),
            tension.actual.clone(),
            tension.parent_id.clone(),
            tension.horizon.as_ref().map(|h| h.to_string()),
        ));

        Ok(tension)
    }

    /// Create a new tension with all fields including parent snapshots and position.
    ///
    /// Used when creating child tensions that need to capture parent state.
    pub fn create_tension_full_with_snapshots(
        &self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
        position: Option<i32>,
        parent_desired_snapshot: Option<String>,
        parent_actual_snapshot: Option<String>,
    ) -> Result<Tension, SdError> {
        let tension = Tension::new_full_with_snapshots(
            desired,
            actual,
            parent_id,
            horizon,
            position,
            parent_desired_snapshot,
            parent_actual_snapshot,
        )?;
        self.persist_tension(&tension)?;

        // Build creation mutation value with optional horizon
        let creation_value = match &tension.horizon {
            Some(h) => format!(
                "desired='{}';actual='{}';horizon='{}'",
                tension.desired, tension.actual, h
            ),
            None => format!("desired='{}';actual='{}'", tension.desired, tension.actual),
        };

        self.record_mutation(&Mutation::new(
            tension.id.clone(),
            tension.created_at,
            "created".to_owned(),
            None,
            creation_value,
        ))?;

        // Emit TensionCreated event
        self.emit_event(&EventBuilder::tension_created(
            tension.id.clone(),
            tension.desired.clone(),
            tension.actual.clone(),
            tension.parent_id.clone(),
            tension.horizon.as_ref().map(|h| h.to_string()),
        ));

        Ok(tension)
    }

    fn persist_tension(&self, tension: &Tension) -> Result<(), SdError> {
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO tensions (id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            &[
                SqliteValue::Text(tension.id.clone()),
                SqliteValue::Text(tension.desired.clone()),
                SqliteValue::Text(tension.actual.clone()),
                match &tension.parent_id {
                    Some(pid) => SqliteValue::Text(pid.clone()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(tension.created_at.to_rfc3339()),
                SqliteValue::Text(tension.status.to_string()),
                match &tension.horizon {
                    Some(h) => SqliteValue::Text(h.to_string()),
                    None => SqliteValue::Null,
                },
                match tension.position {
                    Some(p) => SqliteValue::Integer(p as i64),
                    None => SqliteValue::Null,
                },
                match &tension.parent_desired_snapshot {
                    Some(s) => SqliteValue::Text(s.clone()),
                    None => SqliteValue::Null,
                },
                match &tension.parent_actual_snapshot {
                    Some(s) => SqliteValue::Text(s.clone()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| SdError::ValidationError(format!("failed to persist tension: {:?}", e)))?;
        Ok(())
    }

    /// Record a mutation for a tension.
    ///
    /// This is a low-level method for recording arbitrary mutations.
    /// Most operations automatically record appropriate mutations.
    pub fn record_mutation(&self, mutation: &Mutation) -> Result<(), SdError> {
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO mutations (tension_id, timestamp, field, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                SqliteValue::Text(mutation.tension_id().to_owned()),
                SqliteValue::Text(mutation.timestamp().to_rfc3339()),
                SqliteValue::Text(mutation.field().to_owned()),
                match mutation.old_value() {
                    Some(v) => SqliteValue::Text(v.to_owned()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(mutation.new_value().to_owned()),
            ],
        )
        .map_err(|e| SdError::ValidationError(format!("failed to record mutation: {:?}", e)))?;
        Ok(())
    }

    /// Get a tension by ID.
    ///
    /// Returns None if the tension doesn't exist.
    pub fn get_tension(&self, id: &str) -> Result<Option<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot FROM tensions WHERE id = ?1",
                &[SqliteValue::Text(id.to_owned())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        let id = match row.get(0) {
            Some(SqliteValue::Text(s)) => s.clone(),
            _ => return Err(StoreError::DatabaseError("invalid id column".to_owned())),
        };
        let desired = match row.get(1) {
            Some(SqliteValue::Text(s)) => s.clone(),
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid desired column".to_owned(),
                ));
            }
        };
        let actual = match row.get(2) {
            Some(SqliteValue::Text(s)) => s.clone(),
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid actual column".to_owned(),
                ));
            }
        };
        let parent_id = match row.get(3) {
            Some(SqliteValue::Text(s)) => Some(s.clone()),
            Some(SqliteValue::Null) | None => None,
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid parent_id column".to_owned(),
                ));
            }
        };
        let created_at_str = match row.get(4) {
            Some(SqliteValue::Text(s)) => s.clone(),
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid created_at column".to_owned(),
                ));
            }
        };
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| StoreError::DatabaseError(format!("invalid created_at: {}", e)))?;

        let status_str = match row.get(5) {
            Some(SqliteValue::Text(s)) => s.clone(),
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid status column".to_owned(),
                ));
            }
        };
        let status = match status_str.as_str() {
            "Active" => TensionStatus::Active,
            "Resolved" => TensionStatus::Resolved,
            "Released" => TensionStatus::Released,
            _ => {
                return Err(StoreError::DatabaseError(format!(
                    "invalid status: {}",
                    status_str
                )));
            }
        };

        // Parse horizon column
        let horizon = match row.get(6) {
            Some(SqliteValue::Text(s)) if !s.is_empty() => Some(
                Horizon::parse(s)
                    .map_err(|e| StoreError::DatabaseError(format!("invalid horizon: {}", e)))?,
            ),
            Some(SqliteValue::Text(_)) | Some(SqliteValue::Null) | None => None,
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid horizon column".to_owned(),
                ));
            }
        };

        // Parse position column (column 7)
        let position = match row.get(7) {
            Some(SqliteValue::Integer(n)) => Some(*n as i32),
            Some(SqliteValue::Null) | None => None,
            _ => None,
        };

        // Parse parent_desired_snapshot (column 8)
        let parent_desired_snapshot = match row.get(8) {
            Some(SqliteValue::Text(s)) => Some(s.clone()),
            Some(SqliteValue::Null) | None => None,
            _ => None,
        };

        // Parse parent_actual_snapshot (column 9)
        let parent_actual_snapshot = match row.get(9) {
            Some(SqliteValue::Text(s)) => Some(s.clone()),
            Some(SqliteValue::Null) | None => None,
            _ => None,
        };

        Ok(Some(Tension {
            id,
            desired,
            actual,
            parent_id,
            created_at,
            status,
            horizon,
            position,
            parent_desired_snapshot,
            parent_actual_snapshot,
        }))
    }

    /// List all tensions in creation order.
    pub fn list_tensions(&self) -> Result<Vec<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot FROM tensions ORDER BY created_at ASC")
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_tension_rows(rows)
    }

    /// Get all root tensions (those with no parent_id).
    pub fn get_roots(&self) -> Result<Vec<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot FROM tensions WHERE parent_id IS NULL ORDER BY position ASC NULLS LAST, created_at ASC")
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_tension_rows(rows)
    }

    /// Get all children of a given parent.
    pub fn get_children(&self, parent_id: &str) -> Result<Vec<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot FROM tensions WHERE parent_id = ?1 ORDER BY position ASC NULLS LAST, created_at ASC",
                &[SqliteValue::Text(parent_id.to_owned())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_tension_rows(rows)
    }

    fn parse_tension_rows(&self, rows: Vec<fsqlite::Row>) -> Result<Vec<Tension>, StoreError> {
        let mut tensions = Vec::new();
        for row in &rows {
            let id = match row.get(0) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => return Err(StoreError::DatabaseError("invalid id column".to_owned())),
            };
            let desired = match row.get(1) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid desired column".to_owned(),
                    ));
                }
            };
            let actual = match row.get(2) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid actual column".to_owned(),
                    ));
                }
            };
            let parent_id = match row.get(3) {
                Some(SqliteValue::Text(s)) => Some(s.clone()),
                Some(SqliteValue::Null) | None => None,
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid parent_id column".to_owned(),
                    ));
                }
            };
            let created_at_str = match row.get(4) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid created_at column".to_owned(),
                    ));
                }
            };
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| StoreError::DatabaseError(format!("invalid created_at: {}", e)))?;

            let status_str = match row.get(5) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid status column".to_owned(),
                    ));
                }
            };
            let status = match status_str.as_str() {
                "Active" => TensionStatus::Active,
                "Resolved" => TensionStatus::Resolved,
                "Released" => TensionStatus::Released,
                _ => {
                    return Err(StoreError::DatabaseError(format!(
                        "invalid status: {}",
                        status_str
                    )));
                }
            };

            // Parse horizon column (column 6)
            let horizon = match row.get(6) {
                Some(SqliteValue::Text(s)) if !s.is_empty() => {
                    Some(Horizon::parse(s).map_err(|e| {
                        StoreError::DatabaseError(format!("invalid horizon: {}", e))
                    })?)
                }
                Some(SqliteValue::Text(_)) | Some(SqliteValue::Null) | None => None,
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid horizon column".to_owned(),
                    ));
                }
            };

            // Parse position column (column 7)
            let position = match row.get(7) {
                Some(SqliteValue::Integer(n)) => Some(*n as i32),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            // Parse parent_desired_snapshot (column 8)
            let parent_desired_snapshot = match row.get(8) {
                Some(SqliteValue::Text(s)) => Some(s.clone()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            // Parse parent_actual_snapshot (column 9)
            let parent_actual_snapshot = match row.get(9) {
                Some(SqliteValue::Text(s)) => Some(s.clone()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            tensions.push(Tension {
                id,
                desired,
                actual,
                parent_id,
                created_at,
                status,
                horizon,
                position,
                parent_desired_snapshot,
                parent_actual_snapshot,
            });
        }

        Ok(tensions)
    }

    /// Update the desired state of a tension.
    ///
    /// Persists the change and records a mutation.
    pub fn update_desired(&self, id: &str, new_desired: &str) -> Result<(), SdError> {
        self.update_field(id, "desired", new_desired)
    }

    /// Update the actual state of a tension.
    ///
    /// Persists the change and records a mutation.
    pub fn update_actual(&self, id: &str, new_actual: &str) -> Result<(), SdError> {
        self.update_field(id, "actual", new_actual)
    }

    /// Update the actual state of a tension without starting a transaction.
    ///
    /// For use within an already-active transaction. Call `begin_transaction()`
    /// before using this method, and `commit_transaction()` after all updates.
    pub fn update_actual_no_tx(&self, id: &str, new_actual: &str) -> Result<(), SdError> {
        if new_actual.is_empty() {
            return Err(SdError::ValidationError(
                "actual cannot be empty".to_owned(),
            ));
        }

        let mut tension = self
            .get_tension(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?
            .ok_or_else(|| SdError::ValidationError(format!("tension not found: {}", id)))?;

        if tension.status != TensionStatus::Active {
            return Err(SdError::UpdateOnInactiveTension(tension.status));
        }

        let old_value = tension.update_actual(new_actual)?;

        let conn = self.conn.borrow();
        self.update_tension_in_transaction(&conn, &tension)?;
        self.record_mutation_in_transaction(
            &conn,
            &Mutation::new(
                tension.id.clone(),
                Utc::now(),
                "actual".to_owned(),
                Some(old_value),
                new_actual.to_owned(),
            ),
        )?;

        Ok(())
    }

    /// Update the parent_id of a tension.
    ///
    /// Persists the change and records a mutation.
    pub fn update_parent(&self, id: &str, new_parent_id: Option<&str>) -> Result<(), SdError> {
        let mut tension = self
            .get_tension(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?
            .ok_or_else(|| SdError::ValidationError(format!("tension not found: {}", id)))?;

        let old_parent = tension.parent_id.clone();
        let new_parent = new_parent_id.map(|s| s.to_owned());
        tension.parent_id = new_parent.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN;").map_err(|e| {
                SdError::ValidationError(format!("failed to begin transaction: {:?}", e))
            })?;

            let result = self
                .update_tension_in_transaction(&conn, &tension)
                .and_then(|_| {
                    self.record_mutation_in_transaction(
                        &conn,
                        &Mutation::new(
                            tension.id.clone(),
                            Utc::now(),
                            "parent_id".to_owned(),
                            old_parent.clone(),
                            new_parent.clone().unwrap_or_default(),
                        ),
                    )
                });

            match result {
                Ok(_) => {
                    conn.execute("COMMIT;").map_err(|e| {
                        SdError::ValidationError(format!("failed to commit: {:?}", e))
                    })?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Emit StructureChanged event
        self.emit_event(&EventBuilder::structure_changed(
            tension.id, old_parent, new_parent,
        ));

        Ok(())
    }

    /// Update the temporal horizon of a tension.
    ///
    /// Validates that the tension is Active, persists the change, records a mutation,
    /// and emits a HorizonChanged event.
    ///
    /// Returns an error if:
    /// - The tension doesn't exist
    /// - The tension is not Active (Resolved or Released)
    ///
    /// The new_horizon can be None to clear the horizon.
    pub fn update_horizon(&self, id: &str, new_horizon: Option<Horizon>) -> Result<(), SdError> {
        let mut tension = self
            .get_tension(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?
            .ok_or_else(|| SdError::ValidationError(format!("tension not found: {}", id)))?;

        // Validate that the tension is Active
        if tension.status != TensionStatus::Active {
            return Err(SdError::UpdateOnInactiveTension(tension.status));
        }

        let old_horizon = tension.horizon.clone();
        tension.horizon = new_horizon.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN;").map_err(|e| {
                SdError::ValidationError(format!("failed to begin transaction: {:?}", e))
            })?;

            let result = self
                .update_tension_in_transaction(&conn, &tension)
                .and_then(|_| {
                    self.record_mutation_in_transaction(
                        &conn,
                        &Mutation::new(
                            tension.id.clone(),
                            Utc::now(),
                            "horizon".to_owned(),
                            old_horizon.as_ref().map(|h| h.to_string()),
                            new_horizon
                                .as_ref()
                                .map(|h| h.to_string())
                                .unwrap_or_default(),
                        ),
                    )
                });

            match result {
                Ok(_) => {
                    conn.execute("COMMIT;").map_err(|e| {
                        SdError::ValidationError(format!("failed to commit: {:?}", e))
                    })?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Emit HorizonChanged event
        self.emit_event(&EventBuilder::horizon_changed(
            tension.id,
            old_horizon.as_ref().map(|h| h.to_string()),
            new_horizon.as_ref().map(|h| h.to_string()),
        ));

        Ok(())
    }

    /// Update the status of a tension.
    ///
    /// Persists the change and records a mutation.
    ///
    /// When a tension is resolved or released and has children, all children
    /// are atomically reparented to null (becoming roots) and a parent_id
    /// mutation is recorded for each child.
    pub fn update_status(&self, id: &str, new_status: TensionStatus) -> Result<(), SdError> {
        let mut tension = self
            .get_tension(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?
            .ok_or_else(|| SdError::ValidationError(format!("tension not found: {}", id)))?;

        let old_status = tension.status;
        if old_status == new_status {
            return Ok(()); // No change needed
        }

        // Validate transition
        match (&old_status, &new_status) {
            (TensionStatus::Active, TensionStatus::Resolved) => {}
            (TensionStatus::Active, TensionStatus::Released) => {}
            (TensionStatus::Resolved, TensionStatus::Active) => {}
            (TensionStatus::Released, TensionStatus::Active) => {}
            _ => {
                return Err(SdError::InvalidStatusTransition {
                    from: old_status,
                    to: new_status,
                });
            }
        }

        tension.status = new_status;

        // Check if this tension has children that need reparenting
        let children = self
            .get_children(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?;
        let needs_reparent = !children.is_empty()
            && (new_status == TensionStatus::Resolved || new_status == TensionStatus::Released);

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN;").map_err(|e| {
                SdError::ValidationError(format!("failed to begin transaction: {:?}", e))
            })?;

            // Update the tension status
            let result = self
                .update_tension_in_transaction(&conn, &tension)
                .and_then(|_| {
                    self.record_mutation_in_transaction(
                        &conn,
                        &Mutation::new(
                            tension.id.clone(),
                            Utc::now(),
                            "status".to_owned(),
                            Some(old_status.to_string()),
                            new_status.to_string(),
                        ),
                    )
                })
                .and_then(|_| {
                    // If resolving/releasing with children, reparent all children to null
                    if needs_reparent {
                        let now = Utc::now();
                        for child in &children {
                            // Update child's parent_id to null
                            conn.execute_with_params(
                                "UPDATE tensions SET parent_id = NULL WHERE id = ?1",
                                &[SqliteValue::Text(child.id.clone())],
                            )
                            .map_err(|e| {
                                SdError::ValidationError(format!(
                                    "failed to reparent child: {:?}",
                                    e
                                ))
                            })?;

                            // Record parent_id mutation for the child
                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new(
                                    child.id.clone(),
                                    now,
                                    "parent_id".to_owned(),
                                    child.parent_id.clone(),
                                    String::new(), // Empty string represents null
                                ),
                            )?;
                        }
                    }
                    Ok(())
                });

            match result {
                Ok(_) => {
                    conn.execute("COMMIT;").map_err(|e| {
                        SdError::ValidationError(format!("failed to commit: {:?}", e))
                    })?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Emit appropriate event based on new status
        match new_status {
            TensionStatus::Resolved => {
                self.emit_event(&EventBuilder::tension_resolved(
                    tension.id,
                    tension.desired,
                    tension.actual,
                ));
            }
            TensionStatus::Released => {
                self.emit_event(&EventBuilder::tension_released(
                    tension.id,
                    tension.desired,
                    tension.actual,
                ));
            }
            TensionStatus::Active => {}
        }

        Ok(())
    }

    fn update_field(&self, id: &str, field: &str, new_value: &str) -> Result<(), SdError> {
        if new_value.is_empty() {
            return Err(SdError::ValidationError(format!(
                "{} cannot be empty",
                field
            )));
        }

        let mut tension = self
            .get_tension(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?
            .ok_or_else(|| SdError::ValidationError(format!("tension not found: {}", id)))?;

        if tension.status != TensionStatus::Active {
            return Err(SdError::UpdateOnInactiveTension(tension.status));
        }

        let old_value = match field {
            "desired" => tension.update_desired(new_value)?,
            "actual" => tension.update_actual(new_value)?,
            _ => {
                return Err(SdError::ValidationError(format!(
                    "unknown field: {}",
                    field
                )));
            }
        };
        let old_value_for_event = old_value.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN;").map_err(|e| {
                SdError::ValidationError(format!("failed to begin transaction: {:?}", e))
            })?;

            let result = self
                .update_tension_in_transaction(&conn, &tension)
                .and_then(|_| {
                    self.record_mutation_in_transaction(
                        &conn,
                        &Mutation::new(
                            tension.id.clone(),
                            Utc::now(),
                            field.to_owned(),
                            Some(old_value),
                            new_value.to_owned(),
                        ),
                    )
                });

            match result {
                Ok(_) => {
                    conn.execute("COMMIT;").map_err(|e| {
                        SdError::ValidationError(format!("failed to commit: {:?}", e))
                    })?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Emit appropriate event based on field
        match field {
            "desired" => {
                self.emit_event(&EventBuilder::desire_revised(
                    tension.id,
                    old_value_for_event,
                    new_value.to_owned(),
                ));
            }
            "actual" => {
                self.emit_event(&EventBuilder::reality_confronted(
                    tension.id,
                    old_value_for_event,
                    new_value.to_owned(),
                ));
            }
            _ => {}
        }

        Ok(())
    }

    fn update_tension_in_transaction(
        &self,
        conn: &Connection,
        tension: &Tension,
    ) -> Result<(), SdError> {
        conn.execute_with_params(
            "UPDATE tensions SET desired = ?1, actual = ?2, parent_id = ?3, status = ?4, horizon = ?5 WHERE id = ?6",
            &[
                SqliteValue::Text(tension.desired.clone()),
                SqliteValue::Text(tension.actual.clone()),
                match &tension.parent_id {
                    Some(pid) => SqliteValue::Text(pid.clone()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(tension.status.to_string()),
                match &tension.horizon {
                    Some(h) => SqliteValue::Text(h.to_string()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(tension.id.clone()),
            ],
        )
        .map_err(|e| SdError::ValidationError(format!("failed to update tension: {:?}", e)))?;
        Ok(())
    }

    fn record_mutation_in_transaction(
        &self,
        conn: &Connection,
        mutation: &Mutation,
    ) -> Result<(), SdError> {
        conn.execute_with_params(
            "INSERT INTO mutations (tension_id, timestamp, field, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                SqliteValue::Text(mutation.tension_id().to_owned()),
                SqliteValue::Text(mutation.timestamp().to_rfc3339()),
                SqliteValue::Text(mutation.field().to_owned()),
                match mutation.old_value() {
                    Some(v) => SqliteValue::Text(v.to_owned()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(mutation.new_value().to_owned()),
            ],
        )
        .map_err(|e| SdError::ValidationError(format!("failed to record mutation: {:?}", e)))?;
        Ok(())
    }

    /// Get all mutations for a tension in chronological order.
    pub fn get_mutations(&self, tension_id: &str) -> Result<Vec<Mutation>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT tension_id, timestamp, field, old_value, new_value FROM mutations WHERE tension_id = ?1 ORDER BY timestamp ASC",
                &[SqliteValue::Text(tension_id.to_owned())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_mutation_rows(rows)
    }

    /// Get all mutations across all tensions in chronological order.
    pub fn all_mutations(&self) -> Result<Vec<Mutation>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT tension_id, timestamp, field, old_value, new_value FROM mutations ORDER BY timestamp ASC")
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_mutation_rows(rows)
    }

    /// Get all mutations within a time range, in chronological order.
    ///
    /// The time range is inclusive on both ends: `[start, end]`.
    pub fn mutations_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Mutation>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT tension_id, timestamp, field, old_value, new_value FROM mutations WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp ASC",
                &[
                    SqliteValue::Text(start.to_rfc3339()),
                    SqliteValue::Text(end.to_rfc3339()),
                ],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_mutation_rows(rows)
    }

    fn parse_mutation_rows(&self, rows: Vec<fsqlite::Row>) -> Result<Vec<Mutation>, StoreError> {
        let mut mutations = Vec::new();
        for row in &rows {
            let tension_id = match row.get(0) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid tension_id column".to_owned(),
                    ));
                }
            };
            let timestamp_str = match row.get(1) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid timestamp column".to_owned(),
                    ));
                }
            };
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| StoreError::DatabaseError(format!("invalid timestamp: {}", e)))?;

            let field = match row.get(2) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => return Err(StoreError::DatabaseError("invalid field column".to_owned())),
            };
            let old_value = match row.get(3) {
                Some(SqliteValue::Text(s)) => Some(s.clone()),
                Some(SqliteValue::Null) | None => None,
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid old_value column".to_owned(),
                    ));
                }
            };
            let new_value = match row.get(4) {
                Some(SqliteValue::Text(s)) => s.clone(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid new_value column".to_owned(),
                    ));
                }
            };

            mutations.push(Mutation::new(
                tension_id, timestamp, field, old_value, new_value,
            ));
        }

        Ok(mutations)
    }

    /// Get the database path (None for in-memory stores).
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    /// Set the EventBus for this store.
    ///
    /// After setting, all successful operations will emit events.
    pub fn set_event_bus(&mut self, bus: EventBus) {
        self.event_bus = Some(bus);
    }

    /// Get the EventBus for this store, if any.
    pub fn event_bus(&self) -> Option<&EventBus> {
        self.event_bus.as_ref()
    }

    /// Remove the EventBus from this store.
    pub fn clear_event_bus(&mut self) {
        self.event_bus = None;
    }

    /// Emit an event if an EventBus is attached.
    fn emit_event(&self, event: &Event) {
        if let Some(bus) = &self.event_bus {
            bus.emit(event);
        }
    }

    /// Begin a transaction explicitly.
    ///
    /// Use this for batch operations to improve performance.
    /// Must be paired with a call to `commit_transaction()` or `rollback_transaction()`.
    pub fn begin_transaction(&self) -> Result<(), StoreError> {
        let conn = self.conn.borrow();
        conn.execute("BEGIN;")
            .map(|_| ())
            .map_err(|e| StoreError::DatabaseError(format!("failed to begin transaction: {:?}", e)))
    }

    /// Commit the current transaction.
    ///
    /// Panics if no transaction is active.
    pub fn commit_transaction(&self) -> Result<(), StoreError> {
        let conn = self.conn.borrow();
        conn.execute("COMMIT;").map(|_| ()).map_err(|e| {
            StoreError::DatabaseError(format!("failed to commit transaction: {:?}", e))
        })
    }

    /// Rollback the current transaction.
    ///
    /// Panics if no transaction is active.
    pub fn rollback_transaction(&self) -> Result<(), StoreError> {
        let conn = self.conn.borrow();
        conn.execute("ROLLBACK;").map(|_| ()).map_err(|e| {
            StoreError::DatabaseError(format!("failed to rollback transaction: {:?}", e))
        })
    }

    /// Delete a tension and reparent its children to the grandparent.
    ///
    /// When a tension is deleted:
    /// - All its children are reparented to the deleted tension's parent (grandparent adoption)
    /// - If the deleted tension is a root (parent_id = null), children become roots
    /// - The tension is removed from the database
    /// - A "deleted" mutation is recorded for the deleted tension
    /// - A parent_id mutation is recorded for each child that was reparented
    ///
    /// Returns an error if the tension doesn't exist.
    pub fn delete_tension(&self, id: &str) -> Result<(), SdError> {
        // Get the tension to delete
        let tension = self
            .get_tension(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?
            .ok_or_else(|| SdError::ValidationError(format!("tension not found: {}", id)))?;

        // Get all children of this tension
        let children = self
            .get_children(id)
            .map_err(|e| SdError::ValidationError(e.to_string()))?;

        // The grandparent is the deleted tension's parent_id
        let grandparent_id = tension.parent_id.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN;").map_err(|e| {
                SdError::ValidationError(format!("failed to begin transaction: {:?}", e))
            })?;

            let now = Utc::now();

            // Reparent all children to grandparent
            let result = (|| {
                for child in &children {
                    // Update child's parent_id to grandparent
                    conn.execute_with_params(
                        "UPDATE tensions SET parent_id = ?1 WHERE id = ?2",
                        &[
                            match &grandparent_id {
                                Some(gp) => SqliteValue::Text(gp.clone()),
                                None => SqliteValue::Null,
                            },
                            SqliteValue::Text(child.id.clone()),
                        ],
                    )
                    .map_err(|e| {
                        SdError::ValidationError(format!("failed to reparent child: {:?}", e))
                    })?;

                    // Record parent_id mutation for the child
                    self.record_mutation_in_transaction(
                        &conn,
                        &Mutation::new(
                            child.id.clone(),
                            now,
                            "parent_id".to_owned(),
                            child.parent_id.clone(),
                            grandparent_id.clone().unwrap_or_default(),
                        ),
                    )?;
                }

                // Delete the tension
                conn.execute_with_params(
                    "DELETE FROM tensions WHERE id = ?1",
                    &[SqliteValue::Text(tension.id.clone())],
                )
                .map_err(|e| {
                    SdError::ValidationError(format!("failed to delete tension: {:?}", e))
                })?;

                // Record deletion mutation for the deleted tension
                // (We record this even though the tension is deleted, for audit trail)
                self.record_mutation_in_transaction(
                    &conn,
                    &Mutation::new(
                        tension.id.clone(),
                        now,
                        "deleted".to_owned(),
                        Some(format!(
                            "desired='{}';actual='{}'",
                            tension.desired, tension.actual
                        )),
                        String::new(),
                    ),
                )?;

                Ok(())
            })();

            match result {
                Ok(_) => {
                    conn.execute("COMMIT;").map_err(|e| {
                        SdError::ValidationError(format!("failed to commit: {:?}", e))
                    })?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Emit TensionDeleted event
        self.emit_event(&EventBuilder::tension_deleted(
            tension.id,
            tension.desired,
            tension.actual,
        ));

        Ok(())
    }

    /// Update the position of a tension for sibling ordering.
    ///
    /// Records a mutation and persists the change.
    pub fn update_position(&self, id: &str, new_position: Option<i32>) -> Result<(), SdError> {
        let conn = self.conn.borrow();

        // Get existing tension
        let rows = conn
            .query_with_params(
                "SELECT position FROM tensions WHERE id = ?1",
                &[SqliteValue::Text(id.to_owned())],
            )
            .map_err(|e| SdError::ValidationError(format!("query failed: {:?}", e)))?;

        if rows.is_empty() {
            return Err(SdError::ValidationError(format!("tension not found: {}", id)));
        }

        let old_position = match rows[0].get(0) {
            Some(SqliteValue::Integer(n)) => Some(*n as i32),
            _ => None,
        };

        // Update in database
        conn.execute_with_params(
            "UPDATE tensions SET position = ?1 WHERE id = ?2",
            &[
                match new_position {
                    Some(p) => SqliteValue::Integer(p as i64),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(id.to_owned()),
            ],
        )
        .map_err(|e| SdError::ValidationError(format!("failed to update position: {:?}", e)))?;

        // Record mutation
        self.record_mutation(&crate::mutation::Mutation::new(
            id.to_owned(),
            Utc::now(),
            "position".to_owned(),
            old_position.map(|p| p.to_string()),
            new_position.map(|p| p.to_string()).unwrap_or_else(|| "null".to_string()),
        ))?;

        Ok(())
    }

    /// Reorder siblings by assigning positions to all children of a parent.
    ///
    /// Takes a list of tension IDs in the desired order. Assigns sequential
    /// positions starting from 1. IDs not in the list get position = NULL.
    pub fn reorder_siblings(&self, _parent_id: Option<&str>, ordered_ids: &[String]) -> Result<(), SdError> {
        for (i, id) in ordered_ids.iter().enumerate() {
            let position = (i + 1) as i32;
            let conn = self.conn.borrow();
            conn.execute_with_params(
                "UPDATE tensions SET position = ?1 WHERE id = ?2",
                &[
                    SqliteValue::Integer(position as i64),
                    SqliteValue::Text(id.clone()),
                ],
            )
            .map_err(|e| SdError::ValidationError(format!("failed to reorder: {:?}", e)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    // ── Construction ──────────────────────────────────────────────

    #[test]
    fn test_store_new_in_memory() {
        let store = Store::new_in_memory().unwrap();
        assert!(store.path().is_none());
    }

    #[test]
    fn test_store_new_in_memory_isolated() {
        let store1 = Store::new_in_memory().unwrap();
        let store2 = Store::new_in_memory().unwrap();

        let t1 = store1.create_tension("goal1", "reality1").unwrap();
        let t2 = store2.create_tension("goal2", "reality2").unwrap();

        // Each store has its own data
        assert!(store1.get_tension(&t2.id).unwrap().is_none());
        assert!(store2.get_tension(&t1.id).unwrap().is_none());
    }

    // ── Tension CRUD ──────────────────────────────────────────────

    #[test]
    fn test_create_tension() {
        let store = Store::new_in_memory().unwrap();
        let t = store
            .create_tension("write a novel", "have an outline")
            .unwrap();

        assert!(!t.id.is_empty());
        assert_eq!(t.desired, "write a novel");
        assert_eq!(t.actual, "have an outline");
        assert_eq!(t.status, TensionStatus::Active);
        assert!(t.parent_id.is_none());
    }

    #[test]
    fn test_create_tension_with_parent() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let child = store
            .create_tension_with_parent("child goal", "child reality", Some(parent.id.clone()))
            .unwrap();

        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_create_tension_records_mutation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].field(), "created");
        assert!(mutations[0].old_value().is_none());
    }

    #[test]
    fn test_create_tension_empty_desired_fails() {
        let store = Store::new_in_memory().unwrap();
        let result = store.create_tension("", "reality");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_tension_empty_actual_fails() {
        let store = Store::new_in_memory().unwrap();
        let result = store.create_tension("goal", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_tension_existing() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let retrieved = store.get_tension(&t.id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, t.id);
        assert_eq!(retrieved.desired, t.desired);
        assert_eq!(retrieved.actual, t.actual);
    }

    #[test]
    fn test_get_tension_unknown_returns_none() {
        let store = Store::new_in_memory().unwrap();
        let result = store.get_tension("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_tensions_creation_order() {
        let store = Store::new_in_memory().unwrap();
        let t1 = store.create_tension("first", "r1").unwrap();
        let t2 = store.create_tension("second", "r2").unwrap();
        let t3 = store.create_tension("third", "r3").unwrap();

        let tensions = store.list_tensions().unwrap();
        assert_eq!(tensions.len(), 3);
        assert_eq!(tensions[0].id, t1.id);
        assert_eq!(tensions[1].id, t2.id);
        assert_eq!(tensions[2].id, t3.id);
    }

    #[test]
    fn test_list_tensions_empty() {
        let store = Store::new_in_memory().unwrap();
        let tensions = store.list_tensions().unwrap();
        assert!(tensions.is_empty());
    }

    // ── Root and Child Queries ─────────────────────────────────────

    #[test]
    fn test_get_roots() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let _child = store
            .create_tension_with_parent("child goal", "child reality", Some(parent.id.clone()))
            .unwrap();

        let roots = store.get_roots().unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, parent.id);
    }

    #[test]
    fn test_get_roots_multiple() {
        let store = Store::new_in_memory().unwrap();
        let _r1 = store.create_tension("root1", "r1").unwrap();
        let _r2 = store.create_tension("root2", "r2").unwrap();

        let roots = store.get_roots().unwrap();
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn test_get_children() {
        let store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "p").unwrap();
        let c1 = store
            .create_tension_with_parent("child1", "c1", Some(parent.id.clone()))
            .unwrap();
        let c2 = store
            .create_tension_with_parent("child2", "c2", Some(parent.id.clone()))
            .unwrap();
        let _other = store.create_tension("other", "o").unwrap();

        let children = store.get_children(&parent.id).unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|c| c.id == c1.id));
        assert!(children.iter().any(|c| c.id == c2.id));
    }

    #[test]
    fn test_get_children_empty() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let children = store.get_children(&t.id).unwrap();
        assert!(children.is_empty());
    }

    // ── Update Operations ──────────────────────────────────────────

    #[test]
    fn test_update_desired() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("old desire", "reality").unwrap();

        store.update_desired(&t.id, "new desire").unwrap();

        let updated = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(updated.desired, "new desire");
    }

    #[test]
    fn test_update_desired_records_mutation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("old desire", "reality").unwrap();

        store.update_desired(&t.id, "new desire").unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2); // created + update
        assert_eq!(mutations[1].field(), "desired");
        assert_eq!(mutations[1].old_value(), Some("old desire"));
        assert_eq!(mutations[1].new_value(), "new desire");
    }

    #[test]
    fn test_update_desired_empty_fails() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("desire", "reality").unwrap();

        let result = store.update_desired(&t.id, "");
        assert!(result.is_err());

        // Original preserved
        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.desired, "desire");
    }

    #[test]
    fn test_update_actual() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("desire", "old reality").unwrap();

        store.update_actual(&t.id, "new reality").unwrap();

        let updated = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(updated.actual, "new reality");
    }

    #[test]
    fn test_update_actual_records_mutation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("desire", "old reality").unwrap();

        store.update_actual(&t.id, "new reality").unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[1].field(), "actual");
        assert_eq!(mutations[1].old_value(), Some("old reality"));
        assert_eq!(mutations[1].new_value(), "new reality");
    }

    #[test]
    fn test_update_parent() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        let new_parent = store.create_tension("parent", "p").unwrap();

        store.update_parent(&t.id, Some(&new_parent.id)).unwrap();

        let updated = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(updated.parent_id, Some(new_parent.id));
    }

    #[test]
    fn test_update_parent_records_mutation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        let new_parent = store.create_tension("parent", "p").unwrap();

        store.update_parent(&t.id, Some(&new_parent.id)).unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[1].field(), "parent_id");
    }

    #[test]
    fn test_update_parent_to_none() {
        let store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "p").unwrap();
        let child = store
            .create_tension_with_parent("child", "c", Some(parent.id.clone()))
            .unwrap();

        store.update_parent(&child.id, None).unwrap();

        let updated = store.get_tension(&child.id).unwrap().unwrap();
        assert!(updated.parent_id.is_none());
    }

    #[test]
    fn test_update_status_resolve() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        let updated = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(updated.status, TensionStatus::Resolved);
    }

    #[test]
    fn test_update_status_release() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        store.update_status(&t.id, TensionStatus::Released).unwrap();

        let updated = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(updated.status, TensionStatus::Released);
    }

    #[test]
    fn test_update_status_records_mutation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[1].field(), "status");
        assert_eq!(mutations[1].old_value(), Some("Active"));
        assert_eq!(mutations[1].new_value(), "Resolved");
    }

    #[test]
    fn test_update_status_invalid_transition() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        let result = store.update_status(&t.id, TensionStatus::Released);
        assert!(result.is_err());

        // Status unchanged
        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.status, TensionStatus::Resolved);
    }

    #[test]
    fn test_update_on_resolved_tension_fails() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        let result = store.update_desired(&t.id, "new goal");
        assert!(result.is_err());
    }

    #[test]
    fn test_update_on_released_tension_fails() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_status(&t.id, TensionStatus::Released).unwrap();

        let result = store.update_actual(&t.id, "new reality");
        assert!(result.is_err());
    }

    // ── Mutation Queries ───────────────────────────────────────────

    #[test]
    fn test_get_mutations_empty_for_unknown() {
        let store = Store::new_in_memory().unwrap();
        let mutations = store.get_mutations("nonexistent").unwrap();
        assert!(mutations.is_empty());
    }

    #[test]
    fn test_all_mutations() {
        let store = Store::new_in_memory().unwrap();
        let t1 = store.create_tension("goal1", "reality1").unwrap();
        let t2 = store.create_tension("goal2", "reality2").unwrap();

        store.update_desired(&t1.id, "new goal1").unwrap();
        store.update_actual(&t2.id, "new reality2").unwrap();

        let all = store.all_mutations().unwrap();
        assert_eq!(all.len(), 4); // 2 created + 2 updates
    }

    // ── VAL-MUTATION-007: mutations_between ────────────────────────

    #[test]
    fn test_mutations_between_returns_in_range() {
        let store = Store::new_in_memory().unwrap();

        // Create tensions at specific times
        let t1 = store.create_tension("goal1", "reality1").unwrap();

        // Wait a bit to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _t2 = store.create_tension("goal2", "reality2").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        store.update_desired(&t1.id, "new goal1").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        let _t3 = store.create_tension("goal3", "reality3").unwrap();

        // Get all mutations to see timestamps
        let all = store.all_mutations().unwrap();
        assert_eq!(all.len(), 4);

        // Query mutations in the middle time window (should get t2 creation and t1 update)
        let start = all[1].timestamp();
        let end = all[2].timestamp();
        let middle = store.mutations_between(start, end).unwrap();
        assert_eq!(middle.len(), 2);
    }

    #[test]
    fn test_mutations_between_empty_range() {
        let store = Store::new_in_memory().unwrap();
        let _t = store.create_tension("goal", "reality").unwrap();

        // Query a range before any mutations
        let past = Utc::now() - chrono::Duration::hours(1);
        let more_past = past - chrono::Duration::hours(1);
        let result = store.mutations_between(more_past, past).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_mutations_between_single_mutation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        // Get the creation mutation
        let mutations = store.get_mutations(&t.id).unwrap();
        let creation = &mutations[0];

        // Query exactly at that timestamp
        let result = store
            .mutations_between(creation.timestamp(), creation.timestamp())
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tension_id(), t.id);
    }

    #[test]
    fn test_mutations_between_chronological_order() {
        let store = Store::new_in_memory().unwrap();

        // Create multiple mutations with small delays
        let t = store.create_tension("goal", "reality").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));
        store.update_desired(&t.id, "new goal").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));
        store.update_actual(&t.id, "new reality").unwrap();

        // Query wide range
        let all = store.all_mutations().unwrap();
        let start = all[0].timestamp() - chrono::Duration::seconds(1);
        let end = all[2].timestamp() + chrono::Duration::seconds(1);

        let result = store.mutations_between(start, end).unwrap();
        assert_eq!(result.len(), 3);

        // Verify chronological order
        for i in 1..result.len() {
            assert!(result[i - 1].timestamp() <= result[i].timestamp());
        }
    }

    #[test]
    fn test_mutations_between_multiple_tensions() {
        let store = Store::new_in_memory().unwrap();

        let t1 = store.create_tension("goal1", "reality1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let t2 = store.create_tension("goal2", "reality2").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        store.update_desired(&t1.id, "new goal1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        store.update_actual(&t2.id, "new reality2").unwrap();

        // Query all
        let all = store.all_mutations().unwrap();
        let start = all[0].timestamp() - chrono::Duration::seconds(1);
        let end = all[3].timestamp() + chrono::Duration::seconds(1);

        let result = store.mutations_between(start, end).unwrap();
        assert_eq!(result.len(), 4);

        // Verify all tension IDs are present
        let tension_ids: std::collections::HashSet<_> =
            result.iter().map(|m| m.tension_id()).collect();
        assert!(tension_ids.contains(&t1.id.as_str()));
        assert!(tension_ids.contains(&t2.id.as_str()));
    }

    // ── VAL-MUTATION-011: Mutation replay vs direct query ──────────

    #[test]
    fn test_replay_matches_direct_query() {
        let store = Store::new_in_memory().unwrap();

        // Create tension and perform various updates
        let t = store
            .create_tension("initial goal", "initial reality")
            .unwrap();
        store.update_desired(&t.id, "second goal").unwrap();
        store.update_actual(&t.id, "second reality").unwrap();
        let parent = store.create_tension("parent", "p reality").unwrap();
        store.update_parent(&t.id, Some(&parent.id)).unwrap();
        store.update_desired(&t.id, "final goal").unwrap();
        store.update_actual(&t.id, "final reality").unwrap();

        // Get mutations and replay
        let mutations = store.get_mutations(&t.id).unwrap();
        let reconstructed = crate::mutation::replay_mutations(&mutations).unwrap();

        // Compare with direct query
        let direct = store.get_tension(&t.id).unwrap().unwrap();

        assert_eq!(reconstructed.id, direct.id);
        assert_eq!(reconstructed.desired, direct.desired);
        assert_eq!(reconstructed.actual, direct.actual);
        assert_eq!(reconstructed.parent_id, direct.parent_id);
        assert_eq!(reconstructed.status, direct.status);
        // created_at should be very close (within 1 second due to parsing)
        let diff = (reconstructed.created_at - direct.created_at)
            .num_seconds()
            .abs();
        assert!(diff < 1);
    }

    #[test]
    fn test_replay_resolved_tension_matches_direct_query() {
        let store = Store::new_in_memory().unwrap();

        // Create tension, update, and resolve
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_desired(&t.id, "final goal").unwrap();
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        // Replay mutations
        let mutations = store.get_mutations(&t.id).unwrap();
        let reconstructed = crate::mutation::replay_mutations(&mutations).unwrap();

        // Compare
        let direct = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(reconstructed.desired, direct.desired);
        assert_eq!(reconstructed.status, direct.status);
    }

    #[test]
    fn test_replay_released_tension_matches_direct_query() {
        let store = Store::new_in_memory().unwrap();

        // Create tension, update, and release
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_actual(&t.id, "final reality").unwrap();
        store.update_status(&t.id, TensionStatus::Released).unwrap();

        // Replay mutations
        let mutations = store.get_mutations(&t.id).unwrap();
        let reconstructed = crate::mutation::replay_mutations(&mutations).unwrap();

        // Compare
        let direct = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(reconstructed.actual, direct.actual);
        assert_eq!(reconstructed.status, direct.status);
    }

    // ── Transaction Rollback ───────────────────────────────────────

    #[test]
    fn test_transaction_rollback_on_update_failure() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        // This should fail and rollback
        let result = store.update_desired(&t.id, "");
        assert!(result.is_err());

        // No mutation recorded for failed update
        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 1); // Only the creation mutation
    }

    // ── Schema Correctness ─────────────────────────────────────────

    #[test]
    fn test_schema_tensions_columns() {
        let store = Store::new_in_memory().unwrap();
        let conn = store.conn.borrow();
        let rows = conn.query("PRAGMA table_info(tensions);").unwrap();

        // Check expected columns exist
        let columns: Vec<String> = rows
            .iter()
            .filter_map(|r| {
                if let Some(SqliteValue::Text(name)) = r.get(1) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(columns.contains(&"id".to_owned()));
        assert!(columns.contains(&"desired".to_owned()));
        assert!(columns.contains(&"actual".to_owned()));
        assert!(columns.contains(&"parent_id".to_owned()));
        assert!(columns.contains(&"created_at".to_owned()));
        assert!(columns.contains(&"status".to_owned()));
        assert!(columns.contains(&"horizon".to_owned()));
    }

    #[test]
    fn test_schema_mutations_columns() {
        let store = Store::new_in_memory().unwrap();
        let conn = store.conn.borrow();
        let rows = conn.query("PRAGMA table_info(mutations);").unwrap();

        let columns: Vec<String> = rows
            .iter()
            .filter_map(|r| {
                if let Some(SqliteValue::Text(name)) = r.get(1) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(columns.contains(&"id".to_owned()));
        assert!(columns.contains(&"tension_id".to_owned()));
        assert!(columns.contains(&"timestamp".to_owned()));
        assert!(columns.contains(&"field".to_owned()));
        assert!(columns.contains(&"old_value".to_owned()));
        assert!(columns.contains(&"new_value".to_owned()));
    }

    // ── Concurrent Reads ───────────────────────────────────────────

    #[test]
    fn test_concurrent_reads() {
        use std::thread;

        // fsqlite's Connection uses Rc internally, so it's not Send.
        // We test concurrent reads by having each thread open its own connection
        // to a file-based database.
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::init(temp_dir.path()).unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        let tension_id = t.id.clone();
        let path = temp_dir.path().to_path_buf();

        let mut handles = vec![];
        for _ in 0..5 {
            let path_clone = path.clone();
            let id = tension_id.clone();
            handles.push(thread::spawn(move || {
                // Each thread opens its own connection to the same database
                let thread_store = Store::init(&path_clone).unwrap();
                let retrieved = thread_store.get_tension(&id).unwrap();
                assert!(retrieved.is_some());
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    // ── Unicode ────────────────────────────────────────────────────

    #[test]
    fn test_store_unicode() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("写小说", "有一个大纲 🎵").unwrap();

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.desired, "写小说");
        assert_eq!(retrieved.actual, "有一个大纲 🎵");
    }

    // ── Init Idempotency ───────────────────────────────────────────

    #[test]
    fn test_init_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store1 = Store::init(temp_dir.path()).unwrap();
        let t = store1.create_tension("goal", "reality").unwrap();

        // Re-open the same database
        let store2 = Store::init(temp_dir.path()).unwrap();
        let retrieved = store2.get_tension(&t.id).unwrap();
        assert!(retrieved.is_some());
    }

    // ── Error Types ────────────────────────────────────────────────

    #[test]
    fn test_store_error_display() {
        let e = StoreError::DatabaseError("test".to_owned());
        assert!(e.to_string().contains("database error"));

        let e = StoreError::TensionNotFound("abc".to_owned());
        assert!(e.to_string().contains("abc"));

        let e = StoreError::PermissionDenied("/path".to_owned());
        assert!(e.to_string().contains("permission denied"));
    }

    // ── VAL-TENSION-012: Deletion with children (reparent to roots) ──────

    #[test]
    fn test_resolve_tension_with_children_reparents_to_roots() {
        let store = Store::new_in_memory().unwrap();

        // Create parent with children
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let child1 = store
            .create_tension_with_parent("child1 goal", "child1 reality", Some(parent.id.clone()))
            .unwrap();
        let child2 = store
            .create_tension_with_parent("child2 goal", "child2 reality", Some(parent.id.clone()))
            .unwrap();

        // Resolve the parent
        store
            .update_status(&parent.id, TensionStatus::Resolved)
            .unwrap();

        // Children should now be roots (parent_id = None)
        let child1_after = store.get_tension(&child1.id).unwrap().unwrap();
        let child2_after = store.get_tension(&child2.id).unwrap().unwrap();
        assert!(child1_after.parent_id.is_none());
        assert!(child2_after.parent_id.is_none());

        // Children should appear in get_roots()
        let roots = store.get_roots().unwrap();
        assert!(roots.iter().any(|r| r.id == child1.id));
        assert!(roots.iter().any(|r| r.id == child2.id));

        // Parent should still be in roots (it has null parent_id), but with Resolved status
        let parent_in_roots = roots.iter().find(|r| r.id == parent.id);
        assert!(
            parent_in_roots.is_some(),
            "parent should still be a root (null parent_id)"
        );
        assert_eq!(
            parent_in_roots.unwrap().status,
            TensionStatus::Resolved,
            "parent should have Resolved status"
        );
    }

    #[test]
    fn test_release_tension_with_children_reparents_to_roots() {
        let store = Store::new_in_memory().unwrap();

        // Create parent with children
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let child1 = store
            .create_tension_with_parent("child1 goal", "child1 reality", Some(parent.id.clone()))
            .unwrap();
        let child2 = store
            .create_tension_with_parent("child2 goal", "child2 reality", Some(parent.id.clone()))
            .unwrap();

        // Release the parent
        store
            .update_status(&parent.id, TensionStatus::Released)
            .unwrap();

        // Children should now be roots
        let child1_after = store.get_tension(&child1.id).unwrap().unwrap();
        let child2_after = store.get_tension(&child2.id).unwrap().unwrap();
        assert!(child1_after.parent_id.is_none());
        assert!(child2_after.parent_id.is_none());
    }

    #[test]
    fn test_resolve_tension_with_children_records_parent_mutations() {
        let store = Store::new_in_memory().unwrap();

        // Create parent with children
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();
        let child1 = store
            .create_tension_with_parent("child1 goal", "child1 reality", Some(parent.id.clone()))
            .unwrap();
        let child2 = store
            .create_tension_with_parent("child2 goal", "child2 reality", Some(parent.id.clone()))
            .unwrap();

        // Resolve the parent
        store
            .update_status(&parent.id, TensionStatus::Resolved)
            .unwrap();

        // Each child should have a parent_id mutation recorded
        let child1_mutations = store.get_mutations(&child1.id).unwrap();
        let child2_mutations = store.get_mutations(&child2.id).unwrap();

        // Find the parent_id mutation for each child
        let child1_parent_mutation = child1_mutations.iter().find(|m| m.field() == "parent_id");
        let child2_parent_mutation = child2_mutations.iter().find(|m| m.field() == "parent_id");

        assert!(
            child1_parent_mutation.is_some(),
            "child1 should have parent_id mutation"
        );
        assert!(
            child2_parent_mutation.is_some(),
            "child2 should have parent_id mutation"
        );

        // Verify mutation records the old parent_id and empty new_value (null)
        let m1 = child1_parent_mutation.unwrap();
        assert_eq!(m1.old_value(), Some(parent.id.as_str()));
        assert_eq!(m1.new_value(), ""); // Empty string represents null
    }

    #[test]
    fn test_resolve_tension_without_children_no_reparent() {
        let store = Store::new_in_memory().unwrap();

        // Create parent without children
        let parent = store
            .create_tension("parent goal", "parent reality")
            .unwrap();

        // Resolve the parent
        store
            .update_status(&parent.id, TensionStatus::Resolved)
            .unwrap();

        // Status should be Resolved
        let parent_after = store.get_tension(&parent.id).unwrap().unwrap();
        assert_eq!(parent_after.status, TensionStatus::Resolved);

        // Parent is still a root (has null parent_id), but with Resolved status
        let roots = store.get_roots().unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, parent.id);
        assert_eq!(roots[0].status, TensionStatus::Resolved);
    }

    #[test]
    fn test_resolve_deep_hierarchy_reparents_all_descendants() {
        let store = Store::new_in_memory().unwrap();

        // Create a deep hierarchy: grandparent -> parent -> child -> grandchild
        let grandparent = store.create_tension("grandparent", "gp reality").unwrap();
        let parent = store
            .create_tension_with_parent("parent", "p reality", Some(grandparent.id.clone()))
            .unwrap();
        let child = store
            .create_tension_with_parent("child", "c reality", Some(parent.id.clone()))
            .unwrap();
        let grandchild = store
            .create_tension_with_parent("grandchild", "gc reality", Some(child.id.clone()))
            .unwrap();

        // Resolve the parent (middle of hierarchy)
        // This should reparent child and grandchild
        store
            .update_status(&parent.id, TensionStatus::Resolved)
            .unwrap();

        // Child should now be a root
        let child_after = store.get_tension(&child.id).unwrap().unwrap();
        assert!(child_after.parent_id.is_none());

        // Grandchild should still have child as parent
        let grandchild_after = store.get_tension(&grandchild.id).unwrap().unwrap();
        assert_eq!(grandchild_after.parent_id, Some(child.id));

        // Grandparent should still exist (not resolved)
        let grandparent_after = store.get_tension(&grandparent.id).unwrap().unwrap();
        assert_eq!(grandparent_after.status, TensionStatus::Active);
    }

    // ── Event Emission Tests ────────────────────────────────────────

    #[test]
    fn test_store_emits_tension_created_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::TensionCreated { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let _t = store.create_tension("goal", "reality").unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_emits_reality_confronted_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::RealityConfronted { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();
        store.update_actual(&t.id, "new reality").unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_emits_desire_revised_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::DesireRevised { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();
        store.update_desired(&t.id, "new goal").unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_emits_tension_resolved_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::TensionResolved { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_emits_tension_released_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::TensionReleased { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();
        store.update_status(&t.id, TensionStatus::Released).unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_emits_structure_changed_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::StructureChanged { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let parent = store.create_tension("parent", "p").unwrap();
        let child = store.create_tension("child", "c").unwrap();
        store.update_parent(&child.id, Some(&parent.id)).unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_no_events_on_failed_operation() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |_ev| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();

        // This will fail (empty string not allowed)
        let _ = store.update_desired(&t.id, "");

        // Only the TensionCreated event should have been emitted
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_events_in_causal_order() {
        use crate::events::{Event, EventBus};

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let events = Arc::new(Mutex::new(Vec::new()));

        let e = events.clone();
        let _handle = bus.subscribe(move |ev| {
            e.lock().unwrap().push(ev.clone());
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();
        store.update_actual(&t.id, "new reality").unwrap();
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        let received = events.lock().unwrap().clone();
        assert_eq!(received.len(), 3);

        // Verify causal order: Created -> RealityConfronted -> TensionResolved
        assert!(matches!(&received[0], Event::TensionCreated { .. }));
        assert!(matches!(&received[1], Event::RealityConfronted { .. }));
        assert!(matches!(&received[2], Event::TensionResolved { .. }));
    }

    #[test]
    fn test_store_no_event_bus_no_panic() {
        // Verify that operations work without an event bus attached
        let store = Store::new_in_memory().unwrap();

        let t = store.create_tension("goal", "reality").unwrap();
        store.update_actual(&t.id, "new reality").unwrap();
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        // If we get here without panicking, the test passes
    }

    // ── VAL-TENSION-013: Deletion with children (grandparent adoption) ──────

    #[test]
    fn test_delete_tension_leaf() {
        let store = Store::new_in_memory().unwrap();

        // Create a leaf tension (no children)
        let t = store.create_tension("goal", "reality").unwrap();
        let tension_id = t.id.clone();

        // Delete it
        store.delete_tension(&tension_id).unwrap();

        // Should be gone
        let result = store.get_tension(&tension_id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_tension_not_found() {
        let store = Store::new_in_memory().unwrap();

        // Try to delete a nonexistent tension
        let result = store.delete_tension("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_tension_with_children_adopts_to_grandparent() {
        let store = Store::new_in_memory().unwrap();

        // Create a three-level hierarchy: grandparent -> parent -> child
        let grandparent = store.create_tension("grandparent", "gp reality").unwrap();
        let parent = store
            .create_tension_with_parent("parent", "p reality", Some(grandparent.id.clone()))
            .unwrap();
        let child = store
            .create_tension_with_parent("child", "c reality", Some(parent.id.clone()))
            .unwrap();

        // Delete the middle (parent)
        store.delete_tension(&parent.id).unwrap();

        // Parent should be gone
        assert!(store.get_tension(&parent.id).unwrap().is_none());

        // Child should now have grandparent as parent
        let child_after = store.get_tension(&child.id).unwrap().unwrap();
        assert_eq!(child_after.parent_id, Some(grandparent.id));
    }

    #[test]
    fn test_delete_root_with_children_makes_children_roots() {
        let store = Store::new_in_memory().unwrap();

        // Create parent -> children
        let parent = store.create_tension("parent", "p reality").unwrap();
        let child1 = store
            .create_tension_with_parent("child1", "c1 reality", Some(parent.id.clone()))
            .unwrap();
        let child2 = store
            .create_tension_with_parent("child2", "c2 reality", Some(parent.id.clone()))
            .unwrap();

        // Delete the root parent
        store.delete_tension(&parent.id).unwrap();

        // Parent should be gone
        assert!(store.get_tension(&parent.id).unwrap().is_none());

        // Children should now be roots
        let child1_after = store.get_tension(&child1.id).unwrap().unwrap();
        let child2_after = store.get_tension(&child2.id).unwrap().unwrap();
        assert!(child1_after.parent_id.is_none());
        assert!(child2_after.parent_id.is_none());

        // Children should appear in get_roots()
        let roots = store.get_roots().unwrap();
        assert!(roots.iter().any(|r| r.id == child1.id));
        assert!(roots.iter().any(|r| r.id == child2.id));
    }

    #[test]
    fn test_delete_tension_records_mutation() {
        let store = Store::new_in_memory().unwrap();

        let t = store.create_tension("goal", "reality").unwrap();
        let tension_id = t.id.clone();

        store.delete_tension(&tension_id).unwrap();

        // A "deleted" mutation should be recorded
        let mutations = store.get_mutations(&tension_id).unwrap();
        let deleted_mutation = mutations.iter().find(|m| m.field() == "deleted");
        assert!(
            deleted_mutation.is_some(),
            "should have 'deleted' mutation recorded"
        );
    }

    #[test]
    fn test_delete_tension_records_parent_mutation_for_children() {
        let store = Store::new_in_memory().unwrap();

        // Create parent -> child
        let parent = store.create_tension("parent", "p reality").unwrap();
        let child = store
            .create_tension_with_parent("child", "c reality", Some(parent.id.clone()))
            .unwrap();

        // Delete parent
        store.delete_tension(&parent.id).unwrap();

        // Child should have a parent_id mutation
        let mutations = store.get_mutations(&child.id).unwrap();
        let parent_mutation = mutations.iter().find(|m| m.field() == "parent_id");
        assert!(parent_mutation.is_some());

        let m = parent_mutation.unwrap();
        assert_eq!(m.old_value(), Some(parent.id.as_str()));
        assert_eq!(m.new_value(), ""); // Empty string represents null
    }

    #[test]
    fn test_delete_deep_hierarchy_preserves_lower_levels() {
        let store = Store::new_in_memory().unwrap();

        // Create A -> B -> C -> D (4 levels)
        let a = store.create_tension("A", "a reality").unwrap();
        let b = store
            .create_tension_with_parent("B", "b reality", Some(a.id.clone()))
            .unwrap();
        let c = store
            .create_tension_with_parent("C", "c reality", Some(b.id.clone()))
            .unwrap();
        let d = store
            .create_tension_with_parent("D", "d reality", Some(c.id.clone()))
            .unwrap();

        // Delete B
        store.delete_tension(&b.id).unwrap();

        // C's parent should now be A
        let c_after = store.get_tension(&c.id).unwrap().unwrap();
        assert_eq!(c_after.parent_id, Some(a.id));

        // D's parent should still be C
        let d_after = store.get_tension(&d.id).unwrap().unwrap();
        assert_eq!(d_after.parent_id, Some(c.id));
    }

    #[test]
    fn test_delete_tension_emits_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::TensionDeleted { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();
        store.delete_tension(&t.id).unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    // ── Horizon Tests ──────────────────────────────────────────────

    #[test]
    fn test_create_tension_full_with_horizon_year() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_year(2026).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();

        assert_eq!(t.horizon, Some(h.clone()));

        // Retrieve and verify
        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, Some(h));
    }

    #[test]
    fn test_create_tension_full_with_horizon_month() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_month(2026, 5).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();

        assert_eq!(t.horizon, Some(h.clone()));

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, Some(h));
    }

    #[test]
    fn test_create_tension_full_with_horizon_day() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_day(2026, 5, 15).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();

        assert_eq!(t.horizon, Some(h.clone()));

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, Some(h));
    }

    #[test]
    fn test_create_tension_full_with_horizon_datetime() {
        use chrono::{TimeZone, Utc};
        let store = Store::new_in_memory().unwrap();
        let dt = Utc.with_ymd_and_hms(2026, 5, 15, 14, 30, 0).unwrap();
        let h = Horizon::new_datetime(dt);
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();

        assert_eq!(t.horizon, Some(h.clone()));

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, Some(h));
    }

    #[test]
    fn test_create_tension_full_without_horizon() {
        let store = Store::new_in_memory().unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, None)
            .unwrap();

        assert!(t.horizon.is_none());

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert!(retrieved.horizon.is_none());
    }

    #[test]
    fn test_create_tension_full_with_parent_and_horizon() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_month(2026, 5).unwrap();
        let parent = store.create_tension("parent", "p").unwrap();

        let t = store
            .create_tension_full("child", "c", Some(parent.id.clone()), Some(h.clone()))
            .unwrap();

        assert_eq!(t.parent_id, Some(parent.id));
        assert_eq!(t.horizon, Some(h));
    }

    #[test]
    fn test_create_tension_full_records_mutation_with_horizon() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_month(2026, 5).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].field(), "created");
        assert!(mutations[0].new_value().contains("horizon='2026-05'"));
    }

    #[test]
    fn test_create_tension_full_records_mutation_without_horizon() {
        let store = Store::new_in_memory().unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, None)
            .unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].field(), "created");
        // Should NOT contain horizon field
        assert!(!mutations[0].new_value().contains("horizon"));
    }

    #[test]
    fn test_update_horizon_on_active() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let h = Horizon::new_month(2026, 5).unwrap();
        store.update_horizon(&t.id, Some(h.clone())).unwrap();

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, Some(h));
    }

    #[test]
    fn test_update_horizon_records_mutation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let h = Horizon::new_month(2026, 5).unwrap();
        store.update_horizon(&t.id, Some(h.clone())).unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2); // created + horizon
        assert_eq!(mutations[1].field(), "horizon");
        assert!(mutations[1].old_value().is_none());
        assert_eq!(mutations[1].new_value(), "2026-05");
    }

    #[test]
    fn test_update_horizon_on_resolved_fails() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();

        let h = Horizon::new_month(2026, 5).unwrap();
        let result = store.update_horizon(&t.id, Some(h));
        assert!(result.is_err());

        // Horizon should still be None
        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert!(retrieved.horizon.is_none());

        // No horizon mutation recorded
        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2); // created + status
        assert_eq!(mutations[1].field(), "status");
    }

    #[test]
    fn test_update_horizon_on_released_fails() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_status(&t.id, TensionStatus::Released).unwrap();

        let h = Horizon::new_month(2026, 5).unwrap();
        let result = store.update_horizon(&t.id, Some(h));
        assert!(result.is_err());
    }

    #[test]
    fn test_update_horizon_clear_to_none() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_month(2026, 5).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();

        store.update_horizon(&t.id, None).unwrap();

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert!(retrieved.horizon.is_none());

        // Check mutation recorded
        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[1].field(), "horizon");
        assert_eq!(mutations[1].old_value(), Some("2026-05"));
        assert_eq!(mutations[1].new_value(), ""); // Empty string = None
    }

    #[test]
    fn test_update_horizon_change_value() {
        let store = Store::new_in_memory().unwrap();
        let h1 = Horizon::new_year(2026).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h1.clone()))
            .unwrap();

        let h2 = Horizon::new_month(2026, 5).unwrap();
        store.update_horizon(&t.id, Some(h2.clone())).unwrap();

        let retrieved = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, Some(h2));

        let mutations = store.get_mutations(&t.id).unwrap();
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[1].field(), "horizon");
        assert_eq!(mutations[1].old_value(), Some("2026"));
        assert_eq!(mutations[1].new_value(), "2026-05");
    }

    #[test]
    fn test_list_tensions_returns_horizon() {
        let store = Store::new_in_memory().unwrap();
        let h1 = Horizon::new_year(2026).unwrap();
        let _t1 = store
            .create_tension_full("goal1", "reality1", None, Some(h1.clone()))
            .unwrap();
        let _t2 = store.create_tension("goal2", "reality2").unwrap();

        let tensions = store.list_tensions().unwrap();
        assert_eq!(tensions.len(), 2);
        assert_eq!(tensions[0].horizon, Some(h1));
        assert!(tensions[1].horizon.is_none());
    }

    #[test]
    fn test_get_roots_returns_horizon() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_month(2026, 5).unwrap();
        let _root = store
            .create_tension_full("root", "r", None, Some(h.clone()))
            .unwrap();

        let roots = store.get_roots().unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].horizon, Some(h));
    }

    #[test]
    fn test_get_children_returns_horizon() {
        let store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "p").unwrap();
        let h = Horizon::new_day(2026, 5, 15).unwrap();
        let _child = store
            .create_tension_full("child", "c", Some(parent.id.clone()), Some(h.clone()))
            .unwrap();

        let children = store.get_children(&parent.id).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].horizon, Some(h));
    }

    #[test]
    fn test_update_horizon_emits_event() {
        use crate::events::EventBus;

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        let _handle = bus.subscribe(move |ev| {
            if matches!(ev, crate::events::Event::HorizonChanged { .. }) {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        store.set_event_bus(bus);

        let t = store.create_tension("goal", "reality").unwrap();
        let h = Horizon::new_month(2026, 5).unwrap();
        store.update_horizon(&t.id, Some(h)).unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_update_horizon_clear_emits_event_with_none() {
        use crate::events::{Event, EventBus};

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let events = Arc::new(Mutex::new(Vec::new()));

        let e = events.clone();
        let _handle = bus.subscribe(move |ev| {
            if let Event::HorizonChanged { .. } = ev {
                e.lock().unwrap().push(ev.clone());
            }
        });

        store.set_event_bus(bus);

        let h = Horizon::new_year(2026).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();
        store.update_horizon(&t.id, None).unwrap();

        let evts = events.lock().unwrap();
        assert_eq!(evts.len(), 1);
        if let Event::HorizonChanged {
            old_horizon,
            new_horizon,
            ..
        } = &evts[0]
        {
            assert_eq!(old_horizon, &Some("2026".to_owned()));
            assert_eq!(new_horizon, &None);
        } else {
            panic!("expected HorizonChanged event");
        }
    }

    #[test]
    fn test_replay_matches_direct_query_with_horizon() {
        let store = Store::new_in_memory().unwrap();
        let h = Horizon::new_month(2026, 5).unwrap();
        let t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();
        store.update_actual(&t.id, "new reality").unwrap();
        let new_h = Horizon::new_year(2027).unwrap();
        store.update_horizon(&t.id, Some(new_h.clone())).unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        let reconstructed = crate::mutation::replay_mutations(&mutations).unwrap();
        let direct = store.get_tension(&t.id).unwrap().unwrap();

        assert_eq!(reconstructed.horizon, direct.horizon);
        assert_eq!(reconstructed.horizon, Some(new_h));
    }

    // ── Horizon Event Emission Tests ───────────────────────────────

    #[test]
    fn test_create_tension_full_emits_event_with_horizon() {
        use crate::events::{Event, EventBus};

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let events = Arc::new(Mutex::new(Vec::new()));

        let e = events.clone();
        let _handle = bus.subscribe(move |ev| {
            if let Event::TensionCreated { .. } = ev {
                e.lock().unwrap().push(ev.clone());
            }
        });

        store.set_event_bus(bus);

        let h = Horizon::new_month(2026, 5).unwrap();
        let _t = store
            .create_tension_full("goal", "reality", None, Some(h.clone()))
            .unwrap();

        let evts = events.lock().unwrap();
        assert_eq!(evts.len(), 1);
        if let Event::TensionCreated { horizon, .. } = &evts[0] {
            assert_eq!(horizon, &Some("2026-05".to_owned()));
        } else {
            panic!("expected TensionCreated event");
        }
    }

    #[test]
    fn test_create_tension_full_emits_event_without_horizon() {
        use crate::events::{Event, EventBus};

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let events = Arc::new(Mutex::new(Vec::new()));

        let e = events.clone();
        let _handle = bus.subscribe(move |ev| {
            if let Event::TensionCreated { .. } = ev {
                e.lock().unwrap().push(ev.clone());
            }
        });

        store.set_event_bus(bus);

        let _t = store
            .create_tension_full("goal", "reality", None, None)
            .unwrap();

        let evts = events.lock().unwrap();
        assert_eq!(evts.len(), 1);
        if let Event::TensionCreated { horizon, .. } = &evts[0] {
            assert!(horizon.is_none());
        } else {
            panic!("expected TensionCreated event");
        }
    }

    #[test]
    fn test_create_tension_defaults_no_horizon_event() {
        use crate::events::{Event, EventBus};

        let mut store = Store::new_in_memory().unwrap();
        let bus = EventBus::new();
        let events = Arc::new(Mutex::new(Vec::new()));

        let e = events.clone();
        let _handle = bus.subscribe(move |ev| {
            if let Event::TensionCreated { .. } = ev {
                e.lock().unwrap().push(ev.clone());
            }
        });

        store.set_event_bus(bus);

        let _t = store.create_tension("goal", "reality").unwrap();

        let evts = events.lock().unwrap();
        assert_eq!(evts.len(), 1);
        if let Event::TensionCreated { horizon, .. } = &evts[0] {
            assert!(horizon.is_none());
        } else {
            panic!("expected TensionCreated event");
        }
    }

    // ── Migration Tests (VAL-HSTORE-002) ───────────────────────────

    /// VAL-HSTORE-002: Migration of existing databases without horizon column
    /// When opening an existing DB without horizon column, the migration
    /// should add the column via ALTER TABLE and existing tensions should
    /// have horizon = None.
    #[test]
    fn test_migration_adds_horizon_column() {
        use fsqlite::Connection;

        // For a proper legacy DB test, we need to use a file-based database
        // Store::init() expects a directory and creates .werk/sd.db inside it
        let temp_base = std::env::temp_dir().join("werk_migration_test_dir");
        let werk_dir = temp_base.join(".werk");

        // Clean up any existing test directory
        let _ = std::fs::remove_dir_all(&temp_base);

        // Create the base temp directory
        std::fs::create_dir_all(&temp_base).unwrap();
        std::fs::create_dir_all(&werk_dir).unwrap();

        let db_path = werk_dir.join("sd.db");
        let db_path_str = db_path.to_string_lossy().into_owned();

        // Create legacy file-based database with OLD schema (no horizon column)
        {
            let legacy_conn = Connection::open(&db_path_str).unwrap();
            legacy_conn
                .execute(
                    "CREATE TABLE tensions (
                    id TEXT PRIMARY KEY,
                    desired TEXT NOT NULL,
                    actual TEXT NOT NULL,
                    parent_id TEXT,
                    created_at TEXT NOT NULL,
                    status TEXT NOT NULL
                )",
                )
                .unwrap();
            legacy_conn
                .execute(
                    "CREATE TABLE mutations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    tension_id TEXT NOT NULL,
                    timestamp TEXT NOT NULL,
                    field TEXT NOT NULL,
                    old_value TEXT,
                    new_value TEXT
                )",
                )
                .unwrap();
            legacy_conn
                .execute(
                    "INSERT INTO tensions (id, desired, actual, parent_id, created_at, status)
                     VALUES ('LEGACY001', 'legacy goal', 'legacy reality', NULL, '2025-01-01T00:00:00Z', 'Active')",
                )
                .unwrap();
            legacy_conn
                .execute(
                    "INSERT INTO tensions (id, desired, actual, parent_id, created_at, status)
                     VALUES ('LEGACY002', 'another goal', 'another reality', NULL, '2025-01-02T00:00:00Z', 'Active')",
                )
                .unwrap();
            // Verify no horizon column exists
            let cols: Vec<fsqlite::Row> = legacy_conn.query("PRAGMA table_info(tensions)").unwrap();
            let has_horiz = cols.iter().any(|r| {
                if let Some(fsqlite_types::value::SqliteValue::Text(s)) = r.get(1) {
                    s == "horizon"
                } else {
                    false
                }
            });
            assert!(!has_horiz, "Should have no horizon column in legacy DB");
        } // Connection closed

        // Open via Store::init - this should trigger migration
        let store = Store::init(&temp_base).unwrap();

        // Verify horizon column was added
        let tensions = store.list_tensions().unwrap();
        assert_eq!(tensions.len(), 2, "Should have 2 legacy tensions");

        // All legacy tensions should have horizon = None
        for t in &tensions {
            assert!(
                t.horizon.is_none(),
                "Legacy tension {} should have horizon=None",
                t.id
            );
        }

        // Creating a new tension should work
        let new_t = store.create_tension("new goal", "new reality").unwrap();
        assert!(new_t.horizon.is_none());

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_base);
    }
}
