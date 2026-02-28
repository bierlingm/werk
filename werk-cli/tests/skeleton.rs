//! Integration tests for the CLI skeleton.
//!
//! Tests verify that:
//! - `werk` (no args) prints help
//! - `werk --help` shows global flags
//! - `werk --version` prints version
//! - All subcommands are listed in help

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_werk_no_args_shows_help() {
    let mut cmd = Command::cargo_bin("werk").unwrap();
    cmd.assert().failure().stderr(predicate::str::contains(
        "Operative instrument for structural dynamics",
    ));
}

#[test]
fn test_werk_help_shows_global_flags() {
    let mut cmd = Command::cargo_bin("werk").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--no-color"));
}

#[test]
fn test_werk_version() {
    let mut cmd = Command::cargo_bin("werk").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("werk"));
}

#[test]
fn test_werk_help_lists_all_subcommands() {
    let mut cmd = Command::cargo_bin("werk").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        // Foundation commands
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("reality"))
        .stdout(predicate::str::contains("desire"))
        .stdout(predicate::str::contains("resolve"))
        .stdout(predicate::str::contains("release"))
        .stdout(predicate::str::contains("rm"))
        .stdout(predicate::str::contains("mv"))
        .stdout(predicate::str::contains("note"))
        // Display commands
        .stdout(predicate::str::contains("tree"))
        // Agent commands
        .stdout(predicate::str::contains("context"))
        .stdout(predicate::str::contains("run"));
}

#[test]
fn test_werk_init_is_implemented() {
    let mut cmd = Command::cargo_bin("werk").unwrap();
    cmd.arg("init");
    cmd.assert().success().stdout(
        predicate::str::contains("Workspace initialized")
            .or(predicate::str::contains("already initialized")),
    );
}

#[test]
fn test_werk_json_flag() {
    let mut cmd = Command::cargo_bin("werk").unwrap();
    cmd.arg("--json").arg("init");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"path\""));
}

#[test]
fn test_werk_no_color_flag() {
    let mut cmd = Command::cargo_bin("werk").unwrap();
    cmd.arg("--no-color").arg("init");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Workspace"));
}
