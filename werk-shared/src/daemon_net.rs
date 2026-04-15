//! Network constants shared across the daemon, serve command, and browser
//! extension.
//!
//! These live in `werk-shared` because the browser extension (`werk-tab`)
//! encodes the same values in JavaScript. A Rust test in
//! `werk-cli/tests/port_range_parity.rs` parses `werk-tab/app.js` and asserts
//! the two literals agree, so drift fails CI rather than appearing as a silent
//! port-probe mismatch.

/// Default port range scanned by `werk serve --port-range` and baked into
/// `werk daemon install` when the user doesn't pass `--port-range`.
///
/// End is inclusive. Eleven ports (3749–3759) is generous: if all are taken,
/// the operator has bigger problems than a port conflict.
pub const DEFAULT_PORT_RANGE: (u16, u16) = (3749, 3759);

/// Default single port used when `--port` / `--port-range` are both absent.
/// Matches `DEFAULT_PORT_RANGE.0` intentionally: the common case is "nothing
/// else is listening, bind 3749".
pub const DEFAULT_PORT: u16 = DEFAULT_PORT_RANGE.0;

/// Filename the daemon writes under `<werk_dir>/` with the port it actually
/// bound. The browser extension can't read files from the filesystem sandbox,
/// so this file exists for CLI introspection (`werk daemon status`) only —
/// the extension rediscovers via port probing.
pub const PORT_FILE_NAME: &str = "daemon.port";
