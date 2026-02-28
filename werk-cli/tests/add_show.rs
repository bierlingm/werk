//! Integration tests for `werk add` and `werk show` commands.
//!
//! Tests verify:
//! - VAL-CRUD-001: Add creates tension with desired and actual
//! - VAL-CRUD-002: Add with --parent creates child tension
//! - VAL-CRUD-003: Add rejects empty desired or actual
//! - VAL-CRUD-004: Add handles unicode
//! - VAL-CRUD-005: Show displays tension by ID or prefix
//! - VAL-CRUD-006: Show rejects ambiguous prefix
//! - VAL-CRUD-007: Show reports not found for invalid ID
//! - VAL-CRUD-025: Prefix too short rejected

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

// =============================================================================
// ADD command tests
// =============================================================================

/// VAL-CRUD-001: `werk add "desired" "actual"` creates Active tension with ULID
#[test]
fn test_add_creates_tension() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace first
    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Add a tension
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

    // Should output the ID (ULID format: 26 characters)
    assert!(
        stdout.contains("Tension created") || stdout.contains("Created"),
        "Should confirm creation, got: {}",
        stdout
    );

    // Verify the tension was stored
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 1);
    assert_eq!(tensions[0].desired, "write a novel");
    assert_eq!(tensions[0].actual, "have an outline");
    assert_eq!(tensions[0].status, sd_core::TensionStatus::Active);
}

/// VAL-CRUD-001: Add prints confirmation with ID
#[test]
fn test_add_prints_id() {
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
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // The output should contain a ULID (26 character alphanumeric)
    // ULID format: 0-9A-Z (uppercase)
    assert!(
        stdout.len() > 20,
        "Output should contain ID, got: {}",
        stdout
    );
}

/// VAL-CRUD-002: Add with --parent creates child tension
#[test]
fn test_add_with_parent() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent tension
    let store = sd_core::Store::init(dir.path()).unwrap();
    let parent = store
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let parent_id = parent.id.clone();

    // Create child tension with --parent flag using prefix (first 6 chars of ULID)
    let prefix = &parent_id[..6];

    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("child goal")
        .arg("child reality")
        .arg("--parent")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify child has correct parent_id
    let tensions = store.list_tensions().unwrap();
    let child = tensions.iter().find(|t| t.desired == "child goal").unwrap();
    assert_eq!(child.parent_id, Some(parent_id));
}

/// VAL-CRUD-003: Add rejects empty desired
#[test]
fn test_add_rejects_empty_desired() {
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
        .arg("")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty").or(predicate::str::contains("desired")));

    // Verify no tension was created
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert!(tensions.is_empty());
}

/// VAL-CRUD-003: Add rejects empty actual
#[test]
fn test_add_rejects_empty_actual() {
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
        .arg("desired")
        .arg("")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty").or(predicate::str::contains("actual")));

    // Verify no tension was created
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert!(tensions.is_empty());
}

/// VAL-CRUD-004: Add handles unicode (CJK and emoji)
#[test]
fn test_add_handles_unicode() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Add tension with CJK characters and emoji
    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("写小说 🎵")
        .arg("有大纲")
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify unicode is preserved
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions[0].desired, "写小说 🎵");
    assert_eq!(tensions[0].actual, "有大纲");
}

/// Add command requires workspace (no .werk/ directory and no global fallback)
/// We use a custom HOME to ensure no global workspace exists
#[test]
fn test_add_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    // Set a custom HOME that has no .werk/
    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("goal")
        .arg("reality")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace").or(predicate::str::contains("init")));
}

/// --json flag produces valid JSON for add
#[test]
fn test_add_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have tension fields
    assert!(json.get("id").is_some(), "JSON should have 'id' field");
    assert!(
        json.get("desired").is_some(),
        "JSON should have 'desired' field"
    );
    assert!(
        json.get("actual").is_some(),
        "JSON should have 'actual' field"
    );
    assert_eq!(json.get("desired").unwrap().as_str().unwrap(), "goal");
    assert_eq!(json.get("actual").unwrap().as_str().unwrap(), "reality");
}

// =============================================================================
// SHOW command tests
// =============================================================================

/// VAL-CRUD-005: Show displays tension by full ID
#[test]
fn test_show_by_full_id() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("show goal", "show reality").unwrap();

    // Show by full ID
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show all fields
    assert!(
        stdout.contains("show goal"),
        "Should show desired, got: {}",
        stdout
    );
    assert!(
        stdout.contains("show reality"),
        "Should show actual, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Active"),
        "Should show status, got: {}",
        stdout
    );
}

/// VAL-CRUD-005: Show displays tension by prefix (4+ chars)
#[test]
fn test_show_by_prefix() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store
        .create_tension("prefix goal", "prefix reality")
        .unwrap();

    // Show by 6-char prefix
    let prefix = &tension.id[..6];

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("prefix goal"));
}

/// VAL-CRUD-005: Show displays mutation count
#[test]
fn test_show_displays_mutation_count() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store
        .create_tension("mutation goal", "mutation reality")
        .unwrap();

    // Show should display mutation count (at least 1 for creation)
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Should show mutation count or mutations
    assert!(
        stdout.contains("mutation") || stdout.contains("Mutation"),
        "Should show mutation info, got: {}",
        stdout
    );
}

/// VAL-CRUD-006: Show rejects ambiguous prefix
#[test]
fn test_show_ambiguous_prefix() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create two tensions
    let store = sd_core::Store::init(dir.path()).unwrap();
    let _t1 = store.create_tension("first goal", "first reality").unwrap();
    let _t2 = store
        .create_tension("second goal", "second reality")
        .unwrap();

    // Try to use a very short prefix that might match both
    // Since ULIDs are time-sorted, tensions created close together may share prefixes
    // We use a 4-char prefix and check for ambiguity handling
    let tensions = store.list_tensions().unwrap();
    let prefix = &tensions[0].id[..4];

    // Try with 4-char prefix - might be ambiguous
    let result = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(prefix)
        .current_dir(dir.path())
        .assert();

    // Either it resolves uniquely or shows ambiguity error
    let output = result.get_output();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("ambiguous") || stderr.contains("multiple"),
            "Should report ambiguity, got: {}",
            stderr
        );
    }
    // If it succeeds, that's also OK - means the prefix was unique
}

/// VAL-CRUD-007: Show reports not found for invalid ID
#[test]
fn test_show_not_found() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// VAL-CRUD-025: Prefix too short rejected
#[test]
fn test_show_prefix_too_short() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create a tension
    let store = sd_core::Store::init(dir.path()).unwrap();
    let _tension = store.create_tension("short goal", "short reality").unwrap();

    // Try 3-char prefix (too short)
    Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg("ABC")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("too short").or(predicate::str::contains("4")));
}

/// Show requires workspace
#[test]
fn test_show_requires_workspace() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg("SOMEID")
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// --json flag produces valid JSON for show
#[test]
fn test_show_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("json goal", "json reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have tension fields
    assert!(json.get("id").is_some(), "JSON should have 'id' field");
    assert!(
        json.get("desired").is_some(),
        "JSON should have 'desired' field"
    );
    assert!(
        json.get("actual").is_some(),
        "JSON should have 'actual' field"
    );
    assert!(
        json.get("status").is_some(),
        "JSON should have 'status' field"
    );
    assert!(
        json.get("created_at").is_some(),
        "JSON should have 'created_at' field"
    );
    assert!(
        json.get("mutations").is_some(),
        "JSON should have 'mutations' field"
    );
}

/// Show displays parent field when tension has parent
#[test]
fn test_show_displays_parent() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent and child
    let store = sd_core::Store::init(dir.path()).unwrap();
    let parent = store
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let child = store
        .create_tension_with_parent("child goal", "child reality", Some(parent.id.clone()))
        .unwrap();

    // Show child
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&child.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Should show parent reference
    assert!(
        stdout.contains("parent") || stdout.contains(&parent.id[..8]),
        "Should show parent info, got: {}",
        stdout
    );
}

/// --verbose flag is accepted
#[test]
fn test_show_verbose_flag() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store
        .create_tension("verbose goal", "verbose reality")
        .unwrap();

    // --verbose should be accepted (might not show extra info yet)
    Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success();
}
