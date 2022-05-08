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
    let source_path = Path::new("tests/generate_no_remote");
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
#[case(&["Cargo.toml", "pyproject.toml", "package.json"], "Cargo.toml_knope.toml")]
fn generate_packages(#[case] source_files: &[&str], #[case] target_file: &str) {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/generate_packages");
    init(temp_path);
    commit(temp_path, "feat: Existing Feature");
    tag(temp_path, "v1.0.0");
    copy(source_path.join("no_package_knope.toml"), temp_path.join("knope.toml")).unwrap();
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
    validate_assert.failure().stderr_eq_path(source_path.join(format!("{case}_stderr.txt", case = target_file)));
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
    let source_path = Path::new("tests/generate_package_changelog");
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
#[test]
fn generate_github() {
    // Arrange
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/generate_github");
    init(temp_path);
    add_remote(temp_path, "github.com/knope-dev/knope");

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

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("knope"))
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

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
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

/// Run a `PrepareRelease` in a repo with multiple versionable files—verify only the selected
/// one is modified.
#[rstest]
#[case("Cargo.toml")]
#[case("pyproject.toml")]
#[case("package.json")]
fn prepare_release_selects_files(#[case] versioned_file: &str) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release_package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "1.0.0");
    commit(temp_path, "feat: New feature");

    let knope_toml = format!("{versioned_file}_knope.toml");
    copy(source_path.join(&knope_toml), temp_path.join("knope.toml")).unwrap();
    for file in [
        "CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
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
        .stdout_eq_path(source_path.join("dry_run_output.txt"));
    actual_assert
        .success()
        .stdout_eq_path(source_path.join("output.txt"));
    assert_eq_path(
        source_path.join("EXPECTED_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );

    assert_eq_path(
        source_path.join(&format!("expected_{versioned_file}")),
        read_to_string(temp_path.join(versioned_file)).unwrap(),
    );
    for file in ["Cargo.toml", "pyproject.toml", "package.json"] {
        if file == versioned_file {
            // This one should actually have changed
            continue;
        }
        assert_eq_path(
            source_path.join(file),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// Run a `PrepareRelease` in a repo and verify that the changelog is updated based on config.
#[rstest]
#[case(Some("CHANGELOG.md"))]
#[case(Some("CHANGES.md"))]  // A non-default name
#[case(None)]
fn prepare_release_changelog_selection(#[case] changelog: Option<&str>) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release_changelog_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "1.0.0");
    commit(temp_path, "feat: New feature");
    let all_changelogs = ["CHANGELOG.md", "CHANGES.md"];

    for file in all_changelogs {
        copy(source_path.join("CHANGELOG.md"), temp_path.join(file)).unwrap();
    }
    if let Some(changelog_name) = changelog {
        copy(source_path.join(format!("{changelog_name}_knope.toml")), temp_path.join("knope.toml")).unwrap();
    } else {
        copy(source_path.join("None_knope.toml"), temp_path.join("knope.toml")).unwrap();
    }
    copy(source_path.join("Cargo.toml"), temp_path.join("Cargo.toml")).unwrap();

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
        .stdout_eq_path(source_path.join(format!("dry_run_output_{changelog:?}.txt")));
    actual_assert
        .success()
        .stdout_eq_path(source_path.join("output.txt"));

    for changelog_name in all_changelogs {
        match changelog {
            Some(changelog) if changelog_name == changelog => {
                assert_eq_path(
                    source_path.join("EXPECTED_CHANGELOG.md"),
                    read_to_string(temp_path.join(changelog_name)).unwrap(),
                );
            },
            _ => {
                assert_eq_path(
                    source_path.join("CHANGELOG.md"),
                    read_to_string(temp_path.join(changelog_name)).unwrap(),
                );
            }
        }
    }
    assert_eq_path(
        source_path.join("expected_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}
