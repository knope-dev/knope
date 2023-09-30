//! Test the default workflows that should work when no `knope.toml` exists.

use std::{fs::copy, path::Path};

use helpers::*;
use rstest::rstest;
use snapbox::cmd::{cargo_bin, Command};

mod helpers;

/// Run `knope release --dry-run` on a repo with no remote.
#[test]
fn release_no_remote() {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/no_config/no_remote");
    setup_commits(temp_path);
    copy(source_path.join("Cargo.toml"), temp_path.join("Cargo.toml")).unwrap();

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_path)
        .with_assert(assert())
        .assert();

    // Assert
    assert
        .success()
        .stdout_matches_path(source_path.join("stdout.txt"));
}

/// Run `knope release --dry-run` on a repo with supported metadata files.
#[rstest]
#[case(&["Cargo.toml"], "Cargo")]
#[case(&["pyproject.toml"], "pyproject")]
#[case(&["package.json"], "package")]
#[case(&["go.mod"], "go")]
#[case(&["Cargo.toml", "pyproject.toml", "package.json"], "multiple")]
fn test_packages(#[case] source_files: &[&str], #[case] case: &str) {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/no_config/packages");
    setup_commits(temp_path);

    for source_file in source_files {
        copy(source_path.join(source_file), temp_path.join(source_file)).unwrap();
    }
    copy(
        source_path.join("CHANGELOG.md"),
        temp_path.join("CHANGELOG.md"),
    )
    .unwrap();

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .with_assert(assert())
        .current_dir(temp_path)
        .assert();

    // Assert
    assert
        .success()
        .stdout_matches_path(source_path.join(format!("{case}_stdout.txt")));
}

/// Run `knope release --dry-run` on a repo with a GitHub remote to test that integration.
#[rstest]
#[case::ssh("git@github.com:knope-dev/knope.git")]
#[case::https("https://github.com/knope-dev/knope.git")]
fn generate_github(#[case] remote: &str) {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/no_config/github");
    setup_commits(temp_path);
    add_remote(temp_path, remote);
    copy(source_path.join("Cargo.toml"), temp_path.join("Cargo.toml")).unwrap();

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .with_assert(assert())
        .current_dir(temp_path)
        .assert();

    // Assert
    assert
        .success()
        .stdout_matches_path(source_path.join("stdout.txt"));
}

fn setup_commits(path: &Path) {
    init(path);
    commit(path, "feat: Existing Feature");
    tag(path, "v1.0.0");
    commit(path, "feat: Something");
}
