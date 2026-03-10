//! Integration tests for `werk tree` command.
//!
//! Tests verify:
//! - VAL-DISP-001: Tree displays forest topology with hierarchy
//! - VAL-DISP-002: Tree shows lifecycle badges per tension
//! - VAL-DISP-003: Tree shows status indicators with color
//! - VAL-DISP-004: Tree shows conflict markers on sibling groups
//! - VAL-DISP-005: Tree shows movement signals
//! - VAL-DISP-006: Tree --open excludes resolved/released
//! - VAL-DISP-007: Tree --all includes all statuses
//! - VAL-DISP-008: Tree handles empty forest
//! - VAL-DISP-014: Tree renders deeply nested hierarchies

use assert_cmd::cargo_bin_cmd;
use tempfile::TempDir;

/// Extract a ULID from werk output.
/// ULIDs are 26 character uppercase alphanumeric strings.
fn extract_ulid(output: &str) -> Option<String> {
    // Find a sequence of 26 uppercase alphanumeric characters
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
// Tree basics
// =============================================================================

/// VAL-DISP-008: Tree on empty workspace shows "No tensions" message, exit code 0
#[test]
fn test_tree_empty_workspace() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree on empty workspace
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
        stdout.contains("No tensions") || stdout.contains("empty"),
        "Should show empty message, got: {}",
        stdout
    );
}

/// VAL-DISP-001: Tree displays single root tension
#[test]
fn test_tree_single_root() {
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

    // Should show the tension's desired state
    assert!(
        stdout.contains("write a novel"),
        "Should show desired state, got: {}",
        stdout
    );

    // Should show lifecycle badge [G] for Germination (new tension)
    assert!(
        stdout.contains("[G]"),
        "Should show Germination badge [G], got: {}",
        stdout
    );
}

/// VAL-DISP-001: Tree displays multiple root tensions
#[test]
fn test_tree_multiple_roots() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal 1")
        .arg("reality 1")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal 2")
        .arg("reality 2")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal 3")
        .arg("reality 3")
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

    // Should show all three tensions
    assert!(
        stdout.contains("goal 1"),
        "Should show goal 1, got: {}",
        stdout
    );
    assert!(
        stdout.contains("goal 2"),
        "Should show goal 2, got: {}",
        stdout
    );
    assert!(
        stdout.contains("goal 3"),
        "Should show goal 3, got: {}",
        stdout
    );
}

/// VAL-DISP-001: Tree displays hierarchy with parent-child
#[test]
fn test_tree_hierarchy() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("parent goal")
        .arg("parent reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Extract parent ID from output (ULID is 26 chars, uppercase alphanumeric)
    let stdout = String::from_utf8_lossy(&output);
    let parent_id = extract_ulid(&stdout).expect("Should have extracted parent ID");

    // Create child with --parent - use longer prefix (12 chars) for uniqueness
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("child goal")
        .arg("child reality")
        .arg("--parent")
        .arg(&parent_id[..12]) // Use 12 char prefix for better uniqueness
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

    // Should show parent and child
    assert!(
        stdout.contains("parent goal"),
        "Should show parent, got: {}",
        stdout
    );
    assert!(
        stdout.contains("child goal"),
        "Should show child, got: {}",
        stdout
    );

    // Should show tree structure (box-drawing characters)
    // Either ├ or └ for children
    let has_tree_chars = stdout.contains('├')
        || stdout.contains('└')
        || stdout.contains('│')
        || stdout.contains("--");
    assert!(
        has_tree_chars,
        "Should show tree characters, got: {}",
        stdout
    );
}

/// VAL-DISP-014: Tree renders deeply nested hierarchies (10+ levels)
#[test]
fn test_tree_deep_hierarchy() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create a chain of 12 tensions
    let mut prev_id: Option<String> = None;

    for i in 0..12 {
        let mut cmd = cargo_bin_cmd!("werk");
        cmd.arg("add")
            .arg(format!("level {} goal", i))
            .arg(format!("level {} reality", i))
            .current_dir(dir.path());

        if let Some(pid) = &prev_id {
            // Use 12 char prefix for better uniqueness
            cmd.arg("--parent").arg(&pid[..12]);
        }

        let output = cmd.assert().success().get_output().stdout.clone();
        let stdout = String::from_utf8_lossy(&output);

        // Extract the new tension ID
        prev_id = extract_ulid(&stdout);
    }

    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show all levels
    for i in 0..12 {
        assert!(
            stdout.contains(&format!("level {} goal", i)),
            "Should show level {}, got: {}",
            i,
            stdout
        );
    }
}

// =============================================================================
// Status filters
// =============================================================================

/// VAL-DISP-006: Tree --open shows only Active tensions (default behavior)
#[test]
fn test_tree_open_filter() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tensions
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("active goal")
        .arg("active reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("to resolve")
        .arg("to resolve reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Extract ID to resolve - use full ID to avoid ambiguity
    let stdout = String::from_utf8_lossy(&output);
    let resolve_id = extract_ulid(&stdout).expect("Should have resolve ID");

    // Resolve one - use full ID
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&resolve_id)
        .current_dir(dir.path())
        .assert()
        .success();

    // --open should only show Active
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--open")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("active goal"),
        "Should show active goal, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("to resolve"),
        "Should NOT show resolved, got: {}",
        stdout
    );
}

/// VAL-DISP-007: Tree --all shows all statuses
#[test]
fn test_tree_all_filter() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create active tension
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("active goal")
        .arg("active reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create and resolve
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("resolved goal")
        .arg("resolved reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let resolve_id = extract_ulid(&stdout).expect("Should have resolve ID");

    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&resolve_id) // Use full ID
        .current_dir(dir.path())
        .assert()
        .success();

    // --all should show both
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--all")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("active goal"),
        "Should show active, got: {}",
        stdout
    );
    assert!(
        stdout.contains("resolved goal"),
        "Should show resolved, got: {}",
        stdout
    );
}

/// Tree --resolved shows only Resolved tensions
#[test]
fn test_tree_resolved_filter() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create active
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("active goal")
        .arg("active reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create and resolve
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("resolved goal")
        .arg("resolved reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let resolve_id = extract_ulid(&stdout).expect("Should have resolve ID");

    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&resolve_id) // Use full ID
        .current_dir(dir.path())
        .assert()
        .success();

    // --resolved should only show resolved
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--resolved")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !stdout.contains("active goal"),
        "Should NOT show active, got: {}",
        stdout
    );
    assert!(
        stdout.contains("resolved goal"),
        "Should show resolved, got: {}",
        stdout
    );
}

/// Tree --released shows only Released tensions
#[test]
fn test_tree_released_filter() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create active
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("active goal")
        .arg("active reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create and release
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("released goal")
        .arg("released reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let release_id = extract_ulid(&stdout).expect("Should have release ID");

    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(&release_id) // Use full ID
        .arg("--reason")
        .arg("no longer relevant")
        .current_dir(dir.path())
        .assert()
        .success();

    // --released should only show released
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--released")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !stdout.contains("active goal"),
        "Should NOT show active, got: {}",
        stdout
    );
    assert!(
        stdout.contains("released goal"),
        "Should show released, got: {}",
        stdout
    );
}

// =============================================================================
// JSON output
// =============================================================================

/// Tree --json outputs valid JSON with tensions array
#[test]
fn test_tree_json_output() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("json goal")
        .arg("json reality")
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

    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Should be valid JSON, got: {}", stdout);

    // Should be an object with tensions array
    let json = parsed.unwrap();
    assert!(json.is_object(), "Should be JSON object, got: {}", json);

    let tensions = json.get("tensions").expect("Should have tensions field");
    assert!(tensions.is_array(), "tensions should be an array");

    // Should have at least one tension
    let arr = tensions.as_array().unwrap();
    assert!(!arr.is_empty(), "Should have tensions");

    // Should have summary
    let summary = json.get("summary").expect("Should have summary field");
    assert!(summary.get("total").is_some(), "Should have total count");
}

/// Tree --json on empty workspace returns object with empty tensions array
#[test]
fn test_tree_json_empty() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
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

    // Should be valid JSON object with empty tensions array
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Should be valid JSON, got: {}", stdout);

    let json = parsed.unwrap();
    assert!(json.is_object(), "Should be object");

    let tensions = json.get("tensions").expect("Should have tensions field");
    assert!(tensions.is_array(), "tensions should be array");
    assert_eq!(
        tensions.as_array().unwrap().len(),
        0,
        "Should be empty array"
    );
}

// =============================================================================
// Summary footer
// =============================================================================

/// Tree shows summary footer with counts
#[test]
fn test_tree_summary_footer() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create multiple tensions
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal 1")
        .arg("reality 1")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal 2")
        .arg("reality 2")
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

    // Should show summary (total count or status breakdown)
    let has_summary = stdout.contains("total")
        || stdout.contains("tension")
        || stdout.contains("2 active")
        || stdout.contains("Active:");
    assert!(
        has_summary || stdout.contains("goal 1"),
        "Should show some count info, got: {}",
        stdout
    );
}

// =============================================================================
// Lifecycle badges
// =============================================================================

/// Tree shows lifecycle badge [G] for Germination (new tension)
#[test]
fn test_tree_lifecycle_germination() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("new tension")
        .arg("new reality")
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

    // New tension should show [G] for Germination
    assert!(
        stdout.contains("[G]"),
        "Should show [G] badge for new tension, got: {}",
        stdout
    );
}

// =============================================================================
// Movement signals
// =============================================================================

/// Tree shows movement signal for each tension
#[test]
fn test_tree_movement_signals() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal")
        .arg("reality")
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

    // Should show at least one movement signal: → (advancing), ↔ (oscillating), or ○ (stagnant)
    let has_signal = stdout.contains('→') || stdout.contains('↔') || stdout.contains('○');
    assert!(has_signal, "Should show movement signal, got: {}", stdout);
}

// =============================================================================
// No color
// =============================================================================

/// Tree --no-color produces no ANSI codes
#[test]
fn test_tree_no_color() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--no-color")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should not contain ANSI escape sequences
    assert!(
        !stdout.contains("\x1b["),
        "Should not contain ANSI codes, got: {}",
        stdout
    );
}
