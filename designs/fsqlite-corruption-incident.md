# fsqlite Corruption Incident Report

**Date:** 2025-03-25
**Severity:** Data at risk, fully recovered, no loss
**Root cause:** Parallel CLI processes writing to the same fsqlite WAL

## What Happened

During a Claude Code session, five `cargo run --bin werk -- add` commands were launched in parallel (via concurrent tool calls). All five attempted to write to the same `.werk/sd.db` simultaneously. Two succeeded (#62, #63). The third failed with `malformed SQLite record blob`, and the database became unreadable.

After the corruption:
- `cargo run --bin werk -- tree` → `DatabaseCorrupt { detail: "invalid B-tree page type flag: 0x00" }`
- All subsequent commands failed
- The base DB file was 548KB, the WAL was 3.9MB (most data lived in uncommitted WAL pages)

## Why It Happened

werk uses **fsqlite** (FrankenSQLite, `fsqlite` crate v0.1.1), a pure-Rust SQLite-compatible database engine. Key facts discovered during recovery:

- fsqlite is **not** a wrapper around limbo-rs or C SQLite — it's an independent implementation with its own B-tree, pager, WAL, and MVCC layers
- fsqlite uses **WAL mode** by default (write-ahead logging)
- fsqlite's WAL implementation does **not safely handle multi-process concurrent writes**
- Each `cargo run --bin werk` invocation is a separate OS process with its own fsqlite connection
- The parallel writes corrupted B-tree page pointers in the WAL, creating broken overflow chains

Standard SQLite handles this via `POSIX advisory locks` on the WAL. fsqlite either doesn't implement this locking or implements it incorrectly for multi-process scenarios.

## Recovery Process

### Attempt 1: sqlite3 CLI (failed)

Standard `sqlite3` could not read the database either:
```
Error: in prepare, malformed database schema (sessions) - invalid rootpage (11)
```

This is expected — even a freshly-created fsqlite database is unreadable by standard sqlite3. Despite claiming "compatibility mode," fsqlite's binary format diverges from C SQLite at the page/schema level.

### Attempt 2: sqlite3 .recover → .dump → reimport (failed)

```sh
sqlite3 .werk/sd.db ".recover" | sqlite3 /tmp/recovered.db
sqlite3 /tmp/recovered.db ".dump" > /tmp/dump.sql
sqlite3 /tmp/fresh.db < /tmp/dump.sql
```

The recovered DB was readable by sqlite3 (verified: 60 tensions, 425 mutations) but fsqlite rejected it with `overflow chain ended prematurely: got 5019 of 9111 bytes`. The `.recover` command mapped all TEXT columns as BLOB and produced a schema that fsqlite's stricter pager couldn't parse.

VACUUM didn't help. Matching the original page size (4096) didn't help.

### Attempt 3: JSON export → programmatic reimport via fsqlite (succeeded)

1. Exported all tables from the sqlite3-recovered DB as JSON:
   ```sh
   sqlite3 /tmp/recovered.db -json "SELECT * FROM tensions;" > /tmp/tensions.json
   sqlite3 /tmp/recovered.db -json "SELECT * FROM mutations ORDER BY id;" > /tmp/mutations.json
   sqlite3 /tmp/recovered.db -json "SELECT * FROM gestures;" > /tmp/gestures.json
   sqlite3 /tmp/recovered.db -json "SELECT * FROM epochs;" > /tmp/epochs.json
   ```

2. Wrote a one-shot Rust binary (`migrate-db`) that:
   - Called `Store::init()` to create a fresh fsqlite DB with correct schema
   - Dropped the Store (releasing the connection)
   - Opened the fresh DB directly via `fsqlite::Connection::open()`
   - Inserted all records using `execute_with_params` with properly typed `SqliteValue` parameters (Text, Integer, Null — not Blob)
   - Recreated indexes

3. Verified: `cargo run --bin werk -- tree` worked. All 60 tensions, 425 mutations, 153 gestures, 29 epochs present.

The key insight: data must flow **through fsqlite's own write path** to produce pages its pager can read. You cannot create a DB with standard sqlite3 and expect fsqlite to read it, even though both claim SQLite format compatibility.

## Data Inventory

| Table | Records | Status |
|-------|---------|--------|
| tensions | 60 | Fully recovered |
| mutations | 425 | Fully recovered |
| gestures | 153 | Fully recovered |
| epochs | 29 | Fully recovered |
| sessions | 0 | No data to lose |

The two tensions created before the crash (#62, #63) were present in the recovered data. The three that failed (#64, #65, #66 — created later via sequential commands after recovery) were created fresh.

## Mitigations

### Immediate
- Never run parallel `werk` CLI commands against the same store
- The migration binary was removed after use (one-shot tool)

### Recommended
1. **Advisory file locking in Store::open()** — take an exclusive `flock()` on `.werk/sd.db.lock` to prevent concurrent writes. This is the standard SQLite approach.
2. **Backup before write** — periodic WAL checkpointing or snapshot on session start
3. **Document the constraint** — fsqlite is single-writer. Note this in CLAUDE.md or the store module.

## Files

- `.werk/sd.db` — production database (recovered, live)
- `/tmp/tensions.json`, `/tmp/mutations.json`, `/tmp/gestures.json`, `/tmp/epochs.json` — recovery artifacts (will be cleaned on reboot)
- `/tmp/sd_werk_recovered.db` — intermediate sqlite3 recovery (will be cleaned on reboot)
