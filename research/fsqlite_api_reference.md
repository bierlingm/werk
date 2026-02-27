# fsqlite (FrankenSQLite) Rust API Reference

## Overview

**fsqlite** is the public API facade for FrankenSQLite — a from-scratch SQLite-compatible database engine written in pure safe Rust (zero unsafe blocks). It provides concurrent writers via MVCC, self-healing storage via RaptorQ fountain codes, and full SQLite file format compatibility.

- **Crate**: `fsqlite`
- **Version**: 0.1.1
- **Author**: Jeffrey Emanuel (Dicklesworthstone)
- **Repository**: https://github.com/Dicklesworthstone/frankensqlite
- **License**: MIT (with OpenAI/Anthropic Rider)

---

## Requirements

### Rust Toolchain
- **Edition**: 2024
- **Minimum Rust Version**: 1.85.0
- **Channel**: Nightly (required for edition 2024)

The repository includes `rust-toolchain.toml` that specifies the nightly channel. If you encounter `#![feature]` errors, install nightly:

```bash
rustup default nightly
# or
rustup update nightly
```

---

## Cargo.toml Dependency

### Basic Usage (default features)
```toml
[dependencies]
fsqlite = "0.1.1"
```

### With Specific Features
```toml
[dependencies]
fsqlite = { version = "0.1.1", features = ["mvcc", "raptorq"] }
```

### All Features
```toml
[dependencies]
fsqlite = { version = "0.1.1", features = ["json", "fts5", "fts3", "rtree", "session", "icu", "misc", "raptorq", "mvcc"] }
```

---

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `json` | yes | JSON1 extension (`json()`, `json_extract()`, etc.) |
| `fts5` | yes | Full-text search v5 |
| `rtree` | yes | R-Tree spatial index |
| `fts3` | no | Full-text search v3/v4 (legacy) |
| `session` | no | Session extension (changeset/patchset) |
| `icu` | no | ICU Unicode collation/tokenization |
| `misc` | no | Miscellaneous extensions (generate_series, carray, dbstat, dbpage) |
| `raptorq` | no | RaptorQ erasure coding support (self-healing storage) |
| `mvcc` | no | Multi-version concurrency control (explicit feature, but MVCC is always available via PRAGMA) |

**Note**: The `mvcc` feature flag exists but MVCC is always compiled in. Enable concurrent writers via `PRAGMA fsqlite.concurrent_mode=ON;`.

---

## Core Types

### `Connection`
A database connection. The primary entry point for all database operations.

### `PreparedStatement`
A compiled SQL statement for repeated execution with different parameters.

### `Row`
A single result row. Access columns by index with `row.get(i)` or get all values with `row.values()`.

### `SqliteValue` (from `fsqlite_types::value::SqliteValue`)
The value type used for parameters and results. Variants:
- `SqliteValue::Null`
- `SqliteValue::Integer(i64)`
- `SqliteValue::Float(f64)`
- `SqliteValue::Text(String)`
- `SqliteValue::Blob(Vec<u8>)`

---

## API Usage Examples

### 1. Opening a Database

```rust
use fsqlite::Connection;
use fsqlite_error::Result;

fn main() -> Result<()> {
    // In-memory database
    let conn = Connection::open(":memory:")?;
    
    // File-based database
    let conn = Connection::open("myapp.db")?;
    
    // Get connection path
    assert_eq!(conn.path(), "myapp.db");
    
    Ok(())
}
```

### 2. Creating Tables (DDL)

```rust
use fsqlite::Connection;
use fsqlite_error::Result;

fn setup_schema(conn: &Connection) -> Result<()> {
    // Create table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE
        )"
    )?;
    
    Ok(())
}
```

### 3. Inserting Data

```rust
use fsqlite::Connection;
use fsqlite_error::Result;
use fsqlite_types::value::SqliteValue;

fn insert_data(conn: &Connection) -> Result<()> {
    // Simple insert (returns affected row count)
    let count = conn.execute("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');")?;
    assert_eq!(count, 1);
    
    // Insert multiple rows
    let count = conn.execute(
        "INSERT INTO users VALUES (2, 'Bob', 'bob@example.com'), (3, 'Carol', 'carol@example.com');"
    )?;
    assert_eq!(count, 2);
    
    // Parameterized insert (safe, prevents SQL injection)
    let count = conn.execute_with_params(
        "INSERT INTO users (name, email) VALUES (?1, ?2)",
        &[
            SqliteValue::Text("David".to_owned()),
            SqliteValue::Text("david@example.com".to_owned()),
        ],
    )?;
    
    Ok(())
}
```

### 4. Querying Data

```rust
use fsqlite::Connection;
use fsqlite_error::Result;
use fsqlite_types::value::SqliteValue;

fn query_data(conn: &Connection) -> Result<()> {
    // Query all rows
    let rows = conn.query("SELECT id, name, email FROM users;")?;
    for row in &rows {
        let id = row.get(0).expect("id column");
        let name = row.get(1).expect("name column");
        let email = row.get(2).expect("email column");
        println!("id={:?}, name={:?}, email={:?}", id, name, email);
    }
    
    // Query with parameters
    let rows = conn.query_with_params(
        "SELECT id, name FROM users WHERE name = ?1",
        &[SqliteValue::Text("Alice".to_owned())],
    )?;
    
    // Single row convenience
    let row = conn.query_row("SELECT count(*) FROM users;")?;
    let count = row.get(0).expect("count");
    println!("Total users: {:?}", count);
    
    // Query row with parameters
    let row = conn.query_row_with_params(
        "SELECT email FROM users WHERE id = ?1",
        &[SqliteValue::Integer(1)]
    )?;
    
    Ok(())
}
```

### 5. Prepared Statements

```rust
use fsqlite::Connection;
use fsqlite_error::Result;
use fsqlite_types::value::SqliteValue;

fn prepared_statements(conn: &Connection) -> Result<()> {
    // Prepare once, execute many times
    let stmt = conn.prepare("SELECT * FROM users WHERE id = ?1;")?;
    
    // Query with params
    let rows = stmt.query_with_params(&[SqliteValue::Integer(1)])?;
    
    // Query single row
    let row = stmt.query_row_with_params(&[SqliteValue::Integer(2)])?;
    
    // Execute (for non-SELECT statements)
    let count = conn.prepare("DELETE FROM users WHERE id = ?1;")?
        .execute_with_params(&[SqliteValue::Integer(3)])?;
    
    // EXPLAIN query plan
    let explain = stmt.explain();
    println!("Query plan: {}", explain);
    
    Ok(())
}
```

### 6. Transactions

```rust
use fsqlite::Connection;
use fsqlite_error::Result;

fn basic_transaction(conn: &Connection) -> Result<()> {
    // Check transaction state
    assert!(!conn.in_transaction());
    
    // Begin transaction
    conn.execute("BEGIN;")?;
    assert!(conn.in_transaction());
    
    // Perform operations
    conn.execute("INSERT INTO users VALUES (10, 'Temp', 'temp@example.com');")?;
    
    // Commit
    conn.execute("COMMIT;")?;
    assert!(!conn.in_transaction());
    
    // Or rollback
    conn.execute("BEGIN;")?;
    conn.execute("INSERT INTO users VALUES (11, 'Temp2', 'temp2@example.com');")?;
    conn.execute("ROLLBACK;")?; // Changes discarded
    
    Ok(())
}

fn savepoints(conn: &Connection) -> Result<()> {
    conn.execute("CREATE TABLE t (v INTEGER);")?;
    conn.execute("INSERT INTO t VALUES (1);")?;
    
    // Create savepoint
    conn.execute("SAVEPOINT sp1;")?;
    conn.execute("INSERT INTO t VALUES (2);")?;
    
    // Rollback to savepoint
    conn.execute("ROLLBACK TO sp1;")?;
    // Row 2 is now gone
    
    // Or release savepoint (commits changes since savepoint)
    conn.execute("SAVEPOINT sp2;")?;
    conn.execute("INSERT INTO t VALUES (3);")?;
    conn.execute("RELEASE sp2;")?; // Row 3 is committed
    
    Ok(())
}
```

### 7. MVCC Mode (Concurrent Writers)

```rust
use fsqlite::Connection;
use fsqlite_error::Result;
use fsqlite_types::value::SqliteValue;

fn enable_mvcc(conn: &Connection) -> Result<()> {
    // Enable concurrent mode (MVCC) - enables multiple concurrent writers
    conn.execute("PRAGMA fsqlite.concurrent_mode=ON;")?;
    
    Ok(())
}

// Concurrent writers example
fn concurrent_writers_example() -> Result<()> {
    use std::thread;
    use std::sync::Barrier;
    use std::sync::Arc;
    
    let db_path = "concurrent_test.db";
    
    // Setup
    {
        let conn = Connection::open(db_path)?;
        conn.execute("PRAGMA fsqlite.concurrent_mode=ON;")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY,
                balance INTEGER NOT NULL
            );"
        )?;
        conn.execute("INSERT INTO accounts VALUES (1, 0);")?;
    }
    
    // Spawn concurrent writers
    let barrier = Arc::new(Barrier::new(2));
    let barrier2 = Arc::clone(&barrier);
    
    let handle1 = thread::spawn(move || {
        let conn = Connection::open(db_path).unwrap();
        conn.execute("PRAGMA fsqlite.concurrent_mode=ON;").unwrap();
        conn.execute("BEGIN CONCURRENT;").unwrap();
        
        barrier.wait();
        
        conn.execute("UPDATE accounts SET balance = balance + 1 WHERE id = 1;").unwrap();
        conn.execute("COMMIT;").unwrap();
    });
    
    let handle2 = thread::spawn(move || {
        let conn = Connection::open(db_path).unwrap();
        conn.execute("PRAGMA fsqlite.concurrent_mode=ON;").unwrap();
        conn.execute("BEGIN CONCURRENT;").unwrap();
        
        barrier2.wait();
        
        // This may fail with transient error if conflicts
        match conn.execute("UPDATE accounts SET balance = balance + 1 WHERE id = 1;") {
            Ok(_) => { let _ = conn.execute("COMMIT;"); }
            Err(e) => { 
                if e.is_transient() {
                    let _ = conn.execute("ROLLBACK;");
                }
            }
        }
    });
    
    handle1.join().unwrap();
    handle2.join().unwrap();
    
    Ok(())
}
```

### 8. Storage Mode Selection

```rust
use fsqlite::Connection;
use fsqlite_error::Result;

fn storage_modes(conn: &Connection) -> Result<()> {
    // Compatibility mode (default) - standard SQLite .db file format
    // Readable by C SQLite, libSQL, etc.
    conn.execute("PRAGMA fsqlite.mode = compatibility;")?;
    
    // Native mode - Erasure-Coded Stream (ECS) format
    // Append-only, content-addressed, RaptorQ-protected
    // NOT readable by standard SQLite
    conn.execute("PRAGMA fsqlite.mode = native;")?;
    
    Ok(())
}
```

### 9. Encryption

```rust
use fsqlite::Connection;
use fsqlite_error::Result;

fn encrypted_database() -> Result<()> {
    let conn = Connection::open("encrypted.db")?;
    
    // Set encryption key (uses Argon2id + XChaCha20-Poly1305)
    conn.execute("PRAGMA key = 'my_secret_passphrase';")?;
    
    // All subsequent operations are encrypted
    conn.execute("CREATE TABLE secrets (data TEXT);")?;
    conn.execute("INSERT INTO secrets VALUES ('sensitive data');")?;
    
    // Re-key (O(1) - just re-wraps the DEK)
    conn.execute("PRAGMA rekey = 'new_passphrase';")?;
    
    Ok(())
}
```

### 10. Error Handling

```rust
use fsqlite::Connection;
use fsqlite_error::{Result, FrankenError};

fn error_handling() {
    let conn = Connection::open(":memory:").unwrap();
    
    // Check for specific errors
    match conn.execute("SELECT * FROM nonexistent_table;") {
        Ok(_) => {}
        Err(FrankenError::Internal(msg)) => {
            println!("Internal error: {}", msg);
        }
        Err(FrankenError::QueryReturnedNoRows) => {
            println!("No rows returned");
        }
        Err(FrankenError::CannotOpen { path }) => {
            println!("Cannot open: {}", path);
        }
        Err(e) => {
            // Check if error is transient (retryable)
            if e.is_transient() {
                println!("Transient error, can retry: {:?}", e);
            } else {
                println!("Other error: {:?}", e);
            }
        }
    }
}
```

---

## Complete Quickstart Example

```rust
use fsqlite::Connection;
use fsqlite_error::Result;
use fsqlite_types::value::SqliteValue;

fn main() -> Result<()> {
    // Open an in-memory database
    let db = Connection::open(":memory:")?;
    
    // Create table
    db.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE
        )",
    )?;
    
    // Insert with parameters
    db.execute_with_params(
        "INSERT INTO users (name, email) VALUES (?1, ?2)",
        &[
            SqliteValue::Text("Alice".to_owned()),
            SqliteValue::Text("alice@example.com".to_owned()),
        ],
    )?;
    
    // Query with prepared statement
    let stmt = db.prepare("SELECT id, name FROM users WHERE name = ?1")?;
    let rows = stmt.query_with_params(
        &[SqliteValue::Text("Alice".to_owned())],
    )?;
    
    for row in &rows {
        let id = row.get(0).expect("id column");
        let name = row.get(1).expect("name column");
        println!("Found: {id:?} — {name:?}");
    }
    
    Ok(())
}
```

---

## Storage Format Compatibility

| Mode | File Extension | Readable by C SQLite | Notes |
|------|----------------|----------------------|-------|
| Compatibility | `.db`, `.sqlite3`, `.sqlite` | Yes | Standard SQLite format |
| Native (ECS) | `.ecs` | No | Append-only erasure-coded format |

**Important**: Native mode databases cannot be opened by standard SQLite tools. Use compatibility mode for interoperability.

---

## Known Limitations and Gotchas

1. **Nightly Rust Required**: Edition 2024 requires nightly compiler. The `rust-toolchain.toml` in the repo handles this automatically.

2. **Pre-release Software**: This is early-stage software. The README states "Production Maturity: ⚠ Early".

3. **No Async API**: The current API is synchronous. Async operations are mentioned in architecture docs but not yet exposed.

4. **SSI Write Skew**: With Serializable Snapshot Isolation enabled (default), write skew anomalies cause transaction aborts. Downgrade to plain SI with `PRAGMA fsqlite.serializable = OFF` if acceptable.

5. **Native Mode Compatibility**: Native/ECS format databases are NOT readable by C SQLite. Use compatibility mode for interoperability.

6. **Windows VFS**: Windows is supported via `WindowsVfs`, but some features may have platform-specific behavior.

7. **Long-running Readers**: Readers holding snapshots open for long periods pin old page versions, preventing garbage collection. Use connection timeouts to prevent memory growth.

8. **Feature Flags**: Some features like `raptorq` and `mvcc` exist as feature flags but MVCC is always available via PRAGMA.

9. **Error::is_transient()**: Check this method on errors to determine if an operation can be retried (e.g., concurrent write conflicts).

10. **No Releases Published**: As of the research date, no GitHub releases have been published. Use the crates.io version.

---

## Additional Resources

- **GitHub Repository**: https://github.com/Dicklesworthstone/frankensqlite
- **Documentation**: https://docs.rs/fsqlite
- **Website**: https://frankensqlite.com/
- **Getting Started**: https://frankensqlite.com/getting-started

---

## Architecture Summary

FrankenSQLite is organized as a 26-crate workspace:

```
fsqlite (Public API facade - you use this)
├── fsqlite-core (Core engine integration)
│   ├── fsqlite-types (Core types)
│   ├── fsqlite-error (Error handling)
│   ├── fsqlite-vfs (Virtual filesystem)
│   ├── fsqlite-pager (Page cache)
│   ├── fsqlite-wal (Write-ahead log)
│   ├── fsqlite-mvcc (MVCC concurrency)
│   ├── fsqlite-btree (B-tree storage)
│   ├── fsqlite-ast (SQL AST)
│   ├── fsqlite-parser (SQL parser)
│   ├── fsqlite-planner (Query planner)
│   ├── fsqlite-vdbe (Bytecode VM)
│   └── fsqlite-func (Built-in functions)
└── Extensions (fts3, fts5, json, rtree, session, icu, misc)
```

All crates compile with `unsafe_code = "forbid"` — zero unsafe blocks throughout.

---

*Last updated: 2026-02-27*
*Version researched: fsqlite 0.1.1*
