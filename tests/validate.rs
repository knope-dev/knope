use std::{fs::copy, path::Path};

use helpers::*;
use snapbox::cmd::{cargo_bin, Command};

mod helpers;

/// Run `--validate` with a config file that has lots of problems.
#[test]
fn test_validate() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/validate");
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "1.0.0");
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();

    let assert = Command::new(cargo_bin!("knope"))
        .arg("--validate")
        .current_dir(temp_path)
        .assert();
    assert.failure().stderr_eq_path("tests/validate/output.txt");
}

/// Run `--validate` with a config file that has the old packages format.
#[test]
fn validate_old_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/validate");
    copy(
        source_path.join("old_package_syntax.toml"),
        temp_path.join("knope.toml"),
    )
    .unwrap();

    let assert = Command::new(cargo_bin!("knope"))
        .arg("--validate")
        .current_dir(temp_path)
        .assert();
    assert
        .success()
        .stdout_matches_path("tests/validate/old_package_syntax.txt");
}

/// Run `--validate` with a config file that has both package configs—which is a conflict.
#[test]
fn validate_conflicting_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/validate");
    copy(
        source_path.join("multiple_package_formats.toml"),
        temp_path.join("knope.toml"),
    )
    .unwrap();

    let assert = Command::new(cargo_bin!("knope"))
        .arg("--validate")
        .current_dir(temp_path)
        .assert();
    assert
        .failure()
        .stderr_eq_path("tests/validate/multiple_package_formats.txt");
}
