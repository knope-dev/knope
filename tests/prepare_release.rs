use std::fs::{copy, read_to_string, write};
use std::path::Path;

use rstest::rstest;
use snapbox::assert_eq_path;
use snapbox::cmd::{cargo_bin, Command};

use git_repo_helpers::*;

mod git_repo_helpers;

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release.
#[test]
fn prerelease_after_release() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/prerelease_after_release");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
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

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but change
/// the configured `prerelease_label` at runtime using the `--prerelease-label` argument.
#[test]
fn override_prerelease_label_with_option() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/override_prerelease_label");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .arg("--prerelease-label=alpha")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .arg("--prerelease-label=alpha")
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

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but change
/// the configured `prerelease_label` at runtime using the `KNOPE_PRERELEASE_LABEL` environment variable.
#[test]
fn override_prerelease_label_with_env() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/override_prerelease_label");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .env("KNOPE_PRERELEASE_LABEL", "alpha")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .env("KNOPE_PRERELEASE_LABEL", "alpha")
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

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using the `--prerelease-label` argument.
#[test]
fn enable_prerelease_label_with_option() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/enable_prerelease");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .arg("--prerelease-label=rc")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .arg("--prerelease-label=rc")
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

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using the `KNOPE_PRERELEASE_LABEL` environment variable.
#[test]
fn enable_prerelease_label_with_env() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/enable_prerelease");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .env("KNOPE_PRERELEASE_LABEL", "rc")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .env("KNOPE_PRERELEASE_LABEL", "rc")
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

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using both the `--prerelease-label` argument and the
/// `KNOPE_PRERELEASE_LABEL` environment variable.
///
/// The `--prerelease-label` argument should take precedence over the environment variable.
#[test]
fn prerelease_label_option_overrides_env() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/enable_prerelease");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .env("KNOPE_PRERELEASE_LABEL", "alpha")
        .arg("--prerelease-label=rc")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .env("KNOPE_PRERELEASE_LABEL", "alpha")
        .arg("--prerelease-label=rc")
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
    let source_path = Path::new("tests/prepare_release/second_prerelease");

    init(temp_path);
    commit(temp_path, "feat: New feature in first RC");
    tag(temp_path, "v1.0.0");
    tag(temp_path, "v1.1.0-rc.1");
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
#[case("go.mod_knope.toml", &["go.mod"])]
#[case("multiple_files_in_package_knope.toml", &["Cargo.toml", "pyproject.toml"])]
fn prepare_release_selects_files(#[case] knope_toml: &str, #[case] versioned_files: &[&str]) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    copy(source_path.join(knope_toml), temp_path.join("knope.toml")).unwrap();
    for file in [
        "CHANGELOG.md",
        "Cargo.toml",
        "go.mod",
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

    for file in ["Cargo.toml", "pyproject.toml", "package.json", "go.mod"] {
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

/// Snapshot the error messages when a required file is missing.
#[rstest]
#[case("Cargo.toml_knope.toml")]
#[case("pyproject.toml_knope.toml")]
#[case("package.json_knope.toml")]
#[case("go.mod_knope.toml")]
#[case("multiple_files_in_package_knope.toml")]
fn prepare_release_versioned_file_not_found(#[case] knope_toml: &str) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    copy(source_path.join(knope_toml), temp_path.join("knope.toml")).unwrap();
    let file = "CHANGELOG.md";
    copy(source_path.join(file), temp_path.join(file)).unwrap();

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
        .stderr_eq_path(source_path.join(&format!("{knope_toml}_MISSING_output.txt")));
    actual_assert
        .failure()
        .stderr_eq_path(source_path.join(&format!("{knope_toml}_MISSING_output.txt")));
    assert_eq_path(
        source_path.join("CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
}

/// Run a `PrepareRelease` in a repo where the versioned files are invalid.
#[rstest]
#[case("Cargo.toml_knope.toml")]
#[case("pyproject.toml_knope.toml")]
#[case("package.json_knope.toml")]
#[case("multiple_files_in_package_knope.toml")]
fn prepare_release_invalid_versioned_files(#[case] knope_toml: &str) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    copy(source_path.join(knope_toml), temp_path.join("knope.toml")).unwrap();
    copy(
        source_path.join("CHANGELOG.md"),
        temp_path.join("CHANGELOG.md"),
    )
    .unwrap();
    for file in ["Cargo.toml", "go.mod", "pyproject.toml", "package.json"] {
        write(temp_path.join(file), "").unwrap();
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
        .stderr_eq_path(source_path.join(&format!("{knope_toml}_INVALID_output.txt")));
    actual_assert
        .failure()
        .stderr_eq_path(source_path.join(&format!("{knope_toml}_INVALID_output.txt")));
}

/// Run a `PrepareRelease` where the CHANGELOG.md file is missing and verify it's created.
#[test]
fn prepare_release_creates_missing_changelog() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/package_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    copy(
        source_path.join("Cargo.toml_knope.toml"),
        temp_path.join("knope.toml"),
    )
    .unwrap();
    let file = "Cargo.toml";
    copy(source_path.join(file), temp_path.join(file)).unwrap();

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
        source_path.join("NEW_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert_eq_path(
        source_path.join("expected_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

/// Run a `PrepareRelease` in a repo with multiple files that have different versions
#[test]
fn test_prepare_release_multiple_files_inconsistent_versions() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/package_selection");

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
    let source_path = Path::new("tests/prepare_release/package_selection");

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
    let source_path = Path::new("tests/prepare_release/changelog_selection");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
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

/// If `PrepareRelease` is run with no `versioned_files`, it should determine the version from the
/// previous valid tag.
#[test]
fn no_versioned_files() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/no_versioned_files");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();
    copy(
        source_path.join("CHANGELOG.md"),
        temp_path.join("CHANGELOG.md"),
    )
    .unwrap();

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

    // The release step should have created a tag with the right new version.
    let expected_tag = "v1.1.0";
    let actual_tag = describe(temp_path);
    assert_eq!(expected_tag, actual_tag);
}

/// If `PrepareRelease` is run with no `prerelease_label`, it should skip any prerelease tags
/// when parsing commits, as well as determine the next version from the previous released version
/// (not from the pre-release version).
#[test]
fn release_after_prerelease() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/release_after_prerelease");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0"); // Here is the last released version
    commit(temp_path, "feat!: Breaking change");
    commit(temp_path, "feat: New feature");
    // Here is the pre-release version, intentionally wrong to test that all the commits are re-parsed
    tag(temp_path, "v1.1.0-rc.1");

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
        .stdout_eq_path(source_path.join("dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    assert_eq_path(
        source_path.join("EXPECTED_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert_eq_path(
        source_path.join("Expected_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

/// Go modules have a peculiar way of versioning in that only the major version is recorded to the
/// `go.mod` file and only for major versions >1. This tests that.
#[test]
fn go_modules() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/go_modules");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    for file in ["knope.toml", "CHANGELOG.md", "go.mod"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act 1—version stays at 1.x
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert 1—version stays at 1.x
    dry_run_assert
        .success()
        .stdout_eq_path(source_path.join("1.1_dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    assert_eq_path(
        source_path.join("EXPECTED_1.1_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert_eq_path(
        source_path.join("EXPECTED_1.1_go.mod"),
        read_to_string(temp_path.join("go.mod")).unwrap(),
    );
    let tag = describe(temp_path);
    assert_eq!("v1.1.0", tag);

    // Arrange 2—version goes to 2.0
    commit(temp_path, "feat!: Breaking change");

    // Act 2—version goes to 2.0
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert 2—version goes to 2.0
    dry_run_assert
        .success()
        .stdout_eq_path(source_path.join("2.0_dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    assert_eq_path(
        source_path.join("EXPECTED_2.0_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert_eq_path(
        source_path.join("EXPECTED_2.0_go.mod"),
        read_to_string(temp_path.join("go.mod")).unwrap(),
    );
    let tag = describe(temp_path);
    assert_eq!("v2.0.0", tag);
}
