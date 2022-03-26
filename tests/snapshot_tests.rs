use std::fs::{copy, read_to_string};
use std::path::Path;

use snapbox::assert_eq_path;
use snapbox::cmd::{cargo_bin, Command};

use git_repo_helpers::*;

mod git_repo_helpers;

/// Run `--generate` on a repo with no remote.
#[test]
fn generate_no_remote() {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/generate_no_remote");
    init(temp_path);

    // Act
    let assert = Command::new(cargo_bin!("dobby"))
        .arg("--generate")
        .current_dir(temp_path)
        .assert();

    // Assert
    assert.success().stdout_eq("Generating a dobby.toml file\n");
    assert_eq_path(
        source_path.join("dobby.toml"),
        read_to_string(temp_path.join("dobby.toml")).unwrap(),
    );
}

/// Run `--generate` on a repo with a GitHub remote.
#[test]
fn generate_github() {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/generate_github");
    init(temp_path);
    add_remote(temp_path, "github.com/dobby-dev/dobby");

    // Act
    let assert = Command::new(cargo_bin!("dobby"))
        .arg("--generate")
        .current_dir(temp_path)
        .assert();

    // Assert
    assert
        .success()
        .stdout_eq("Generating a dobby.toml file\n")
        .stderr_eq("");
    assert_eq_path(
        source_path.join("dobby.toml"),
        read_to_string(temp_path.join("dobby.toml")).unwrap(),
    );
}

/// Run `--validate` with a config file that has lots of problems.
#[test]
fn test_validate() {
    let assert = Command::new(cargo_bin!("dobby"))
        .arg("--validate")
        .current_dir("tests/validate")
        .assert();
    assert.failure().stderr_eq_path("tests/validate/output.txt");
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release.
#[test]
fn prerelease_after_release() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prerelease_after_release");

    init(temp_path);
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    for file in ["dobby.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let assert = Command::new(cargo_bin!("dobby"))
        .arg("prerelease")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("dobby"))
        .arg("prerelease")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    assert
        .success()
        .stdout_eq_path(source_path.join("output.txt"));
    dry_run_assert
        .success()
        .stdout_eq_path(source_path.join("dry_run_output.txt"));

    assert_eq_path(
        source_path.join("EXPECTED_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert_eq_path(
        source_path.join("Expected_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a pre-release.
#[test]
fn second_prerelease() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/second_prerelease");

    init(temp_path);
    commit(temp_path, "feat: New feature in first RC");
    tag(temp_path, "1.1.0-rc.1");
    commit(temp_path, "feat: New feature in second RC");

    for file in ["dobby.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("dobby"))
        .arg("prerelease")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("dobby"))
        .arg("prerelease")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .stdout_eq_path(source_path.join("dry_run_output.txt"));
    actual_assert
        .success()
        .stdout_eq_path(source_path.join("output.txt"));
    assert_eq_path(
        source_path.join("EXPECTED_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert_eq_path(
        source_path.join("Expected_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}
