//! Integration tests for color and formatting control.
//!
//! Tests verify:
//! - VAL-FMT-001: NO_COLOR=1 env var disables all ANSI escape codes
//! - VAL-FMT-002: --no-color flag disables colors
//! - VAL-FMT-003: Colors auto-disabled for pipe/non-TTY
//!
//! ANSI escape codes start with \x1b[ (ESC followed by [)

use assert_cmd::Command;
use predicates::prelude::*;
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
// VAL-FMT-001: NO_COLOR env var disables ANSI codes
// =============================================================================

/// NO_COLOR=1 werk tree produces no ANSI codes
#[test]
fn test_no_color_env_tree() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("tree")
        .env("NO_COLOR", "1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "NO_COLOR=1 should disable ANSI codes in tree output, got: {}",
        stdout
    );
}

/// NO_COLOR=1 werk show produces no ANSI codes
#[test]
fn test_no_color_env_show() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
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
    let id = extract_ulid(&stdout).expect("Should have tension ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&id[..8])
        .env("NO_COLOR", "1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "NO_COLOR=1 should disable ANSI codes in show output, got: {}",
        stdout
    );
}

/// NO_COLOR=1 werk add produces no ANSI codes
#[test]
fn test_no_color_env_add() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .env("NO_COLOR", "1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "NO_COLOR=1 should disable ANSI codes in add output, got: {}",
        stdout
    );
}

/// NO_COLOR=1 werk init produces no ANSI codes
#[test]
fn test_no_color_env_init() {
    let dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .env("NO_COLOR", "1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "NO_COLOR=1 should disable ANSI codes in init output, got: {}",
        stdout
    );
}

// =============================================================================
// VAL-FMT-002: --no-color flag disables colors
// =============================================================================

/// werk tree --no-color produces no ANSI codes
#[test]
fn test_no_color_flag_tree() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
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
        "--no-color should disable ANSI codes in tree output, got: {}",
        stdout
    );
}

/// werk show --no-color produces no ANSI codes
#[test]
fn test_no_color_flag_show() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
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
    let id = extract_ulid(&stdout).expect("Should have tension ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
        .arg("show")
        .arg(&id[..8])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "--no-color should disable ANSI codes in show output, got: {}",
        stdout
    );
}

/// werk add --no-color produces no ANSI codes
#[test]
fn test_no_color_flag_add() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
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
        "--no-color should disable ANSI codes in add output, got: {}",
        stdout
    );
}

/// werk init --no-color produces no ANSI codes
#[test]
fn test_no_color_flag_init() {
    let dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
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
        "--no-color should disable ANSI codes in init output, got: {}",
        stdout
    );
}

/// werk show --verbose --no-color produces no ANSI codes
#[test]
fn test_no_color_flag_show_verbose() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
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
    let id = extract_ulid(&stdout).expect("Should have tension ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
        .arg("show")
        .arg(&id[..8])
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "--no-color should disable ANSI codes in show --verbose output, got: {}",
        stdout
    );
}

// =============================================================================
// VAL-FMT-003: Colors auto-disabled for pipe/non-TTY
// =============================================================================

/// werk tree piped to cat produces no ANSI codes (auto-detect non-TTY)
#[test]
fn test_piped_output_tree() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // When stdout is piped (not a TTY), colors should be auto-disabled
    // assert_cmd pipes output by default, so this tests non-TTY behavior
    let output = Command::cargo_bin("werk")
        .unwrap()
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
        "Piped output should have no ANSI codes (non-TTY auto-detection), got: {}",
        stdout
    );
}

/// werk show piped to cat produces no ANSI codes (auto-detect non-TTY)
#[test]
fn test_piped_output_show() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
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
    let id = extract_ulid(&stdout).expect("Should have tension ID");

    // assert_cmd pipes output by default
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&id[..8])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !has_ansi_codes(&stdout),
        "Piped output should have no ANSI codes (non-TTY auto-detection), got: {}",
        stdout
    );
}

/// werk add piped to cat produces no ANSI codes (auto-detect non-TTY)
#[test]
fn test_piped_output_add() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // assert_cmd pipes output by default
    let output = Command::cargo_bin("werk")
        .unwrap()
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
        "Piped output should have no ANSI codes (non-TTY auto-detection), got: {}",
        stdout
    );
}

/// werk init piped to cat produces no ANSI codes (auto-detect non-TTY)
#[test]
fn test_piped_output_init() {
    let dir = TempDir::new().unwrap();

    // assert_cmd pipes output by default
    let output = Command::cargo_bin("werk")
        .unwrap()
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
        "Piped output should have no ANSI codes (non-TTY auto-detection), got: {}",
        stdout
    );
}

// =============================================================================
// Combined tests: Verify output is still readable without colors
// =============================================================================

/// Tree output is readable without colors (contains expected content)
#[test]
fn test_tree_readable_without_colors() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("write a novel")
        .arg("have an outline")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
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

    // Should show lifecycle badge
    assert!(
        stdout.contains("[G]"),
        "Output should show lifecycle badge [G], got: {}",
        stdout
    );

    // Should show movement signal
    let has_signal = stdout.contains('→') || stdout.contains('↔') || stdout.contains('○');
    assert!(
        has_signal,
        "Output should show movement signal, got: {}",
        stdout
    );
}

/// Show output is readable without colors (contains expected content)
#[test]
fn test_show_readable_without_colors() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("write a novel")
        .arg("have an outline")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have tension ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
        .arg("show")
        .arg(&id[..8])
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
        stdout.contains("Phase:"),
        "Output should contain 'Phase:', got: {}",
        stdout
    );
}

/// Error messages are readable without colors
#[test]
fn test_error_readable_without_colors() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--no-color")
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

/// JSON output has no ANSI codes (colors not applicable to JSON)
#[test]
fn test_json_no_ansi_codes() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
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
