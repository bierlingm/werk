//! Cross-area dynamics integration tests.
//!
//! These tests verify that all cross-area flows work correctly after the
//! dynamics overhaul (gap magnitude, new events, TOON output, CLI refactor).
//!
//! Covers:
//! - VAL-CROSS-001: Gap magnitude flows through all dynamics correctly
//! - VAL-CROSS-002: New events visible through DynamicsEngine usage in CLI
//! - VAL-CROSS-003: TOON output includes all new dynamics fields
//! - VAL-CROSS-004: Full workspace tests pass (covered by cargo test --workspace)
//! - VAL-CROSS-005: Existing databases backward compatible
//! - VAL-CROSS-006: show --verbose displays compensating strategy, urgency threshold, horizon drift
//! - VAL-CROSS-007: Canonical scenario with recalibrated defaults produces correct classifications

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

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

/// Helper: initialize a workspace and return the temp dir.
fn init_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

/// Helper: add a tension and return its ID.
fn add_tension(dir: &TempDir, desired: &str, actual: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg(desired)
        .arg(actual)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    extract_ulid(&stdout).expect("Should get tension ID from add output")
}

/// Helper: add a tension with horizon and return its ID.
fn add_tension_with_horizon(dir: &TempDir, desired: &str, actual: &str, horizon: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("--horizon")
        .arg(horizon)
        .arg(desired)
        .arg(actual)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    extract_ulid(&stdout).expect("Should get tension ID from add output")
}

/// Helper: get JSON output from show command.
fn show_json(dir: &TempDir, id: &str) -> Value {
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    serde_json::from_str(&stdout).expect("show --json should produce valid JSON")
}

/// Helper: get TOON output from show command.
fn show_toon(dir: &TempDir, id: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("--toon")
        .arg("show")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8_lossy(&output).to_string()
}

/// Helper: update reality for a tension.
fn update_reality(dir: &TempDir, id: &str, new_reality: &str) {
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(id)
        .arg(new_reality)
        .current_dir(dir.path())
        .assert()
        .success();
}

// =============================================================================
// VAL-CROSS-001: Gap magnitude flows through all dynamics correctly
// =============================================================================

/// VAL-CROSS-001: After creating tensions and mutating them, dynamics values
/// should be computed correctly using the hybrid Levenshtein+Jaccard gap metric.
/// Verify structural_tension, phase, resolution, and assimilation_depth are
/// in expected ranges.
#[test]
fn test_gap_magnitude_flows_through_dynamics() {
    let dir = init_workspace();

    // Create tension with different desired vs actual (gap exists)
    let id = add_tension(
        &dir,
        "write a complete novel manuscript",
        "have a rough outline",
    );

    // Check initial dynamics: should have structural_tension with magnitude > 0
    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // Structural tension should exist with magnitude > 0
    let st = &dynamics["structural_tension"];
    assert!(
        st.is_object(),
        "Should have structural_tension object: {:?}",
        st
    );
    let magnitude = st["magnitude"].as_f64().unwrap();
    assert!(
        magnitude > 0.0 && magnitude <= 1.0,
        "Gap magnitude should be in (0, 1], got: {}",
        magnitude
    );
    assert_eq!(
        st["has_gap"].as_bool(),
        Some(true),
        "Should detect a gap between different strings"
    );

    // Phase should be Germination initially
    assert_eq!(
        dynamics["phase"]["phase"].as_str(),
        Some("Germination"),
        "Initial phase should be Germination"
    );

    // Structural tendency should exist
    assert!(
        dynamics["structural_tendency"]["tendency"].is_string(),
        "Should have structural_tendency"
    );

    // Make several reality updates to create mutation history
    for i in 1..=5 {
        update_reality(
            &dir,
            &id,
            &format!("have drafted {} chapters of the novel", i),
        );
    }

    // Check dynamics after updates
    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // Structural tension should still exist
    let st = &dynamics["structural_tension"];
    assert!(
        st.is_object(),
        "Should still have structural_tension after updates"
    );

    // Gap magnitude should be > 0 (still different strings)
    let magnitude_after = st["magnitude"].as_f64().unwrap();
    assert!(
        magnitude_after > 0.0 && magnitude_after <= 1.0,
        "Gap magnitude after updates should be in (0, 1], got: {}",
        magnitude_after
    );

    // Assimilation depth should now exist (we have mutations)
    let ad = &dynamics["assimilation_depth"];
    assert!(
        ad.is_object(),
        "Should have assimilation_depth after mutations: {:?}",
        ad
    );

    // Mutation frequency should be > 0
    let freq = ad["mutation_frequency"].as_f64().unwrap();
    assert!(
        freq > 0.0,
        "Mutation frequency should be positive after updates: {}",
        freq
    );
}

/// VAL-CROSS-001: Verify that identical desired/actual yields 0 magnitude.
#[test]
fn test_gap_magnitude_zero_for_identical() {
    let dir = init_workspace();

    // Create tension with identical desired and actual
    let id = add_tension(&dir, "same state", "same state");

    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // Structural tension should either be None or have magnitude 0
    let st = &dynamics["structural_tension"];
    if st.is_object() {
        let magnitude = st["magnitude"].as_f64().unwrap();
        assert!(
            magnitude == 0.0,
            "Identical strings should produce 0.0 magnitude, got: {}",
            magnitude
        );
        assert_eq!(
            st["has_gap"].as_bool(),
            Some(false),
            "Identical strings should have no gap"
        );
    }
    // If structural_tension is null, that's also acceptable (no gap)
}

/// VAL-CROSS-001: Verify compute_temporal_pressure receives gap magnitude.
/// A tension with a horizon should show pressure (magnitude * urgency).
#[test]
fn test_gap_magnitude_flows_to_temporal_pressure() {
    let dir = init_workspace();

    // Create tension with a close horizon (high urgency)
    let id = add_tension_with_horizon(
        &dir,
        "ship product v1",
        "have prototype",
        "2026-04", // month horizon relatively soon
    );

    let json = show_json(&dir, &id);

    // Should have urgency (because of horizon)
    let urgency = json["urgency"].as_f64();
    assert!(
        urgency.is_some(),
        "Should have urgency with horizon: {:?}",
        json["urgency"]
    );

    // Should have pressure (magnitude * urgency)
    let pressure = json["pressure"].as_f64();
    assert!(
        pressure.is_some(),
        "Should have pressure (magnitude * urgency) with horizon"
    );

    // Pressure should be non-negative (magnitude * urgency, both >= 0)
    if let Some(p) = pressure {
        assert!(p >= 0.0, "Pressure should be non-negative, got: {}", p);
    }
}

/// VAL-CROSS-001: Resolution detection uses the gap metric.
/// After reality converges toward desired, resolution should be detected.
#[test]
fn test_gap_magnitude_flows_to_resolution_detection() {
    let dir = init_workspace();

    let id = add_tension(&dir, "learn to cook pasta", "never cooked before");

    // Make progressive reality updates that converge toward desired
    update_reality(&dir, &id, "watched cooking tutorials");
    update_reality(&dir, &id, "cooked pasta once with guidance");
    update_reality(&dir, &id, "cooked pasta several times independently");
    update_reality(&dir, &id, "can cook multiple pasta dishes reliably");
    update_reality(&dir, &id, "learned to cook pasta well");

    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // After progressive convergence, structural_tension magnitude should be smaller
    let st = &dynamics["structural_tension"];
    if st.is_object() {
        let magnitude = st["magnitude"].as_f64().unwrap();
        // Magnitude should reflect that reality is closer to desired now
        assert!(
            magnitude <= 1.0,
            "Magnitude should be <= 1.0, got: {}",
            magnitude
        );
    }

    // Resolution might or might not be detected depending on the specific gap magnitudes,
    // but the resolution field should exist in the JSON (even if null)
    assert!(
        dynamics.get("resolution").is_some(),
        "Should have resolution field in dynamics output"
    );
}

// =============================================================================
// VAL-CROSS-002: New events visible through DynamicsEngine usage in CLI
// =============================================================================

/// VAL-CROSS-002: When dynamics change (e.g., horizon drift occurs), the CLI
/// dynamics output should reflect the updated state. Verify that horizon_drift
/// appears in dynamics after horizon changes.
#[test]
fn test_new_events_reflected_in_cli_dynamics() {
    let dir = init_workspace();

    // Create tension with a horizon
    let id = add_tension_with_horizon(
        &dir,
        "complete project",
        "just started",
        "2026-06", // June 2026
    );

    // Check initial horizon drift
    let json = show_json(&dir, &id);
    let drift = &json["dynamics"]["horizon_drift"];
    assert!(
        drift.is_object(),
        "Should have horizon_drift in dynamics: {:?}",
        drift
    );
    assert_eq!(
        drift["drift_type"].as_str(),
        Some("Stable"),
        "Initial horizon drift should be Stable"
    );
    assert_eq!(
        drift["change_count"].as_u64(),
        Some(0),
        "Initial change count should be 0"
    );

    // Postpone the horizon (this creates a horizon mutation that drift detects)
    cargo_bin_cmd!("werk")
        .arg("horizon")
        .arg(&id)
        .arg("2026-09") // Push to September
        .current_dir(dir.path())
        .assert()
        .success();

    // Now check horizon drift — it should detect the postponement
    let json = show_json(&dir, &id);
    let drift = &json["dynamics"]["horizon_drift"];
    assert!(
        drift["change_count"].as_u64().unwrap() >= 1,
        "Should detect at least 1 horizon change after postponement, got: {:?}",
        drift
    );
    // net_shift_seconds should be positive (horizon moved later)
    assert!(
        drift["net_shift_seconds"].as_i64().unwrap() > 0,
        "Net shift should be positive after postponement: {:?}",
        drift
    );
}

/// VAL-CROSS-002: Verify that compensating strategy can be detected in CLI dynamics.
/// This test creates conditions that might trigger compensating strategy detection
/// (oscillation pattern + strategy detection).
#[test]
fn test_compensating_strategy_visible_in_dynamics() {
    let dir = init_workspace();

    let id = add_tension(&dir, "achieve fitness goal", "sedentary lifestyle");

    // Create an oscillation-like pattern (advance then regress repeatedly)
    update_reality(&dir, &id, "started exercising regularly");
    update_reality(&dir, &id, "stopped exercising, back to sedentary");
    update_reality(&dir, &id, "started exercising again");
    update_reality(&dir, &id, "gave up again, sedentary");
    update_reality(&dir, &id, "trying once more");
    update_reality(&dir, &id, "gave up");

    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // Compensating strategy field should be present in the output (even if null)
    assert!(
        dynamics.get("compensating_strategy").is_some(),
        "Should have compensating_strategy field in dynamics output"
    );

    // Oscillation field should be present
    assert!(
        dynamics.get("oscillation").is_some(),
        "Should have oscillation field in dynamics output"
    );
}

// =============================================================================
// VAL-CROSS-003: TOON output includes all new dynamics fields
// =============================================================================

/// VAL-CROSS-003: TOON output from show should include horizon_drift,
/// resolution velocity sufficiency, staleness_ratio, and all dynamics fields.
#[test]
fn test_toon_includes_all_new_dynamics_fields() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(
        &dir,
        "deliver project milestone",
        "initial planning phase",
        "2026-06",
    );

    // Make some updates
    update_reality(&dir, &id, "requirements gathered");
    update_reality(&dir, &id, "implementation started");

    // Get JSON and TOON outputs
    let json = show_json(&dir, &id);
    let toon = show_toon(&dir, &id);

    // Verify JSON has all expected fields
    let dynamics = &json["dynamics"];
    assert!(
        dynamics["horizon_drift"].is_object(),
        "JSON should have horizon_drift"
    );
    assert!(
        dynamics["horizon_drift"]["drift_type"].is_string(),
        "horizon_drift should have drift_type"
    );
    assert!(
        dynamics["horizon_drift"]["change_count"].is_number(),
        "horizon_drift should have change_count"
    );
    assert!(
        dynamics["horizon_drift"]["net_shift_seconds"].is_number(),
        "horizon_drift should have net_shift_seconds"
    );

    // Resolution field should exist (even if null)
    assert!(
        dynamics.get("resolution").is_some(),
        "JSON should have resolution field"
    );

    // staleness_ratio in top-level tension info
    assert!(
        json.get("staleness_ratio").is_some(),
        "JSON should have staleness_ratio field"
    );

    // Verify TOON contains the same key fields
    assert!(
        toon.contains("horizon_drift")
            || toon.contains("horizonDrift")
            || toon.contains("horizon drift"),
        "TOON should contain horizon_drift field, got TOON:\n{}",
        &toon[..toon.len().min(500)]
    );
    assert!(
        toon.contains("drift_type") || toon.contains("driftType") || toon.contains("drift type"),
        "TOON should contain drift_type field"
    );
    assert!(
        toon.contains("staleness_ratio")
            || toon.contains("stalenessRatio")
            || toon.contains("staleness ratio"),
        "TOON should contain staleness_ratio field"
    );
}

/// VAL-CROSS-003: Compare TOON and JSON for field set equivalence.
/// Both outputs should contain the same data fields.
#[test]
fn test_toon_and_json_same_fields() {
    let dir = init_workspace();

    let id = add_tension(&dir, "test field consistency", "initial state");
    update_reality(&dir, &id, "updated state");

    let json = show_json(&dir, &id);
    let toon = show_toon(&dir, &id);

    // Verify key top-level fields exist in both formats
    let expected_json_fields = ["id", "desired", "actual", "status", "dynamics", "mutations"];
    for field in expected_json_fields {
        assert!(
            json.get(field).is_some(),
            "JSON should have '{}' field",
            field
        );
        // TOON uses key-value format - check the key appears
        assert!(
            toon.contains(field),
            "TOON should reference '{}' field, TOON:\n{}",
            field,
            &toon[..toon.len().min(500)]
        );
    }

    // Verify dynamics sub-fields exist in both
    let dynamics = &json["dynamics"];
    let expected_dynamics = [
        "structural_tension",
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
    for field in expected_dynamics {
        assert!(
            dynamics.get(field).is_some(),
            "JSON dynamics should have '{}' field",
            field
        );
    }
}

/// VAL-CROSS-003: TOON output for tension with horizon includes velocity fields.
#[test]
fn test_toon_includes_resolution_velocity_fields() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(&dir, "finish manuscript", "chapter 1 drafted", "2026-07");

    // Make progressive updates to trigger resolution detection
    update_reality(&dir, &id, "chapter 2 drafted");
    update_reality(&dir, &id, "chapter 3 drafted");
    update_reality(&dir, &id, "chapter 4 drafted");
    update_reality(&dir, &id, "chapter 5 drafted");

    let json = show_json(&dir, &id);
    let toon = show_toon(&dir, &id);

    // If resolution is detected, check velocity fields
    if json["dynamics"]["resolution"].is_object() {
        let resolution = &json["dynamics"]["resolution"];
        // velocity should be present
        assert!(
            resolution.get("velocity").is_some(),
            "Resolution should have velocity field"
        );
        // required_velocity and is_sufficient should be present (may be null without horizon)
        assert!(
            resolution.get("required_velocity").is_some(),
            "Resolution should have required_velocity field"
        );
        assert!(
            resolution.get("is_sufficient").is_some(),
            "Resolution should have is_sufficient field"
        );

        // TOON should also contain velocity
        assert!(
            toon.contains("velocity"),
            "TOON should contain velocity field when resolution is detected"
        );
    }
}

// =============================================================================
// VAL-CROSS-005: Existing databases backward compatible
// =============================================================================

/// VAL-CROSS-005: An existing .werk/sd.db database should load correctly
/// after the gap magnitude algorithm change. No migration needed.
#[test]
fn test_existing_database_backward_compatible() {
    let dir = init_workspace();

    // Create data using the current schema
    let id = add_tension(&dir, "existing goal", "existing reality");
    update_reality(&dir, &id, "updated reality");
    update_reality(&dir, &id, "further updated");

    // Verify we can still read and compute dynamics
    let json = show_json(&dir, &id);
    assert_eq!(
        json["id"].as_str().unwrap(),
        id,
        "Should load tension correctly"
    );
    assert!(
        json["dynamics"].is_object(),
        "Should compute dynamics from existing data"
    );

    // Add a child tension to test forest operations
    let child_output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("--parent")
        .arg(&id)
        .arg("sub-goal")
        .arg("sub-reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let child_stdout = String::from_utf8_lossy(&child_output);
    let child_id = extract_ulid(&child_stdout).unwrap();

    // Verify tree still works
    let tree_output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let tree_json: Value = serde_json::from_str(&String::from_utf8_lossy(&tree_output)).unwrap();
    let tensions = tree_json["tensions"].as_array().unwrap();
    assert!(
        tensions.len() >= 2,
        "Tree should show parent and child: {}",
        tensions.len()
    );

    // Verify context still works with existing data
    let ctx_output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let ctx_json: Value = serde_json::from_str(&String::from_utf8_lossy(&ctx_output)).unwrap();
    assert!(
        ctx_json["dynamics"].is_object(),
        "Context dynamics should be computed from existing data"
    );
    assert!(
        !ctx_json["children"].as_array().unwrap().is_empty(),
        "Context should show children"
    );

    // Verify show still works on child
    let child_json = show_json(&dir, &child_id);
    assert!(
        child_json["dynamics"].is_object(),
        "Child dynamics should be computed"
    );
}

/// VAL-CROSS-005: A database created pre-algorithm-change should still work.
/// We simulate this by creating tensions via sd-core Store directly.
#[test]
fn test_database_loads_without_migration() {
    let dir = TempDir::new().unwrap();

    // Init workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tensions directly via Store (simulates pre-existing data)
    {
        let store = sd_core::Store::init(dir.path()).unwrap();
        let t1 = store.create_tension("goal alpha", "reality alpha").unwrap();
        store.update_actual(&t1.id, "updated alpha").unwrap();
        let _t2 = store.create_tension("goal beta", "reality beta").unwrap();
    }

    // Now use CLI to read the data — this should work without migration
    let tree_output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let tree_json: Value = serde_json::from_str(&String::from_utf8_lossy(&tree_output)).unwrap();
    assert_eq!(
        tree_json["tensions"].as_array().unwrap().len(),
        2,
        "Should show both tensions from store-created data"
    );

    // Show each tension — dynamics should compute without errors
    for t in tree_json["tensions"].as_array().unwrap() {
        let id = t["id"].as_str().unwrap();
        let json = show_json(&dir, id);
        assert!(
            json["dynamics"].is_object(),
            "Dynamics should compute for store-created tension"
        );
    }
}

// =============================================================================
// VAL-CROSS-006: show --verbose displays compensating strategy, urgency
// threshold, horizon drift
// =============================================================================

/// VAL-CROSS-006: show --verbose should display CompensatingStrategy line.
#[test]
fn test_show_verbose_displays_compensating_strategy() {
    let dir = init_workspace();

    let id = add_tension(&dir, "achieve ambitious goal", "far from goal");

    // Make some mutations to build history
    update_reality(&dir, &id, "made some progress");
    update_reality(&dir, &id, "lost progress again");
    update_reality(&dir, &id, "trying again");

    // Run show --verbose
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("CompensatingStrategy:"));
}

/// VAL-CROSS-006: show --verbose should display UrgencyThreshold line when horizon present.
#[test]
fn test_show_verbose_displays_urgency_threshold() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(&dir, "meet deadline", "haven't started yet", "2026-06");

    // Run show --verbose
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("UrgencyThreshold:"))
        .stdout(predicate::str::contains("threshold"));
}

/// VAL-CROSS-006: show --verbose should display HorizonDrift line when horizon present.
#[test]
fn test_show_verbose_displays_horizon_drift() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(&dir, "complete project", "just starting", "2026-08");

    // Run show --verbose
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("HorizonDrift:"));
}

/// VAL-CROSS-006: show --verbose should display all three items together.
#[test]
fn test_show_verbose_displays_all_three_new_items() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(&dir, "launch product", "initial concept", "2026-07");

    // Make some mutations
    update_reality(&dir, &id, "design phase");
    update_reality(&dir, &id, "development phase");
    update_reality(&dir, &id, "testing phase");

    // Run show --verbose — should display all three items
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // All three should be present in verbose output
    assert!(
        stdout.contains("CompensatingStrategy:"),
        "Verbose output should contain CompensatingStrategy line:\n{}",
        stdout
    );
    assert!(
        stdout.contains("UrgencyThreshold:"),
        "Verbose output should contain UrgencyThreshold line:\n{}",
        stdout
    );
    assert!(
        stdout.contains("HorizonDrift:"),
        "Verbose output should contain HorizonDrift line:\n{}",
        stdout
    );
}

// =============================================================================
// VAL-CROSS-007: Canonical scenario with recalibrated defaults produces
// correct classifications
// =============================================================================

/// VAL-CROSS-007: A canonical tension lifecycle should produce expected
/// dynamics classifications with the recalibrated default thresholds.
#[test]
fn test_canonical_scenario_correct_classifications() {
    let dir = init_workspace();

    // Create a typical tension with a horizon
    let id = add_tension_with_horizon(
        &dir,
        "write and publish a technical article",
        "have an idea for an article topic",
        "2026-06",
    );

    // Step 1: Initial state should be Germination
    let json = show_json(&dir, &id);
    let phase = json["dynamics"]["phase"]["phase"].as_str().unwrap();
    assert_eq!(phase, "Germination", "Initial phase should be Germination");

    // Step 2: Make progressive updates
    update_reality(&dir, &id, "outlined the article structure");
    update_reality(&dir, &id, "drafted the introduction section");
    update_reality(&dir, &id, "completed first draft of all sections");

    // Check dynamics after progress
    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // Should have structural_tension
    assert!(
        dynamics["structural_tension"].is_object(),
        "Should have structural_tension after updates"
    );

    // Tendency should be meaningful
    let tendency = dynamics["structural_tendency"]["tendency"]
        .as_str()
        .unwrap();
    assert!(
        tendency == "Advancing" || tendency == "Oscillating" || tendency == "Stagnant",
        "Tendency should be a valid classification: {}",
        tendency
    );

    // Step 3: Continue to resolution
    update_reality(&dir, &id, "article fully written and edited");
    update_reality(&dir, &id, "article published on blog");

    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // After significant progress, magnitude should reflect convergence
    let st = &dynamics["structural_tension"];
    if st.is_object() {
        let magnitude = st["magnitude"].as_f64().unwrap();
        assert!(
            magnitude <= 1.0,
            "Magnitude should be in valid range: {}",
            magnitude
        );
    }

    // Phase should have evolved (not necessarily Germination anymore)
    let phase = dynamics["phase"]["phase"].as_str().unwrap();
    assert!(
        phase == "Germination"
            || phase == "Assimilation"
            || phase == "Completion"
            || phase == "Momentum",
        "Phase should be a valid classification after updates: {}",
        phase
    );

    // Horizon drift should be Stable (we didn't change the horizon)
    assert_eq!(
        dynamics["horizon_drift"]["drift_type"].as_str(),
        Some("Stable"),
        "Horizon drift should be Stable when horizon unchanged"
    );
}

/// VAL-CROSS-007: Verify that dynamics classifications are consistent
/// between show --json and context commands.
#[test]
fn test_canonical_scenario_json_context_consistency() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(
        &dir,
        "learn new programming language",
        "no experience with the language",
        "2026-09",
    );

    update_reality(&dir, &id, "completed tutorial");
    update_reality(&dir, &id, "built first small project");
    update_reality(&dir, &id, "contributing to open source");

    // Get show --json
    let show = show_json(&dir, &id);

    // Get context (always structured output)
    let ctx_output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let ctx: Value = serde_json::from_str(&String::from_utf8_lossy(&ctx_output)).unwrap();

    // Phase should match (field names differ: show uses "phase", context uses "creative_cycle_phase")
    let show_phase = show["dynamics"]["phase"]["phase"].as_str().unwrap();
    let ctx_phase = ctx["dynamics"]["creative_cycle_phase"]["phase"]
        .as_str()
        .unwrap();
    assert_eq!(
        show_phase, ctx_phase,
        "Phase should match between show and context"
    );

    // Structural tendency should match
    let show_tendency = show["dynamics"]["structural_tendency"]["tendency"]
        .as_str()
        .unwrap();
    let ctx_tendency = ctx["dynamics"]["structural_tendency"]["tendency"]
        .as_str()
        .unwrap();
    assert_eq!(
        show_tendency, ctx_tendency,
        "Structural tendency should match between show and context"
    );

    // Horizon drift should match
    let show_drift = show["dynamics"]["horizon_drift"]["drift_type"]
        .as_str()
        .unwrap();
    let ctx_drift = ctx["dynamics"]["horizon_drift"]["drift_type"]
        .as_str()
        .unwrap();
    assert_eq!(
        show_drift, ctx_drift,
        "Horizon drift should match between show and context"
    );

    // staleness_ratio in context tension info should exist
    assert!(
        ctx["tension"].get("staleness_ratio").is_some(),
        "Context tension should have staleness_ratio field"
    );

    // show should also have staleness_ratio at top level
    assert!(
        show.get("staleness_ratio").is_some(),
        "Show should have staleness_ratio at top level"
    );
}

/// VAL-CROSS-007: Verify horizon_drift in context and run outputs.
#[test]
fn test_context_run_include_horizon_drift() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(
        &dir,
        "write research paper",
        "collected references",
        "2026-08",
    );

    update_reality(&dir, &id, "drafted abstract");

    // Get context output
    let ctx_output = cargo_bin_cmd!("werk")
        .arg("context")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let ctx: Value = serde_json::from_str(&String::from_utf8_lossy(&ctx_output)).unwrap();

    // Context should have horizon_drift
    let ctx_drift = &ctx["dynamics"]["horizon_drift"];
    assert!(
        ctx_drift.is_object(),
        "Context dynamics should have horizon_drift: {:?}",
        ctx_drift
    );
    assert!(
        ctx_drift["drift_type"].is_string(),
        "Should have drift_type"
    );
    assert!(
        ctx_drift["change_count"].is_number(),
        "Should have change_count"
    );
    assert!(
        ctx_drift["net_shift_seconds"].is_number(),
        "Should have net_shift_seconds"
    );

    // Context resolution should have required_velocity and is_sufficient fields
    // (may be null if resolution not detected)
    let dynamics = &ctx["dynamics"];
    if dynamics["resolution"].is_object() {
        assert!(
            dynamics["resolution"].get("required_velocity").is_some(),
            "Context resolution should have required_velocity field"
        );
        assert!(
            dynamics["resolution"].get("is_sufficient").is_some(),
            "Context resolution should have is_sufficient field"
        );
    }
}

/// VAL-CROSS-003/007: Verify TOON context output includes new dynamics fields.
#[test]
fn test_toon_context_includes_new_fields() {
    let dir = init_workspace();

    let id = add_tension_with_horizon(
        &dir,
        "build a web application",
        "have a design mockup",
        "2026-07",
    );

    update_reality(&dir, &id, "set up project scaffolding");
    update_reality(&dir, &id, "implemented authentication");

    // Get TOON context output
    let ctx_output = cargo_bin_cmd!("werk")
        .arg("--toon")
        .arg("context")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let toon = String::from_utf8_lossy(&ctx_output);

    // TOON should contain the key dynamics fields
    assert!(
        toon.contains("horizon_drift"),
        "TOON context should contain horizon_drift"
    );
    assert!(
        toon.contains("staleness_ratio"),
        "TOON context should contain staleness_ratio"
    );
}

// =============================================================================
// Additional cross-area validation tests
// =============================================================================

/// Verify that a tension without horizon still produces valid dynamics output.
#[test]
fn test_dynamics_without_horizon_all_fields_present() {
    let dir = init_workspace();

    let id = add_tension(&dir, "learn guitar", "never played before");
    update_reality(&dir, &id, "learned basic chords");

    let json = show_json(&dir, &id);
    let dynamics = &json["dynamics"];

    // All dynamics fields should be present (even if null)
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
    for field in expected_fields {
        assert!(
            dynamics.get(field).is_some(),
            "Dynamics should have '{}' field (even if null), dynamics: {:?}",
            field,
            dynamics
        );
    }

    // Horizon drift should be Stable with 0 changes when no horizon
    assert_eq!(
        dynamics["horizon_drift"]["drift_type"].as_str(),
        Some("Stable"),
    );
    assert_eq!(dynamics["horizon_drift"]["change_count"].as_u64(), Some(0),);
}

/// Verify that urgency, pressure, and staleness_ratio fields exist at top level.
#[test]
fn test_show_json_top_level_horizon_fields() {
    let dir = init_workspace();

    // Without horizon
    let id_no_horizon = add_tension(&dir, "goal without horizon", "starting point");
    let json = show_json(&dir, &id_no_horizon);
    assert!(
        json.get("urgency").is_some(),
        "Should have urgency field (null)"
    );
    assert!(
        json.get("pressure").is_some(),
        "Should have pressure field (null)"
    );
    assert!(
        json.get("staleness_ratio").is_some(),
        "Should have staleness_ratio field"
    );

    // With horizon
    let id_with_horizon =
        add_tension_with_horizon(&dir, "goal with horizon", "starting point", "2026-06");
    update_reality(&dir, &id_with_horizon, "made progress");

    let json = show_json(&dir, &id_with_horizon);
    assert!(
        json["urgency"].is_number(),
        "Should have non-null urgency with horizon"
    );
    // staleness_ratio should be non-null if there's a horizon and mutations
    assert!(
        json["staleness_ratio"].is_number(),
        "Should have non-null staleness_ratio with horizon and mutations"
    );
}
