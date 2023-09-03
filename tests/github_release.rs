use std::{
    fs::{copy, create_dir, write},
    path::Path,
};

use helpers::*;
use snapbox::cmd::{cargo_bin, Command};

mod helpers;

/// Run a `PreRelease` then `Release` for a repo not configured for GitHub.
///
/// # Expected
///
/// Version should be bumped, and a new tag should be added to the repo.
#[test]
fn github_release() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act. Cannot run real release without integration testing GitHub.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
}

/// Verify that Release will operate on all defined packages independently
#[test]
fn multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/multiple_packages");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "first/v1.2.3");
    tag(temp_path, "second/v0.4.6");
    commit(temp_path, "feat!: New breaking feature");

    for file in [
        "knope.toml",
        "FIRST_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
        "SECOND_CHANGELOG.md",
        "package.json",
    ] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
}

#[test]
fn separate_prepare_and_release_workflows() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/separate_prepare_and_release_workflows");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");
    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Run the actual release (but dry-run because don't test GitHub)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
}

#[test]
fn release_assets() {
    // Arrange a package that's ready to release with some artifacts
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/release_assets");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");
    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    let assets_dir = temp_path.join("assets");
    create_dir(&assets_dir).unwrap();
    write(assets_dir.join("first"), "first").unwrap();
    write(assets_dir.join("second"), "second").unwrap();

    // Actual release
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
}

#[test]
fn auto_generate_release_notes() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/separate_prepare_and_release_workflows");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");
    for file in ["knope.toml", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Run the actual release (but dry-run because don't test GitHub)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("auto_generate_dry_run_output.txt"));
}
