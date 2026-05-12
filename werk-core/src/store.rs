//! fsqlite-backed Store for tensions and mutations.
//!
//! The Store provides persistence for tensions and their mutation history.
//! It uses fsqlite (FrankenSQLite) for storage, supporting both file-based
//! and in-memory databases.
//!
//! # Directory Discovery
//!
//! `Store::open()` walks up from the current working directory looking for
//! a `.werk/` directory containing `werk.db`. If not found, it falls back to
//! `~/.werk/werk.db`.
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
//!     parent_actual_snapshot TEXT,
//!     parent_snapshot_json TEXT,
//!     short_code INTEGER
//! );
//!
//! CREATE TABLE mutations (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     tension_id TEXT NOT NULL,
//!     timestamp TEXT NOT NULL,
//!     field TEXT NOT NULL,
//!     old_value TEXT,
//!     new_value TEXT,
//!     gesture_id TEXT,
//!     actual_at TEXT
//! );
//!
//! CREATE TABLE sessions (
//!     id TEXT PRIMARY KEY,
//!     started_at TEXT NOT NULL,
//!     ended_at TEXT,
//!     summary_note TEXT
//! );
//!
//! CREATE TABLE gestures (
//!     id TEXT PRIMARY KEY,
//!     session_id TEXT,
//!     timestamp TEXT NOT NULL,
//!     description TEXT
//! );
//!
//! CREATE TABLE edges (
//!     id TEXT PRIMARY KEY,
//!     from_id TEXT NOT NULL,
//!     to_id TEXT NOT NULL,
//!     edge_type TEXT NOT NULL,
//!     created_at TEXT NOT NULL,
//!     gesture_id TEXT,
//!     UNIQUE(from_id, to_id, edge_type)
//! );
//!
//! CREATE TABLE epochs (
//!     id TEXT PRIMARY KEY,
//!     tension_id TEXT NOT NULL,
//!     timestamp TEXT NOT NULL,
//!     desire_snapshot TEXT NOT NULL,
//!     reality_snapshot TEXT NOT NULL,
//!     children_snapshot_json TEXT,
//!     trigger_gesture_id TEXT
//! );
//!
//! CREATE TABLE IF NOT EXISTS sigils (
//!     id INTEGER PRIMARY KEY,
//!     short_code INTEGER UNIQUE NOT NULL,
//!     scope_canonical TEXT NOT NULL,
//!     logic_id TEXT NOT NULL,
//!     logic_version TEXT NOT NULL,
//!     seed INTEGER NOT NULL,
//!     rendered_at TEXT NOT NULL,
//!     file_path TEXT NOT NULL,
//!     label TEXT NULL
//! );
//! CREATE INDEX IF NOT EXISTS idx_sigils_short_code ON sigils(short_code);
//! CREATE INDEX IF NOT EXISTS idx_sigils_logic ON sigils(logic_id);
//! ```

use chrono::{DateTime, Utc};
use fsqlite::Connection;
use fsqlite_types::value::SqliteValue;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Maximum retries for transient MVCC conflicts (e.g., SQLITE_BUSY_SNAPSHOT).
const CONCURRENT_COMMIT_MAX_RETRIES: u32 = 10;
/// Base backoff delay between retries (doubles each attempt).
const CONCURRENT_COMMIT_BASE_DELAY_MS: u64 = 5;

use crate::events::{Event, EventBuilder, EventBus};
use crate::horizon::Horizon;
use crate::mutation::Mutation;
use crate::tension::{CoreError, Tension, TensionStatus};

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

/// Convert StoreError to CoreError for use in operations that return CoreError.
impl From<StoreError> for CoreError {
    fn from(e: StoreError) -> Self {
        CoreError::ValidationError(e.to_string())
    }
}

/// The persistent store for tensions and mutations.
///
/// Uses fsqlite for storage with MVCC concurrent writers enabled.
/// Multiple processes can safely read and write to the same database
/// simultaneously — conflicts are detected at the page level and
/// retried automatically with exponential backoff.
///
/// Note: fsqlite's Connection uses Rc internally, so Store cannot be
/// sent between threads. Multi-surface access (CLI + TUI + MCP) works
/// because each surface is a separate OS process with its own Connection.
///
/// # Events
///
/// The store can optionally emit events to an attached EventBus.
/// Use `set_event_bus()` to attach a bus, then all successful operations
/// will emit corresponding events.
pub struct Store {
    conn: Rc<RefCell<Connection>>, // ubs:ignore deliberate !Send — single-writer fsqlite design
    path: Option<PathBuf>,
    event_bus: Option<EventBus>,
    /// The currently active gesture. When set, all mutations are linked to this gesture.
    active_gesture_id: Option<String>,
    /// Pending actual_at timestamp for the next mutation(s). Supports "I did this yesterday."
    pending_actual_at: Option<DateTime<Utc>>,
}

impl Store {
    fn migrate_legacy_db(werk_dir: &Path, db_path: &Path) -> Result<(), StoreError> {
        let old_db_path = werk_dir.join("sd.db");
        if !old_db_path.exists() || db_path.exists() {
            return Ok(());
        }

        match std::fs::rename(&old_db_path, db_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && db_path.exists() => Ok(()),
            Err(e) => Err(StoreError::IoError(format!(
                "failed to migrate sd.db to werk.db: {}",
                e
            ))),
        }
    }

    /// Initialize a new store at the given path.
    ///
    /// Creates `.werk/werk.db` with the correct schema. Idempotent —
    /// opening an existing database preserves data.
    ///
    /// Enables MVCC concurrent writers so multiple processes (CLI, TUI,
    /// MCP agents) can safely write to the same database simultaneously.
    pub fn init(path: &std::path::Path) -> Result<Self, StoreError> {
        let werk_dir = path.join(".werk");
        std::fs::create_dir_all(&werk_dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                StoreError::PermissionDenied(format!("{}", werk_dir.display()))
            } else {
                StoreError::IoError(format!("failed to create .werk directory: {}", e))
            }
        })?;

        let db_path = werk_dir.join("werk.db");
        Self::migrate_legacy_db(&werk_dir, &db_path)?;

        // Back up the database before opening (rotates, keeps last 10)
        if db_path.exists() {
            Self::backup_db(&werk_dir, &db_path);
        }

        let db_path_str = db_path.to_string_lossy().into_owned();
        let conn = Connection::open(db_path_str)
            .map_err(|e| StoreError::DatabaseError(format!("failed to open database: {:?}", e)))?;
        Self::enable_concurrent_mode(&conn)?;

        let store = Self {
            conn: Rc::new(RefCell::new(conn)),
            path: Some(db_path),
            event_bus: None,
            active_gesture_id: None,
            pending_actual_at: None,
        };
        store.create_schema()?;
        Ok(store)
    }

    /// Initialize a store without backup-on-open.
    ///
    /// Intended for tests and tooling where backup rotation is unnecessary.
    /// MVCC concurrent mode is still enabled.
    pub fn init_unlocked(path: &std::path::Path) -> Result<Self, StoreError> {
        let werk_dir = path.join(".werk");
        std::fs::create_dir_all(&werk_dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                StoreError::PermissionDenied(format!("{}", werk_dir.display()))
            } else {
                StoreError::IoError(format!("failed to create .werk directory: {}", e))
            }
        })?;

        let db_path = werk_dir.join("werk.db");
        Self::migrate_legacy_db(&werk_dir, &db_path)?;
        let db_path_str = db_path.to_string_lossy().into_owned();
        let conn = Connection::open(db_path_str)
            .map_err(|e| StoreError::DatabaseError(format!("failed to open database: {:?}", e)))?;
        Self::enable_concurrent_mode(&conn)?;

        let store = Self {
            conn: Rc::new(RefCell::new(conn)),
            path: Some(db_path),
            event_bus: None,
            active_gesture_id: None,
            pending_actual_at: None,
        };
        store.create_schema()?;
        Ok(store)
    }

    /// Open an existing store, discovering .werk/ by walking up from CWD.
    ///
    /// Falls back to ~/.werk/werk.db if no local .werk/ found.
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
        Self::enable_concurrent_mode(&conn)?;
        let store = Self {
            conn: Rc::new(RefCell::new(conn)),
            path: None,
            event_bus: None,
            active_gesture_id: None,
            pending_actual_at: None,
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

    /// Enable MVCC concurrent writer mode on a connection.
    ///
    /// Must be called immediately after `Connection::open()`. With this enabled,
    /// `BEGIN CONCURRENT` allows page-level MVCC with Serializable Snapshot
    /// Isolation — multiple processes can write simultaneously.
    fn enable_concurrent_mode(conn: &Connection) -> Result<(), StoreError> {
        conn.execute("PRAGMA fsqlite.concurrent_mode=ON;")
            .map(|_| ())
            .map_err(|e| {
                StoreError::DatabaseError(format!("failed to enable concurrent mode: {:?}", e))
            })
    }

    /// Commit with retry for transient MVCC conflicts.
    ///
    /// When concurrent writers contend for the WAL write lock, the loser gets
    /// a transient Busy error. The transaction's writes are still valid — we
    /// just need to wait for the lock and retry COMMIT.
    fn commit_with_retry(conn: &Connection) -> Result<(), CoreError> {
        for attempt in 0..CONCURRENT_COMMIT_MAX_RETRIES {
            match conn.execute("COMMIT;") {
                Ok(_) => return Ok(()),
                Err(e) if e.is_transient() && attempt + 1 < CONCURRENT_COMMIT_MAX_RETRIES => {
                    let delay = CONCURRENT_COMMIT_BASE_DELAY_MS * (1 << attempt.min(6));
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(CoreError::ValidationError(format!(
                        "commit failed: {:?}",
                        e
                    )));
                }
            }
        }
        let _ = conn.execute("ROLLBACK;");
        Err(CoreError::ValidationError(
            "commit failed after max retries — concurrent write contention".to_owned(),
        ))
    }

    fn backup_db(werk_dir: &std::path::Path, db_path: &std::path::Path) {
        let backup_dir = werk_dir.join("backups");
        let _ = std::fs::create_dir_all(&backup_dir);
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let backup_path = backup_dir.join(format!("werk.db.{}", timestamp));
        let mut wrote_local = false;
        if !backup_path.exists() {
            wrote_local = std::fs::copy(db_path, &backup_path).is_ok();
        }
        if let Ok(entries) = std::fs::read_dir(&backup_dir) {
            let mut db_backups: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    name_str.starts_with("werk.db.") || name_str.starts_with("sd.db.")
                })
                .collect();
            db_backups.sort_by_key(|e| e.file_name());
            if db_backups.len() > 10 {
                for entry in &db_backups[..db_backups.len() - 10] {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }

        // Mirror backups outside the workspace so `werk nuke` cannot destroy
        // its own recovery substrate. Best-effort; failures are silent.
        // Audit reference: recommendations.jsonl R-002.
        if wrote_local {
            Self::mirror_backup_to_home(werk_dir, &backup_path, &timestamp.to_string());
        }
    }

    /// Compute a stable, readable slug for a workspace's home-mirror
    /// backup directory. The slug is the workspace's absolute path with
    /// path separators and unsafe characters mapped to `_`, leading
    /// underscores trimmed. Two workspaces at different absolute paths
    /// always produce different slugs.
    fn workspace_slug(werk_dir: &std::path::Path) -> String {
        let abs = std::fs::canonicalize(werk_dir).unwrap_or_else(|_| werk_dir.to_path_buf());
        let mut slug: String = abs
            .to_string_lossy()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        while slug.starts_with('_') {
            slug.remove(0);
        }
        // Filesystem-safe length cap (most filesystems cap names at 255 bytes).
        if slug.len() > 200 {
            slug.truncate(200);
        }
        slug
    }

    fn mirror_backup_to_home(
        werk_dir: &std::path::Path,
        src_backup: &std::path::Path,
        timestamp: &str,
    ) {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        // Skip mirror for workspaces under any known tempdir — tests and
        // throwaway workspaces should not accumulate forever in ~/.werk/backups/.
        // env::temp_dir() honors TMPDIR but on macOS that points at
        // /var/folders/... — common ad-hoc tempdirs like /tmp must be listed
        // explicitly. All paths are canonicalized for symlink resolution
        // (macOS /tmp -> /private/tmp).
        if let Ok(abs_werk) = std::fs::canonicalize(werk_dir) {
            for candidate in [
                std::env::temp_dir(),
                std::path::PathBuf::from("/tmp"),
                std::path::PathBuf::from("/private/tmp"),
                std::path::PathBuf::from("/var/folders"),
                std::path::PathBuf::from("/private/var/folders"),
            ] {
                if let Ok(abs_tmp) = std::fs::canonicalize(&candidate)
                    && abs_werk.starts_with(&abs_tmp)
                {
                    return;
                }
            }
        }
        let slug = Self::workspace_slug(werk_dir);
        if slug.is_empty() {
            return;
        }
        let mirror_dir = home.join(".werk").join("backups").join(&slug);
        if std::fs::create_dir_all(&mirror_dir).is_err() {
            return;
        }
        let mirror_path = mirror_dir.join(format!("werk.db.{}", timestamp));
        if !mirror_path.exists() {
            let _ = std::fs::copy(src_backup, &mirror_path);
        }
        // Retention: keep 30 mirror snapshots (more than local's 10 since
        // these are the post-nuke recovery substrate).
        if let Ok(entries) = std::fs::read_dir(&mirror_dir) {
            let mut mirrors: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("werk.db."))
                .collect();
            mirrors.sort_by_key(|e| e.file_name());
            if mirrors.len() > 30 {
                for entry in &mirrors[..mirrors.len() - 30] {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
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
                parent_actual_snapshot TEXT,
                short_code INTEGER
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
                new_value TEXT,
                gesture_id TEXT,
                actual_at TEXT
            )",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create mutations table: {:?}", e))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                summary_note TEXT
            )",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create sessions table: {:?}", e))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS gestures (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                timestamp TEXT NOT NULL,
                description TEXT,
                undone_gesture_id TEXT
            )",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create gestures table: {:?}", e))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS epochs (
                id TEXT PRIMARY KEY,
                tension_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                desire_snapshot TEXT NOT NULL,
                reality_snapshot TEXT NOT NULL,
                children_snapshot_json TEXT,
                trigger_gesture_id TEXT
            )",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create epochs table: {:?}", e))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sigils (
                id INTEGER PRIMARY KEY,
                short_code INTEGER UNIQUE NOT NULL,
                scope_canonical TEXT NOT NULL,
                logic_id TEXT NOT NULL,
                logic_version TEXT NOT NULL,
                seed INTEGER NOT NULL,
                rendered_at TEXT NOT NULL,
                file_path TEXT NOT NULL,
                label TEXT NULL
            )",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create sigils table: {:?}", e))
        })?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_sigils_short_code ON sigils(short_code)")
            .map_err(|e| {
                StoreError::DatabaseError(format!(
                    "failed to create sigils short_code index: {:?}",
                    e
                ))
            })?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_sigils_logic ON sigils(logic_id)")
            .map_err(|e| {
                StoreError::DatabaseError(format!(
                    "failed to create sigils logic_id index: {:?}",
                    e
                ))
            })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS edges (
                id TEXT PRIMARY KEY,
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                gesture_id TEXT,
                UNIQUE(from_id, to_id, edge_type)
            )",
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to create edges table: {:?}", e)))?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id)")
            .map_err(|e| {
                StoreError::DatabaseError(format!("failed to create edges from index: {:?}", e))
            })?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id)")
            .map_err(|e| {
                StoreError::DatabaseError(format!("failed to create edges to index: {:?}", e))
            })?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_edges_type ON edges(edge_type)")
            .map_err(|e| {
                StoreError::DatabaseError(format!("failed to create edges type index: {:?}", e))
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
                &**s == "horizon"
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
                &**s == "position"
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
                &**s == "parent_desired_snapshot"
            } else {
                false
            }
        });

        if !has_parent_desired_snapshot {
            conn.execute("ALTER TABLE tensions ADD COLUMN parent_desired_snapshot TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!(
                        "failed to add parent_desired_snapshot column: {:?}",
                        e
                    ))
                })?;
            conn.execute("ALTER TABLE tensions ADD COLUMN parent_actual_snapshot TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!(
                        "failed to add parent_actual_snapshot column: {:?}",
                        e
                    ))
                })?;
        }

        // Migration: Add short_code to tensions
        let has_short_code = columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                &**s == "short_code"
            } else {
                false
            }
        });

        if !has_short_code {
            conn.execute("ALTER TABLE tensions ADD COLUMN short_code INTEGER")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add short_code column: {:?}", e))
                })?;
            // Backfill short_codes for existing tensions
            let existing = conn
                .query("SELECT id FROM tensions ORDER BY created_at ASC")
                .map_err(|e| {
                    StoreError::DatabaseError(format!(
                        "failed to query tensions for backfill: {:?}",
                        e
                    ))
                })?;
            for (i, row) in existing.iter().enumerate() {
                if let Some(SqliteValue::Text(tid)) = row.get(0) {
                    conn.execute_with_params(
                        "UPDATE tensions SET short_code = ?1 WHERE id = ?2",
                        &[
                            SqliteValue::Integer((i + 1) as i64),
                            SqliteValue::Text(tid.to_string().into()),
                        ],
                    )
                    .map_err(|e| {
                        StoreError::DatabaseError(format!("failed to backfill short_code: {:?}", e))
                    })?;
                }
            }
        }

        // Migration: Add parent_snapshot_json to tensions
        let has_parent_snapshot_json = columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                &**s == "parent_snapshot_json"
            } else {
                false
            }
        });

        if !has_parent_snapshot_json {
            conn.execute("ALTER TABLE tensions ADD COLUMN parent_snapshot_json TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!(
                        "failed to add parent_snapshot_json column: {:?}",
                        e
                    ))
                })?;
        }

        // Migration: Add gesture_id and actual_at to mutations
        let mutation_columns: Vec<fsqlite::Row> =
            conn.query("PRAGMA table_info(mutations)").map_err(|e| {
                StoreError::DatabaseError(format!("failed to query mutations schema: {:?}", e))
            })?;

        let has_gesture_id = mutation_columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                &**s == "gesture_id"
            } else {
                false
            }
        });

        if !has_gesture_id {
            conn.execute("ALTER TABLE mutations ADD COLUMN gesture_id TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add gesture_id column: {:?}", e))
                })?;
        }

        let has_actual_at = mutation_columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                &**s == "actual_at"
            } else {
                false
            }
        });

        if !has_actual_at {
            conn.execute("ALTER TABLE mutations ADD COLUMN actual_at TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add actual_at column: {:?}", e))
                })?;
        }

        // Indexes for query performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_mutations_tension_id ON mutations(tension_id)",
        )
        .map_err(|e| {
            StoreError::DatabaseError(format!("failed to create mutations index: {:?}", e))
        })?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_tensions_parent_id ON tensions(parent_id)")
            .map_err(|e| {
                StoreError::DatabaseError(format!(
                    "failed to create tensions parent index: {:?}",
                    e
                ))
            })?;

        // Migration: Add epoch_type to epochs
        let epoch_columns: Vec<fsqlite::Row> =
            conn.query("PRAGMA table_info(epochs)").map_err(|e| {
                StoreError::DatabaseError(format!("failed to query epochs schema: {:?}", e))
            })?;

        let has_epoch_type = epoch_columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                &**s == "epoch_type"
            } else {
                false
            }
        });

        if !has_epoch_type {
            conn.execute("ALTER TABLE epochs ADD COLUMN epoch_type TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add epoch_type column: {:?}", e))
                })?;
        }

        // Migration: Add agent_type and agent_session_id to sessions
        let session_columns: Vec<fsqlite::Row> =
            conn.query("PRAGMA table_info(sessions)").map_err(|e| {
                StoreError::DatabaseError(format!("failed to query sessions schema: {:?}", e))
            })?;

        let has_agent_type = session_columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                &**s == "agent_type"
            } else {
                false
            }
        });

        if !has_agent_type {
            conn.execute("ALTER TABLE sessions ADD COLUMN agent_type TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!("failed to add agent_type column: {:?}", e))
                })?;
            conn.execute("ALTER TABLE sessions ADD COLUMN agent_session_id TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!(
                        "failed to add agent_session_id column: {:?}",
                        e
                    ))
                })?;
        }

        // Migration: Add undone_gesture_id to gestures (for gesture undo)
        let gesture_columns: Vec<fsqlite::Row> =
            conn.query("PRAGMA table_info(gestures)").map_err(|e| {
                StoreError::DatabaseError(format!("failed to query gestures schema: {:?}", e))
            })?;

        let has_undone_gesture_id = gesture_columns.iter().any(|row| {
            if let Some(SqliteValue::Text(s)) = row.get(1) {
                &**s == "undone_gesture_id"
            } else {
                false
            }
        });

        if !has_undone_gesture_id {
            conn.execute("ALTER TABLE gestures ADD COLUMN undone_gesture_id TEXT")
                .map_err(|e| {
                    StoreError::DatabaseError(format!(
                        "failed to add undone_gesture_id column: {:?}",
                        e
                    ))
                })?;
        }

        // Migration: Populate edges table from existing parent_id relationships.
        // Check if edges are already populated by looking for any contains edges.
        let edge_count: Vec<fsqlite::Row> = conn
            .query("SELECT COUNT(*) FROM edges WHERE edge_type = 'contains'")
            .map_err(|e| StoreError::DatabaseError(format!("failed to count edges: {:?}", e)))?;

        let has_edges = match edge_count.first().and_then(|r| r.get(0)) {
            Some(SqliteValue::Integer(n)) => *n > 0,
            _ => false,
        };

        if !has_edges {
            // Migrate parent_id relationships to contains edges
            let parent_rows: Vec<fsqlite::Row> = conn
                .query("SELECT id, parent_id FROM tensions WHERE parent_id IS NOT NULL")
                .map_err(|e| {
                    StoreError::DatabaseError(format!(
                        "failed to query tensions for edge migration: {:?}",
                        e
                    ))
                })?;

            let now = Utc::now().to_rfc3339();
            for row in &parent_rows {
                if let (Some(SqliteValue::Text(child_id)), Some(SqliteValue::Text(parent_id))) =
                    (row.get(0), row.get(1))
                {
                    let edge_id = ulid::Ulid::new().to_string();
                    conn.execute_with_params(
                        "INSERT OR IGNORE INTO edges (id, from_id, to_id, edge_type, created_at) VALUES (?1, ?2, ?3, 'contains', ?4)",
                        &[
                            SqliteValue::Text(edge_id.into()),
                            SqliteValue::Text(parent_id.to_string().into()),
                            SqliteValue::Text(child_id.to_string().into()),
                            SqliteValue::Text(now.clone().into()),
                        ],
                    )
                    .map_err(|e| {
                        StoreError::DatabaseError(format!(
                            "failed to migrate parent_id to edge: {:?}",
                            e
                        ))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Create a new tension and persist it.
    ///
    /// Generates a ULID id, persists the tension, and records a "created" mutation.
    /// The horizon defaults to None.
    pub fn create_tension(&self, desired: &str, actual: &str) -> Result<Tension, CoreError> {
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
    ) -> Result<Tension, CoreError> {
        self.create_tension_full(desired, actual, parent_id, None)
    }

    /// Create a new tension with all optional fields including horizon.
    ///
    /// Generates a ULID id, persists the tension, and records a "created" mutation.
    /// The creation mutation includes horizon if present.
    /// Automatically captures parent snapshots when parent_id is provided.
    pub fn create_tension_full(
        &self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> Result<Tension, CoreError> {
        let mut tension = Tension::new_full(desired, actual, parent_id, horizon)?;

        // Auto-assign short_code
        tension.short_code = Some(self.next_short_code()?);

        // Auto-capture parent snapshots if creating a child
        if let Some(ref pid) = tension.parent_id
            && let Ok(Some(parent)) = self.get_tension(pid)
        {
            tension.parent_desired_snapshot = Some(parent.desired.clone());
            tension.parent_actual_snapshot = Some(parent.actual.clone());
            // Build full JSON snapshot with children state
            if let Ok(siblings) = self.get_children(pid) {
                let children_json: Vec<serde_json::Value> = siblings
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "desired": c.desired,
                            "actual": c.actual,
                            "status": c.status.to_string(),
                            "position": c.position,
                            "horizon": c.horizon.as_ref().map(|h| h.to_string()),
                        })
                    })
                    .collect();
                let snapshot = serde_json::json!({
                    "desired": parent.desired,
                    "actual": parent.actual,
                    "status": parent.status.to_string(),
                    "horizon": parent.horizon.as_ref().map(|h| h.to_string()),
                    "children": children_json,
                });
                tension.parent_snapshot_json = serde_json::to_string(&snapshot).ok();
            }
        }

        self.persist_tension(&tension)?;

        // Create contains edge if parent exists
        if let Some(ref pid) = tension.parent_id {
            let _ = self.create_edge(pid, &tension.id, crate::edge::EDGE_CONTAINS);
        }

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
    #[allow(clippy::too_many_arguments)]
    pub fn create_tension_full_with_snapshots(
        &self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
        position: Option<i32>,
        parent_desired_snapshot: Option<String>,
        parent_actual_snapshot: Option<String>,
        parent_snapshot_json: Option<String>,
    ) -> Result<Tension, CoreError> {
        let mut tension = Tension::new_full_with_snapshots(
            desired,
            actual,
            parent_id,
            horizon,
            position,
            parent_desired_snapshot,
            parent_actual_snapshot,
            parent_snapshot_json,
        )?;
        tension.short_code = Some(self.next_short_code()?);
        self.persist_tension(&tension)?;

        // Create contains edge if parent exists
        if let Some(ref pid) = tension.parent_id {
            let _ = self.create_edge(pid, &tension.id, crate::edge::EDGE_CONTAINS);
        }

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

    fn persist_tension(&self, tension: &Tension) -> Result<(), CoreError> {
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO tensions (id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot, parent_snapshot_json, short_code) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            &[
                SqliteValue::Text(tension.id.to_string().into()),
                SqliteValue::Text(tension.desired.to_string().into()),
                SqliteValue::Text(tension.actual.to_string().into()),
                match &tension.parent_id {
                    Some(pid) => SqliteValue::Text(pid.to_string().into()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(tension.created_at.to_rfc3339().into()),
                SqliteValue::Text(tension.status.to_string().into()),
                match &tension.horizon {
                    Some(h) => SqliteValue::Text(h.to_string().into()),
                    None => SqliteValue::Null,
                },
                match tension.position {
                    Some(p) => SqliteValue::Integer(p as i64),
                    None => SqliteValue::Null,
                },
                match &tension.parent_desired_snapshot {
                    Some(s) => SqliteValue::Text(s.to_string().into()),
                    None => SqliteValue::Null,
                },
                match &tension.parent_actual_snapshot {
                    Some(s) => SqliteValue::Text(s.to_string().into()),
                    None => SqliteValue::Null,
                },
                match &tension.parent_snapshot_json {
                    Some(s) => SqliteValue::Text(s.to_string().into()),
                    None => SqliteValue::Null,
                },
                match tension.short_code {
                    Some(sc) => SqliteValue::Integer(sc as i64),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| CoreError::ValidationError(format!("failed to persist tension: {:?}", e)))?;
        Ok(())
    }

    /// Record a mutation for a tension.
    ///
    /// This is a low-level method for recording arbitrary mutations.
    /// Most operations automatically record appropriate mutations.
    pub fn record_mutation(&self, mutation: &Mutation) -> Result<(), CoreError> {
        // Use the mutation's gesture_id if set, otherwise fall back to store's active gesture
        let effective_gesture_id = mutation
            .gesture_id()
            .map(|g| g.to_owned())
            .or_else(|| self.active_gesture_id.clone());
        // Use the mutation's actual_at if set, otherwise fall back to store's pending actual_at
        let effective_actual_at = mutation.actual_at().or(self.pending_actual_at);

        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO mutations (tension_id, timestamp, field, old_value, new_value, gesture_id, actual_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                SqliteValue::Text(mutation.tension_id().to_owned().into()),
                SqliteValue::Text(mutation.timestamp().to_rfc3339().into()),
                SqliteValue::Text(mutation.field().to_owned().into()),
                match mutation.old_value() {
                    Some(v) => SqliteValue::Text(v.to_owned().into()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(mutation.new_value().to_owned().into()),
                match &effective_gesture_id {
                    Some(g) => SqliteValue::Text(g.to_string().into()),
                    None => SqliteValue::Null,
                },
                match effective_actual_at {
                    Some(t) => SqliteValue::Text(t.to_rfc3339().into()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| CoreError::ValidationError(format!("failed to record mutation: {:?}", e)))?;
        Ok(())
    }

    /// Record a note on a tension and emit NoteTaken event.
    pub fn record_note(&self, tension_id: &str, text: &str) -> Result<(), CoreError> {
        self.record_mutation(&Mutation::new(
            tension_id.to_owned(),
            Utc::now(),
            "note".to_owned(),
            None,
            text.to_owned(),
        ))?;
        self.emit_event(&EventBuilder::note_taken(
            tension_id.to_owned(),
            text.to_owned(),
        ));
        Ok(())
    }

    /// Record a note retraction on a tension and emit NoteRetracted event.
    pub fn retract_note(
        &self,
        tension_id: &str,
        note_text: &str,
        note_timestamp: &str,
    ) -> Result<(), CoreError> {
        self.record_mutation(&Mutation::new(
            tension_id.to_owned(),
            Utc::now(),
            "note_retracted".to_owned(),
            Some(note_text.to_owned()),
            note_timestamp.to_owned(),
        ))?;
        self.emit_event(&EventBuilder::note_retracted(
            tension_id.to_owned(),
            note_text.to_owned(),
        ));
        Ok(())
    }

    /// Count no-op position mutations where old_value equals new_value.
    /// Use this to preview before purging.
    pub fn count_noop_mutations(&self) -> Result<usize, CoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT COUNT(*) FROM mutations WHERE field = 'position' AND old_value IS NOT NULL AND old_value = new_value")
            .map_err(|e| CoreError::ValidationError(format!("query failed: {:?}", e)))?;
        match rows.first().and_then(|r| r.get(0)) {
            Some(SqliteValue::Integer(n)) => Ok(*n as usize),
            _ => Ok(0),
        }
    }

    /// Delete no-op position mutations where old_value equals new_value.
    /// Scoped to position mutations only — other fields are left untouched.
    /// Returns the number of deleted rows.
    pub fn purge_noop_mutations(&self) -> Result<usize, CoreError> {
        let count = self.count_noop_mutations()?;
        if count > 0 {
            let conn = self.conn.borrow();
            conn.execute("DELETE FROM mutations WHERE field = 'position' AND old_value IS NOT NULL AND old_value = new_value")
                .map_err(|e| CoreError::ValidationError(format!("delete failed: {:?}", e)))?;
        }
        Ok(count)
    }

    /// Get a tension by ID.
    ///
    /// Returns None if the tension doesn't exist.
    pub fn get_tension(&self, id: &str) -> Result<Option<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot, parent_snapshot_json, short_code FROM tensions WHERE id = ?1",
                &[SqliteValue::Text(id.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        let id = match row.get(0) {
            Some(SqliteValue::Text(s)) => s.to_string(),
            _ => return Err(StoreError::DatabaseError("invalid id column".to_owned())),
        };
        let desired = match row.get(1) {
            Some(SqliteValue::Text(s)) => s.to_string(),
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid desired column".to_owned(),
                ));
            }
        };
        let actual = match row.get(2) {
            Some(SqliteValue::Text(s)) => s.to_string(),
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid actual column".to_owned(),
                ));
            }
        };
        let parent_id = match row.get(3) {
            Some(SqliteValue::Text(s)) => Some(s.to_string()),
            Some(SqliteValue::Null) | None => None,
            _ => {
                return Err(StoreError::DatabaseError(
                    "invalid parent_id column".to_owned(),
                ));
            }
        };
        let created_at_str = match row.get(4) {
            Some(SqliteValue::Text(s)) => s.to_string(),
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
            Some(SqliteValue::Text(s)) => s.to_string(),
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
            Some(SqliteValue::Text(s)) => Some(s.to_string()),
            Some(SqliteValue::Null) | None => None,
            _ => None,
        };

        // Parse parent_actual_snapshot (column 9)
        let parent_actual_snapshot = match row.get(9) {
            Some(SqliteValue::Text(s)) => Some(s.to_string()),
            Some(SqliteValue::Null) | None => None,
            _ => None,
        };

        // Parse parent_snapshot_json (column 10)
        let parent_snapshot_json = match row.get(10) {
            Some(SqliteValue::Text(s)) => Some(s.to_string()),
            Some(SqliteValue::Null) | None => None,
            _ => None,
        };

        // Parse short_code (column 11)
        let short_code = match row.get(11) {
            Some(SqliteValue::Integer(n)) => Some(*n as i32),
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
            parent_snapshot_json,
            short_code,
        }))
    }

    /// List all tensions in creation order.
    /// Count total and active tensions without loading all rows.
    pub fn count_tensions(&self) -> Result<(usize, usize), StoreError> {
        let conn = self.conn.borrow();
        let total_rows = conn
            .query("SELECT COUNT(*) FROM tensions")
            .map_err(|e| StoreError::DatabaseError(format!("count query failed: {:?}", e)))?;
        let active_rows = conn
            .query("SELECT COUNT(*) FROM tensions WHERE status = 'Active'")
            .map_err(|e| StoreError::DatabaseError(format!("count query failed: {:?}", e)))?;

        let total = total_rows
            .first()
            .and_then(|r| r.get(0))
            .and_then(|v| {
                if let SqliteValue::Integer(n) = v {
                    Some(*n as usize)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let active = active_rows
            .first()
            .and_then(|r| r.get(0))
            .and_then(|v| {
                if let SqliteValue::Integer(n) = v {
                    Some(*n as usize)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        Ok((total, active))
    }

    /// Check which tension IDs have children, returning a set of parent IDs.
    /// Count children per parent for a batch of tension IDs.
    pub fn count_children_by_parent(
        &self,
        parent_ids: &[&str],
    ) -> Result<std::collections::HashMap<String, usize>, StoreError> {
        if parent_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let conn = self.conn.borrow();
        let placeholders: Vec<String> = (1..=parent_ids.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT parent_id, COUNT(*) FROM tensions WHERE parent_id IN ({}) GROUP BY parent_id",
            placeholders.join(", ")
        );
        let params: Vec<SqliteValue> = parent_ids
            .iter()
            .map(|id| SqliteValue::Text(id.to_string().into()))
            .collect();
        let rows = conn.query_with_params(&sql, &params).map_err(|e| {
            StoreError::DatabaseError(format!("batch children count failed: {:?}", e))
        })?;

        let mut result = std::collections::HashMap::new();
        for row in &rows {
            if let (Some(SqliteValue::Text(pid)), Some(SqliteValue::Integer(count))) =
                (row.get(0), row.get(1))
            {
                result.insert(pid.to_string(), *count as usize);
            }
        }
        Ok(result)
    }

    /// Get last mutation timestamp per tension for a batch of tension IDs, filtered by field.
    pub fn get_last_mutation_timestamps(
        &self,
        tension_ids: &[&str],
        fields: &[&str],
    ) -> Result<std::collections::HashMap<String, chrono::DateTime<chrono::Utc>>, StoreError> {
        if tension_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let conn = self.conn.borrow();
        let id_placeholders: Vec<String> =
            (1..=tension_ids.len()).map(|i| format!("?{}", i)).collect();
        let field_placeholders: Vec<String> = (tension_ids.len() + 1
            ..=tension_ids.len() + fields.len())
            .map(|i| format!("?{}", i))
            .collect();
        let sql = format!(
            "SELECT tension_id, MAX(timestamp) FROM mutations WHERE tension_id IN ({}) AND field IN ({}) GROUP BY tension_id",
            id_placeholders.join(", "),
            field_placeholders.join(", ")
        );
        let mut params: Vec<SqliteValue> = tension_ids
            .iter()
            .map(|id| SqliteValue::Text(id.to_string().into()))
            .collect();
        for f in fields {
            params.push(SqliteValue::Text(f.to_string().into()));
        }
        let rows = conn.query_with_params(&sql, &params).map_err(|e| {
            StoreError::DatabaseError(format!("batch mutation query failed: {:?}", e))
        })?;

        let mut result = std::collections::HashMap::new();
        for row in &rows {
            if let (Some(SqliteValue::Text(tid)), Some(SqliteValue::Text(ts))) =
                (row.get(0), row.get(1))
                && let Ok(dt) = ts.parse::<chrono::DateTime<chrono::Utc>>()
            {
                result.insert(tid.to_string(), dt);
            }
        }
        Ok(result)
    }

    pub fn list_tensions(&self) -> Result<Vec<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot, parent_snapshot_json, short_code FROM tensions ORDER BY created_at ASC")
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_tension_rows(rows)
    }

    /// Insert a sigil metadata record (no SVG bytes stored).
    pub fn record_sigil(&self, record: &SigilRecord) -> Result<(), StoreError> {
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO sigils (short_code, scope_canonical, logic_id, logic_version, seed, rendered_at, file_path, label) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            &[
                SqliteValue::Integer(record.short_code as i64),
                SqliteValue::Text(record.scope_canonical.clone().into()),
                SqliteValue::Text(record.logic_id.clone().into()),
                SqliteValue::Text(record.logic_version.clone().into()),
                SqliteValue::Integer(record.seed),
                SqliteValue::Text(record.rendered_at.to_rfc3339().into()),
                SqliteValue::Text(record.file_path.clone().into()),
                match &record.label {
                    Some(label) => SqliteValue::Text(label.clone().into()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map(|_| ())
        .map_err(|e| StoreError::DatabaseError(format!("failed to insert sigil record: {:?}", e)))
    }

    /// List all sigils in chronological order (rendered_at ascending).
    pub fn list_sigils(&self) -> Result<Vec<SigilRecord>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query(
                "SELECT id, short_code, scope_canonical, logic_id, logic_version, seed, rendered_at, file_path, label FROM sigils ORDER BY rendered_at ASC, id ASC",
            )
            .map_err(|e| StoreError::DatabaseError(format!("sigils query failed: {:?}", e)))?;

        self.parse_sigil_rows(rows)
    }

    /// Fetch a sigil by short code.
    pub fn get_sigil_by_short_code(
        &self,
        short_code: i32,
    ) -> Result<Option<SigilRecord>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, short_code, scope_canonical, logic_id, logic_version, seed, rendered_at, file_path, label FROM sigils WHERE short_code = ?1",
                &[SqliteValue::Integer(short_code as i64)],
            )
            .map_err(|e| StoreError::DatabaseError(format!("sigil lookup failed: {:?}", e)))?;

        let mut records = self.parse_sigil_rows(rows)?;
        Ok(records.pop())
    }

    /// Delete a sigil metadata row by short code. Returns true if a row was removed.
    pub fn delete_sigil(&self, short_code: i32) -> Result<bool, StoreError> {
        let conn = self.conn.borrow();
        let pre_check = conn
            .query_with_params(
                "SELECT COUNT(*) FROM sigils WHERE short_code = ?1",
                &[SqliteValue::Integer(short_code as i64)],
            )
            .map_err(|e| {
                StoreError::DatabaseError(format!("sigil delete check failed: {:?}", e))
            })?;
        let existed = match pre_check.first().and_then(|r| r.get(0)) {
            Some(SqliteValue::Integer(n)) => *n > 0,
            _ => false,
        };
        if !existed {
            return Ok(false);
        }

        conn.execute_with_params(
            "DELETE FROM sigils WHERE short_code = ?1",
            &[SqliteValue::Integer(short_code as i64)],
        )
        .map_err(|e| StoreError::DatabaseError(format!("sigil delete failed: {:?}", e)))?;

        let check = conn
            .query_with_params(
                "SELECT COUNT(*) FROM sigils WHERE short_code = ?1",
                &[SqliteValue::Integer(short_code as i64)],
            )
            .map_err(|e| {
                StoreError::DatabaseError(format!("sigil delete check failed: {:?}", e))
            })?;

        let still_exists = match check.first().and_then(|r| r.get(0)) {
            Some(SqliteValue::Integer(n)) => *n > 0,
            _ => false,
        };

        Ok(!still_exists)
    }

    /// Get all root tensions (those with no parent_id).
    pub fn get_roots(&self) -> Result<Vec<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot, parent_snapshot_json, short_code FROM tensions WHERE parent_id IS NULL ORDER BY position DESC NULLS LAST, created_at ASC")
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_tension_rows(rows)
    }

    /// Get all children of a given parent.
    pub fn get_children(&self, parent_id: &str) -> Result<Vec<Tension>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, desired, actual, parent_id, created_at, status, horizon, position, parent_desired_snapshot, parent_actual_snapshot, parent_snapshot_json, short_code FROM tensions WHERE parent_id = ?1 ORDER BY position DESC NULLS LAST, created_at ASC",
                &[SqliteValue::Text(parent_id.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_tension_rows(rows)
    }

    /// Get all descendant IDs of a tension (recursive children).
    ///
    /// Returns IDs of children, grandchildren, etc. Does NOT include the
    /// tension itself.
    pub fn get_descendant_ids(&self, tension_id: &str) -> Result<Vec<String>, StoreError> {
        let mut result = Vec::new();
        let mut queue = vec![tension_id.to_owned()];
        while let Some(parent) = queue.pop() {
            let children = self.get_children(&parent)?;
            for child in children {
                result.push(child.id.clone());
                queue.push(child.id);
            }
        }
        Ok(result)
    }

    /// Get mutations for a tension and all its descendants within a time range.
    ///
    /// Returns mutations in chronological order. The range is inclusive on
    /// start and exclusive on end (start <= timestamp < end).
    pub fn get_epoch_mutations(
        &self,
        tension_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Mutation>, StoreError> {
        // Collect all relevant IDs: the tension itself + descendants
        let mut ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        ids.insert(tension_id.to_owned());
        for desc_id in self.get_descendant_ids(tension_id)? {
            ids.insert(desc_id);
        }

        // Get all mutations in the time range, then filter by IDs
        let all = self.mutations_between(start, end)?;
        let filtered: Vec<Mutation> = all
            .into_iter()
            .filter(|m| ids.contains(m.tension_id()))
            .collect();
        Ok(filtered)
    }

    fn parse_tension_rows(&self, rows: Vec<fsqlite::Row>) -> Result<Vec<Tension>, StoreError> {
        let mut tensions = Vec::new();
        for row in &rows {
            let id = match row.get(0) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => return Err(StoreError::DatabaseError("invalid id column".to_owned())),
            };
            let desired = match row.get(1) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid desired column".to_owned(),
                    ));
                }
            };
            let actual = match row.get(2) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid actual column".to_owned(),
                    ));
                }
            };
            let parent_id = match row.get(3) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid parent_id column".to_owned(),
                    ));
                }
            };
            let created_at_str = match row.get(4) {
                Some(SqliteValue::Text(s)) => s.to_string(),
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
                Some(SqliteValue::Text(s)) => s.to_string(),
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
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            // Parse parent_actual_snapshot (column 9)
            let parent_actual_snapshot = match row.get(9) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            // Parse parent_snapshot_json (column 10)
            let parent_snapshot_json = match row.get(10) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            // Parse short_code (column 11)
            let short_code = match row.get(11) {
                Some(SqliteValue::Integer(n)) => Some(*n as i32),
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
                parent_snapshot_json,
                short_code,
            });
        }

        Ok(tensions)
    }

    fn parse_sigil_rows(&self, rows: Vec<fsqlite::Row>) -> Result<Vec<SigilRecord>, StoreError> {
        let mut sigils = Vec::new();
        for row in &rows {
            let id = match row.get(0) {
                Some(SqliteValue::Integer(n)) => *n,
                _ => return Err(StoreError::DatabaseError("invalid sigil id".to_owned())),
            };
            let short_code = match row.get(1) {
                Some(SqliteValue::Integer(n)) => i32::try_from(*n).map_err(|_| {
                    StoreError::DatabaseError("invalid sigil short_code".to_owned())
                })?,
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid sigil short_code".to_owned(),
                    ));
                }
            };
            let scope_canonical = match row.get(2) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid sigil scope_canonical".to_owned(),
                    ));
                }
            };
            let logic_id = match row.get(3) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid sigil logic_id".to_owned(),
                    ));
                }
            };
            let logic_version = match row.get(4) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid sigil logic_version".to_owned(),
                    ));
                }
            };
            let seed = match row.get(5) {
                Some(SqliteValue::Integer(n)) => *n,
                _ => return Err(StoreError::DatabaseError("invalid sigil seed".to_owned())),
            };
            let rendered_at_str = match row.get(6) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid sigil rendered_at".to_owned(),
                    ));
                }
            };
            let rendered_at = DateTime::parse_from_rfc3339(&rendered_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    StoreError::DatabaseError(format!("invalid sigil rendered_at: {}", e))
                })?;
            let file_path = match row.get(7) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid sigil file_path".to_owned(),
                    ));
                }
            };
            let label = match row.get(8) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            sigils.push(SigilRecord {
                id,
                short_code,
                scope_canonical,
                logic_id,
                logic_version,
                seed,
                rendered_at,
                file_path,
                label,
            });
        }

        Ok(sigils)
    }

    /// Update the desired state of a tension.
    ///
    /// Persists the change and records a mutation.
    pub fn update_desired(&self, id: &str, new_desired: &str) -> Result<(), CoreError> {
        self.update_field(id, "desired", new_desired)
    }

    /// Update the actual state of a tension.
    ///
    /// Persists the change and records a mutation.
    pub fn update_actual(&self, id: &str, new_actual: &str) -> Result<(), CoreError> {
        self.update_field(id, "actual", new_actual)
    }

    /// Update the actual state of a tension without starting a transaction.
    ///
    /// For use within an already-active transaction. Call `begin_transaction()`
    /// before using this method, and `commit_transaction()` after all updates.
    pub fn update_actual_no_tx(&self, id: &str, new_actual: &str) -> Result<(), CoreError> {
        if new_actual.is_empty() {
            return Err(CoreError::ValidationError(
                "actual cannot be empty".to_owned(),
            ));
        }

        let mut tension = self
            .get_tension(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?
            .ok_or_else(|| CoreError::ValidationError(format!("tension not found: {}", id)))?;

        if tension.status != TensionStatus::Active {
            return Err(CoreError::UpdateOnInactiveTension(tension.status));
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
    pub fn update_parent(&self, id: &str, new_parent_id: Option<&str>) -> Result<(), CoreError> {
        let mut tension = self
            .get_tension(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?
            .ok_or_else(|| CoreError::ValidationError(format!("tension not found: {}", id)))?;

        let old_parent = tension.parent_id.clone();
        let new_parent = new_parent_id.map(|s| s.to_owned());
        tension.parent_id = new_parent.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN CONCURRENT;").map_err(|e| {
                CoreError::ValidationError(format!("failed to begin transaction: {:?}", e))
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
                    Self::commit_with_retry(&conn)?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Update edges: remove old contains edge, create new one
        if let Some(ref old_pid) = old_parent {
            let _ = self.remove_edge(old_pid, id, crate::edge::EDGE_CONTAINS);
        }
        if let Some(ref new_pid) = new_parent {
            let _ = self.create_edge(new_pid, id, crate::edge::EDGE_CONTAINS);
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
    pub fn update_horizon(&self, id: &str, new_horizon: Option<Horizon>) -> Result<(), CoreError> {
        let mut tension = self
            .get_tension(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?
            .ok_or_else(|| CoreError::ValidationError(format!("tension not found: {}", id)))?;

        // Validate that the tension is Active
        if tension.status != TensionStatus::Active {
            return Err(CoreError::UpdateOnInactiveTension(tension.status));
        }

        let old_horizon = tension.horizon.clone();
        tension.horizon = new_horizon.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN CONCURRENT;").map_err(|e| {
                CoreError::ValidationError(format!("failed to begin transaction: {:?}", e))
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
                    Self::commit_with_retry(&conn)?;
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
    pub fn update_status(&self, id: &str, new_status: TensionStatus) -> Result<(), CoreError> {
        let mut tension = self
            .get_tension(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?
            .ok_or_else(|| CoreError::ValidationError(format!("tension not found: {}", id)))?;

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
                return Err(CoreError::InvalidStatusTransition {
                    from: old_status,
                    to: new_status,
                });
            }
        }

        tension.status = new_status;

        // Check if this tension has children that need auto-resolving
        let children = self
            .get_children(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?;
        let needs_child_resolve = !children.is_empty()
            && (new_status == TensionStatus::Resolved || new_status == TensionStatus::Released);

        // Collect all active children (and their descendants) for recursive resolution
        let mut children_to_resolve: Vec<Tension> = Vec::new();
        if needs_child_resolve {
            let mut stack: Vec<Tension> = children
                .iter()
                .filter(|c| c.status == TensionStatus::Active)
                .cloned()
                .collect();
            while let Some(child) = stack.pop() {
                children_to_resolve.push(child.clone());
                // Get grandchildren for recursive resolution
                if let Ok(grandchildren) = self.get_children(&child.id) {
                    for gc in grandchildren {
                        if gc.status == TensionStatus::Active {
                            stack.push(gc);
                        }
                    }
                }
            }
        }

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN CONCURRENT;").map_err(|e| {
                CoreError::ValidationError(format!("failed to begin transaction: {:?}", e))
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
                    // Auto-resolve active children (recursively) under the same gesture
                    if needs_child_resolve {
                        let now = Utc::now();
                        for child in &children_to_resolve {
                            conn.execute_with_params(
                                "UPDATE tensions SET status = ?1 WHERE id = ?2",
                                &[
                                    SqliteValue::Text(new_status.to_string().into()),
                                    SqliteValue::Text(child.id.to_string().into()),
                                ],
                            )
                            .map_err(|e| {
                                CoreError::ValidationError(format!(
                                    "failed to resolve child: {:?}",
                                    e
                                ))
                            })?;

                            // Record status mutation for the child
                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new(
                                    child.id.clone(),
                                    now,
                                    "status".to_owned(),
                                    Some(child.status.to_string()),
                                    new_status.to_string(),
                                ),
                            )?;
                        }
                    }
                    Ok(())
                });

            match result {
                Ok(_) => {
                    Self::commit_with_retry(&conn)?;
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

    fn update_field(&self, id: &str, field: &str, new_value: &str) -> Result<(), CoreError> {
        if new_value.is_empty() {
            return Err(CoreError::ValidationError(format!(
                "{} cannot be empty",
                field
            )));
        }

        let mut tension = self
            .get_tension(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?
            .ok_or_else(|| CoreError::ValidationError(format!("tension not found: {}", id)))?;

        if tension.status != TensionStatus::Active {
            return Err(CoreError::UpdateOnInactiveTension(tension.status));
        }

        let old_value = match field {
            "desired" => tension.update_desired(new_value)?,
            "actual" => tension.update_actual(new_value)?,
            _ => {
                return Err(CoreError::ValidationError(format!(
                    "unknown field: {}",
                    field
                )));
            }
        };
        let old_value_for_event = old_value.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN CONCURRENT;").map_err(|e| {
                CoreError::ValidationError(format!("failed to begin transaction: {:?}", e))
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
                    Self::commit_with_retry(&conn)?;
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
    ) -> Result<(), CoreError> {
        conn.execute_with_params(
            "UPDATE tensions SET desired = ?1, actual = ?2, parent_id = ?3, status = ?4, horizon = ?5 WHERE id = ?6",
            &[
                SqliteValue::Text(tension.desired.to_string().into()),
                SqliteValue::Text(tension.actual.to_string().into()),
                match &tension.parent_id {
                    Some(pid) => SqliteValue::Text(pid.to_string().into()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(tension.status.to_string().into()),
                match &tension.horizon {
                    Some(h) => SqliteValue::Text(h.to_string().into()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(tension.id.to_string().into()),
            ],
        )
        .map_err(|e| CoreError::ValidationError(format!("failed to update tension: {:?}", e)))?;
        Ok(())
    }

    fn record_mutation_in_transaction(
        &self,
        conn: &Connection,
        mutation: &Mutation,
    ) -> Result<(), CoreError> {
        let effective_gesture_id = mutation
            .gesture_id()
            .map(|g| g.to_owned())
            .or_else(|| self.active_gesture_id.clone());
        let effective_actual_at = mutation.actual_at().or(self.pending_actual_at);

        conn.execute_with_params(
            "INSERT INTO mutations (tension_id, timestamp, field, old_value, new_value, gesture_id, actual_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                SqliteValue::Text(mutation.tension_id().to_owned().into()),
                SqliteValue::Text(mutation.timestamp().to_rfc3339().into()),
                SqliteValue::Text(mutation.field().to_owned().into()),
                match mutation.old_value() {
                    Some(v) => SqliteValue::Text(v.to_owned().into()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(mutation.new_value().to_owned().into()),
                match &effective_gesture_id {
                    Some(g) => SqliteValue::Text(g.to_string().into()),
                    None => SqliteValue::Null,
                },
                match effective_actual_at {
                    Some(t) => SqliteValue::Text(t.to_rfc3339().into()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| CoreError::ValidationError(format!("failed to record mutation: {:?}", e)))?;
        Ok(())
    }

    /// Get all mutations for a tension in chronological order.
    pub fn get_mutations(&self, tension_id: &str) -> Result<Vec<Mutation>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT tension_id, timestamp, field, old_value, new_value, gesture_id, actual_at FROM mutations WHERE tension_id = ?1 ORDER BY timestamp ASC",
                &[SqliteValue::Text(tension_id.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_mutation_rows(rows)
    }

    /// Get all mutations across all tensions in chronological order.
    pub fn all_mutations(&self) -> Result<Vec<Mutation>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT tension_id, timestamp, field, old_value, new_value, gesture_id, actual_at FROM mutations ORDER BY timestamp ASC")
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
                "SELECT tension_id, timestamp, field, old_value, new_value, gesture_id, actual_at FROM mutations WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp ASC",
                &[
                    SqliteValue::Text(start.to_rfc3339().into()),
                    SqliteValue::Text(end.to_rfc3339().into()),
                ],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_mutation_rows(rows)
    }

    fn parse_mutation_rows(&self, rows: Vec<fsqlite::Row>) -> Result<Vec<Mutation>, StoreError> {
        let mut mutations = Vec::new();
        for row in &rows {
            let tension_id = match row.get(0) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid tension_id column".to_owned(),
                    ));
                }
            };
            let timestamp_str = match row.get(1) {
                Some(SqliteValue::Text(s)) => s.to_string(),
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
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => return Err(StoreError::DatabaseError("invalid field column".to_owned())),
            };
            let old_value = match row.get(3) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid old_value column".to_owned(),
                    ));
                }
            };
            let new_value = match row.get(4) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid new_value column".to_owned(),
                    ));
                }
            };

            let gesture_id = match row.get(5) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            let actual_at = match row.get(6) {
                Some(SqliteValue::Text(s)) => DateTime::parse_from_rfc3339(s)
                    .map(|dt| Some(dt.with_timezone(&Utc)))
                    .unwrap_or(None),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            mutations.push(Mutation::new_with_gesture(
                tension_id, timestamp, field, old_value, new_value, gesture_id, actual_at,
            ));
        }

        Ok(mutations)
    }

    /// Get the database path (None for in-memory stores).
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    /// Force a WAL checkpoint (TRUNCATE mode) so the `werk.db` file
    /// reflects all committed bytes and the `werk.db-wal` sidecar is
    /// reset to length zero.
    ///
    /// Called by the doctor before backing up the DB triplet — without
    /// it, the live `werk.db` bytes may lag the WAL and a copy of the
    /// three files captured at different instants can be internally
    /// inconsistent. TRUNCATE is best-effort: under contention it may
    /// return SQLITE_BUSY, in which case we surface the error so the
    /// caller can choose to retry or fall through to a triplet copy
    /// (still correct under WAL replay; just larger).
    ///
    /// Idempotent. Safe to call on an empty WAL.
    pub fn wal_checkpoint_truncate(&self) -> Result<(), CoreError> {
        let conn = self.conn.borrow();
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE);")
            .map(|_| ())
            .map_err(|e| {
                CoreError::ValidationError(format!(
                    "wal_checkpoint(TRUNCATE) failed: {:?}",
                    e
                ))
            })
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

    /// Begin a gesture. Creates the gesture record and sets it as active.
    /// All subsequent mutations will be linked to this gesture until
    /// `end_gesture()` is called or a new gesture is begun.
    ///
    /// The gesture is sessionless by default. Use `begin_gesture_in_session`
    /// to associate with a specific session (e.g., from a TUI instance).
    /// Sessions are process-scoped — a CLI command should not inherit a
    /// TUI's active session.
    pub fn begin_gesture(&mut self, description: Option<&str>) -> Result<String, StoreError> {
        let gesture_id = self.create_gesture(None, description)?;
        self.active_gesture_id = Some(gesture_id.clone());
        Ok(gesture_id)
    }

    /// Begin a gesture within a specific session.
    /// Used by TUI instances that manage their own session lifecycle.
    pub fn begin_gesture_in_session(
        &mut self,
        session_id: &str,
        description: Option<&str>,
    ) -> Result<String, StoreError> {
        let gesture_id = self.create_gesture(Some(session_id), description)?;
        self.active_gesture_id = Some(gesture_id.clone());
        Ok(gesture_id)
    }

    /// End the current gesture, returning its ID.
    pub fn end_gesture(&mut self) -> Option<String> {
        self.active_gesture_id.take()
    }

    /// Get the currently active gesture ID, if any.
    pub fn active_gesture(&self) -> Option<&str> {
        self.active_gesture_id.as_deref()
    }

    /// Set a pending actual_at for subsequent mutations.
    /// Supports "I did this yesterday" — the gap between actual_at and
    /// the mutation timestamp is engagement pattern data.
    pub fn set_actual_at(&mut self, actual_at: DateTime<Utc>) {
        self.pending_actual_at = Some(actual_at);
    }

    /// Clear the pending actual_at.
    pub fn clear_actual_at(&mut self) {
        self.pending_actual_at = None;
    }

    /// Get the next available short_code for a new tension.
    fn next_short_code(&self) -> Result<i32, CoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT MAX(short_code) FROM tensions")
            .map_err(|e| {
                CoreError::ValidationError(format!("failed to get max short_code: {:?}", e))
            })?;
        match rows.first().and_then(|r| r.get(0)) {
            Some(SqliteValue::Integer(n)) => Ok((*n as i32) + 1),
            Some(SqliteValue::Null) | None => Ok(1),
            _ => Ok(1),
        }
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
        conn.execute("BEGIN CONCURRENT;")
            .map(|_| ())
            .map_err(|e| StoreError::DatabaseError(format!("failed to begin transaction: {:?}", e)))
    }

    /// Commit the current transaction with retry for MVCC conflicts.
    pub fn commit_transaction(&self) -> Result<(), StoreError> {
        let conn = self.conn.borrow();
        Self::commit_with_retry(&conn)
            .map_err(|e| StoreError::DatabaseError(format!("failed to commit transaction: {}", e)))
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
    pub fn delete_tension(&self, id: &str) -> Result<(), CoreError> {
        // Get the tension to delete
        let tension = self
            .get_tension(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?
            .ok_or_else(|| CoreError::ValidationError(format!("tension not found: {}", id)))?;

        // Get all children of this tension
        let children = self
            .get_children(id)
            .map_err(|e| CoreError::ValidationError(e.to_string()))?;

        // The grandparent is the deleted tension's parent_id
        let grandparent_id = tension.parent_id.clone();

        // Persist in transaction
        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN CONCURRENT;").map_err(|e| {
                CoreError::ValidationError(format!("failed to begin transaction: {:?}", e))
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
                                Some(gp) => SqliteValue::Text(gp.to_string().into()),
                                None => SqliteValue::Null,
                            },
                            SqliteValue::Text(child.id.to_string().into()),
                        ],
                    )
                    .map_err(|e| {
                        CoreError::ValidationError(format!("failed to reparent child: {:?}", e))
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

                conn.execute_with_params(
                    "DELETE FROM tensions WHERE id = ?1",
                    &[SqliteValue::Text(tension.id.to_string().into())],
                )
                .map_err(|e| {
                    CoreError::ValidationError(format!("failed to delete tension: {:?}", e))
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
                    Self::commit_with_retry(&conn)?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Clean up edges: remove all edges involving this tension
        // and update contains edges for reparented children
        if let Some(ref old_pid) = grandparent_id {
            // Remove old contains edge from deleted tension's parent
            let _ = self.remove_edge(old_pid, id, crate::edge::EDGE_CONTAINS);
        }
        for child in &children {
            // Remove contains edge from deleted tension to child
            let _ = self.remove_edge(id, &child.id, crate::edge::EDGE_CONTAINS);
            // Create new contains edge from grandparent to child
            if let Some(ref gp_id) = grandparent_id {
                let _ = self.create_edge(gp_id, &child.id, crate::edge::EDGE_CONTAINS);
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
    /// Returns true if the position was actually changed, false if it was already the target value.
    pub fn update_position(&self, id: &str, new_position: Option<i32>) -> Result<bool, CoreError> {
        let conn = self.conn.borrow();

        // Get existing tension
        let rows = conn
            .query_with_params(
                "SELECT position FROM tensions WHERE id = ?1",
                &[SqliteValue::Text(id.to_owned().into())],
            )
            .map_err(|e| CoreError::ValidationError(format!("query failed: {:?}", e)))?;

        if rows.is_empty() {
            return Err(CoreError::ValidationError(format!(
                "tension not found: {}",
                id
            )));
        }

        let old_position = match rows[0].get(0) {
            Some(SqliteValue::Integer(n)) => Some(*n as i32),
            _ => None,
        };

        // No-op guard: skip if position isn't actually changing
        if old_position == new_position {
            return Ok(false);
        }

        // Update in database
        conn.execute_with_params(
            "UPDATE tensions SET position = ?1 WHERE id = ?2",
            &[
                match new_position {
                    Some(p) => SqliteValue::Integer(p as i64),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(id.to_owned().into()),
            ],
        )
        .map_err(|e| CoreError::ValidationError(format!("failed to update position: {:?}", e)))?;

        // Record mutation
        self.record_mutation(&crate::mutation::Mutation::new(
            id.to_owned(),
            Utc::now(),
            "position".to_owned(),
            old_position.map(|p| p.to_string()),
            new_position
                .map(|p| p.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ))?;

        Ok(true)
    }

    /// Reorder siblings by assigning positions to all children of a parent.
    ///
    /// Takes a list of tension IDs in the desired order. Assigns sequential
    /// positions starting from 1. Records a mutation for each position change.
    pub fn reorder_siblings(&self, ordered_ids: &[String]) -> Result<(), CoreError> {
        for (i, id) in ordered_ids.iter().enumerate() {
            let position = (i + 1) as i32;
            self.update_position(id, Some(position))?;
        }
        Ok(())
    }

    // ── Session (run) management ───────────────────────────────────

    /// Start a new session. Returns the session ID.
    pub fn start_session(&self) -> Result<String, StoreError> {
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO sessions (id, started_at) VALUES (?1, ?2)",
            &[
                SqliteValue::Text(id.to_string().into()),
                SqliteValue::Text(now.to_rfc3339().into()),
            ],
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to start session: {:?}", e)))?;
        Ok(id)
    }

    /// End a session, optionally with a summary note.
    pub fn end_session(&self, id: &str, summary_note: Option<&str>) -> Result<(), StoreError> {
        let now = Utc::now();
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "UPDATE sessions SET ended_at = ?1, summary_note = ?2 WHERE id = ?3",
            &[
                SqliteValue::Text(now.to_rfc3339().into()),
                match summary_note {
                    Some(s) => SqliteValue::Text(s.to_owned().into()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(id.to_owned().into()),
            ],
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to end session: {:?}", e)))?;
        Ok(())
    }

    /// Get the currently active session (started but not ended), if any.
    pub fn active_session(&self) -> Result<Option<String>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query(
                "SELECT id FROM sessions WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;
        if rows.is_empty() {
            return Ok(None);
        }
        match rows[0].get(0) {
            Some(SqliteValue::Text(s)) => Ok(Some(s.to_string())),
            _ => Ok(None),
        }
    }

    // ── Gesture management ─────────────────────────────────────────

    /// Create a new gesture, optionally within a session.
    pub fn create_gesture(
        &self,
        session_id: Option<&str>,
        description: Option<&str>,
    ) -> Result<String, StoreError> {
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO gestures (id, session_id, timestamp, description) VALUES (?1, ?2, ?3, ?4)",
            &[
                SqliteValue::Text(id.to_string().into()),
                match session_id {
                    Some(s) => SqliteValue::Text(s.to_owned().into()),
                    None => SqliteValue::Null,
                },
                SqliteValue::Text(now.to_rfc3339().into()),
                match description {
                    Some(d) => SqliteValue::Text(d.to_owned().into()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to create gesture: {:?}", e)))?;
        Ok(id)
    }

    /// Get all mutations belonging to a gesture.
    pub fn get_gesture_mutations(&self, gesture_id: &str) -> Result<Vec<Mutation>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT tension_id, timestamp, field, old_value, new_value, gesture_id, actual_at FROM mutations WHERE gesture_id = ?1 ORDER BY timestamp ASC",
                &[SqliteValue::Text(gesture_id.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;
        self.parse_mutation_rows(rows)
    }

    /// Get the most recent gesture ID (by timestamp).
    pub fn get_last_gesture_id(&self) -> Result<Option<String>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT id FROM gestures ORDER BY timestamp DESC LIMIT 1")
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;
        match rows.first().and_then(|r| r.get(0)) {
            Some(SqliteValue::Text(s)) => Ok(Some(s.to_string())),
            _ => Ok(None),
        }
    }

    /// Undo a gesture by appending reversal mutations.
    ///
    /// Creates a new gesture G' ("undo of G") containing reverse mutations
    /// for every mutation in the original gesture. The original mutations
    /// are never deleted — undo is append-only.
    ///
    /// # Conflict Detection
    ///
    /// Before applying any reversals, checks that every mutation's `new_value`
    /// still matches the field's current value. If any field has been changed
    /// by another gesture since, returns an error listing the conflicts.
    /// No partial reversals — gesture atomicity is sacred.
    ///
    /// # Returns
    ///
    /// The undo gesture ID on success.
    pub fn undo_gesture(&self, gesture_id: &str) -> Result<String, CoreError> {
        use crate::edge;
        use crate::tension::TensionStatus;

        // 1. Check gesture exists and isn't already undone
        let conn = self.conn.borrow();
        let gesture_rows = conn
            .query_with_params(
                "SELECT id, undone_gesture_id FROM gestures WHERE id = ?1",
                &[SqliteValue::Text(gesture_id.to_owned().into())],
            )
            .map_err(|e| CoreError::ValidationError(format!("failed to query gesture: {:?}", e)))?;

        if gesture_rows.is_empty() {
            return Err(CoreError::ValidationError(format!(
                "gesture not found: {}",
                gesture_id
            )));
        }

        // Check if this gesture is already undone (another gesture has undone_gesture_id pointing to it)
        let already_undone = conn
            .query_with_params(
                "SELECT id FROM gestures WHERE undone_gesture_id = ?1",
                &[SqliteValue::Text(gesture_id.to_owned().into())],
            )
            .map_err(|e| {
                CoreError::ValidationError(format!("failed to check undo status: {:?}", e))
            })?;

        if !already_undone.is_empty() {
            return Err(CoreError::ValidationError(format!(
                "gesture {} is already undone",
                gesture_id
            )));
        }
        drop(conn);

        // 2. Get all mutations for this gesture
        let mutations = self.get_gesture_mutations(gesture_id).map_err(|e| {
            CoreError::ValidationError(format!("failed to get gesture mutations: {}", e))
        })?;

        if mutations.is_empty() {
            return Err(CoreError::ValidationError(format!(
                "gesture {} has no mutations",
                gesture_id
            )));
        }

        // 3. Conflict detection — verify every mutation's new_value matches current state
        let mut conflicts: Vec<String> = Vec::new();

        for m in &mutations {
            let field = m.field();

            // Skip fields that don't have a reversible current-value check
            match field {
                "note" => continue, // notes are append-only, retraction doesn't need conflict check
                "created" => {
                    // For creation: check that no other gestures have touched this tension
                    if m.old_value().is_none() {
                        let conn = self.conn.borrow();
                        let other_mutations = conn
                            .query_with_params(
                                "SELECT COUNT(*) FROM mutations WHERE tension_id = ?1 AND gesture_id != ?2",
                                &[
                                    SqliteValue::Text(m.tension_id().to_owned().into()),
                                    SqliteValue::Text(gesture_id.to_owned().into()),
                                ],
                            )
                            .map_err(|e| CoreError::ValidationError(format!("query failed: {:?}", e)))?;
                        let count = match other_mutations.first().and_then(|r| r.get(0)) {
                            Some(SqliteValue::Integer(n)) => *n,
                            _ => 0,
                        };
                        if count > 0 {
                            conflicts.push(format!(
                                "tension {} has {} mutations from other gestures — cannot undo creation",
                                m.tension_id(), count
                            ));
                        }
                    }
                    continue;
                }
                "deleted" => {
                    conflicts.push(format!(
                        "undo of deletion is not supported in v1 (tension {})",
                        m.tension_id()
                    ));
                    continue;
                }
                _ => {}
            }

            // Read current value from the tension
            let tension = self
                .get_tension(m.tension_id())
                .map_err(|e| CoreError::ValidationError(e.to_string()))?;

            let tension = match tension {
                Some(t) => t,
                None => {
                    conflicts.push(format!("tension {} no longer exists", m.tension_id()));
                    continue;
                }
            };

            let current_value = match field {
                "desired" => tension.desired.clone(),
                "actual" => tension.actual.clone(),
                "status" => tension.status.to_string(),
                "parent_id" => tension.parent_id.clone().unwrap_or_default(),
                "horizon" => tension
                    .horizon
                    .as_ref()
                    .map(|h| h.to_string())
                    .unwrap_or_default(),
                "position" => tension.position.map(|p| p.to_string()).unwrap_or_default(),
                _ => continue,
            };

            if current_value != m.new_value() {
                conflicts.push(format!(
                    "tension {} field '{}': expected '{}', found '{}'",
                    m.tension_id(),
                    field,
                    m.new_value(),
                    current_value
                ));
            }
        }

        if !conflicts.is_empty() {
            return Err(CoreError::ValidationError(format!(
                "undo conflict — fields changed since gesture:\n  {}",
                conflicts.join("\n  ")
            )));
        }

        // 4. All checks pass — apply reversals in a single transaction
        let undo_gesture_id = ulid::Ulid::new().to_string();
        let now = Utc::now();

        {
            let conn = self.conn.borrow();
            conn.execute("BEGIN CONCURRENT;").map_err(|e| {
                CoreError::ValidationError(format!("failed to begin transaction: {:?}", e))
            })?;

            let result = (|| -> Result<usize, CoreError> {
                // Create the undo gesture with undone_gesture_id set
                conn.execute_with_params(
                    "INSERT INTO gestures (id, timestamp, description, undone_gesture_id) VALUES (?1, ?2, ?3, ?4)",
                    &[
                        SqliteValue::Text(undo_gesture_id.clone().into()),
                        SqliteValue::Text(now.to_rfc3339().into()),
                        SqliteValue::Text(format!("undo of {}", gesture_id).into()),
                        SqliteValue::Text(gesture_id.to_owned().into()),
                    ],
                )
                .map_err(|e| CoreError::ValidationError(format!("failed to create undo gesture: {:?}", e)))?;

                let mut reversed_count = 0;

                // Process mutations in reverse order
                for m in mutations.iter().rev() {
                    let field = m.field();
                    let tension_id = m.tension_id();

                    match field {
                        "deleted" => {
                            // Already rejected in conflict detection
                            return Err(CoreError::ValidationError(
                                "undo of deletion is not supported in v1".to_owned(),
                            ));
                        }

                        "created" => {
                            // Undo creation = delete the tension
                            conn.execute_with_params(
                                "DELETE FROM tensions WHERE id = ?1",
                                &[SqliteValue::Text(tension_id.to_owned().into())],
                            )
                            .map_err(|e| {
                                CoreError::ValidationError(format!(
                                    "failed to delete tension on undo-create: {:?}",
                                    e
                                ))
                            })?;

                            // Record the reverse mutation
                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new_with_gesture(
                                    tension_id.to_owned(),
                                    now,
                                    "deleted".to_owned(),
                                    Some(m.new_value().to_owned()),
                                    String::new(),
                                    Some(undo_gesture_id.clone()),
                                    None,
                                ),
                            )?;
                            reversed_count += 1;
                        }

                        "note" => {
                            // Record a note_retracted mutation
                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new_with_gesture(
                                    tension_id.to_owned(),
                                    now,
                                    "note_retracted".to_owned(),
                                    Some(m.new_value().to_owned()),
                                    String::new(),
                                    Some(undo_gesture_id.clone()),
                                    None,
                                ),
                            )?;
                            reversed_count += 1;
                        }

                        "desired" | "actual" => {
                            let old_val = m.old_value().unwrap_or("").to_owned();
                            if old_val.is_empty() {
                                // Can't revert to empty — skip
                                continue;
                            }
                            let column = field;
                            conn.execute_with_params(
                                &format!("UPDATE tensions SET {} = ?1 WHERE id = ?2", column),
                                &[
                                    SqliteValue::Text(old_val.clone().into()),
                                    SqliteValue::Text(tension_id.to_owned().into()),
                                ],
                            )
                            .map_err(|e| {
                                CoreError::ValidationError(format!(
                                    "failed to revert {}: {:?}",
                                    field, e
                                ))
                            })?;

                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new_with_gesture(
                                    tension_id.to_owned(),
                                    now,
                                    field.to_owned(),
                                    Some(m.new_value().to_owned()),
                                    old_val,
                                    Some(undo_gesture_id.clone()),
                                    None,
                                ),
                            )?;
                            reversed_count += 1;
                        }

                        "status" => {
                            let old_status_str = m.old_value().unwrap_or("Active");
                            let old_status = match old_status_str {
                                "Active" => TensionStatus::Active,
                                "Resolved" => TensionStatus::Resolved,
                                "Released" => TensionStatus::Released,
                                _ => TensionStatus::Active,
                            };

                            conn.execute_with_params(
                                "UPDATE tensions SET status = ?1 WHERE id = ?2",
                                &[
                                    SqliteValue::Text(old_status.to_string().into()),
                                    SqliteValue::Text(tension_id.to_owned().into()),
                                ],
                            )
                            .map_err(|e| {
                                CoreError::ValidationError(format!(
                                    "failed to revert status: {:?}",
                                    e
                                ))
                            })?;

                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new_with_gesture(
                                    tension_id.to_owned(),
                                    now,
                                    "status".to_owned(),
                                    Some(m.new_value().to_owned()),
                                    old_status.to_string(),
                                    Some(undo_gesture_id.clone()),
                                    None,
                                ),
                            )?;
                            reversed_count += 1;
                        }

                        "parent_id" => {
                            let old_parent = m.old_value().map(|v| v.to_owned());
                            let old_parent_or_null = if old_parent.as_deref() == Some("") {
                                None
                            } else {
                                old_parent.clone()
                            };

                            conn.execute_with_params(
                                "UPDATE tensions SET parent_id = ?1 WHERE id = ?2",
                                &[
                                    match &old_parent_or_null {
                                        Some(p) => SqliteValue::Text(p.clone().into()),
                                        None => SqliteValue::Null,
                                    },
                                    SqliteValue::Text(tension_id.to_owned().into()),
                                ],
                            )
                            .map_err(|e| {
                                CoreError::ValidationError(format!(
                                    "failed to revert parent_id: {:?}",
                                    e
                                ))
                            })?;

                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new_with_gesture(
                                    tension_id.to_owned(),
                                    now,
                                    "parent_id".to_owned(),
                                    Some(m.new_value().to_owned()),
                                    old_parent.unwrap_or_default(),
                                    Some(undo_gesture_id.clone()),
                                    None,
                                ),
                            )?;
                            reversed_count += 1;
                        }

                        "horizon" => {
                            let old_horizon_str = m.old_value().unwrap_or("");
                            conn.execute_with_params(
                                "UPDATE tensions SET horizon = ?1 WHERE id = ?2",
                                &[
                                    if old_horizon_str.is_empty() {
                                        SqliteValue::Null
                                    } else {
                                        SqliteValue::Text(old_horizon_str.to_owned().into())
                                    },
                                    SqliteValue::Text(tension_id.to_owned().into()),
                                ],
                            )
                            .map_err(|e| {
                                CoreError::ValidationError(format!(
                                    "failed to revert horizon: {:?}",
                                    e
                                ))
                            })?;

                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new_with_gesture(
                                    tension_id.to_owned(),
                                    now,
                                    "horizon".to_owned(),
                                    Some(m.new_value().to_owned()),
                                    old_horizon_str.to_owned(),
                                    Some(undo_gesture_id.clone()),
                                    None,
                                ),
                            )?;
                            reversed_count += 1;
                        }

                        "position" => {
                            let old_pos_str = m.old_value().unwrap_or("");
                            conn.execute_with_params(
                                "UPDATE tensions SET position = ?1 WHERE id = ?2",
                                &[
                                    if old_pos_str.is_empty() {
                                        SqliteValue::Null
                                    } else {
                                        match old_pos_str.parse::<i64>() {
                                            Ok(n) => SqliteValue::Integer(n),
                                            Err(_) => SqliteValue::Null,
                                        }
                                    },
                                    SqliteValue::Text(tension_id.to_owned().into()),
                                ],
                            )
                            .map_err(|e| {
                                CoreError::ValidationError(format!(
                                    "failed to revert position: {:?}",
                                    e
                                ))
                            })?;

                            self.record_mutation_in_transaction(
                                &conn,
                                &Mutation::new_with_gesture(
                                    tension_id.to_owned(),
                                    now,
                                    "position".to_owned(),
                                    Some(m.new_value().to_owned()),
                                    old_pos_str.to_owned(),
                                    Some(undo_gesture_id.clone()),
                                    None,
                                ),
                            )?;
                            reversed_count += 1;
                        }

                        _ => {
                            // Unknown field — skip silently
                        }
                    }
                }

                // Delete edges created by the original gesture
                conn.execute_with_params(
                    "DELETE FROM edges WHERE gesture_id = ?1",
                    &[SqliteValue::Text(gesture_id.to_owned().into())],
                )
                .map_err(|e| {
                    CoreError::ValidationError(format!(
                        "failed to delete edges for gesture: {:?}",
                        e
                    ))
                })?;

                // Delete epochs triggered by the original gesture
                conn.execute_with_params(
                    "DELETE FROM epochs WHERE trigger_gesture_id = ?1",
                    &[SqliteValue::Text(gesture_id.to_owned().into())],
                )
                .map_err(|e| {
                    CoreError::ValidationError(format!(
                        "failed to delete epochs for gesture: {:?}",
                        e
                    ))
                })?;

                Ok(reversed_count)
            })();

            match result {
                Ok(_count) => {
                    Self::commit_with_retry(&conn)?;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;");
                    return Err(e);
                }
            }
        }

        // Re-sync contains edges for any parent_id reversals
        // (edges were deleted in-transaction; now rebuild from current parent_id state)
        for m in &mutations {
            if m.field() == "parent_id" {
                let old_parent = m.old_value().filter(|v| !v.is_empty());
                let new_parent_was = if m.new_value().is_empty() {
                    None
                } else {
                    Some(m.new_value())
                };

                // The revert set parent_id back to old_parent.
                // Remove the edge that was pointing from new_parent -> tension
                if let Some(np) = new_parent_was {
                    let _ = self.remove_edge(np, m.tension_id(), edge::EDGE_CONTAINS);
                }
                // Create edge from old_parent -> tension (restoring original structure)
                if let Some(op) = old_parent {
                    let _ = self.create_edge(op, m.tension_id(), edge::EDGE_CONTAINS);
                }
            }
        }

        // Emit GestureUndone event
        self.emit_event(&EventBuilder::gesture_undone(
            gesture_id.to_owned(),
            undo_gesture_id.clone(),
            mutations.len(),
        ));

        Ok(undo_gesture_id)
    }

    // ── Epoch management ───────────────────────────────────────────

    /// Create a new epoch snapshot for a tension.
    pub fn create_epoch(
        &self,
        tension_id: &str,
        desire_snapshot: &str,
        reality_snapshot: &str,
        children_snapshot_json: Option<&str>,
        trigger_gesture_id: Option<&str>,
    ) -> Result<String, StoreError> {
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO epochs (id, tension_id, timestamp, desire_snapshot, reality_snapshot, children_snapshot_json, trigger_gesture_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                SqliteValue::Text(id.to_string().into()),
                SqliteValue::Text(tension_id.to_owned().into()),
                SqliteValue::Text(now.to_rfc3339().into()),
                SqliteValue::Text(desire_snapshot.to_owned().into()),
                SqliteValue::Text(reality_snapshot.to_owned().into()),
                match children_snapshot_json {
                    Some(s) => SqliteValue::Text(s.to_owned().into()),
                    None => SqliteValue::Null,
                },
                match trigger_gesture_id {
                    Some(s) => SqliteValue::Text(s.to_owned().into()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to create epoch: {:?}", e)))?;
        Ok(id)
    }

    /// Get all epochs for a tension in chronological order.
    pub fn get_epochs(&self, tension_id: &str) -> Result<Vec<EpochRecord>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, tension_id, timestamp, desire_snapshot, reality_snapshot, children_snapshot_json, trigger_gesture_id, epoch_type FROM epochs WHERE tension_id = ?1 ORDER BY timestamp ASC",
                &[SqliteValue::Text(tension_id.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        let mut epochs = Vec::new();
        for row in &rows {
            let id = match row.get(0) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => return Err(StoreError::DatabaseError("invalid epoch id".to_owned())),
            };
            let tid = match row.get(1) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid epoch tension_id".to_owned(),
                    ));
                }
            };
            let ts_str = match row.get(2) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid epoch timestamp".to_owned(),
                    ));
                }
            };
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    StoreError::DatabaseError(format!("invalid epoch timestamp: {}", e))
                })?;
            let desire = match row.get(3) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid epoch desire_snapshot".to_owned(),
                    ));
                }
            };
            let reality = match row.get(4) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid epoch reality_snapshot".to_owned(),
                    ));
                }
            };
            let children_json = match row.get(5) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };
            let trigger = match row.get(6) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };
            epochs.push(EpochRecord {
                id,
                tension_id: tid,
                timestamp,
                desire_snapshot: desire,
                reality_snapshot: reality,
                children_snapshot_json: children_json,
                trigger_gesture_id: trigger,
                epoch_type: match row.get(7) {
                    Some(SqliteValue::Text(s)) => Some(s.to_string()),
                    Some(SqliteValue::Null) | None => None,
                    _ => None,
                },
            });
        }
        Ok(epochs)
    }

    /// Get the timestamp of the last epoch for a tension (lightweight, no full record load).
    pub fn get_last_epoch_timestamp(
        &self,
        tension_id: &str,
    ) -> Result<Option<DateTime<Utc>>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT MAX(timestamp) FROM epochs WHERE tension_id = ?1",
                &[SqliteValue::Text(tension_id.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        if let Some(row) = rows.first()
            && let Some(SqliteValue::Text(ts)) = row.get(0)
        {
            let dt = DateTime::parse_from_rfc3339(ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| StoreError::DatabaseError(format!("invalid timestamp: {}", e)))?;
            return Ok(Some(dt));
        }
        Ok(None)
    }

    /// Create an epoch with a type annotation (for split/merge provenance).
    pub fn create_epoch_typed(
        &self,
        tension_id: &str,
        desire_snapshot: &str,
        reality_snapshot: &str,
        children_snapshot_json: Option<&str>,
        trigger_gesture_id: Option<&str>,
        epoch_type: Option<&str>,
    ) -> Result<String, StoreError> {
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO epochs (id, tension_id, timestamp, desire_snapshot, reality_snapshot, children_snapshot_json, trigger_gesture_id, epoch_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            &[
                SqliteValue::Text(id.to_string().into()),
                SqliteValue::Text(tension_id.to_owned().into()),
                SqliteValue::Text(now.to_rfc3339().into()),
                SqliteValue::Text(desire_snapshot.to_owned().into()),
                SqliteValue::Text(reality_snapshot.to_owned().into()),
                match children_snapshot_json {
                    Some(s) => SqliteValue::Text(s.to_owned().into()),
                    None => SqliteValue::Null,
                },
                match trigger_gesture_id {
                    Some(s) => SqliteValue::Text(s.to_owned().into()),
                    None => SqliteValue::Null,
                },
                match epoch_type {
                    Some(s) => SqliteValue::Text(s.to_owned().into()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to create typed epoch: {:?}", e)))?;
        Ok(id)
    }

    // ── Edge management ───────────────────────────────────────────

    /// Create a typed edge between two tensions.
    pub fn create_edge(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: &str,
    ) -> Result<crate::edge::Edge, StoreError> {
        let edge = crate::edge::Edge {
            id: ulid::Ulid::new().to_string(),
            from_id: from_id.to_owned(),
            to_id: to_id.to_owned(),
            edge_type: edge_type.to_owned(),
            created_at: Utc::now(),
            gesture_id: self.active_gesture_id.clone(),
        };

        let conn = self.conn.borrow();
        conn.execute_with_params(
            "INSERT INTO edges (id, from_id, to_id, edge_type, created_at, gesture_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                SqliteValue::Text(edge.id.clone().into()),
                SqliteValue::Text(edge.from_id.clone().into()),
                SqliteValue::Text(edge.to_id.clone().into()),
                SqliteValue::Text(edge.edge_type.clone().into()),
                SqliteValue::Text(edge.created_at.to_rfc3339().into()),
                match &edge.gesture_id {
                    Some(g) => SqliteValue::Text(g.clone().into()),
                    None => SqliteValue::Null,
                },
            ],
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to create edge: {:?}", e)))?;

        Ok(edge)
    }

    /// Remove an edge by from_id, to_id, and type.
    pub fn remove_edge(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: &str,
    ) -> Result<bool, StoreError> {
        let conn = self.conn.borrow();
        conn.execute_with_params(
            "DELETE FROM edges WHERE from_id = ?1 AND to_id = ?2 AND edge_type = ?3",
            &[
                SqliteValue::Text(from_id.to_owned().into()),
                SqliteValue::Text(to_id.to_owned().into()),
                SqliteValue::Text(edge_type.to_owned().into()),
            ],
        )
        .map_err(|e| StoreError::DatabaseError(format!("failed to remove edge: {:?}", e)))?;

        // fsqlite doesn't return affected rows directly; check if edge still exists
        let check = conn
            .query_with_params(
                "SELECT COUNT(*) FROM edges WHERE from_id = ?1 AND to_id = ?2 AND edge_type = ?3",
                &[
                    SqliteValue::Text(from_id.to_owned().into()),
                    SqliteValue::Text(to_id.to_owned().into()),
                    SqliteValue::Text(edge_type.to_owned().into()),
                ],
            )
            .map_err(|e| StoreError::DatabaseError(format!("edge check failed: {:?}", e)))?;

        let still_exists = match check.first().and_then(|r| r.get(0)) {
            Some(SqliteValue::Integer(n)) => *n > 0,
            _ => false,
        };

        Ok(!still_exists)
    }

    /// Get all edges involving a tension (as source or target).
    pub fn get_edges_for_tension(
        &self,
        tension_id: &str,
    ) -> Result<Vec<crate::edge::Edge>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, from_id, to_id, edge_type, created_at, gesture_id FROM edges WHERE from_id = ?1 OR to_id = ?1 ORDER BY created_at ASC",
                &[SqliteValue::Text(tension_id.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_edge_rows(rows)
    }

    /// Get all edges of a specific type.
    pub fn get_edges_by_type(&self, edge_type: &str) -> Result<Vec<crate::edge::Edge>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT id, from_id, to_id, edge_type, created_at, gesture_id FROM edges WHERE edge_type = ?1 ORDER BY created_at ASC",
                &[SqliteValue::Text(edge_type.to_owned().into())],
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_edge_rows(rows)
    }

    /// Get all edges (for Forest construction).
    pub fn get_all_edges(&self) -> Result<Vec<crate::edge::Edge>, StoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query(
                "SELECT id, from_id, to_id, edge_type, created_at, gesture_id FROM edges ORDER BY created_at ASC",
            )
            .map_err(|e| StoreError::DatabaseError(format!("query failed: {:?}", e)))?;

        self.parse_edge_rows(rows)
    }

    fn parse_edge_rows(
        &self,
        rows: Vec<fsqlite::Row>,
    ) -> Result<Vec<crate::edge::Edge>, StoreError> {
        let mut edges = Vec::new();
        for row in &rows {
            let id = match row.get(0) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => return Err(StoreError::DatabaseError("invalid edge id".to_owned())),
            };
            let from_id = match row.get(1) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => return Err(StoreError::DatabaseError("invalid edge from_id".to_owned())),
            };
            let to_id = match row.get(2) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => return Err(StoreError::DatabaseError("invalid edge to_id".to_owned())),
            };
            let edge_type = match row.get(3) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid edge edge_type".to_owned(),
                    ));
                }
            };
            let created_at_str = match row.get(4) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => {
                    return Err(StoreError::DatabaseError(
                        "invalid edge created_at".to_owned(),
                    ));
                }
            };
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| StoreError::DatabaseError(format!("invalid edge timestamp: {}", e)))?;
            let gesture_id = match row.get(5) {
                Some(SqliteValue::Text(s)) => Some(s.to_string()),
                Some(SqliteValue::Null) | None => None,
                _ => None,
            };

            edges.push(crate::edge::Edge {
                id,
                from_id,
                to_id,
                edge_type,
                created_at,
                gesture_id,
            });
        }
        Ok(edges)
    }

    // ── Doctor: Quint-invariant detectors and fixers (R-005) ──────────
    //
    // All methods in this block are designed for use ONLY through the
    // `werk doctor` chokepoint (`werk-cli/src/commands/doctor.rs`).
    // - `list_*` / `count_*` methods are read-only detectors.
    // - `doctor_*` methods are mutators that must be journaled by the
    //   caller via `DoctorRun::record_action`. Each runs inside a
    //   `BEGIN CONCURRENT; … commit_with_retry;` envelope so cross-table
    //   mutations are atomic under MVCC two-writer reality (W-4).

    /// Test-fixture helper: insert a raw edge row bypassing the gesture
    /// API. Used ONLY by the doctor's fixture round-trip tests to inject
    /// Quint-invariant violations. Not part of the public surface.
    #[doc(hidden)]
    pub fn doctor_test_insert_edge_raw(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: &str,
    ) -> Result<String, CoreError> {
        let conn = self.conn.borrow();
        let id = ulid::Ulid::new().to_string();
        conn.execute_with_params(
            "INSERT INTO edges (id, from_id, to_id, edge_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                SqliteValue::Text(id.clone().into()),
                SqliteValue::Text(from_id.to_owned().into()),
                SqliteValue::Text(to_id.to_owned().into()),
                SqliteValue::Text(edge_type.to_owned().into()),
                SqliteValue::Text(Utc::now().to_rfc3339().into()),
            ],
        )
        .map_err(|e| CoreError::ValidationError(format!("test edge insert: {:?}", e)))?;
        Ok(id)
    }

    /// Test-fixture helper: directly set a tension's `position` column.
    #[doc(hidden)]
    pub fn doctor_test_set_position_raw(
        &self,
        tension_id: &str,
        position: Option<i64>,
    ) -> Result<(), CoreError> {
        let conn = self.conn.borrow();
        let v = match position {
            Some(p) => SqliteValue::Integer(p),
            None => SqliteValue::Null,
        };
        conn.execute_with_params(
            "UPDATE tensions SET position = ?1 WHERE id = ?2",
            &[v, SqliteValue::Text(tension_id.to_owned().into())],
        )
        .map_err(|e| CoreError::ValidationError(format!("test position set: {:?}", e)))?;
        Ok(())
    }

    /// Test-fixture helper: directly set a tension's `horizon` column.
    #[doc(hidden)]
    pub fn doctor_test_set_horizon_raw(
        &self,
        tension_id: &str,
        horizon: Option<&str>,
    ) -> Result<(), CoreError> {
        let conn = self.conn.borrow();
        let v = match horizon {
            Some(h) => SqliteValue::Text(h.to_owned().into()),
            None => SqliteValue::Null,
        };
        conn.execute_with_params(
            "UPDATE tensions SET horizon = ?1 WHERE id = ?2",
            &[v, SqliteValue::Text(tension_id.to_owned().into())],
        )
        .map_err(|e| CoreError::ValidationError(format!("test horizon set: {:?}", e)))?;
        Ok(())
    }

    /// Test-fixture helper: insert a gesture row with an arbitrary
    /// `undone_gesture_id`, including ids that don't reference a real
    /// gesture (so we can inject the `undoneSubsetOfCompleted` violation).
    #[doc(hidden)]
    pub fn doctor_test_insert_gesture_raw(
        &self,
        description: &str,
        undone_gesture_id: Option<&str>,
    ) -> Result<String, CoreError> {
        let conn = self.conn.borrow();
        let id = ulid::Ulid::new().to_string();
        let v = match undone_gesture_id {
            Some(u) => SqliteValue::Text(u.to_owned().into()),
            None => SqliteValue::Null,
        };
        conn.execute_with_params(
            "INSERT INTO gestures (id, timestamp, description, undone_gesture_id) VALUES (?1, ?2, ?3, ?4)",
            &[
                SqliteValue::Text(id.clone().into()),
                SqliteValue::Text(Utc::now().to_rfc3339().into()),
                SqliteValue::Text(description.to_owned().into()),
                v,
            ],
        )
        .map_err(|e| CoreError::ValidationError(format!("test gesture insert: {:?}", e)))?;
        Ok(id)
    }

    /// `singleParent`: list every tension that has more than one
    /// `contains` edge pointing at it, with the offending edge ids sorted
    /// by ULID ascending (deterministic, monotonic).
    pub fn list_multi_parent_violations(
        &self,
    ) -> Result<Vec<DoctorMultiParentRow>, CoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query(
                "SELECT to_id FROM edges WHERE edge_type = 'contains' GROUP BY to_id HAVING COUNT(*) > 1 ORDER BY to_id ASC",
            )
            .map_err(|e| CoreError::ValidationError(format!("singleParent query: {:?}", e)))?;
        let mut out = Vec::new();
        for r in rows {
            let to_id = match r.get(0) {
                Some(SqliteValue::Text(s)) => s.to_string(),
                _ => continue,
            };
            let edge_rows = conn
                .query_with_params(
                    "SELECT id FROM edges WHERE edge_type = 'contains' AND to_id = ?1 ORDER BY id ASC",
                    &[SqliteValue::Text(to_id.clone().into())],
                )
                .map_err(|e| CoreError::ValidationError(format!("singleParent detail: {:?}", e)))?;
            let edge_ids: Vec<String> = edge_rows
                .into_iter()
                .filter_map(|er| match er.get(0) {
                    Some(SqliteValue::Text(s)) => Some(s.to_string()),
                    _ => None,
                })
                .collect();
            if edge_ids.len() > 1 {
                out.push(DoctorMultiParentRow {
                    tension_id: to_id,
                    parent_edge_ids: edge_ids,
                });
            }
        }
        Ok(out)
    }

    /// `noSelfEdges`: list every edge with `from_id == to_id`.
    pub fn list_self_edges(&self) -> Result<Vec<DoctorEdgeRow>, CoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query("SELECT id, from_id, to_id, edge_type FROM edges WHERE from_id = to_id ORDER BY id ASC")
            .map_err(|e| CoreError::ValidationError(format!("noSelfEdges query: {:?}", e)))?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let id = text(r.get(0))?;
                let from_id = text(r.get(1))?;
                let to_id = text(r.get(2))?;
                let edge_type = text(r.get(3))?;
                Some(DoctorEdgeRow {
                    id,
                    from_id,
                    to_id,
                    edge_type,
                })
            })
            .collect())
    }

    /// `edgesValid`: list every edge whose endpoints don't reference an
    /// existing tension. Runs under `BEGIN CONCURRENT` for a consistent
    /// snapshot across the `edges` and `tensions` tables (W-4).
    pub fn list_dangling_edges(&self) -> Result<Vec<DoctorEdgeRow>, CoreError> {
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("edgesValid begin: {:?}", e)))?;
        let result: Result<Vec<DoctorEdgeRow>, CoreError> = (|| {
            let rows = conn
                .query(
                    "SELECT e.id, e.from_id, e.to_id, e.edge_type \
                     FROM edges e \
                     LEFT JOIN tensions tf ON tf.id = e.from_id \
                     LEFT JOIN tensions tt ON tt.id = e.to_id \
                     WHERE tf.id IS NULL OR tt.id IS NULL \
                     ORDER BY e.id ASC",
                )
                .map_err(|e| CoreError::ValidationError(format!("edgesValid query: {:?}", e)))?;
            Ok(rows
                .into_iter()
                .filter_map(|r| {
                    Some(DoctorEdgeRow {
                        id: text(r.get(0))?,
                        from_id: text(r.get(1))?,
                        to_id: text(r.get(2))?,
                        edge_type: text(r.get(3))?,
                    })
                })
                .collect())
        })();
        // Read-only transaction; rollback to release the snapshot. No
        // commit_with_retry needed since we didn't write.
        let _ = conn.execute("ROLLBACK;");
        result
    }

    /// `siblingPositionsUnique`: list every `(parent_id, position)` group
    /// among children connected by a `contains` edge whose position is
    /// non-NULL and which has more than one occupant.
    pub fn list_sibling_position_collisions(
        &self,
    ) -> Result<Vec<DoctorSiblingCollisionRow>, CoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query(
                "SELECT e.from_id, t.position, COUNT(*) as cnt \
                 FROM edges e JOIN tensions t ON t.id = e.to_id \
                 WHERE e.edge_type = 'contains' AND t.position IS NOT NULL \
                 GROUP BY e.from_id, t.position \
                 HAVING cnt > 1 \
                 ORDER BY e.from_id ASC, t.position ASC",
            )
            .map_err(|e| {
                CoreError::ValidationError(format!("siblingPositionsUnique query: {:?}", e))
            })?;
        let mut out = Vec::new();
        for r in rows {
            let parent_id = match text(r.get(0)) {
                Some(s) => s,
                None => continue,
            };
            let position = match r.get(1) {
                Some(SqliteValue::Integer(p)) => *p,
                _ => continue,
            };
            // Pull the colliding children, ordered by edge ULID asc (the
            // earliest-recorded edge is the "winner" by §2.5 tiebreak).
            let child_rows = conn
                .query_with_params(
                    "SELECT e.to_id, e.id FROM edges e \
                     JOIN tensions t ON t.id = e.to_id \
                     WHERE e.from_id = ?1 AND e.edge_type = 'contains' AND t.position = ?2 \
                     ORDER BY e.id ASC",
                    &[
                        SqliteValue::Text(parent_id.clone().into()),
                        SqliteValue::Integer(position),
                    ],
                )
                .map_err(|e| {
                    CoreError::ValidationError(format!("siblingPositionsUnique detail: {:?}", e))
                })?;
            let children: Vec<String> = child_rows
                .into_iter()
                .filter_map(|cr| text(cr.get(0)))
                .collect();
            if children.len() > 1 {
                out.push(DoctorSiblingCollisionRow {
                    parent_id,
                    position,
                    child_ids: children,
                });
            }
        }
        Ok(out)
    }

    /// `noContainmentViolations`: list every `contains` edge whose parent
    /// and child both have a non-NULL `horizon`. Returns the raw rows;
    /// the caller (the doctor CLI) does the horizon comparison via
    /// `Horizon::parse` to keep the spec-vs-implementation gap small.
    /// Runs under `BEGIN CONCURRENT` for a consistent multi-table snapshot.
    pub fn list_horizon_pairs_for_contains_edges(
        &self,
    ) -> Result<Vec<DoctorHorizonPairRow>, CoreError> {
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("horizon begin: {:?}", e)))?;
        let result: Result<Vec<DoctorHorizonPairRow>, CoreError> = (|| {
            let rows = conn
                .query(
                    "SELECT e.from_id, e.to_id, tp.horizon, tc.horizon \
                     FROM edges e \
                     JOIN tensions tp ON tp.id = e.from_id \
                     JOIN tensions tc ON tc.id = e.to_id \
                     WHERE e.edge_type = 'contains' \
                       AND tp.horizon IS NOT NULL \
                       AND tc.horizon IS NOT NULL",
                )
                .map_err(|e| CoreError::ValidationError(format!("horizon query: {:?}", e)))?;
            Ok(rows
                .into_iter()
                .filter_map(|r| {
                    Some(DoctorHorizonPairRow {
                        parent_id: text(r.get(0))?,
                        child_id: text(r.get(1))?,
                        parent_horizon: text(r.get(2))?,
                        child_horizon: text(r.get(3))?,
                    })
                })
                .collect())
        })();
        let _ = conn.execute("ROLLBACK;");
        result
    }

    /// `undoneSubsetOfCompleted`: list every gesture whose
    /// `undone_gesture_id` doesn't reference an existing gesture row.
    pub fn list_dangling_undo_gestures(
        &self,
    ) -> Result<Vec<DoctorDanglingUndoRow>, CoreError> {
        let conn = self.conn.borrow();
        let rows = conn
            .query(
                "SELECT g.id, g.undone_gesture_id FROM gestures g \
                 WHERE g.undone_gesture_id IS NOT NULL \
                   AND g.undone_gesture_id NOT IN (SELECT id FROM gestures WHERE id IS NOT NULL) \
                 ORDER BY g.id ASC",
            )
            .map_err(|e| {
                CoreError::ValidationError(format!("undoneSubsetOfCompleted query: {:?}", e))
            })?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                Some(DoctorDanglingUndoRow {
                    gesture_id: text(r.get(0))?,
                    dangling_referent: text(r.get(1))?,
                })
            })
            .collect())
    }

    /// Helper: reconcile `tensions.parent_id` for the given tension ids
    /// based on the current `contains`-edge set. For each id:
    /// - If exactly one contains-edge points at it, `parent_id = <from_id>`.
    /// - If zero or more-than-one (latter caught by harness), `parent_id = NULL`.
    /// Caller is responsible for the surrounding transaction.
    pub fn doctor_reconcile_parent_ids(
        &self,
        tension_ids: &[String],
    ) -> Result<usize, CoreError> {
        if tension_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.borrow();
        let mut changed = 0usize;
        for tid in tension_ids {
            let edge_rows = conn
                .query_with_params(
                    "SELECT from_id FROM edges WHERE edge_type = 'contains' AND to_id = ?1",
                    &[SqliteValue::Text(tid.clone().into())],
                )
                .map_err(|e| {
                    CoreError::ValidationError(format!("reconcile lookup: {:?}", e))
                })?;
            let new_parent: Option<String> = if edge_rows.len() == 1 {
                text(edge_rows[0].get(0))
            } else {
                None
            };
            // Read current parent_id to know if this is a real change.
            let cur_rows = conn
                .query_with_params(
                    "SELECT parent_id FROM tensions WHERE id = ?1",
                    &[SqliteValue::Text(tid.clone().into())],
                )
                .map_err(|e| {
                    CoreError::ValidationError(format!("reconcile current parent: {:?}", e))
                })?;
            let cur_parent: Option<String> = cur_rows
                .first()
                .and_then(|r| match r.get(0) {
                    Some(SqliteValue::Text(s)) => Some(s.to_string()),
                    _ => None,
                });
            if cur_parent == new_parent {
                continue;
            }
            let param_val = match &new_parent {
                Some(p) => SqliteValue::Text(p.clone().into()),
                None => SqliteValue::Null,
            };
            conn.execute_with_params(
                "UPDATE tensions SET parent_id = ?1 WHERE id = ?2",
                &[param_val, SqliteValue::Text(tid.clone().into())],
            )
            .map_err(|e| {
                CoreError::ValidationError(format!("reconcile update: {:?}", e))
            })?;
            changed += 1;
        }
        Ok(changed)
    }

    /// Fixer 2.2: prune duplicate contains-edges, keeping one per tension
    /// per `PreferEdge` policy. Reconciles `parent_id` in the same
    /// transaction.
    pub fn doctor_prune_duplicate_parent_edges(
        &self,
        prefer: PreferEdge,
    ) -> Result<DoctorPruneResult, CoreError> {
        let violations = self.list_multi_parent_violations()?;
        if violations.is_empty() {
            return Ok(DoctorPruneResult::default());
        }
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("prune begin: {:?}", e)))?;
        let work = (|| -> Result<DoctorPruneResult, CoreError> {
            let mut deleted_ids: Vec<String> = Vec::new();
            let mut affected: Vec<String> = Vec::new();
            for v in &violations {
                // edge_ids are already sorted ULID asc by list_multi_parent_violations.
                let keep = match prefer {
                    PreferEdge::Oldest => v.parent_edge_ids.first().cloned(),
                    PreferEdge::Newest => v.parent_edge_ids.last().cloned(),
                };
                let Some(keep_id) = keep else { continue };
                for eid in &v.parent_edge_ids {
                    if eid == &keep_id {
                        continue;
                    }
                    conn.execute_with_params(
                        "DELETE FROM edges WHERE id = ?1",
                        &[SqliteValue::Text(eid.clone().into())],
                    )
                    .map_err(|e| {
                        CoreError::ValidationError(format!("prune delete: {:?}", e))
                    })?;
                    deleted_ids.push(eid.clone());
                }
                affected.push(v.tension_id.clone());
            }
            let reconciled = self.doctor_reconcile_parent_ids(&affected)?;
            Ok(DoctorPruneResult {
                deleted_edge_ids: deleted_ids,
                affected_tension_ids: affected,
                parent_ids_reconciled: reconciled,
            })
        })();
        match work {
            Ok(r) => {
                Self::commit_with_retry(&conn).map_err(|e| {
                    CoreError::ValidationError(format!("prune commit: {}", e))
                })?;
                Ok(r)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK;");
                Err(e)
            }
        }
    }

    /// Fixer 2.3: delete every self-edge. Returns the `to_id`s of any
    /// deleted contains-self-edges so the caller can reconcile parent_id
    /// (those tensions had `parent_id == id`).
    pub fn doctor_delete_self_edges(&self) -> Result<DoctorSelfEdgeResult, CoreError> {
        let self_edges = self.list_self_edges()?;
        if self_edges.is_empty() {
            return Ok(DoctorSelfEdgeResult::default());
        }
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("self-edge begin: {:?}", e)))?;
        let work = (|| -> Result<DoctorSelfEdgeResult, CoreError> {
            let mut deleted = 0usize;
            let mut contains_to_ids: Vec<String> = Vec::new();
            for e in &self_edges {
                conn.execute_with_params(
                    "DELETE FROM edges WHERE id = ?1",
                    &[SqliteValue::Text(e.id.clone().into())],
                )
                .map_err(|err| {
                    CoreError::ValidationError(format!("self-edge delete: {:?}", err))
                })?;
                deleted += 1;
                if e.edge_type == "contains" {
                    contains_to_ids.push(e.to_id.clone());
                }
            }
            let reconciled = self.doctor_reconcile_parent_ids(&contains_to_ids)?;
            Ok(DoctorSelfEdgeResult {
                deleted,
                affected_tension_ids: contains_to_ids,
                parent_ids_reconciled: reconciled,
            })
        })();
        match work {
            Ok(r) => {
                Self::commit_with_retry(&conn).map_err(|e| {
                    CoreError::ValidationError(format!("self-edge commit: {}", e))
                })?;
                Ok(r)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK;");
                Err(e)
            }
        }
    }

    /// Fixer 2.4: delete every dangling edge (endpoint references missing
    /// tension). Reconciles `parent_id` for surviving tensions whose
    /// contains-edge pointed at a missing parent.
    pub fn doctor_delete_dangling_edges(&self) -> Result<DoctorDanglingResult, CoreError> {
        let dangling = self.list_dangling_edges()?;
        if dangling.is_empty() {
            return Ok(DoctorDanglingResult::default());
        }
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("dangling begin: {:?}", e)))?;
        let work = (|| -> Result<DoctorDanglingResult, CoreError> {
            let mut deleted = 0usize;
            let mut surviving_to_ids: Vec<String> = Vec::new();
            for e in &dangling {
                conn.execute_with_params(
                    "DELETE FROM edges WHERE id = ?1",
                    &[SqliteValue::Text(e.id.clone().into())],
                )
                .map_err(|err| {
                    CoreError::ValidationError(format!("dangling delete: {:?}", err))
                })?;
                deleted += 1;
                // For contains-edges where the CHILD (to_id) still exists
                // as a tension but the parent (from_id) is missing, the
                // child's `parent_id` is now stale and must be NULLed.
                if e.edge_type == "contains" {
                    let tcheck = conn
                        .query_with_params(
                            "SELECT id FROM tensions WHERE id = ?1",
                            &[SqliteValue::Text(e.to_id.clone().into())],
                        )
                        .map_err(|err| {
                            CoreError::ValidationError(format!("dangling check: {:?}", err))
                        })?;
                    if !tcheck.is_empty() {
                        surviving_to_ids.push(e.to_id.clone());
                    }
                }
            }
            let reconciled = self.doctor_reconcile_parent_ids(&surviving_to_ids)?;
            Ok(DoctorDanglingResult {
                deleted,
                affected_tension_ids: surviving_to_ids,
                parent_ids_reconciled: reconciled,
            })
        })();
        match work {
            Ok(r) => {
                Self::commit_with_retry(&conn).map_err(|e| {
                    CoreError::ValidationError(format!("dangling commit: {}", e))
                })?;
                Ok(r)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK;");
                Err(e)
            }
        }
    }

    /// Fixer 2.5: for each `(parent, position)` collision, keep the
    /// child whose contains-edge ULID is smallest and NULL the others'
    /// `position`. Does NOT affect parent_id (the contains-edges
    /// themselves are unchanged).
    pub fn doctor_null_colliding_sibling_positions(
        &self,
    ) -> Result<DoctorSiblingFixResult, CoreError> {
        let collisions = self.list_sibling_position_collisions()?;
        if collisions.is_empty() {
            return Ok(DoctorSiblingFixResult::default());
        }
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("sibling begin: {:?}", e)))?;
        let work = (|| -> Result<DoctorSiblingFixResult, CoreError> {
            let mut nulled: Vec<(String, i64)> = Vec::new();
            let mut affected_parents: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            for c in &collisions {
                // child_ids are sorted by edge ULID asc; keep the first.
                affected_parents.insert(c.parent_id.clone());
                for losing_child in c.child_ids.iter().skip(1) {
                    conn.execute_with_params(
                        "UPDATE tensions SET position = NULL WHERE id = ?1",
                        &[SqliteValue::Text(losing_child.clone().into())],
                    )
                    .map_err(|e| {
                        CoreError::ValidationError(format!("sibling null: {:?}", e))
                    })?;
                    nulled.push((losing_child.clone(), c.position));
                }
            }
            Ok(DoctorSiblingFixResult {
                nulled,
                parent_count: affected_parents.len(),
            })
        })();
        match work {
            Ok(r) => {
                Self::commit_with_retry(&conn).map_err(|e| {
                    CoreError::ValidationError(format!("sibling commit: {}", e))
                })?;
                Ok(r)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK;");
                Err(e)
            }
        }
    }

    /// Fixer 2.6: NULL the `horizon` column on the given child tensions.
    /// Caller supplies the target ids because Rust did the
    /// `Horizon::parse`-driven comparison (the Quint `<=` is on `Time`).
    /// Chunks at 500 ids per `IN`-clause to avoid pathological prepared-
    /// statement sizes.
    pub fn doctor_null_violating_child_horizons(
        &self,
        target_ids: &[String],
    ) -> Result<Vec<(String, String)>, CoreError> {
        if target_ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("horizon-fix begin: {:?}", e)))?;
        let work = (|| -> Result<Vec<(String, String)>, CoreError> {
            let mut nulled: Vec<(String, String)> = Vec::new();
            for chunk in target_ids.chunks(500) {
                // Snapshot the old horizons first (one round-trip).
                let mut placeholders = String::new();
                let mut params: Vec<SqliteValue> = Vec::with_capacity(chunk.len());
                for (i, id) in chunk.iter().enumerate() {
                    if i > 0 {
                        placeholders.push(',');
                    }
                    placeholders.push('?');
                    placeholders.push_str(&(i + 1).to_string());
                    params.push(SqliteValue::Text(id.clone().into()));
                }
                let sql = format!(
                    "SELECT id, horizon FROM tensions WHERE id IN ({})",
                    placeholders
                );
                let rows = conn.query_with_params(&sql, &params).map_err(|e| {
                    CoreError::ValidationError(format!("horizon-fix snapshot: {:?}", e))
                })?;
                for r in rows {
                    let id = match text(r.get(0)) {
                        Some(s) => s,
                        None => continue,
                    };
                    let h = match text(r.get(1)) {
                        Some(s) => s,
                        None => continue,
                    };
                    nulled.push((id, h));
                }
                let update_sql = format!(
                    "UPDATE tensions SET horizon = NULL WHERE id IN ({})",
                    placeholders
                );
                conn.execute_with_params(&update_sql, &params).map_err(|e| {
                    CoreError::ValidationError(format!("horizon-fix update: {:?}", e))
                })?;
            }
            Ok(nulled)
        })();
        match work {
            Ok(r) => {
                Self::commit_with_retry(&conn).map_err(|e| {
                    CoreError::ValidationError(format!("horizon-fix commit: {}", e))
                })?;
                Ok(r)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK;");
                Err(e)
            }
        }
    }

    /// Fixer 2.7: NULL the `undone_gesture_id` column on every gesture
    /// row whose target is dangling. We never DELETE the row — other
    /// tables carry `gesture_id` FKs and deleting would create fresh
    /// dangling references elsewhere.
    pub fn doctor_null_dangling_undo_gestures(
        &self,
        target_ids: &[String],
    ) -> Result<usize, CoreError> {
        if target_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.borrow();
        conn.execute("BEGIN CONCURRENT;")
            .map_err(|e| CoreError::ValidationError(format!("undo-fix begin: {:?}", e)))?;
        let work = (|| -> Result<usize, CoreError> {
            let mut count = 0usize;
            for chunk in target_ids.chunks(500) {
                let mut placeholders = String::new();
                let mut params: Vec<SqliteValue> = Vec::with_capacity(chunk.len());
                for (i, id) in chunk.iter().enumerate() {
                    if i > 0 {
                        placeholders.push(',');
                    }
                    placeholders.push('?');
                    placeholders.push_str(&(i + 1).to_string());
                    params.push(SqliteValue::Text(id.clone().into()));
                }
                let sql = format!(
                    "UPDATE gestures SET undone_gesture_id = NULL WHERE id IN ({})",
                    placeholders
                );
                conn.execute_with_params(&sql, &params).map_err(|e| {
                    CoreError::ValidationError(format!("undo-fix update: {:?}", e))
                })?;
                count += chunk.len();
            }
            Ok(count)
        })();
        match work {
            Ok(r) => {
                Self::commit_with_retry(&conn).map_err(|e| {
                    CoreError::ValidationError(format!("undo-fix commit: {}", e))
                })?;
                Ok(r)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK;");
                Err(e)
            }
        }
    }
}

/// SqliteValue → Option<String> extractor used by Quint detectors.
fn text(v: Option<&SqliteValue>) -> Option<String> {
    match v? {
        SqliteValue::Text(s) => Some(s.to_string()),
        _ => None,
    }
}

/// Policy for `doctor_prune_duplicate_parent_edges`. The doctor surface
/// names this `--prefer=oldest|newest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferEdge {
    /// Keep the contains-edge with the smallest ULID (earliest in time).
    Oldest,
    /// Keep the contains-edge with the largest ULID (last-write-wins).
    Newest,
}

/// One row of `list_multi_parent_violations`: a tension whose `to_id`
/// is referenced by more than one contains-edge.
#[derive(Debug, Clone)]
pub struct DoctorMultiParentRow {
    pub tension_id: String,
    /// Contains-edge ids sorted by ULID ascending (oldest first).
    pub parent_edge_ids: Vec<String>,
}

/// One row of `list_self_edges` / `list_dangling_edges`.
#[derive(Debug, Clone)]
pub struct DoctorEdgeRow {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub edge_type: String,
}

/// One row of `list_sibling_position_collisions`.
#[derive(Debug, Clone)]
pub struct DoctorSiblingCollisionRow {
    pub parent_id: String,
    pub position: i64,
    /// Child tension ids sorted by their contains-edge ULID ascending.
    pub child_ids: Vec<String>,
}

/// One row of `list_horizon_pairs_for_contains_edges`. The CLI does
/// `Horizon::parse` and filters to actual violations.
#[derive(Debug, Clone)]
pub struct DoctorHorizonPairRow {
    pub parent_id: String,
    pub child_id: String,
    pub parent_horizon: String,
    pub child_horizon: String,
}

/// One row of `list_dangling_undo_gestures`.
#[derive(Debug, Clone)]
pub struct DoctorDanglingUndoRow {
    pub gesture_id: String,
    pub dangling_referent: String,
}

/// Result of `doctor_prune_duplicate_parent_edges`.
#[derive(Debug, Clone, Default)]
pub struct DoctorPruneResult {
    pub deleted_edge_ids: Vec<String>,
    pub affected_tension_ids: Vec<String>,
    pub parent_ids_reconciled: usize,
}

/// Result of `doctor_delete_self_edges`.
#[derive(Debug, Clone, Default)]
pub struct DoctorSelfEdgeResult {
    pub deleted: usize,
    pub affected_tension_ids: Vec<String>,
    pub parent_ids_reconciled: usize,
}

/// Result of `doctor_delete_dangling_edges`.
#[derive(Debug, Clone, Default)]
pub struct DoctorDanglingResult {
    pub deleted: usize,
    pub affected_tension_ids: Vec<String>,
    pub parent_ids_reconciled: usize,
}

/// Result of `doctor_null_colliding_sibling_positions`.
#[derive(Debug, Clone, Default)]
pub struct DoctorSiblingFixResult {
    /// `(losing_child_id, old_position)` pairs.
    pub nulled: Vec<(String, i64)>,
    pub parent_count: usize,
}

/// A record of a rendered sigil (metadata only).
#[derive(Debug, Clone, PartialEq)]
pub struct SigilRecord {
    pub id: i64,
    pub short_code: i32,
    pub scope_canonical: String,
    pub logic_id: String,
    pub logic_version: String,
    pub seed: i64,
    pub rendered_at: DateTime<Utc>,
    pub file_path: String,
    pub label: Option<String>,
}

/// A record of an epoch snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct EpochRecord {
    pub id: String,
    pub tension_id: String,
    pub timestamp: DateTime<Utc>,
    pub desire_snapshot: String,
    pub reality_snapshot: String,
    pub children_snapshot_json: Option<String>,
    pub trigger_gesture_id: Option<String>,
    /// Type of epoch: None for normal, "split_source", "split_target",
    /// "merge_source", "merge_target" for provenance events.
    pub epoch_type: Option<String>,
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
                    Some(name.to_string())
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
                    Some(name.to_string())
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

    // ── Concurrent Access ──────────────────────────────────────────

    #[test]
    fn test_concurrent_stores_coexist() {
        // MVCC concurrent mode: multiple stores on the same database can
        // read and write simultaneously. Conflicts retry automatically.
        let temp_dir = tempfile::tempdir().unwrap();
        let store1 = Store::init(temp_dir.path()).unwrap();
        let store2 = Store::init_unlocked(temp_dir.path()).unwrap();

        // Both can read
        assert!(store1.get_roots().unwrap().is_empty());
        assert!(store2.get_roots().unwrap().is_empty());

        // Both can write (MVCC handles concurrent access at page level)
        let t1 = store1.create_tension("goal 1", "reality 1").unwrap();
        let t2 = store2.create_tension("goal 2", "reality 2").unwrap();

        // Both see each other's writes
        assert_eq!(store1.get_roots().unwrap().len(), 2);
        assert!(store2.get_tension(&t1.id).unwrap().is_some());
        assert!(store1.get_tension(&t2.id).unwrap().is_some());
    }

    #[test]
    fn test_sequential_reopen_after_drop() {
        let temp_dir = tempfile::tempdir().unwrap();
        let tension_id = {
            let store1 = Store::init(temp_dir.path()).unwrap();
            let t = store1.create_tension("goal", "reality").unwrap();
            t.id.clone()
        }; // store1 dropped

        // Re-opening should succeed
        let store2 = Store::init(temp_dir.path()).unwrap();
        let retrieved = store2.get_tension(&tension_id).unwrap();
        assert!(retrieved.is_some());
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
        let tension_id = {
            let store1 = Store::init(temp_dir.path()).unwrap();
            let t = store1.create_tension("goal", "reality").unwrap();
            t.id.clone()
        }; // store1 dropped

        // Re-open the same database
        let store2 = Store::init(temp_dir.path()).unwrap();
        let retrieved = store2.get_tension(&tension_id).unwrap();
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
    fn test_resolve_tension_with_children_auto_resolves() {
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

        // Children should be auto-resolved (not reparented)
        let child1_after = store.get_tension(&child1.id).unwrap().unwrap();
        let child2_after = store.get_tension(&child2.id).unwrap().unwrap();
        assert_eq!(child1_after.status, TensionStatus::Resolved);
        assert_eq!(child2_after.status, TensionStatus::Resolved);
        // Parent relationship preserved
        assert_eq!(child1_after.parent_id, Some(parent.id.clone()));
        assert_eq!(child2_after.parent_id, Some(parent.id.clone()));
    }

    #[test]
    fn test_release_tension_with_children_auto_releases() {
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

        // Children should be auto-released
        let child1_after = store.get_tension(&child1.id).unwrap().unwrap();
        let child2_after = store.get_tension(&child2.id).unwrap().unwrap();
        assert_eq!(child1_after.status, TensionStatus::Released);
        assert_eq!(child2_after.status, TensionStatus::Released);
    }

    #[test]
    fn test_resolve_tension_with_children_records_status_mutations() {
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

        // Each child should have a status mutation recorded
        let child1_mutations = store.get_mutations(&child1.id).unwrap();
        let child2_mutations = store.get_mutations(&child2.id).unwrap();

        let child1_status_mutation = child1_mutations.iter().find(|m| m.field() == "status");
        let child2_status_mutation = child2_mutations.iter().find(|m| m.field() == "status");

        assert!(
            child1_status_mutation.is_some(),
            "child1 should have status mutation"
        );
        assert!(
            child2_status_mutation.is_some(),
            "child2 should have status mutation"
        );

        // Verify mutation records Active -> Resolved
        let m1 = child1_status_mutation.unwrap();
        assert_eq!(m1.old_value(), Some("Active"));
        assert_eq!(m1.new_value(), "Resolved");
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
    fn test_resolve_deep_hierarchy_auto_resolves_descendants() {
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
        // This should recursively resolve child and grandchild
        store
            .update_status(&parent.id, TensionStatus::Resolved)
            .unwrap();

        // Child should be resolved, parent relationship preserved
        let child_after = store.get_tension(&child.id).unwrap().unwrap();
        assert_eq!(child_after.status, TensionStatus::Resolved);
        assert_eq!(child_after.parent_id, Some(parent.id.clone()));

        // Grandchild should also be resolved
        let grandchild_after = store.get_tension(&grandchild.id).unwrap().unwrap();
        assert_eq!(grandchild_after.status, TensionStatus::Resolved);
        assert_eq!(grandchild_after.parent_id, Some(child.id));

        // Grandparent should still be active
        let grandparent_after = store.get_tension(&grandparent.id).unwrap().unwrap();
        assert_eq!(grandparent_after.status, TensionStatus::Active);
    }

    // ── Gesture Undo Tests ──────────────────────────────────────────

    #[test]
    fn test_undo_simple_field_change() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("update desire"));
        store.update_desired(&t.id, "new goal").unwrap();
        let gid = store.end_gesture().unwrap();

        // Verify current state
        let before = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(before.desired, "new goal");

        // Undo
        let undo_id = store.undo_gesture(&gid).unwrap();
        assert!(!undo_id.is_empty());

        // Verify reverted
        let after = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(after.desired, "goal");
    }

    #[test]
    fn test_undo_multi_mutation_gesture() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("multi update"));
        store.update_desired(&t.id, "new goal").unwrap();
        store.update_actual(&t.id, "new reality").unwrap();
        let gid = store.end_gesture().unwrap();

        store.undo_gesture(&gid).unwrap();

        let after = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(after.desired, "goal");
        assert_eq!(after.actual, "reality");
    }

    #[test]
    fn test_undo_conflict_detection() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        // Gesture 1: change desired
        let _ = store.begin_gesture(Some("g1"));
        store.update_desired(&t.id, "goal v2").unwrap();
        let g1 = store.end_gesture().unwrap();

        // Gesture 2: change desired again (creates conflict for g1 undo)
        let _ = store.begin_gesture(Some("g2"));
        store.update_desired(&t.id, "goal v3").unwrap();
        store.end_gesture();

        // Undo g1 should fail — desired was changed by g2
        let result = store.undo_gesture(&g1);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("conflict"),
            "Error should mention conflict: {}",
            err
        );
    }

    #[test]
    fn test_undo_double_undo_prevention() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("update"));
        store.update_desired(&t.id, "new goal").unwrap();
        let gid = store.end_gesture().unwrap();

        // First undo succeeds
        store.undo_gesture(&gid).unwrap();

        // Second undo of same gesture should fail
        let result = store.undo_gesture(&gid);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already undone"), "Error: {}", err);
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("update"));
        store.update_desired(&t.id, "new goal").unwrap();
        let gid = store.end_gesture().unwrap();

        // Undo
        let undo_id = store.undo_gesture(&gid).unwrap();
        let after_undo = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(after_undo.desired, "goal");

        // Redo (undo the undo)
        let _redo_id = store.undo_gesture(&undo_id).unwrap();
        let after_redo = store.get_tension(&t.id).unwrap().unwrap();
        assert_eq!(after_redo.desired, "new goal");
    }

    #[test]
    fn test_undo_status_change() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("resolve"));
        store.update_status(&t.id, TensionStatus::Resolved).unwrap();
        let gid = store.end_gesture().unwrap();

        assert_eq!(
            store.get_tension(&t.id).unwrap().unwrap().status,
            TensionStatus::Resolved
        );

        store.undo_gesture(&gid).unwrap();

        assert_eq!(
            store.get_tension(&t.id).unwrap().unwrap().status,
            TensionStatus::Active
        );
    }

    #[test]
    fn test_undo_parent_change() {
        let mut store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "p reality").unwrap();
        let child = store.create_tension("child", "c reality").unwrap();

        let _ = store.begin_gesture(Some("reparent"));
        store.update_parent(&child.id, Some(&parent.id)).unwrap();
        let gid = store.end_gesture().unwrap();

        assert_eq!(
            store.get_tension(&child.id).unwrap().unwrap().parent_id,
            Some(parent.id.clone())
        );

        store.undo_gesture(&gid).unwrap();

        assert_eq!(
            store.get_tension(&child.id).unwrap().unwrap().parent_id,
            None
        );
    }

    #[test]
    fn test_undo_resolve_with_children_restores_active() {
        let mut store = Store::new_in_memory().unwrap();
        let parent = store.create_tension("parent", "p reality").unwrap();
        let child = store
            .create_tension_with_parent("child", "c reality", Some(parent.id.clone()))
            .unwrap();

        let _ = store.begin_gesture(Some("resolve parent"));
        store
            .update_status(&parent.id, TensionStatus::Resolved)
            .unwrap();
        let gid = store.end_gesture().unwrap();

        // Both should be resolved
        assert_eq!(
            store.get_tension(&parent.id).unwrap().unwrap().status,
            TensionStatus::Resolved
        );
        assert_eq!(
            store.get_tension(&child.id).unwrap().unwrap().status,
            TensionStatus::Resolved
        );

        // Undo should restore both to Active
        store.undo_gesture(&gid).unwrap();

        assert_eq!(
            store.get_tension(&parent.id).unwrap().unwrap().status,
            TensionStatus::Active
        );
        assert_eq!(
            store.get_tension(&child.id).unwrap().unwrap().status,
            TensionStatus::Active
        );
    }

    #[test]
    fn test_undo_creation_deletes_tension() {
        let mut store = Store::new_in_memory().unwrap();

        let _ = store.begin_gesture(Some("create"));
        let t = store.create_tension("goal", "reality").unwrap();
        let gid = store.end_gesture().unwrap();

        // Tension exists
        assert!(store.get_tension(&t.id).unwrap().is_some());

        // Undo creation
        store.undo_gesture(&gid).unwrap();

        // Tension should be gone
        assert!(store.get_tension(&t.id).unwrap().is_none());
    }

    #[test]
    fn test_undo_creation_refuses_if_other_gestures_touched() {
        let mut store = Store::new_in_memory().unwrap();

        let _ = store.begin_gesture(Some("create"));
        let t = store.create_tension("goal", "reality").unwrap();
        let g1 = store.end_gesture().unwrap();

        // Another gesture touches the tension
        let _ = store.begin_gesture(Some("update"));
        store.update_desired(&t.id, "new goal").unwrap();
        store.end_gesture();

        // Undo creation should fail
        let result = store.undo_gesture(&g1);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("mutations from other gestures"),
            "Error: {}",
            err
        );
    }

    #[test]
    fn test_undo_deletion_refused() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("delete"));
        store.delete_tension(&t.id).unwrap();
        let gid = store.end_gesture().unwrap();

        let result = store.undo_gesture(&gid);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("deletion is not supported"), "Error: {}", err);
    }

    #[test]
    fn test_undo_note_creates_retraction() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("add note"));
        store
            .record_mutation(&Mutation::new(
                t.id.clone(),
                Utc::now(),
                "note".to_owned(),
                None,
                "my observation".to_owned(),
            ))
            .unwrap();
        let gid = store.end_gesture().unwrap();

        let undo_id = store.undo_gesture(&gid).unwrap();

        // Check that a note_retracted mutation was recorded
        let undo_mutations = store.get_gesture_mutations(&undo_id).unwrap();
        let retraction = undo_mutations
            .iter()
            .find(|m| m.field() == "note_retracted");
        assert!(retraction.is_some(), "Should have note_retracted mutation");
        assert_eq!(retraction.unwrap().old_value(), Some("my observation"));
    }

    #[test]
    fn test_undo_edge_cleanup() {
        let mut store = Store::new_in_memory().unwrap();
        let t1 = store.create_tension("t1", "r1").unwrap();
        let t2 = store.create_tension("t2", "r2").unwrap();

        // A gesture that creates an edge AND a mutation (edges alone don't count as mutations)
        let _ = store.begin_gesture(Some("split with edge"));
        store.update_actual(&t1.id, "r1 updated").unwrap();
        store
            .create_edge(&t1.id, &t2.id, crate::edge::EDGE_SPLIT_FROM)
            .unwrap();
        let gid = store.end_gesture().unwrap();

        // Edge should exist
        let edges = store.get_edges_for_tension(&t1.id).unwrap();
        assert!(
            edges
                .iter()
                .any(|e| e.edge_type == crate::edge::EDGE_SPLIT_FROM)
        );

        // Undo should delete the edge (and revert the mutation)
        store.undo_gesture(&gid).unwrap();
        let edges_after = store.get_edges_for_tension(&t1.id).unwrap();
        assert!(
            !edges_after
                .iter()
                .any(|e| e.edge_type == crate::edge::EDGE_SPLIT_FROM)
        );

        // Mutation should also be reverted
        let t1_after = store.get_tension(&t1.id).unwrap().unwrap();
        assert_eq!(t1_after.actual, "r1");
    }

    #[test]
    fn test_undo_epoch_cleanup() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let _ = store.begin_gesture(Some("with epoch"));
        store.update_actual(&t.id, "new reality").unwrap();
        let gid = store.active_gesture().map(|s| s.to_owned());
        // Create an epoch linked to this gesture
        let _ = store.create_epoch(&t.id, "goal", "new reality", None, gid.as_deref());
        let gid = store.end_gesture().unwrap();

        // Undo should remove the epoch
        store.undo_gesture(&gid).unwrap();

        // Verify epoch was cleaned up by checking the tension's epochs
        let conn = store.conn.borrow();
        let rows = conn
            .query_with_params(
                "SELECT COUNT(*) FROM epochs WHERE trigger_gesture_id = ?1",
                &[fsqlite::SqliteValue::Text(gid.into())],
            )
            .unwrap();
        let count = match rows.first().and_then(|r| r.get(0)) {
            Some(fsqlite::SqliteValue::Integer(n)) => *n,
            _ => 0,
        };
        assert_eq!(count, 0, "Epochs should be deleted on undo");
    }

    #[test]
    fn test_undo_gesture_not_found() {
        let store = Store::new_in_memory().unwrap();
        let result = store.undo_gesture("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_get_last_gesture_id() {
        let mut store = Store::new_in_memory().unwrap();
        assert!(store.get_last_gesture_id().unwrap().is_none());

        let _ = store.begin_gesture(Some("first"));
        let g1 = store.end_gesture().unwrap();

        assert_eq!(store.get_last_gesture_id().unwrap(), Some(g1.clone()));

        let _ = store.begin_gesture(Some("second"));
        let g2 = store.end_gesture().unwrap();

        assert_eq!(store.get_last_gesture_id().unwrap(), Some(g2));
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
            panic!("expected HorizonChanged event"); // ubs:ignore test assertion
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
            panic!("expected TensionCreated event"); // ubs:ignore test assertion
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
            panic!("expected TensionCreated event"); // ubs:ignore test assertion
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
            panic!("expected TensionCreated event"); // ubs:ignore test assertion
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
        // Create a legacy database file (sd.db) to test migration path
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
                    &**s == "horizon"
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

    // ── Gesture tests ────────────────────────────────────────────────

    #[test]
    fn test_gesture_creation() {
        let store = Store::new_in_memory().unwrap();
        let gesture_id = store.create_gesture(None, Some("test gesture")).unwrap();
        assert!(!gesture_id.is_empty());
    }

    #[test]
    fn test_gesture_links_to_mutations() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let gesture_id = store.begin_gesture(Some("update reality")).unwrap();
        store.update_actual(&t.id, "new reality").unwrap();
        store.end_gesture();

        let mutations = store.get_mutations(&t.id).unwrap();
        // Last mutation should have the gesture_id
        let last = mutations.last().unwrap();
        assert_eq!(last.gesture_id(), Some(gesture_id.as_str()));
    }

    #[test]
    fn test_gesture_groups_multiple_mutations() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let gesture_id = store.begin_gesture(Some("restructure")).unwrap();
        store.update_desired(&t.id, "new goal").unwrap();
        store.update_actual(&t.id, "new reality").unwrap();
        store.end_gesture();

        let gesture_mutations = store.get_gesture_mutations(&gesture_id).unwrap();
        assert_eq!(gesture_mutations.len(), 2);
        assert!(
            gesture_mutations
                .iter()
                .all(|m| m.gesture_id() == Some(gesture_id.as_str()))
        );
    }

    #[test]
    fn test_mutations_without_gesture_have_none() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();
        store.update_actual(&t.id, "new reality").unwrap();

        let mutations = store.get_mutations(&t.id).unwrap();
        for m in &mutations {
            assert!(m.gesture_id().is_none());
        }
    }

    // ── Session tests ────────────────────────────────────────────────

    #[test]
    fn test_session_lifecycle() {
        let store = Store::new_in_memory().unwrap();
        assert!(store.active_session().unwrap().is_none());

        let session_id = store.start_session().unwrap();
        assert_eq!(store.active_session().unwrap(), Some(session_id.clone()));

        store
            .end_session(&session_id, Some("good session"))
            .unwrap();
        assert!(store.active_session().unwrap().is_none());
    }

    #[test]
    fn test_gesture_inherits_active_session() {
        let mut store = Store::new_in_memory().unwrap();
        let session_id = store.start_session().unwrap();

        let gesture_id = store.begin_gesture(Some("test")).unwrap();
        store.end_gesture();

        // Verify the gesture was created with the session_id
        // (We can check by querying mutations in the gesture)
        assert!(!gesture_id.is_empty());
        assert!(!session_id.is_empty());
    }

    // ── Epoch tests ──────────────────────────────────────────────────

    #[test]
    fn test_epoch_creation() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let epoch_id = store
            .create_epoch(&t.id, "goal", "reality", Some(r#"{"children":[]}"#), None)
            .unwrap();

        let epochs = store.get_epochs(&t.id).unwrap();
        assert_eq!(epochs.len(), 1);
        assert_eq!(epochs[0].id, epoch_id);
        assert_eq!(epochs[0].tension_id, t.id);
        assert_eq!(epochs[0].desire_snapshot, "goal");
        assert_eq!(epochs[0].reality_snapshot, "reality");
        assert_eq!(
            epochs[0].children_snapshot_json,
            Some(r#"{"children":[]}"#.to_string())
        );
    }

    #[test]
    fn test_epoch_with_trigger_gesture() {
        let mut store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let gesture_id = store.begin_gesture(Some("update reality")).unwrap();
        store.update_actual(&t.id, "new reality").unwrap();
        store.end_gesture();

        let epoch_id = store
            .create_epoch(&t.id, "goal", "new reality", None, Some(&gesture_id))
            .unwrap();

        let epochs = store.get_epochs(&t.id).unwrap();
        assert_eq!(epochs.len(), 1);
        assert_eq!(epochs[0].id, epoch_id);
        assert_eq!(epochs[0].trigger_gesture_id, Some(gesture_id));
    }

    #[test]
    fn test_multiple_epochs_chronological() {
        let store = Store::new_in_memory().unwrap();
        let t = store.create_tension("goal", "reality").unwrap();

        let e1 = store
            .create_epoch(&t.id, "goal v1", "reality v1", None, None)
            .unwrap();
        let e2 = store
            .create_epoch(&t.id, "goal v2", "reality v2", None, None)
            .unwrap();

        let epochs = store.get_epochs(&t.id).unwrap();
        assert_eq!(epochs.len(), 2);
        assert_eq!(epochs[0].id, e1);
        assert_eq!(epochs[1].id, e2);
        assert!(epochs[0].timestamp <= epochs[1].timestamp);
    }
}

#[cfg(test)]
mod sigils {
    use super::*;

    fn sample_record(
        short_code: i32,
        rendered_at: DateTime<Utc>,
        file_path: String,
    ) -> SigilRecord {
        SigilRecord {
            id: 0,
            short_code,
            scope_canonical: "space:default".to_owned(),
            logic_id: "contemplative".to_owned(),
            logic_version: "v1".to_owned(),
            seed: 42,
            rendered_at,
            file_path,
            label: None,
        }
    }

    #[test]
    fn table_and_indexes_present() {
        let store = Store::new_in_memory().unwrap();
        let conn = store.conn.borrow();

        let table_rows = conn
            .query("SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'sigils'")
            .unwrap();
        assert!(!table_rows.is_empty(), "sigils table missing");

        let index_rows = conn.query("PRAGMA index_list(sigils)").unwrap();
        let mut index_names = Vec::new();
        for row in &index_rows {
            if let Some(SqliteValue::Text(name)) = row.get(1) {
                index_names.push(name.to_string());
            }
        }
        assert!(index_names.contains(&"idx_sigils_short_code".to_string()));
        assert!(index_names.contains(&"idx_sigils_logic".to_string()));
    }

    #[test]
    fn insert_list_get_roundtrip() {
        let store = Store::new_in_memory().unwrap();
        let rendered_at_1 = DateTime::parse_from_rfc3339("2026-01-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let rendered_at_2 = DateTime::parse_from_rfc3339("2026-01-02T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let record_1 = sample_record(1, rendered_at_1, "/tmp/sigil-1.svg".to_owned());
        let record_2 = sample_record(2, rendered_at_2, "/tmp/sigil-2.svg".to_owned());

        store.record_sigil(&record_1).unwrap();
        store.record_sigil(&record_2).unwrap();

        let sigils = store.list_sigils().unwrap();
        assert_eq!(sigils.len(), 2);
        assert_eq!(sigils[0].short_code, 1);
        assert_eq!(sigils[1].short_code, 2);

        let fetched = store.get_sigil_by_short_code(2).unwrap().unwrap();
        assert_eq!(fetched.short_code, 2);
        assert_eq!(fetched.logic_id, "contemplative");
        assert_eq!(fetched.file_path, "/tmp/sigil-2.svg");
    }

    #[test]
    fn delete_returns_existence() {
        let store = Store::new_in_memory().unwrap();
        let rendered_at = DateTime::parse_from_rfc3339("2026-01-03T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let temp_path = std::env::temp_dir()
            .join(format!("sigil-{}.svg", ulid::Ulid::new()))
            .to_string_lossy()
            .to_string();
        std::fs::write(&temp_path, "<svg/>").unwrap();

        let record = sample_record(7, rendered_at, temp_path.clone());
        store.record_sigil(&record).unwrap();

        assert!(store.delete_sigil(7).unwrap());
        assert!(!store.delete_sigil(7).unwrap());
        assert!(std::path::Path::new(&temp_path).exists());
        let _ = std::fs::remove_file(&temp_path);
    }
}
