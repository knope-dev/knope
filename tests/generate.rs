use std::fs::{copy, read_to_string, remove_file, write};
use std::path::Path;

use rstest::rstest;
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
    let source_path = Path::new("tests/generate/no_remote");
    init(temp_path);

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("--generate")
        .current_dir(temp_path)
        .assert();

    // Assert
    assert.success().stdout_eq("Generating a knope.toml file\n");
    assert_eq_path(
        source_path.join("knope.toml"),
        read_to_string(temp_path.join("knope.toml")).unwrap(),
    );
}

/// Run `--generate` on a repo with supported metadata files.
#[rstest]
#[case(&["Cargo.toml"], "Cargo.toml_knope.toml")]
#[case(&["pyproject.toml"], "pyproject.toml_knope.toml")]
#[case(&["package.json"], "package.json_knope.toml")]
#[case(&["Cargo.toml", "pyproject.toml", "package.json"], "multiple_knope.toml")]
fn generate_packages(#[case] source_files: &[&str], #[case] target_file: &str) {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/generate/packages");
    init(temp_path);
    commit(temp_path, "feat: Existing Feature");
    tag(temp_path, "v1.0.0");
    copy(
        source_path.join("no_package_knope.toml"),
        temp_path.join("knope.toml"),
    )
    .unwrap();
    for source_file in source_files {
        copy(source_path.join(source_file), temp_path.join(source_file)).unwrap();
    }

    // Act
    // Validate should give a useful error message similar to generate
    let validate_assert = Command::new(cargo_bin!("knope"))
        .arg("--validate")
        .current_dir(temp_path)
        .assert();
    remove_file(temp_path.join("knope.toml")).unwrap();
    let assert = Command::new(cargo_bin!("knope"))
        .arg("--generate")
        .current_dir(temp_path)
        .assert();

    // Assert
    validate_assert
        .failure()
        .stderr_eq_path(source_path.join(format!("{case}_stderr.txt", case = target_file)));
    assert.success().stdout_eq("Generating a knope.toml file\n");
    assert_eq_path(
        source_path.join(target_file),
        read_to_string(temp_path.join("knope.toml")).unwrap(),
    );
}

/// Run `--generate` on a repo with supported metadata and optional CHANGELOG.md.
#[rstest]
#[case(true, "changelog_knope.toml")]
#[case(false, "no_changelog_knope.toml")]
fn generate_packages_changelog(#[case] has_changelog: bool, #[case] target_file: &str) {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/generate/package_changelog");
    init(temp_path);
    copy(source_path.join("Cargo.toml"), temp_path.join("Cargo.toml")).unwrap();
    if has_changelog {
        write(temp_path.join("CHANGELOG.md"), "").unwrap();
    }

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("--generate")
        .current_dir(temp_path)
        .assert();

    // Assert
    assert.success().stdout_eq("Generating a knope.toml file\n");
    assert_eq_path(
        source_path.join(target_file),
        read_to_string(temp_path.join("knope.toml")).unwrap(),
    );
}

/// Run `--generate` on a repo with a GitHub remote.
#[rstest]
#[case::ssh("git@github.com:knope-dev/knope.git")]
#[case::https("https://github.com/knope-dev/knope.git")]
fn generate_github(#[case] remote: &str) {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/generate/github");
    init(temp_path);
    add_remote(temp_path, remote);

    // Act
    let assert = Command::new(cargo_bin!("knope"))
        .arg("--generate")
        .current_dir(temp_path)
        .assert();

    // Assert
    assert
        .success()
        .stdout_eq("Generating a knope.toml file\n")
        .stderr_eq("");
    assert_eq_path(
        source_path.join("knope.toml"),
        read_to_string(temp_path.join("knope.toml")).unwrap(),
    );
}
