//! Parity test: `werk-tab/app.js` port range literals must match
//! `werk_shared::daemon_net::DEFAULT_PORT_RANGE`.
//!
//! The browser extension is hand-loaded via "Load unpacked", which makes a
//! build-script-generated constants file awkward. Instead we keep the literals
//! in both places and fail CI if they diverge. One test is cheaper than a
//! build-time code generator and easier to read than either.

use std::path::PathBuf;

use werk_shared::daemon_net::DEFAULT_PORT_RANGE;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `.../werk/werk-cli`. Its parent is the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("werk-cli has a parent")
        .to_path_buf()
}

fn extract_u16(js: &str, name: &str) -> u16 {
    // Matches `const NAME = 1234;` with arbitrary whitespace around `=`.
    let needle = format!("const {name}");
    let start = js
        .find(&needle)
        .unwrap_or_else(|| panic!("{name} not found in werk-tab/app.js"));
    let after_eq = js[start..]
        .find('=')
        .unwrap_or_else(|| panic!("{name} missing '=' in werk-tab/app.js"));
    let rest = &js[start + after_eq + 1..];
    let digits: String = rest
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse()
        .unwrap_or_else(|e| panic!("{name} not a u16 in werk-tab/app.js: {e}"))
}

#[test]
fn app_js_port_range_matches_rust() {
    let js_path = workspace_root().join("werk-tab").join("app.js");
    let js = std::fs::read_to_string(&js_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", js_path.display()));

    let start = extract_u16(&js, "PORT_RANGE_START");
    let end = extract_u16(&js, "PORT_RANGE_END");

    assert_eq!(
        (start, end),
        DEFAULT_PORT_RANGE,
        "werk-tab/app.js port range ({start}-{end}) diverges from \
         werk_shared::daemon_net::DEFAULT_PORT_RANGE ({}-{}). \
         Update both and re-run.",
        DEFAULT_PORT_RANGE.0,
        DEFAULT_PORT_RANGE.1,
    );
}
