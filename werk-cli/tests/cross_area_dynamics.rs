//! Cross-area integration tests.
//!
//! Verifies that honest facts (urgency, temporal signals, closure) flow
//! correctly through CLI JSON output after the dynamics removal.

use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

/// Extract a ULID from werk output (26 uppercase alphanumeric chars).
fn extract_ulid(output: &str) -> Option<String> {
    for word in output.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
        if clean.len() == 26 && clean.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            return Some(clean.to_string());
        }
    }
    None
}

#[test]
fn test_show_json_has_temporal_signals() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create via Store directly for reliable ID
    let store = sd_core::Store::init(dir.path()).unwrap();
    let h = sd_core::Horizon::new_month(2027, 6).unwrap();
    let tension = store.create_tension_full("test goal", "test reality", None, Some(h)).unwrap();

    // Show JSON
    let output = cargo_bin_cmd!("werk")
        .args(["show", "--json", &tension.id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "show failed: {}", String::from_utf8_lossy(&output.stderr));

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    // Should have urgency (has horizon)
    assert!(json["urgency"].is_number(), "should have urgency with horizon");

    // Should have temporal signals
    assert!(json["temporal"].is_object(), "should have temporal signals");

    // Should have frontier with closure progress
    assert!(json["frontier"].is_object(), "should have frontier");
    assert!(json["frontier"]["closure_progress"].is_object(), "should have closure_progress");

    // Should NOT have dynamics field
    assert!(json.get("dynamics").is_none(), "dynamics field should not exist");
}

#[test]
fn test_show_json_no_urgency_without_horizon() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("no horizon goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .args(["show", "--json", &tension.id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "show failed: {}", String::from_utf8_lossy(&output.stderr));

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    // Should have null urgency (no horizon)
    assert!(json["urgency"].is_null(), "urgency should be null without horizon");
}
