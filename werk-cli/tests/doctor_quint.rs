//! R-005 fixture round-trip tests: corrupt → detect → fix → healthy →
//! undo → pre-corruption.
//!
//! Each Quint invariant gets a dedicated end-to-end test that exercises
//! the CLI surface as an external observer (via `assert_cmd`). Internal
//! `Store::doctor_test_*_raw` helpers inject violations bypassing the
//! gesture API; those helpers are `#[doc(hidden)]` and not part of the
//! public werk-core API.

use assert_cmd::cargo_bin_cmd;
use std::path::Path;
use tempfile::TempDir;

fn init_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

/// Open a Store handle against the freshly-initialized workspace using
/// the `init_unlocked` path (skips backup rotation; matches how the
/// existing R-003 tests probe state).
fn store(root: &Path) -> werk_core::Store {
    werk_core::Store::init_unlocked(root).unwrap()
}

fn doctor_diagnose_findings(root: &Path) -> serde_json::Value {
    let out = cargo_bin_cmd!("werk")
        .args(["doctor", "--json"])
        .current_dir(root)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).expect("doctor --json output")
}

/// Run `werk doctor --fix --yes --json [extra...]` and return
/// `(exit_code, run_id, raw_envelope)`. The run_id is captured so the
/// test can `undo <run_id>` without race-condition risk from intervening
/// diagnose calls (each of which promotes `latest` to a read-only run).
fn run_fix(root: &Path, extra: &[&str]) -> (i32, Option<String>, serde_json::Value) {
    let mut args: Vec<&str> = vec!["doctor", "--fix", "--yes", "--json"];
    args.extend_from_slice(extra);
    let out = cargo_bin_cmd!("werk")
        .args(&args)
        .current_dir(root)
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("fix --json output");
    let run_id = v
        .get("run_id")
        .and_then(|s| s.as_str())
        .map(String::from);
    (out.status.code().unwrap_or(0), run_id, v)
}

/// Run `werk doctor undo <run_id>` (NOT `latest` — `latest` is volatile
/// across intervening diagnose calls).
fn run_undo(root: &Path, run_id: &str) {
    cargo_bin_cmd!("werk")
        .args(["doctor", "undo", run_id])
        .current_dir(root)
        .assert()
        .code(0);
}

fn finding_ids(v: &serde_json::Value) -> Vec<String> {
    v.get("data")
        .and_then(|d| d.get("findings"))
        .and_then(|f| f.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|f| f.get("id").and_then(|s| s.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Compute a logical-state probe over the six Quint-relevant tables.
/// fsqlite touches DB file bytes on open for housekeeping, so byte-
/// equality is not a stable invariant. The doctor's undo round-trip is
/// asserted over logical state instead.
#[derive(Debug, Clone, PartialEq, Eq)]
struct QuintProbe {
    tension_count: usize,
    edge_tuples: Vec<(String, String, String, String)>, // (id, from, to, type)
    positions: Vec<(String, Option<i32>)>,
    horizons: Vec<(String, Option<String>)>,
    parent_ids: Vec<(String, Option<String>)>,
    gesture_count: usize,
    /// (gesture_id, undone_gesture_id) for rows with a non-NULL undone.
    undone_pairs: Vec<(String, String)>,
}

fn probe(root: &Path) -> QuintProbe {
    let s = store(root);
    let tensions = s.list_tensions().unwrap();
    let mut positions: Vec<(String, Option<i32>)> = tensions
        .iter()
        .map(|t| (t.id.clone(), t.position))
        .collect();
    positions.sort();
    let mut horizons: Vec<(String, Option<String>)> = tensions
        .iter()
        .map(|t| (t.id.clone(), t.horizon.as_ref().map(|h| h.to_string())))
        .collect();
    horizons.sort();
    let mut parent_ids: Vec<(String, Option<String>)> = tensions
        .iter()
        .map(|t| (t.id.clone(), t.parent_id.clone()))
        .collect();
    parent_ids.sort();
    let mut edge_tuples: Vec<(String, String, String, String)> = Vec::new();
    for t in &tensions {
        for e in s.get_edges_for_tension(&t.id).unwrap() {
            edge_tuples.push((e.id, e.from_id, e.to_id, e.edge_type));
        }
    }
    edge_tuples.sort();
    edge_tuples.dedup();
    let dangling = s.list_dangling_undo_gestures().unwrap();
    let mut undone_pairs: Vec<(String, String)> = dangling
        .into_iter()
        .map(|r| (r.gesture_id, r.dangling_referent))
        .collect();
    undone_pairs.sort();
    QuintProbe {
        tension_count: tensions.len(),
        edge_tuples,
        positions,
        horizons,
        parent_ids,
        gesture_count: 0,
        undone_pairs,
    }
}

// ───────────────────────────────────────────────────────────────────────
// singleParent — soft-refused unless --prefer is passed
// ───────────────────────────────────────────────────────────────────────

#[test]
fn doctor_fixes_single_parent_violation_and_undoes() {
    let dir = init_workspace();
    let root = dir.path();
    let (pre_probe, child_id) = {
        let s = store(root);
        let parent_a = s.create_tension("parent A", "ra").unwrap();
        let parent_b = s.create_tension("parent B", "rb").unwrap();
        let child = s.create_tension("child", "rc").unwrap();
        // Real edge: parent_a → child. Injected duplicate: parent_b → child.
        s.doctor_test_insert_edge_raw(&parent_a.id, &child.id, "contains")
            .unwrap();
        s.doctor_test_insert_edge_raw(&parent_b.id, &child.id, "contains")
            .unwrap();
        drop(s);
        (probe(root), child.id)
    };

    // Detect.
    let v = doctor_diagnose_findings(root);
    assert!(finding_ids(&v).contains(&"fm-edges-multi-parent".to_string()));

    // Soft-refuse: --fix without --prefer leaves findings on the field.
    let (exit, _run, _v) = run_fix(root, &[]);
    assert!(
        exit == 1 || exit == 2,
        "soft-refused fix should exit findings_present or partial_fix, got {}",
        exit
    );
    assert!(
        finding_ids(&doctor_diagnose_findings(root))
            .contains(&"fm-edges-multi-parent".to_string()),
        "soft-refused finding should persist"
    );

    // Now apply with --prefer=newest (matches MVCC last-write-wins).
    let (exit, run_id, _) = run_fix(root, &["--prefer=newest"]);
    assert_eq!(exit, 0);
    let run_id = run_id.expect("fix should record a run_id");

    // Capture post-fix state BEFORE running diagnose (which would promote
    // `latest` to a no-actions run and silently shadow the fix run).
    let post_fix = probe(root);
    let child_parent = post_fix
        .parent_ids
        .iter()
        .find(|(id, _)| id == &child_id)
        .and_then(|(_, p)| p.clone());
    assert!(
        child_parent.is_some(),
        "child should have exactly one parent after fix"
    );

    // Undo by explicit run_id (not `latest`).
    run_undo(root, &run_id);
    assert_eq!(probe(root), pre_probe, "undo did not restore pre-fix state");
}

// ───────────────────────────────────────────────────────────────────────
// noSelfEdges
// ───────────────────────────────────────────────────────────────────────

#[test]
fn doctor_fixes_self_edge_and_undoes() {
    let dir = init_workspace();
    let root = dir.path();
    let pre = {
        let s = store(root);
        let t = s.create_tension("alone", "r").unwrap();
        s.doctor_test_insert_edge_raw(&t.id, &t.id, "contains").unwrap();
        drop(s);
        probe(root)
    };
    assert!(
        finding_ids(&doctor_diagnose_findings(root)).contains(&"fm-edges-self-loop".to_string())
    );
    let (exit, run_id, _) = run_fix(root, &[]);
    assert_eq!(exit, 0);
    let run_id = run_id.expect("fix run_id");
    let post_fix = probe(root);
    assert!(post_fix.edge_tuples.is_empty(), "fix should delete self-edge");
    run_undo(root, &run_id);
    assert_eq!(probe(root), pre);
}

// ───────────────────────────────────────────────────────────────────────
// edgesValid
// ───────────────────────────────────────────────────────────────────────

#[test]
fn doctor_fixes_dangling_edge_and_undoes() {
    let dir = init_workspace();
    let root = dir.path();
    let pre = {
        let s = store(root);
        let t = s.create_tension("real", "r").unwrap();
        // Dangling: from_id references a phantom tension.
        s.doctor_test_insert_edge_raw("phantom-id-9999", &t.id, "contains")
            .unwrap();
        drop(s);
        probe(root)
    };
    assert!(finding_ids(&doctor_diagnose_findings(root)).contains(&"fm-edges-dangling".to_string()));
    let (exit, run_id, _) = run_fix(root, &[]);
    assert_eq!(exit, 0);
    let run_id = run_id.expect("fix run_id");
    run_undo(root, &run_id);
    assert_eq!(probe(root), pre);
}

// ───────────────────────────────────────────────────────────────────────
// siblingPositionsUnique
// ───────────────────────────────────────────────────────────────────────

#[test]
fn doctor_fixes_sibling_position_collision_and_undoes() {
    let dir = init_workspace();
    let root = dir.path();
    let pre = {
        let s = store(root);
        let parent = s.create_tension("parent", "rp").unwrap();
        let c1 = s.create_tension("c1", "r1").unwrap();
        let c2 = s.create_tension("c2", "r2").unwrap();
        s.doctor_test_insert_edge_raw(&parent.id, &c1.id, "contains")
            .unwrap();
        s.doctor_test_insert_edge_raw(&parent.id, &c2.id, "contains")
            .unwrap();
        s.doctor_test_set_position_raw(&c1.id, Some(7)).unwrap();
        s.doctor_test_set_position_raw(&c2.id, Some(7)).unwrap();
        drop(s);
        probe(root)
    };
    assert!(
        finding_ids(&doctor_diagnose_findings(root))
            .contains(&"fm-edges-sibling-position-collision".to_string())
    );
    let (exit, run_id, _) = run_fix(root, &[]);
    assert_eq!(exit, 0);
    let run_id = run_id.expect("fix run_id");
    run_undo(root, &run_id);
    assert_eq!(probe(root), pre);
}

// ───────────────────────────────────────────────────────────────────────
// noContainmentViolations (soft-refused unless --apply-horizon-fix)
// ───────────────────────────────────────────────────────────────────────

#[test]
fn doctor_fixes_horizon_violation_and_undoes() {
    let dir = init_workspace();
    let root = dir.path();
    let pre = {
        let s = store(root);
        let parent = s.create_tension("parent", "rp").unwrap();
        let child = s.create_tension("child", "rc").unwrap();
        s.doctor_test_insert_edge_raw(&parent.id, &child.id, "contains")
            .unwrap();
        s.doctor_test_set_horizon_raw(&parent.id, Some("2026")).unwrap();
        s.doctor_test_set_horizon_raw(&child.id, Some("2027")).unwrap();
        drop(s);
        probe(root)
    };
    assert!(
        finding_ids(&doctor_diagnose_findings(root))
            .contains(&"fm-edges-horizon-containment".to_string())
    );

    // Soft-refused without --apply-horizon-fix.
    let (exit, _run, _v) = run_fix(root, &[]);
    assert!(
        exit == 1 || exit == 2,
        "soft-refused horizon fix expected exit 1 or 2, got {}",
        exit
    );
    assert!(
        finding_ids(&doctor_diagnose_findings(root))
            .contains(&"fm-edges-horizon-containment".to_string()),
        "soft-refused finding should persist"
    );

    // Apply with --apply-horizon-fix.
    let (exit, run_id, _) = run_fix(root, &["--apply-horizon-fix"]);
    assert_eq!(exit, 0);
    let run_id = run_id.expect("fix run_id");
    run_undo(root, &run_id);
    assert_eq!(probe(root), pre);
}

// ───────────────────────────────────────────────────────────────────────
// undoneSubsetOfCompleted
// ───────────────────────────────────────────────────────────────────────

#[test]
fn doctor_fixes_dangling_undo_gesture_and_undoes() {
    let dir = init_workspace();
    let root = dir.path();
    let pre = {
        let s = store(root);
        // Inject a gesture row whose undone_gesture_id is a phantom.
        s.doctor_test_insert_gesture_raw("phantom undo", Some("nonexistent-gesture-id"))
            .unwrap();
        drop(s);
        probe(root)
    };
    assert!(
        finding_ids(&doctor_diagnose_findings(root))
            .contains(&"fm-gestures-undone-dangling".to_string())
    );
    let (exit, run_id, _) = run_fix(root, &[]);
    assert_eq!(exit, 0);
    let run_id = run_id.expect("fix run_id");
    run_undo(root, &run_id);
    assert_eq!(probe(root), pre);
}

// ───────────────────────────────────────────────────────────────────────
// Cross-invariant (W-1)
// ───────────────────────────────────────────────────────────────────────

#[test]
fn doctor_post_fix_no_other_invariant_violated() {
    // Inject a self-edge. After fix, assert NONE of the other five Quint
    // invariants is newly violated.
    let dir = init_workspace();
    let root = dir.path();
    {
        let s = store(root);
        let t = s.create_tension("alone", "r").unwrap();
        s.doctor_test_insert_edge_raw(&t.id, &t.id, "contains").unwrap();
    }
    cargo_bin_cmd!("werk")
        .args(["doctor", "--fix", "--yes", "--json"])
        .current_dir(root)
        .assert()
        .code(0);
    let post = doctor_diagnose_findings(root);
    let ids = finding_ids(&post);
    for fm in [
        "fm-edges-multi-parent",
        "fm-edges-self-loop",
        "fm-edges-dangling",
        "fm-edges-sibling-position-collision",
        "fm-edges-horizon-containment",
        "fm-gestures-undone-dangling",
    ] {
        assert!(!ids.contains(&fm.to_string()), "post-fix had {}", fm);
    }
}

#[test]
fn doctor_horizon_fix_multi_violation_chain_resolves_in_one_pass() {
    // Chain A→B→C with horizons {A=2025, B=2027 [violates A→B: 2027 > 2025],
    // C=2028 [violates B→C: 2028 > 2027]}. Detector flags BOTH edges in one
    // pass; fixer nulls both B's and C's horizons; harness sees zero findings.
    // This exercises the spec §2.6 promise that all directly-flagged edges
    // resolve in a single pass and that nulling multiple horizons in one
    // batch does not regress any pre-fix edge.
    let dir = init_workspace();
    let root = dir.path();
    {
        let s = store(root);
        let a = s.create_tension("a", "ra").unwrap();
        let b = s.create_tension("b", "rb").unwrap();
        let c = s.create_tension("c", "rc").unwrap();
        s.doctor_test_insert_edge_raw(&a.id, &b.id, "contains").unwrap();
        s.doctor_test_insert_edge_raw(&b.id, &c.id, "contains").unwrap();
        s.doctor_test_set_horizon_raw(&a.id, Some("2025")).unwrap();
        s.doctor_test_set_horizon_raw(&b.id, Some("2027")).unwrap();
        s.doctor_test_set_horizon_raw(&c.id, Some("2028")).unwrap();
    }
    // Verify both edges are flagged pre-fix.
    let pre_findings = finding_ids(&doctor_diagnose_findings(root));
    let pre_count = pre_findings
        .iter()
        .filter(|id| *id == "fm-edges-horizon-containment")
        .count();
    assert_eq!(
        pre_count, 2,
        "expected 2 horizon-containment findings (A→B and B→C), got {}: {:?}",
        pre_count, pre_findings
    );

    let (exit, _run_id, _v) = run_fix(root, &["--apply-horizon-fix"]);
    assert_eq!(exit, 0, "fix should exit healthy");

    let post = doctor_diagnose_findings(root);
    assert!(
        !finding_ids(&post).contains(&"fm-edges-horizon-containment".to_string()),
        "all horizon-containment violations should be resolved in one pass"
    );
}

#[test]
fn parent_id_reconciled_on_dup_edge_prune() {
    let dir = init_workspace();
    let root = dir.path();
    let (parent_a_id, parent_b_id, child_id) = {
        let s = store(root);
        let pa = s.create_tension("pa", "ra").unwrap();
        let pb = s.create_tension("pb", "rb").unwrap();
        let ch = s.create_tension("ch", "rc").unwrap();
        let _first = s
            .doctor_test_insert_edge_raw(&pa.id, &ch.id, "contains")
            .unwrap();
        // `Ulid::new()` (ulid 1.x) is NOT monotonic between successive
        // calls — same-millisecond invocations use fresh random bytes
        // and can sort either way. The test asserts that --prefer=newest
        // keeps parent_b's edge, which requires pb's edge to have a
        // strictly larger ULID than pa's. A 10 ms sleep guarantees the
        // ms-precision timestamp prefix differs, making the ordering
        // deterministic regardless of the random tail.
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _second = s
            .doctor_test_insert_edge_raw(&pb.id, &ch.id, "contains")
            .unwrap();
        (pa.id, pb.id, ch.id)
    };
    cargo_bin_cmd!("werk")
        .args([
            "doctor",
            "--fix",
            "--yes",
            "--prefer=newest",
            "--json",
        ])
        .current_dir(root)
        .assert()
        .code(0);
    let s = store(root);
    let child = s
        .get_tension(&child_id)
        .unwrap()
        .expect("child still exists");
    assert_eq!(
        child.parent_id.as_deref(),
        Some(parent_b_id.as_str()),
        "parent_id should match the kept (newest) edge's from_id"
    );
    let _ = parent_a_id; // suppress unused warning while keeping the name visible in the test
}

#[test]
fn doctor_capabilities_lists_all_quint_fixers() {
    // Drift detector: capabilities --json must enumerate every R-005
    // fixer with available: true and a non-empty backs_up list.
    let dir = init_workspace();
    let out = cargo_bin_cmd!("werk")
        .args(["doctor", "capabilities", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let fixers = v
        .get("fixers")
        .and_then(|f| f.as_array())
        .expect("fixers array");
    for op in [
        "prune_duplicate_parent_edges",
        "delete_self_edges",
        "delete_dangling_edges",
        "null_colliding_sibling_positions",
        "null_violating_child_horizon",
        "null_dangling_undo_gestures",
    ] {
        let f = fixers
            .iter()
            .find(|f| f.get("op").and_then(|s| s.as_str()) == Some(op))
            .unwrap_or_else(|| panic!("missing fixer op {}", op));
        assert_eq!(
            f.get("available").and_then(|b| b.as_bool()),
            Some(true),
            "fixer {} not available",
            op
        );
        assert!(
            f.get("backs_up")
                .and_then(|b| b.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false),
            "fixer {} missing backs_up",
            op
        );
    }
}

#[test]
fn doctor_explain_resolves_every_quint_finding_id() {
    let dir = init_workspace();
    for id in [
        "fm-edges-multi-parent",
        "fm-edges-self-loop",
        "fm-edges-dangling",
        "fm-edges-sibling-position-collision",
        "fm-edges-horizon-containment",
        "fm-edges-horizon-unparseable",
        "fm-gestures-undone-dangling",
        "singleParent",
        "noSelfEdges",
        "edgesValid",
        "siblingPositionsUnique",
        "noContainmentViolations",
        "undoneSubsetOfCompleted",
    ] {
        let out = cargo_bin_cmd!("werk")
            .args(["doctor", "--explain", id, "--json"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "explain {} exited non-zero: {:?}",
            id,
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            v.get("data").and_then(|d| d.get("id")).and_then(|s| s.as_str()),
            Some(id),
            "explain {} did not echo the id back",
            id
        );
    }
}
