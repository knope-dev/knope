//! Test the `--upgrade` option.

use std::{
    fs::{copy, read_to_string},
    path::Path,
};

use snapbox::{
    assert_eq_path,
    cmd::{cargo_bin, Command},
};

/// Test upgrading the deprecated `[[packages]]` section to the new `[package]` section.
#[test]
fn upgrade_packages() {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/upgrade/packages");
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("--upgrade")
        .current_dir(temp_path)
        .assert();

    // Assert
    assert
        .success()
        .stdout_eq("Upgrading deprecated [[packages]] syntax to [package]\n");
    assert_eq_path(
        source_path.join("expected_knope.toml"),
        read_to_string(temp_path.join("knope.toml")).unwrap(),
    );
}

/// Test running `--upgrade` when there is nothing to upgrade
#[test]
fn upgrade_nothing() {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/upgrade/nothing");
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("--upgrade")
        .current_dir(temp_path)
        .assert();

    // Assert
    assert.success().stdout_eq("Nothing to upgrade\n");
    assert_eq_path(
        source_path.join("expected_knope.toml"),
        read_to_string(temp_path.join("knope.toml")).unwrap(),
    );
}
