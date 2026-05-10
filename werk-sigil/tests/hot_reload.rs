#![cfg(feature = "hot-reload")]

use tempfile::TempDir;
use werk_sigil::{compute_logic_hash, start_hot_reload};
use std::time::Duration;

#[test]
fn hot_reload_file_change_invalidates() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("logic.toml");
    std::fs::write(&path, "[meta]\nname=\"x\"\nversion=\"1\"\n").unwrap();
    let initial = compute_logic_hash(&std::fs::read_to_string(&path).unwrap());
    let watcher = start_hot_reload(vec![path.clone()]).unwrap();

    std::fs::write(&path, "[meta]\nname=\"x\"\nversion=\"1\"\n[scope]\ndefault={ kind=\"space\", name=\"active\" }\n").unwrap();
    let event = watcher
        .rx
        .recv_timeout(Duration::from_secs(1))
        .expect("expected hot reload event");
    assert_ne!(event.content_hash, initial);
    let expected = std::fs::canonicalize(&path).unwrap();
    let actual = std::fs::canonicalize(&event.path).unwrap_or(event.path);
    assert_eq!(actual, expected);
}
