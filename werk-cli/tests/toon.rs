//! Integration tests for TOON output format across all commands.
//!
//! Tests verify:
//! - VAL-TOON-001: --toon flag produces valid TOON output
//! - VAL-TOON-002: TOON output roundtrips to equivalent data as JSON
//! - VAL-TOON-003: TOON works for all output commands
//! - VAL-TOON-004: --json flag still works (existing behavior preserved)
//! - VAL-TOON-005: toon-format crate dependency present
//! - VAL-TOON-006: TOON handles None/missing optional fields gracefully

use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

/// Helper: initialize a workspace in a temp directory.
fn init_workspace(dir: &TempDir) {
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
}

/// Helper: add a tension and return its ID (via --json for reliable parsing).
fn add_tension(dir: &TempDir, desired: &str, actual: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg(desired)
        .arg(actual)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value =
        serde_json::from_str(&String::from_utf8_lossy(&output)).expect("Should parse JSON");
    json["id"].as_str().unwrap().to_string()
}

/// Helper: add a tension with a horizon and return its ID.
fn add_tension_with_horizon(dir: &TempDir, desired: &str, actual: &str, horizon: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg(desired)
        .arg(actual)
        .arg("--horizon")
        .arg(horizon)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value =
        serde_json::from_str(&String::from_utf8_lossy(&output)).expect("Should parse JSON");
    json["id"].as_str().unwrap().to_string()
}

/// Helper: add a child tension and return its ID.
fn add_child_tension(dir: &TempDir, desired: &str, actual: &str, parent: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg(desired)
        .arg(actual)
        .arg("--parent")
        .arg(parent)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value =
        serde_json::from_str(&String::from_utf8_lossy(&output)).expect("Should parse JSON");
    json["id"].as_str().unwrap().to_string()
}

/// Helper: get the TOON output for a command as a decoded serde_json::Value.
fn run_toon(dir: &TempDir, args: &[&str]) -> (String, Value) {
    let mut cmd = cargo_bin_cmd!("werk");
    cmd.arg("--toon");
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output).to_string();
    let decoded: Value =
        toon_format::decode_default(&stdout).expect("TOON output should be valid and parseable");
    (stdout, decoded)
}

/// Helper: get the JSON output for a command as a serde_json::Value.
fn run_json(dir: &TempDir, args: &[&str]) -> Value {
    let mut cmd = cargo_bin_cmd!("werk");
    cmd.arg("--json");
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    serde_json::from_str(&stdout).expect("JSON output should be valid")
}

/// Compare the keys of two JSON Values recursively.
/// Ensures the TOON-decoded value has the same field structure as JSON.
fn assert_same_keys(json_val: &Value, toon_val: &Value, path: &str) {
    match (json_val, toon_val) {
        (Value::Object(jmap), Value::Object(tmap)) => {
            for key in jmap.keys() {
                assert!(
                    tmap.contains_key(key),
                    "TOON output missing key '{}' at path '{}'. JSON keys: {:?}, TOON keys: {:?}",
                    key,
                    path,
                    jmap.keys().collect::<Vec<_>>(),
                    tmap.keys().collect::<Vec<_>>()
                );
                assert_same_keys(&jmap[key], &tmap[key], &format!("{}.{}", path, key));
            }
        }
        (Value::Array(jarr), Value::Array(tarr)) => {
            // Just check that if JSON has items, TOON has the same number
            assert_eq!(
                jarr.len(),
                tarr.len(),
                "Array length mismatch at path '{}'",
                path
            );
            for (i, (jitem, titem)) in jarr.iter().zip(tarr.iter()).enumerate() {
                assert_same_keys(jitem, titem, &format!("{}[{}]", path, i));
            }
        }
        _ => {
            // Leaf values - types may differ slightly (TOON may decode numbers differently),
            // but both should be present
        }
    }
}

// =============================================================================
// VAL-TOON-001: --toon flag produces valid TOON output
// =============================================================================

#[test]
fn test_toon_show_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "write a novel", "have an outline");

    let (raw, decoded) = run_toon(&dir, &["show", &id]);
    assert!(!raw.is_empty(), "TOON output should not be empty");
    assert!(decoded.is_object(), "Decoded TOON should be an object");
    assert!(
        decoded.get("id").is_some(),
        "Should have id field in decoded TOON"
    );
}

#[test]
fn test_toon_tree_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    add_tension(&dir, "goal one", "reality one");

    let (raw, decoded) = run_toon(&dir, &["tree"]);
    assert!(!raw.is_empty(), "TOON output should not be empty");
    assert!(decoded.is_object(), "Decoded TOON should be an object");
    assert!(
        decoded.get("tensions").is_some(),
        "Should have tensions field"
    );
    assert!(
        decoded.get("summary").is_some(),
        "Should have summary field"
    );
}

#[test]
fn test_toon_add_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    let mut cmd = cargo_bin_cmd!("werk");
    let output = cmd
        .arg("--toon")
        .arg("add")
        .arg("desired state")
        .arg("actual state")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let decoded: Value =
        toon_format::decode_default(&stdout).expect("Add TOON output should be valid");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(
        decoded.get("desired").is_some(),
        "Should have desired field"
    );
    assert!(decoded.get("actual").is_some(), "Should have actual field");
    assert!(decoded.get("status").is_some(), "Should have status field");
}

// =============================================================================
// VAL-TOON-002: TOON output roundtrips to equivalent data as JSON
// =============================================================================

#[test]
fn test_toon_show_roundtrips_to_json() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "build a house", "have blueprints");

    let json_val = run_json(&dir, &["show", &id]);
    let (_, toon_val) = run_toon(&dir, &["show", &id]);

    // Both should have the same top-level keys
    assert_same_keys(&json_val, &toon_val, "root");

    // Verify specific fields match
    assert_eq!(
        json_val["id"].as_str(),
        toon_val["id"].as_str(),
        "IDs should match"
    );
    assert_eq!(
        json_val["desired"].as_str(),
        toon_val["desired"].as_str(),
        "Desired should match"
    );
    assert_eq!(
        json_val["status"].as_str(),
        toon_val["status"].as_str(),
        "Status should match"
    );
}

#[test]
fn test_toon_tree_roundtrips_to_json() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    add_tension(&dir, "goal alpha", "reality alpha");
    add_tension(&dir, "goal beta", "reality beta");

    let json_val = run_json(&dir, &["tree"]);
    let (_, toon_val) = run_toon(&dir, &["tree"]);

    assert_same_keys(&json_val, &toon_val, "root");

    // Verify summary fields match
    assert_eq!(
        json_val["summary"]["total"], toon_val["summary"]["total"],
        "Total count should match"
    );
    assert_eq!(
        json_val["summary"]["active"], toon_val["summary"]["active"],
        "Active count should match"
    );
}

#[test]
fn test_toon_add_roundtrips_to_json() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    // Run add with --json
    let json_output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired roundtrip")
        .arg("actual roundtrip")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_val: Value = serde_json::from_str(&String::from_utf8_lossy(&json_output)).unwrap();

    // Run add with --toon (creates a second tension, but same schema)
    let toon_output = cargo_bin_cmd!("werk")
        .arg("--toon")
        .arg("add")
        .arg("desired roundtrip 2")
        .arg("actual roundtrip 2")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let toon_val: Value =
        toon_format::decode_default(&String::from_utf8_lossy(&toon_output)).unwrap();

    // Both should have the same keys (different values since different tensions)
    assert_same_keys(&json_val, &toon_val, "root");
}

// =============================================================================
// VAL-TOON-003: TOON works for all output commands
// =============================================================================

#[test]
fn test_toon_context_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "context goal", "context reality");

    let (_, decoded) = run_toon(&dir, &["context", &id]);
    assert!(decoded.is_object(), "Context TOON should be an object");
    assert!(
        decoded.get("tension").is_some(),
        "Should have tension field"
    );
    assert!(
        decoded.get("dynamics").is_some(),
        "Should have dynamics field"
    );
    assert!(
        decoded.get("mutations").is_some(),
        "Should have mutations field"
    );
}

#[test]
fn test_toon_horizon_display_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension_with_horizon(&dir, "horizon goal", "horizon reality", "2027-06");

    let (_, decoded) = run_toon(&dir, &["horizon", &id]);
    assert!(decoded.is_object(), "Horizon TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(
        decoded.get("horizon").is_some(),
        "Should have horizon field"
    );
}

#[test]
fn test_toon_horizon_set_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "set horizon goal", "set horizon reality");

    let (_, decoded) = run_toon(&dir, &["horizon", &id, "2027-12"]);
    assert!(decoded.is_object(), "Horizon set TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(
        decoded.get("horizon").is_some(),
        "Should have horizon field"
    );
}

#[test]
fn test_toon_reality_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "reality goal", "initial reality");

    let (_, decoded) = run_toon(&dir, &["reality", &id, "updated reality"]);
    assert!(decoded.is_object(), "Reality TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(decoded.get("actual").is_some(), "Should have actual field");
    assert!(
        decoded.get("old_actual").is_some(),
        "Should have old_actual field"
    );
}

#[test]
fn test_toon_desire_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "initial desire", "desire reality");

    let (_, decoded) = run_toon(&dir, &["desire", &id, "updated desire"]);
    assert!(decoded.is_object(), "Desire TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(
        decoded.get("desired").is_some(),
        "Should have desired field"
    );
    assert!(
        decoded.get("old_desired").is_some(),
        "Should have old_desired field"
    );
}

#[test]
fn test_toon_resolve_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "resolve goal", "resolve reality");

    let (_, decoded) = run_toon(&dir, &["resolve", &id]);
    assert!(decoded.is_object(), "Resolve TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(decoded.get("status").is_some(), "Should have status field");
    assert_eq!(
        decoded["status"].as_str(),
        Some("Resolved"),
        "Status should be Resolved"
    );
}

#[test]
fn test_toon_release_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "release goal", "release reality");

    let (_, decoded) = run_toon(&dir, &["release", &id, "--reason", "no longer needed"]);
    assert!(decoded.is_object(), "Release TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(decoded.get("status").is_some(), "Should have status field");
    assert!(decoded.get("reason").is_some(), "Should have reason field");
    assert_eq!(
        decoded["status"].as_str(),
        Some("Released"),
        "Status should be Released"
    );
}

#[test]
fn test_toon_rm_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "delete me", "delete reality");

    let (_, decoded) = run_toon(&dir, &["rm", &id]);
    assert!(decoded.is_object(), "Rm TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(
        decoded.get("deleted").is_some(),
        "Should have deleted field"
    );
}

#[test]
fn test_toon_move_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let parent_id = add_tension(&dir, "parent goal", "parent reality");
    let child_id = add_tension(&dir, "child goal", "child reality");

    let (_, decoded) = run_toon(&dir, &["move", &child_id, "--parent", &parent_id]);
    assert!(decoded.is_object(), "Move TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(
        decoded.get("parent_id").is_some(),
        "Should have parent_id field"
    );
}

#[test]
fn test_toon_note_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "note goal", "note reality");

    let (_, decoded) = run_toon(&dir, &["note", &id, "test annotation"]);
    assert!(decoded.is_object(), "Note TOON should be an object");
    assert!(decoded.get("id").is_some(), "Should have id field");
    assert!(decoded.get("note").is_some(), "Should have note field");
}

#[test]
fn test_toon_notes_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    // Add a workspace note first
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("workspace note for testing")
        .current_dir(dir.path())
        .assert()
        .success();

    let (_, decoded) = run_toon(&dir, &["notes"]);
    assert!(decoded.is_object(), "Notes TOON should be an object");
    assert!(decoded.get("notes").is_some(), "Should have notes field");
}

#[test]
fn test_toon_nuke_dry_run_produces_valid_output() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    // Nuke without --confirm is a dry run
    let (_, decoded) = run_toon(&dir, &["nuke"]);
    assert!(decoded.is_object(), "Nuke TOON should be an object");
    assert!(decoded.get("path").is_some(), "Should have path field");
    assert!(
        decoded.get("deleted").is_some(),
        "Should have deleted field"
    );
}

// =============================================================================
// VAL-TOON-004: --json flag still works (existing behavior preserved)
// =============================================================================

#[test]
fn test_json_still_works_with_toon_present() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "json test", "json reality");

    // --json should still produce valid JSON
    let json_val = run_json(&dir, &["show", &id]);
    assert!(json_val.is_object(), "JSON output should still work");
    assert!(json_val.get("id").is_some(), "Should have id field");
    assert!(
        json_val.get("dynamics").is_some(),
        "Should have dynamics field"
    );
}

#[test]
fn test_toon_and_json_mutually_exclusive() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    // --json and --toon together should fail (clap conflict)
    cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("--toon")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .failure();
}

// =============================================================================
// VAL-TOON-006: TOON handles None/missing optional fields gracefully
// =============================================================================

#[test]
fn test_toon_minimal_tension_no_horizon_no_mutations() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "minimal desired", "minimal actual");

    // Show a tension with no horizon, no mutations beyond creation
    let (raw, decoded) = run_toon(&dir, &["show", &id]);
    assert!(
        !raw.is_empty(),
        "TOON output should not be empty for minimal tension"
    );
    assert!(
        decoded.is_object(),
        "Decoded TOON should be an object for minimal tension"
    );

    // Optional fields should be present as null (not cause errors)
    // horizon, urgency, pressure are all None for no-horizon tensions
    assert!(
        decoded.get("id").is_some(),
        "Should have id even on minimal tension"
    );
    assert!(
        decoded.get("dynamics").is_some(),
        "Should have dynamics even on minimal tension"
    );
}

#[test]
fn test_toon_empty_tree() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    let (raw, decoded) = run_toon(&dir, &["tree"]);
    assert!(
        !raw.is_empty(),
        "Empty tree TOON should still produce output"
    );
    assert!(decoded.is_object(), "Empty tree TOON should be an object");

    let tensions = decoded.get("tensions").expect("Should have tensions field");
    assert!(
        tensions.is_array(),
        "tensions should be an array even when empty"
    );
    assert_eq!(
        tensions.as_array().unwrap().len(),
        0,
        "tensions should be empty array for empty workspace"
    );
}

#[test]
fn test_toon_empty_notes() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    let (_, decoded) = run_toon(&dir, &["notes"]);
    assert!(decoded.is_object(), "Empty notes TOON should be an object");
    let notes = decoded.get("notes").expect("Should have notes field");
    assert!(notes.is_array(), "notes should be an array");
    assert_eq!(
        notes.as_array().unwrap().len(),
        0,
        "notes should be empty for new workspace"
    );
}

#[test]
fn test_toon_tension_with_horizon_has_urgency() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension_with_horizon(&dir, "horizon test", "horizon reality", "2027-12");

    let (_, decoded) = run_toon(&dir, &["show", &id]);
    assert!(decoded.is_object());

    // With a horizon, urgency should be present (non-null)
    assert!(
        decoded.get("urgency").is_some(),
        "Should have urgency field for tension with horizon"
    );
    // urgency should be a number (not null)
    assert!(
        decoded["urgency"].is_number(),
        "urgency should be a number for tension with horizon, got: {:?}",
        decoded["urgency"]
    );
}

// =============================================================================
// VAL-TOON-002 (additional): Roundtrip tests for more commands
// =============================================================================

#[test]
fn test_toon_context_roundtrips_to_json() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "context roundtrip", "context reality");

    let json_val = run_json(&dir, &["context", &id]);
    let (_, toon_val) = run_toon(&dir, &["context", &id]);

    assert_same_keys(&json_val, &toon_val, "root");
}

#[test]
fn test_toon_dynamics_fields_present() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension(&dir, "dynamics check", "dynamics reality");

    let (_, decoded) = run_toon(&dir, &["show", &id]);
    let dynamics = decoded.get("dynamics").expect("Should have dynamics");

    // All 10 dynamics + horizon_drift should be present
    let expected_fields = [
        "structural_tension",
        "structural_conflict",
        "oscillation",
        "resolution",
        "phase",
        "orientation",
        "compensating_strategy",
        "structural_tendency",
        "assimilation_depth",
        "neglect",
        "horizon_drift",
    ];

    for field in &expected_fields {
        assert!(
            dynamics.get(*field).is_some(),
            "TOON dynamics should contain field '{}'",
            field
        );
    }
}

#[test]
fn test_toon_show_with_children() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let parent = add_tension(&dir, "parent desired", "parent actual");
    let _child = add_child_tension(&dir, "child desired", "child actual", &parent);

    let (_, decoded) = run_toon(&dir, &["show", &parent]);
    assert!(decoded.get("children").is_some(), "Should have children");

    let children = decoded["children"]
        .as_array()
        .expect("children should be array");
    assert_eq!(children.len(), 1, "Should have 1 child");
}

// =============================================================================
// VAL-TOON-003: TOON error responses
// =============================================================================

#[test]
fn test_toon_error_not_found() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);

    let output = cargo_bin_cmd!("werk")
        .arg("--toon")
        .arg("show")
        .arg("NONEXISTENTID12345678")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Error output should be valid TOON
    let decoded: Value = toon_format::decode_default(&stdout).expect("Error TOON should be valid");

    assert!(
        decoded.get("error").is_some(),
        "Should have 'error' object in TOON error"
    );

    let error = decoded.get("error").unwrap();
    assert!(error.get("code").is_some(), "Error should have code");
    assert!(error.get("message").is_some(), "Error should have message");
}

// =============================================================================
// VAL-TOON-002: Roundtrip with complex data (horizon, mutations, etc.)
// =============================================================================

#[test]
fn test_toon_show_with_mutations_roundtrips() {
    let dir = TempDir::new().unwrap();
    init_workspace(&dir);
    let id = add_tension_with_horizon(&dir, "evolving goal", "starting reality", "2027-06");

    // Add some mutations
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&id)
        .arg("updated reality v1")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&id)
        .arg("updated reality v2")
        .current_dir(dir.path())
        .assert()
        .success();

    // Now compare JSON and TOON
    let json_val = run_json(&dir, &["show", &id]);
    let (_, toon_val) = run_toon(&dir, &["show", &id]);

    assert_same_keys(&json_val, &toon_val, "root");

    // Verify mutations are present in both
    let json_mutations = json_val["mutations"].as_array().expect("JSON mutations");
    let toon_mutations = toon_val["mutations"].as_array().expect("TOON mutations");
    assert_eq!(
        json_mutations.len(),
        toon_mutations.len(),
        "Should have same number of mutations"
    );
}
