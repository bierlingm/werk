//! Integration tests for `werk config` command.
//!
//! Tests verify:
//! - VAL-CFG-001: Config set/get stores values in .werk/config.toml
//! - VAL-CFG-002: Config with no workspace uses ~/.werk/config.toml
//! - VAL-CFG-003: Malformed config.toml shows clear error (not panic)

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// =============================================================================
// VAL-CFG-001: Config set/get stores values in .werk/config.toml
// =============================================================================

/// Basic set and get roundtrip
#[test]
fn test_config_set_and_get() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set a config value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo test")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get the config value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo test"));
}

/// Config file is created on first set
#[test]
fn test_config_file_created_on_first_set() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Config file should not exist yet
    assert!(!dir.path().join(".werk").join("config.toml").exists());

    // Set a config value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo test")
        .current_dir(dir.path())
        .assert()
        .success();

    // Config file should now exist
    assert!(dir.path().join(".werk").join("config.toml").exists());
}

/// Set overwrites existing value
#[test]
fn test_config_set_overwrites() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set a config value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo first")
        .current_dir(dir.path())
        .assert()
        .success();

    // Overwrite with new value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo second")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get should return new value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo second"))
        .stdout(predicate::str::contains("echo first").not());
}

/// Get missing key returns error
#[test]
fn test_config_get_missing_key() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get a non-existent key
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("nonexistent.key")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Nested keys work correctly
#[test]
fn test_config_nested_keys() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set nested values
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("display.theme")
        .arg("dark")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("display.colors")
        .arg("true")
        .current_dir(dir.path())
        .assert()
        .success();

    // Both should be readable
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("display.theme")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dark"));

    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("display.colors")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("true"));
}

/// --json flag on config set
#[test]
fn test_config_set_json_output() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set with --json
    cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo test")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"key\""))
        .stdout(predicate::str::contains("\"value\""));
}

/// --json flag on config get
#[test]
fn test_config_get_json_output() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set a value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo test")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get with --json
    cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"key\""))
        .stdout(predicate::str::contains("\"value\""));
}

/// --json on missing key produces structured error
#[test]
fn test_config_get_missing_json_output() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get missing key with --json
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("config")
        .arg("get")
        .arg("nonexistent.key")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .clone();

    // Error should be JSON format in stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"error\""),
        "JSON error should have 'error' field, got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"code\""),
        "JSON error should have 'code' field, got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"message\""),
        "JSON error should have 'message' field, got: {}",
        stdout
    );

    // Verify the error code
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(
        json["error"]["code"].as_str(),
        Some("CONFIG_ERROR"),
        "Error code should be CONFIG_ERROR, got: {:?}",
        json
    );
}

// =============================================================================
// VAL-CFG-002: Config with no workspace uses ~/.werk/config.toml
// =============================================================================

/// Config in non-workspace directory uses global config
/// Note: We use a temp HOME to avoid touching user's real config
#[test]
fn test_config_uses_global_when_no_workspace() {
    let dir = TempDir::new().unwrap();
    let fake_home = TempDir::new().unwrap();

    // Do NOT init a workspace in dir - so it has no .werk/

    // Set config (should use global since no local workspace)
    cargo_bin_cmd!("werk")
        .env("HOME", fake_home.path())
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo global")
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify global config was created
    assert!(fake_home.path().join(".werk").join("config.toml").exists());

    // Get should work
    cargo_bin_cmd!("werk")
        .env("HOME", fake_home.path())
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo global"));
}

/// Local config takes precedence over global
#[test]
fn test_config_local_precedence() {
    let dir = TempDir::new().unwrap();
    let fake_home = TempDir::new().unwrap();

    // Set global config first
    cargo_bin_cmd!("werk")
        .env("HOME", fake_home.path())
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo global")
        .current_dir(dir.path())
        .assert()
        .success();

    // Now init local workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set local config
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo local")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get should return local value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo local"));
}

// =============================================================================
// VAL-CFG-003: Malformed config.toml shows clear error (not panic)
// =============================================================================

/// Malformed TOML produces descriptive error
#[test]
fn test_config_malformed_toml() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Write malformed TOML
    let config_path = dir.path().join(".werk").join("config.toml");
    std::fs::write(&config_path, "this is not valid toml [[[[").unwrap();

    // Attempt to get config should fail gracefully (no panic)
    let result = cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .clone();

    // Should have descriptive error
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("config") || stderr.contains("parse") || stderr.contains("error"),
        "Error message should mention config or parsing issue, got: {}",
        stderr
    );
}

/// Malformed TOML on set also produces descriptive error
#[test]
fn test_config_malformed_toml_on_set() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Write malformed TOML
    let config_path = dir.path().join(".werk").join("config.toml");
    std::fs::write(&config_path, "invalid = [unclosed").unwrap();

    // Attempt to set config should fail gracefully (no panic)
    let result = cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo test")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .clone();

    // Should have descriptive error
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("config") || stderr.contains("parse") || stderr.contains("error"),
        "Error message should mention config or parsing issue, got: {}",
        stderr
    );
}

// =============================================================================
// Additional edge cases
// =============================================================================

/// Config set with special characters in value
#[test]
fn test_config_set_special_characters() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set value with special characters
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo 'hello world' --flag")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get should return exact value
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo 'hello world' --flag"));
}

/// Empty key is rejected
#[test]
fn test_config_empty_key_rejected() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Empty key should fail
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("")
        .arg("value")
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// Key without dot notation (top-level key)
#[test]
fn test_config_top_level_key() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set a top-level key
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("simple_key")
        .arg("simple_value")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get should work
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("simple_key")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("simple_value"));
}

/// Deeply nested key
#[test]
fn test_config_deeply_nested_key() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set a deeply nested key
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("level1.level2.level3.value")
        .arg("deep_value")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get should work
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("level1.level2.level3.value")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("deep_value"));
}

/// Exit code 0 on success
#[test]
fn test_config_exit_code_success() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set should succeed with exit 0
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo test")
        .current_dir(dir.path())
        .assert()
        .success();
}

/// Exit code 1 on user error
#[test]
fn test_config_exit_code_user_error() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get missing key should fail with exit 1
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("nonexistent.key")
        .current_dir(dir.path())
        .assert()
        .failure()
        .code(predicate::eq(1));
}
