use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;
use werk_core::Store;

fn setup_workspace() -> (TempDir, String, String) {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = Store::init(dir.path()).unwrap();
    let tension = store.create_tension("sigil lifecycle", "baseline").unwrap();
    let short_code = tension.short_code.unwrap().to_string();
    let id = tension.id.clone();
    (dir, short_code, id)
}

#[test]
fn save_then_show() {
    let (dir, short_code, _id) = setup_workspace();

    cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--save")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = Store::init(dir.path()).unwrap();
    let sigils = store.list_sigils().unwrap();
    assert_eq!(sigils.len(), 1);
    let sigil_code = sigils[0].short_code;

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(format!("*{sigil_code}"))
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("sigil show json should parse");
    assert_eq!(
        json.get("short_code").and_then(|v| v.as_i64()),
        Some(sigil_code as i64)
    );
    assert!(json.get("scope").is_some());
    assert!(json.get("logic").is_some());
    assert!(json.get("seed").is_some());
    assert!(json.get("path").is_some());
}

#[test]
fn no_gesture_emitted() {
    let (dir, short_code, id) = setup_workspace();
    let store = Store::init(dir.path()).unwrap();
    let before = store.get_mutations(&id).unwrap().len();

    cargo_bin_cmd!("werk")
        .arg("sigil")
        .arg(&short_code)
        .arg("--save")
        .current_dir(dir.path())
        .assert()
        .success();

    let after = store.get_mutations(&id).unwrap().len();
    assert_eq!(before, after, "sigil rendering should not create mutations");
}
