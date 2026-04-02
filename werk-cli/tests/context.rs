//! Integration tests for `werk context <id>` command.
//!
//! Tests verify:
//! - VAL-AGENT-001: Context outputs complete tension data as JSON
//! - VAL-AGENT-002: Context ancestors ordered root-first
//! - VAL-AGENT-003: Context siblings exclude self
//! - VAL-AGENT-004: Context handles root tension (no ancestors)
//! - VAL-AGENT-005: Context handles leaf tension (no children)
//! - VAL-AGENT-006: Context preserves unicode/special chars in JSON

use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

/// Extract a ULID from werk output.
#[allow(dead_code)]
/// Extract a tension identifier from werk output.
/// Tries short code (#N) first, then ULID (26 uppercase alphanumeric chars).
fn extract_ulid(output: &str) -> Option<String> {
    // Try short code pattern: #N where N is one or more digits
    if let Some(idx) = output.find('#') {
        let rest = &output[idx + 1..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Some(digits);
        }
    }
    // Fall back to ULID extraction
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
// VAL-AGENT-001: Context outputs complete tension data as JSON
// =============================================================================

/// Context outputs valid JSON with all required sections.
#[test]
fn test_context_outputs_valid_json() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should be valid JSON
    let json: Value = serde_json::from_str(&stdout).expect("Context output should be valid JSON");

    // Should have all required sections
    assert!(json.get("tension").is_some(), "Should have tension section");
    assert!(
        json.get("ancestors").is_some(),
        "Should have ancestors section"
    );
    assert!(
        json.get("siblings").is_some(),
        "Should have siblings section"
    );
    assert!(
        json.get("children").is_some(),
        "Should have children section"
    );
    assert!(
        json.get("mutations").is_some(),
        "Should have mutations section"
    );
}

/// Context tension section has all required fields.
#[test]
fn test_context_tension_has_all_fields() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("desired state", "actual state")
        .unwrap();
    let id = tension.id.clone();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let tension_obj = json.get("tension").unwrap();
    assert_eq!(tension_obj.get("id").unwrap().as_str().unwrap(), id);
    assert_eq!(
        tension_obj.get("desired").unwrap().as_str().unwrap(),
        "desired state"
    );
    assert_eq!(
        tension_obj.get("actual").unwrap().as_str().unwrap(),
        "actual state"
    );
    assert_eq!(
        tension_obj.get("status").unwrap().as_str().unwrap(),
        "Active"
    );
    assert!(tension_obj.get("created_at").is_some());
    assert!(tension_obj.get("parent_id").is_some());
}

/// Context no longer includes dynamics, but has projection.
#[test]
fn test_context_has_engagement() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should have engagement field (replaced projection)
    assert!(json.get("engagement").is_some(), "Should have engagement");
}

/// Context mutations are in chronological order.
#[test]
fn test_context_mutations_chronological() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Add several mutations
    store.update_actual(&tension.id, "reality v2").unwrap();
    store.update_actual(&tension.id, "reality v3").unwrap();
    store.update_desired(&tension.id, "refined goal").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let mutations = json.get("mutations").unwrap().as_array().unwrap();

    // Should have 4 mutations: creation + 3 updates
    assert_eq!(mutations.len(), 4);

    // Verify chronological order (oldest first)
    let timestamps: Vec<String> = mutations
        .iter()
        .filter_map(|m| m.get("timestamp").and_then(|t| t.as_str()))
        .map(|s| s.to_string())
        .collect();

    // Timestamps should be in ascending order (ISO 8601 strings compare correctly)
    let mut sorted_timestamps = timestamps.clone();
    sorted_timestamps.sort();
    assert_eq!(
        timestamps, sorted_timestamps,
        "Mutations should be in chronological order"
    );
}

// =============================================================================
// VAL-AGENT-002: Context ancestors ordered root-first
// =============================================================================

/// Ancestors are ordered root-first for deep chain.
#[test]
fn test_context_ancestors_root_first() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();

    // Create chain: A -> B -> C -> D
    let a = store.create_tension("A goal", "A reality").unwrap();
    let b = store
        .create_tension_with_parent("B goal", "B reality", Some(a.id.clone()))
        .unwrap();
    let c = store
        .create_tension_with_parent("C goal", "C reality", Some(b.id.clone()))
        .unwrap();
    let d = store
        .create_tension_with_parent("D goal", "D reality", Some(c.id.clone()))
        .unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&d.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let ancestors = json.get("ancestors").unwrap().as_array().unwrap();

    // Should have 3 ancestors: A, B, C (in that order, root-first)
    assert_eq!(ancestors.len(), 3);

    // Verify root-first order
    assert_eq!(
        ancestors[0].get("id").unwrap().as_str().unwrap(),
        a.id,
        "First ancestor should be root A"
    );
    assert_eq!(
        ancestors[1].get("id").unwrap().as_str().unwrap(),
        b.id,
        "Second ancestor should be B"
    );
    assert_eq!(
        ancestors[2].get("id").unwrap().as_str().unwrap(),
        c.id,
        "Third ancestor should be C"
    );
}

// =============================================================================
// VAL-AGENT-003: Context siblings exclude self
// =============================================================================

/// Siblings exclude self when listing.
#[test]
fn test_context_siblings_exclude_self() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();

    // Create parent with 3 children
    let parent = store
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let c1 = store
        .create_tension_with_parent("C1 goal", "C1 reality", Some(parent.id.clone()))
        .unwrap();
    let c2 = store
        .create_tension_with_parent("C2 goal", "C2 reality", Some(parent.id.clone()))
        .unwrap();
    let c3 = store
        .create_tension_with_parent("C3 goal", "C3 reality", Some(parent.id.clone()))
        .unwrap();

    // Context for C2 should have siblings [C1, C3] (not including C2)
    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&c2.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let siblings = json.get("siblings").unwrap().as_array().unwrap();

    // Should have 2 siblings
    assert_eq!(siblings.len(), 2);

    // Verify C2 is not in siblings
    let sibling_ids: Vec<&str> = siblings
        .iter()
        .filter_map(|s| s.get("id").and_then(|id| id.as_str()))
        .collect();

    assert!(
        !sibling_ids.contains(&c2.id.as_str()),
        "C2 should not be in its own siblings"
    );
    assert!(
        sibling_ids.contains(&c1.id.as_str()),
        "C1 should be in C2's siblings"
    );
    assert!(
        sibling_ids.contains(&c3.id.as_str()),
        "C3 should be in C2's siblings"
    );
}

// =============================================================================
// VAL-AGENT-004: Context handles root tension (no ancestors)
// =============================================================================

/// Root tension has empty ancestors array.
#[test]
fn test_context_root_empty_ancestors() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let root = store.create_tension("root goal", "root reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&root.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let ancestors = json.get("ancestors").unwrap().as_array().unwrap();

    // Root should have empty ancestors
    assert!(
        ancestors.is_empty(),
        "Root tension should have empty ancestors array"
    );
}

/// Root tension's siblings are other roots.
#[test]
fn test_context_root_siblings_are_other_roots() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();

    // Create 3 root tensions
    let r1 = store.create_tension("R1 goal", "R1 reality").unwrap();
    let r2 = store.create_tension("R2 goal", "R2 reality").unwrap();
    let r3 = store.create_tension("R3 goal", "R3 reality").unwrap();

    // Context for R2 should have R1 and R3 as siblings
    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&r2.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let siblings = json.get("siblings").unwrap().as_array().unwrap();

    assert_eq!(siblings.len(), 2);

    let sibling_ids: Vec<&str> = siblings
        .iter()
        .filter_map(|s| s.get("id").and_then(|id| id.as_str()))
        .collect();

    assert!(
        sibling_ids.contains(&r1.id.as_str()),
        "R1 should be in R2's siblings"
    );
    assert!(
        sibling_ids.contains(&r3.id.as_str()),
        "R3 should be in R2's siblings"
    );
}

// =============================================================================
// VAL-AGENT-005: Context handles leaf tension (no children)
// =============================================================================

/// Leaf tension has empty children array.
#[test]
fn test_context_leaf_empty_children() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();

    // Create chain with leaf
    let parent = store
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let leaf = store
        .create_tension_with_parent("leaf goal", "leaf reality", Some(parent.id.clone()))
        .unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&leaf.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let children = json.get("children").unwrap().as_array().unwrap();

    // Leaf should have empty children
    assert!(
        children.is_empty(),
        "Leaf tension should have empty children array"
    );
}

// =============================================================================
// VAL-AGENT-006: Context preserves unicode/special chars in JSON
// =============================================================================

/// Unicode characters are properly escaped in JSON output.
#[test]
fn test_context_preserves_unicode() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("写小说 🎵", "有大纲").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let tension_obj = json.get("tension").unwrap();

    // Unicode should be preserved exactly
    assert_eq!(
        tension_obj.get("desired").unwrap().as_str().unwrap(),
        "写小说 🎵"
    );
    assert_eq!(
        tension_obj.get("actual").unwrap().as_str().unwrap(),
        "有大纲"
    );
}

/// Special characters (quotes, newlines) are properly escaped.
#[test]
fn test_context_escapes_special_chars() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Note: We test that JSON parsing succeeds with special chars
    // The store handles the actual content
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("goal with \"quotes\"", "reality with\nnewline")
        .unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should be valid JSON (no parsing errors)
    let json: Value = serde_json::from_str(&stdout).expect("Should parse JSON with special chars");

    // Verify content is preserved
    let tension_obj = json.get("tension").unwrap();
    assert!(tension_obj
        .get("desired")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("quotes"));
    assert!(tension_obj
        .get("actual")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("newline"));
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

/// Context with nonexistent ID returns error.
#[test]
fn test_context_nonexistent_id_error() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg("NONEXISTENT123456789ABC")
        .current_dir(dir.path())
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    assert!(stderr.contains("not found") || stderr.contains("NOT_FOUND"));
}

/// Context with ambiguous prefix returns error.
#[test]
fn test_context_ambiguous_prefix_error() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();

    // Create two tensions with same starting characters (we'll use prefix matching)
    let _t1 = store.create_tension("goal1", "reality1").unwrap();
    let _t2 = store.create_tension("goal2", "reality2").unwrap();

    // Try to use a short prefix that could match multiple
    // Since ULIDs are unique, we need to create tensions and check if
    // their first characters overlap. In practice, this is unlikely,
    // so we test with a too-short prefix instead.
    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg("AB") // Too short (less than 4 chars)
        .current_dir(dir.path())
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    assert!(stderr.contains("too short") || stderr.contains("prefix"));
}

/// Context with valid prefix resolves correctly.
#[test]
fn test_context_prefix_resolution() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Use first 8 characters as prefix
    let prefix = &tension.id[..8];

    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should resolve to correct tension
    assert_eq!(
        json.get("tension")
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        tension.id
    );
}

/// Context outputs JSON even without --json flag (always JSON).
#[test]
fn test_context_always_json() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Call without --json flag
    let output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should still be valid JSON
    let _: Value = serde_json::from_str(&stdout).expect("Context should always output JSON");
}
