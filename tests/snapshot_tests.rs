use std::fs::{copy, read_to_string};
use std::path::Path;

use rstest::rstest;
use snapbox::assert_eq_path;
use snapbox::cmd::{cargo_bin, Command};

use git_repo_helpers::*;

mod git_repo_helpers;

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
#[case("Cargo.toml_knope.toml", &["Cargo.toml"])]
#[case("pyproject.toml_knope.toml", &["pyproject.toml"])]
#[case("package.json_knope.toml", &["package.json"])]
#[case("multiple_files_in_package_knope.toml", &["Cargo.toml", "pyproject.toml"])]
fn prepare_release_selects_files(#[case] knope_toml: &str, #[case] versioned_files: &[&str]) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release_package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "1.0.0");
    commit(temp_path, "feat: New feature");

    copy(source_path.join(knope_toml), temp_path.join("knope.toml")).unwrap();
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

    for file in ["Cargo.toml", "pyproject.toml", "package.json"] {
        let expected_path = if versioned_files.contains(&file) {
            format!("expected_{file}")
        } else {
            String::from(file)
        };
        assert_eq_path(
            source_path.join(&expected_path),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// Run a `PrepareRelease` in a repo with multiple packages set to verify error message.
#[test]
fn test_prepare_release_multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release_package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "1.0.0");
    commit(temp_path, "feat: New feature");

    let knope_toml = "multiple_packages_knope.toml";
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
        .failure()
        .stderr_eq_path(source_path.join("multiple_packages_output.txt"));
    actual_assert
        .failure()
        .stderr_eq_path(source_path.join("multiple_packages_output.txt"));

    // Nothing should change because it errored.
    assert_eq_path(
        source_path.join("CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    for file in ["Cargo.toml", "pyproject.toml", "package.json"] {
        assert_eq_path(
            source_path.join(file),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// Run a `PrepareRelease` in a repo with multiple files that have different versions
#[test]
fn test_prepare_release_multiple_files_inconsistent_versions() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release_package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "1.0.0");
    commit(temp_path, "feat: New feature");

    let knope_toml = "multiple_files_in_package_knope.toml";
    copy(source_path.join(&knope_toml), temp_path.join("knope.toml")).unwrap();
    copy(
        source_path.join("Cargo_different_version.toml"),
        temp_path.join("Cargo.toml"),
    )
    .unwrap();
    for file in ["CHANGELOG.md", "pyproject.toml", "package.json"] {
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
    dry_run_assert.failure().stderr_eq_path(
        source_path.join("test_prepare_release_multiple_files_inconsistent_versions.txt"),
    );
    actual_assert.failure().stderr_eq_path(
        source_path.join("test_prepare_release_multiple_files_inconsistent_versions.txt"),
    );

    // Nothing should change because it errored.
    assert_eq_path(
        source_path.join("Cargo_different_version.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
    for file in ["pyproject.toml", "package.json", "CHANGELOG.md"] {
        assert_eq_path(
            source_path.join(file),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// Run a `PrepareRelease` where the configured `versioned_file` is not a supported format
#[test]
fn test_prepare_release_invalid_versioned_file_format() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release_package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "1.0.0");
    commit(temp_path, "feat: New feature");

    let knope_toml = "invalid_versioned_file_format_knope.toml";
    copy(source_path.join(&knope_toml), temp_path.join("knope.toml")).unwrap();
    for file in [
        "CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
        "setup.py",
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
        .failure()
        .stderr_eq_path(source_path.join("invalid_versioned_file_format_knope_output.txt"));
    actual_assert
        .failure()
        .stderr_eq_path(source_path.join("invalid_versioned_file_format_knope_output.txt"));

    // Nothing should change because it errored.
    assert_eq_path(
        source_path.join("CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    for file in ["Cargo.toml", "pyproject.toml", "package.json"] {
        assert_eq_path(
            source_path.join(file),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// Run a `PrepareRelease` in a repo and verify that the changelog is updated based on config.
#[rstest]
#[case(Some("CHANGELOG.md"))]
#[case(Some("CHANGES.md"))] // A non-default name
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
        copy(
            source_path.join(format!("{changelog_name}_knope.toml")),
            temp_path.join("knope.toml"),
        )
        .unwrap();
    } else {
        copy(
            source_path.join("None_knope.toml"),
            temp_path.join("knope.toml"),
        )
        .unwrap();
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
    let expected_dry_run_output = if let Some(changelog_name) = changelog {
        source_path.join(format!("dry_run_output_{changelog_name}.txt"))
    } else {
        source_path.join("dry_run_output_None.txt")
    };
    dry_run_assert
        .success()
        .stdout_eq_path(expected_dry_run_output);
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
            }
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
