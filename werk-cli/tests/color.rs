//! Integration tests for output formatting.
//!
//! Tests verify:
//! - VAL-FMT-001: Plain text output has no ANSI escape codes
//! - VAL-FMT-002: JSON output has no ANSI codes
//!
//! Since colors have been removed, all output should be plain text.
//! ANSI escape codes start with \x1b[ (ESC followed by [)

use assert_cmd::cargo_bin_cmd;
use tempfile::TempDir;

/// Helper: Check if output contains ANSI escape codes
fn has_ansi_codes(output: &str) -> bool {
    output.contains("\x1b[") || output.contains("\u{1b}[")
}

/// Extract a ULID from werk output.
fn extract_ulid(output: &str) -> Option<String> {
    let chars: Vec<char> = output.chars().collect();
    for i in 0..chars.len().saturating_sub(25) {
        let slice: String = chars[i..i + 26].iter().collect();
        if slice
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Some(slice);
        }
    }
    None
}

// =============================================================================
// VAL-FMT-001: Plain text output has no ANSI codes (colors removed)
// =============================================================================

/// werk tree produces no ANSI codes
#[test]
fn test_plain_text_tree() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "Plain text output should have no ANSI codes in tree output, got: {}",
        stdout
    );
}

/// werk show produces no ANSI codes
#[test]
fn test_plain_text_show() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Use short code to show (first tension is #1)
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg("1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "Plain text output should have no ANSI codes in show output, got: {}",
        stdout
    );
}

/// werk add produces no ANSI codes
#[test]
fn test_plain_text_add() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "Plain text output should have no ANSI codes in add output, got: {}",
        stdout
    );
}

/// werk init produces no ANSI codes
#[test]
fn test_plain_text_init() {
    let dir = TempDir::new().unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "Plain text output should have no ANSI codes in init output, got: {}",
        stdout
    );
}

// =============================================================================
// Combined tests: Verify output is still readable
// =============================================================================

/// Tree output is readable (contains expected content)
#[test]
fn test_tree_readable() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("write a novel")
        .arg("have an outline")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should still contain the goal text
    assert!(
        stdout.contains("write a novel"),
        "Output should be readable, should contain 'write a novel', got: {}",
        stdout
    );

    // Should show short code
    assert!(
        stdout.contains("#1"),
        "Output should show short code #1, got: {}",
        stdout
    );
}

/// Show output is readable (contains expected content)
#[test]
fn test_show_readable() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("write a novel")
        .arg("have an outline")
        .current_dir(dir.path())
        .assert()
        .success();

    // Use short code to show
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg("1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should still contain expected fields
    assert!(
        stdout.contains("Tension"),
        "Output should contain 'Tension', got: {}",
        stdout
    );
    assert!(
        stdout.contains("Desired:"),
        "Output should contain 'Desired:', got: {}",
        stdout
    );
    assert!(
        stdout.contains("Actual:"),
        "Output should contain 'Actual:', got: {}",
        stdout
    );
    assert!(
        stdout.contains("write a novel"),
        "Output should contain desired text, got: {}",
        stdout
    );
    assert!(
        stdout.contains("have an outline"),
        "Output should contain actual text, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Status:"),
        "Output should contain 'Status:', got: {}",
        stdout
    );
    assert!(
        stdout.contains("Active"),
        "Output should show Active status, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Facts:"),
        "Output should contain 'Facts:', got: {}",
        stdout
    );
    assert!(
        stdout.contains("Closure:"),
        "Output should contain 'Closure:', got: {}",
        stdout
    );
}

/// Error messages are readable
#[test]
fn test_error_readable() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg("NONEXISTENT")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);

    // Should contain error message
    assert!(
        stderr.contains("error:"),
        "Error output should contain 'error:', got: {}",
        stderr
    );

    // Should have no ANSI codes
    assert!(
        !has_ansi_codes(&stderr),
        "Error output should have no ANSI codes, got: {}",
        stderr
    );
}

/// JSON output has no ANSI codes
#[test]
fn test_json_no_ansi_codes() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // JSON output should never have ANSI codes
    assert!(
        !has_ansi_codes(&stdout),
        "JSON output should have no ANSI codes, got: {}",
        stdout
    );

    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Should be valid JSON, got: {}", stdout);
}
