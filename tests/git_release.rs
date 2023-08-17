use std::{
    fs::{copy, read_to_string},
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
fn git_release() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/git_release");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
    actual_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("output.txt"));
    assert().matches_path(
        source_path.join("EXPECTED_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert().matches_path(
        source_path.join("Expected_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
    let tags = get_tags(temp_path);
    assert_eq!(tags, vec!["v1.1.0"]);
}

/// Verify that Release will operate on all defined packages independently
#[test]
fn multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/git_release/multiple_packages");

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
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
    actual_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("output.txt"));

    for file in [
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
    ] {
        assert().matches_path(
            source_path.join(format!("EXPECTED_{file}")),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
    assert_eq!(get_tags(temp_path), vec!["first/v2.0.0", "second/v0.5.0"]);
}
