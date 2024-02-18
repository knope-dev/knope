use std::{
    fs::{copy, create_dir, read_to_string, write},
    path::Path,
    thread::sleep,
    time::Duration,
};

use changesets::{Change, ChangeType, UniqueId, Versioning};
use helpers::*;
use pretty_assertions::assert_eq;
use rstest::rstest;
use snapbox::{
    assert_eq,
    cmd::{cargo_bin, Command},
    Data,
};

mod helpers;

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release.
#[test]
fn prerelease_after_release() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/prepare_release/prerelease_after_release");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    copy_dir_contents(&data_path.join("source"), temp_path);

    // Act.
    let output_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .current_dir(temp_dir.path())
        .assert();
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("prerelease")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    output_assert
        .success()
        .stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));

    assert().subset_matches(data_path.join("expected"), temp_path);
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but change
/// the configured `prerelease_label` at runtime using the `--prerelease-label` argument.
#[test]
fn override_prerelease_label_with_option() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/prepare_release/override_prerelease_label");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    copy_dir_contents(&data_path.join("source"), temp_path);

    // Act.
    let output_assert = Command::new(cargo_bin!("knope"))
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
    output_assert
        .success()
        .stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));

    assert().subset_matches(data_path.join("expected"), temp_path);
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but change
/// the configured `prerelease_label` at runtime using the `KNOPE_PRERELEASE_LABEL` environment variable.
#[test]
fn override_prerelease_label_with_env() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/prepare_release/override_prerelease_label");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    copy_dir_contents(&data_path.join("source"), temp_path);

    // Act.
    let output_assert = Command::new(cargo_bin!("knope"))
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
    output_assert
        .success()
        .stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));

    assert().subset_matches(data_path.join("expected"), temp_path);
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using the `--prerelease-label` argument.
#[test]
fn enable_prerelease_label_with_option() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/prepare_release/enable_prerelease");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    copy_dir_contents(&data_path.join("source"), temp_path);

    // Act.
    let output_assert = Command::new(cargo_bin!("knope"))
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
    output_assert
        .success()
        .stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));

    assert().subset_matches(data_path.join("expected"), temp_path);
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using the `KNOPE_PRERELEASE_LABEL` environment variable.
#[test]
fn enable_prerelease_label_with_env() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/prepare_release/enable_prerelease");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    copy_dir_contents(&data_path.join("source"), temp_path);

    // Act.
    let output_assert = Command::new(cargo_bin!("knope"))
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
    output_assert
        .success()
        .stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));

    assert().subset_matches(data_path.join("expected"), temp_path);
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
    let data_path = Path::new("tests/prepare_release/enable_prerelease");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature in existing release");
    tag(temp_path, "v1.1.0");
    commit(temp_path, "feat!: Breaking feature in new RC");

    copy_dir_contents(&data_path.join("source"), temp_path);

    // Act.
    let output_assert = Command::new(cargo_bin!("knope"))
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
    output_assert
        .success()
        .stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));

    assert().subset_matches(data_path.join("expected"), temp_path);
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a pre-release.
#[test]
fn second_prerelease() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/prepare_release/second_prerelease");

    init(temp_path);
    commit(temp_path, "An old prerelease which should not be checked");
    tag(temp_path, "v1.1.0-rc.2");
    commit(temp_path, "feat: New feature in first RC");
    tag(temp_path, "v1.0.0");
    tag(temp_path, "v1.1.0-rc.1");
    commit(temp_path, "feat: New feature in second RC");

    copy_dir_contents(&data_path.join("source"), temp_path);

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
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));
    actual_assert
        .success()
        .stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
    assert().subset_matches(data_path.join("expected"), temp_path);
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

    add_all(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

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
        .stdout_matches(Data::read_from(
            &source_path.join(format!("{knope_toml}_dry_run_output.txt")),
            None,
        ));
    actual_assert
        .success()
        .stdout_matches(Data::read_from(&source_path.join("output.txt"), None));
    assert().matches(
        Data::read_from(&source_path.join("EXPECTED_CHANGELOG.md"), None),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );

    for file in ["Cargo.toml", "pyproject.toml", "package.json", "go.mod"] {
        let expected_path = if versioned_files.contains(&file) {
            format!("expected_{file}")
        } else {
            String::from(file)
        };
        assert().matches(
            Data::read_from(&source_path.join(expected_path), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
    let mut expected_changes = Vec::with_capacity(versioned_files.len() + 1);
    for file in versioned_files {
        expected_changes.push(format!("M  {file}"));
    }
    expected_changes.push("M  CHANGELOG.md".to_string());
    expected_changes.sort();
    assert_eq!(
        status(temp_path),
        expected_changes,
        "All modified changes should be added to Git"
    );
}

/// Run a `PrepareRelease` against all supported types of `pyproject.toml` files.
#[rstest]
#[case::poetry("poetry_pyproject.toml")]
#[case::pep621("pep621_pyproject.toml")]
#[case::mixed("mixed_pyproject.toml")]
fn prepare_release_pyproject_toml(#[case] input_file: &str) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/pyproject_toml");

    init(temp_path);
    copy(
        source_path.join(input_file),
        temp_path.join("pyproject.toml"),
    )
    .unwrap();
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();
    add_all(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat!: New feature");

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
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");
    assert().matches(
        Data::read_from(
            &source_path.join(format!("expected_{input_file}.toml")),
            None,
        ),
        read_to_string(temp_path.join("pyproject.toml")).unwrap(),
    );

    let expected_changes = ["M  pyproject.toml"];
    assert_eq!(
        status(temp_path),
        expected_changes,
        "All modified changes should be added to Git"
    );
}

#[test]
fn prepare_release_pubspec_yaml() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/pubspec_yaml");

    init(temp_path);
    copy(
        source_path.join("pubspec.yaml"),
        temp_path.join("pubspec.yaml"),
    )
    .unwrap();
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();
    add_all(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat!: New feature");

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
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");
    assert().matches(
        Data::read_from(&source_path.join("expected_pubspec.yaml"), None),
        read_to_string(temp_path.join("pubspec.yaml")).unwrap(),
    );

    let expected_changes = ["M  pubspec.yaml"];
    assert_eq!(
        status(temp_path),
        expected_changes,
        "All modified changes should be added to Git"
    );
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
    dry_run_assert.failure().stderr_eq(Data::read_from(
        &source_path.join(format!("{knope_toml}_MISSING_output.txt")),
        None,
    ));
    actual_assert.failure().stderr_eq(Data::read_from(
        &source_path.join(format!("{knope_toml}_MISSING_output.txt")),
        None,
    ));
    assert().matches(
        Data::read_from(&source_path.join("CHANGELOG.md"), None),
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
    dry_run_assert.failure().stderr_eq(Data::read_from(
        &source_path.join(format!("{knope_toml}_INVALID_output.txt")),
        None,
    ));
    actual_assert.failure().stderr_eq(Data::read_from(
        &source_path.join(format!("{knope_toml}_INVALID_output.txt")),
        None,
    ));
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
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert
        .success()
        .stdout_matches(Data::read_from(&source_path.join("output.txt"), None));
    assert().matches(
        Data::read_from(&source_path.join("NEW_CHANGELOG.md"), None),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert().matches(
        Data::read_from(&source_path.join("expected_Cargo.toml"), None),
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
    copy(source_path.join(knope_toml), temp_path.join("knope.toml")).unwrap();
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
    dry_run_assert.failure().stderr_eq(Data::read_from(
        &source_path.join("test_prepare_release_multiple_files_inconsistent_versions.txt"),
        None,
    ));
    actual_assert.failure().stderr_eq(Data::read_from(
        &source_path.join("test_prepare_release_multiple_files_inconsistent_versions.txt"),
        None,
    ));

    // Nothing should change because it errored.
    assert().matches(
        Data::read_from(&source_path.join("Cargo_different_version.toml"), None),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
    for file in ["pyproject.toml", "package.json", "CHANGELOG.md"] {
        assert().matches(
            Data::read_from(&source_path.join(file), None),
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
    copy(source_path.join(knope_toml), temp_path.join("knope.toml")).unwrap();
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
    dry_run_assert.failure().stderr_eq(Data::read_from(
        &source_path.join("invalid_versioned_file_format_knope_output.txt"),
        None,
    ));
    actual_assert.failure().stderr_eq(Data::read_from(
        &source_path.join("invalid_versioned_file_format_knope_output.txt"),
        None,
    ));

    // Nothing should change because it errored.
    assert().matches(
        Data::read_from(&source_path.join("CHANGELOG.md"), None),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    for file in ["Cargo.toml", "pyproject.toml", "package.json"] {
        assert().matches(
            Data::read_from(&source_path.join(file), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
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
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert
        .success()
        .stdout_matches(Data::read_from(&source_path.join("output.txt"), None));
    assert().matches(
        Data::read_from(&source_path.join("EXPECTED_CHANGELOG.md"), None),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );

    // The release step should have created a tag with the right new version.
    let actual_tags = get_tags(temp_path);
    assert_eq!(vec!["v1.1.0"], actual_tags);
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
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");
    assert().matches(
        Data::read_from(&source_path.join("EXPECTED_CHANGELOG.md"), None),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert().matches(
        Data::read_from(&source_path.join("Expected_Cargo.toml"), None),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

/// Verify that PrepareRelease will operate on all defined packages independently
#[test]
fn multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/multiple_packages");

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
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");

    for file in [
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
    ] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// When no scopes are defined, all commits must apply to all packages
#[test]
fn no_scopes_defined() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/scopes/no_scopes");

    init(temp_path);
    commit(temp_path, "feat: No scope feature");
    commit(temp_path, "feat(scope)!: New breaking feature with a scope");

    for file in ["knope.toml", "Cargo.toml", "pyproject.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");

    for file in [
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
    ] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// When scopes are defined, commits with no scope still apply to all packages
#[test]
fn unscoped_commits_apply_to_all_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/scopes/unscoped_commits");

    init(temp_path);
    commit(temp_path, "fix(first): Fix for first only");
    commit(temp_path, "feat: No-scope feat");
    commit(temp_path, "feat(second)!: Breaking change for second only");

    for file in ["knope.toml", "Cargo.toml", "pyproject.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");

    for file in [
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
    ] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// When scopes are defined, commits with a scope apply only to packages with that scope
/// Multiple scopes can be defined per package
#[test]
fn apply_scopes() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/scopes/shared_commits");

    init(temp_path);
    commit(temp_path, "fix(first): Fix for first only");
    commit(temp_path, "feat(both): Shared feat");
    commit(temp_path, "feat(second)!: Breaking change for second only");

    for file in ["knope.toml", "Cargo.toml", "pyproject.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");

    for file in [
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
    ] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// Don't prepare releases for packages which have not changed
#[test]
fn skip_unchanged_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/scopes/skip_unchanged_packages");

    init(temp_path);
    commit(temp_path, "fix(first): Fix for first only");

    for file in ["knope.toml", "Cargo.toml", "pyproject.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");

    for file in ["FIRST_CHANGELOG.md", "Cargo.toml", "pyproject.toml"] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// Error when no commits cause a change in version
#[test]
fn no_version_change() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/no_version_change");

    init(temp_path);
    commit(temp_path, "docs: Update README");

    for file in ["knope.toml", "Cargo.toml", "CHANGELOG.md"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output.success().stderr_eq(Data::read_from(
        &source_path.join("dry_run_output.txt"),
        None,
    ));
    actual_assert.failure().stderr_eq(Data::read_from(
        &source_path.join("actual_output.txt"),
        None,
    ));
}

#[test]
fn allow_empty() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/allow_empty");

    init(temp_path);
    commit(temp_path, "docs: Update README");

    for file in ["knope.toml", "Cargo.toml", "CHANGELOG.md"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output.success().stderr_eq(Data::read_from(
        &source_path.join("dry_run_output.txt"),
        None,
    ));
    actual_assert
        .success()
        .stderr_eq(Data::read_from(&source_path.join("output.txt"), None));

    for file in ["Cargo.toml", "CHANGELOG.md"] {
        assert().matches(
            Data::read_from(&source_path.join(file), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

#[test]
fn handle_pre_versions_that_are_too_new() {
    // Arrange a folder with a knope file configured to bump versions and a file knope knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v2.0.0-rc.0"); // An earlier pre-release, but 2.0 isn't finished yet!
    tag(temp_path, "v1.2.3"); // The current stable version
    commit(temp_path, "feat: A new feature");
    tag(temp_path, "v1.3.0-rc.0"); // The current pre-release version
    commit(temp_path, "feat: Another new feature");

    let source_path = Path::new("tests/prepare_release/hande_pre_versions_that_are_too_new");
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    write(
        cargo_toml,
        "[package]\nname = \"default\"\nversion = \"1.2.3\"\n",
    )
    .unwrap();

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
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_matches(Data::read_from(
        &source_path.join("actual_output.txt"),
        None,
    ));
    assert().matches(
        Data::read_from(&source_path.join("EXPECTED_Cargo.toml"), None),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

#[test]
fn merge_commits() {
    env_logger::init();
    // Arrange a knope project with a merge commit.
    // Make a directory at a known path
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    create_branch(temp_path, "feature");
    commit(temp_path, "feat: A new feature");
    switch_branch(temp_path, "main");
    // Even if the latest tag commit is newer than the merged, the ancestors from the merge should be processed
    sleep(Duration::from_secs(1));
    commit(temp_path, "feat: existing feature");
    tag(temp_path, "v1.2.3"); // The current stable version
    merge_branch(temp_path, "feature");

    let source_path = Path::new("tests/prepare_release/merge_commits");
    for file in ["knope.toml", "Cargo.toml", "CHANGELOG.md"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_path)
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_path)
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_matches(Data::read_from(
        &source_path.join("actual_output.txt"),
        None,
    ));
    assert().matches(
        Data::read_from(&source_path.join("EXPECTED_Cargo.toml"), None),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
    assert().matches(
        Data::read_from(&source_path.join("EXPECTED_CHANGELOG.md"), None),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
}

#[test]
fn changesets() {
    // Arrange a project with two packages. Add a changeset file for the _first_ package only
    // that has a breaking change. Add a conventional commit for _both_ packages with a feature.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "feat!: Existing feature");
    tag(temp_path, "first/v1.2.3");
    tag(temp_path, "second/v0.4.6");

    let changeset_path = temp_path.join(".changeset");
    create_dir(&changeset_path).unwrap();
    Change {
        unique_id: UniqueId::from("breaking_change"),
        summary: "#### A breaking change\n\nA breaking change for only the first package"
            .to_string(),
        versioning: Versioning::from(("first", ChangeType::Major)),
    }
    .write_to_directory(&changeset_path)
    .unwrap();

    let src_path = Path::new("tests/prepare_release/changesets");
    for file in [
        "knope.toml",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
    ] {
        copy(src_path.join(file), temp_path.join(file)).unwrap();
    }
    add_all(temp_path);
    commit(
        temp_path,
        "feat: A new shared feature from a conventional commit",
    );

    // Act—run a PrepareRelease step to bump versions and update changelogs
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
        .stdout_matches(Data::read_from(&src_path.join("dry_run_output.txt"), None));
    actual_assert.success().stderr_eq("").stdout_eq("");

    let status = status(temp_path);
    for file in [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
    ] {
        assert().matches(
            Data::read_from(&src_path.join(format!("EXPECTED_{}", file)), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
        assert!(status.contains(&format!("M  {}", file)), "{:#?}", status);
    }

    assert_eq!(changeset_path.as_path().read_dir().unwrap().count(), 0);
    assert!(
        status.contains(&"D  .changeset/breaking_change.md".to_string()),
        "{:#?}",
        status
    );
}

#[test]
fn output_of_invalid_changesets() {
    // Arrange a basic project, create an invalid change file
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "feat!: Existing feature");
    tag(temp_path, "v1.2.3");

    let changeset_path = temp_path.join(".changeset");
    create_dir(&changeset_path).unwrap();
    write(changeset_path.join("invalid.md"), "invalid").unwrap();

    let src_path = Path::new("tests/prepare_release/changesets");
    let file = "Cargo.toml";
    copy(src_path.join(file), temp_path.join(file)).unwrap();

    // Act—run a PrepareRelease step to bump versions and update changelogs
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
        .stderr_eq(Data::read_from(&src_path.join("failure_dry_run.txt"), None));
    actual_assert
        .failure()
        .stderr_eq(Data::read_from(&src_path.join("failure.txt"), None));
}

#[test]
fn override_version() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/override_version");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v0.1.0");
    commit(temp_path, "fix: A bug fix");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--override-version=1.0.0")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--override-version=1.0.0")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");

    for file in ["CHANGELOG.md", "Cargo.toml"] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

#[test]
fn override_version_multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/override_version_multiple_packages");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "first/v0.1.0");
    tag(temp_path, "second/v1.2.3");
    tag(temp_path, "third/v4.5.5");
    commit(temp_path, "fix: A bug fix");

    for file in [
        "knope.toml",
        "FIRST_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
        "SECOND_CHANGELOG.md",
        "package.json",
        "THIRD_CHANGELOG.md",
    ] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--override-version=first=1.0.0")
        .arg("--override-version=second=4.5.6")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--override-version=first=1.0.0")
        .arg("--override-version=second=4.5.6")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");

    for file in [
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
        "THIRD_CHANGELOG.md",
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
    ] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// The PrepareRelease step should print out every commit and changeset summary that will be included,
/// which packages those commits/changesets are applicable to,
/// and the semantic rules applicable to each change, as well as the final rule and version selected
/// for each package when the `--verbose` flag is provided.
#[test]
fn verbose() {
    // Arrange a project with two packages. Add a changeset file for the _first_ package only
    // that has a breaking change. Add a conventional commit for _both_ packages with a feature.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "first/v1.2.3");
    tag(temp_path, "second/v0.4.6");
    commit(temp_path, "feat: A feature");
    commit(temp_path, "feat!: A breaking feature");
    commit(temp_path, "fix: A bug fix");
    commit(temp_path, "fix!: A breaking bug fix");
    commit(
        temp_path,
        "chore: A chore with a breaking footer\n\nBREAKING CHANGE: A breaking change",
    );
    commit(temp_path, "feat(first): A feature for the first package");
    commit(temp_path, "feat: A feature with a separate breaking change\n\nBREAKING CHANGE: Another breaking change");

    let changeset_path = temp_path.join(".changeset");
    create_dir(&changeset_path).unwrap();
    Change {
        unique_id: UniqueId::from("breaking_change"),
        summary: "#### A breaking changeset\n\nA breaking change for only the first package"
            .to_string(),
        versioning: Versioning::from(("first", ChangeType::Major)),
    }
    .write_to_directory(&changeset_path)
    .unwrap();
    Change {
        unique_id: UniqueId::from("feature"),
        summary:
            "#### A feature for first, fix for second\n\nAnd even some details which aren't visible"
                .to_string(),
        versioning: Versioning::try_from_iter([
            ("first", ChangeType::Minor),
            ("second", ChangeType::Patch),
        ])
        .unwrap(),
    }
    .write_to_directory(&changeset_path)
    .unwrap();

    let src_path = Path::new("tests/prepare_release/verbose");
    for file in [
        "knope.toml",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "FIRST_CHANGELOG.md",
        "SECOND_CHANGELOG.md",
    ] {
        copy(src_path.join(file), temp_path.join(file)).unwrap();
    }
    add_all(temp_path);

    // Act—run a PrepareRelease step to bump versions and update changelogs
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .arg("--verbose")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("--verbose")
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&src_path.join("dry_run_output.txt"), None));
    actual_assert
        .success()
        .stderr_eq("")
        .stdout_matches(Data::read_from(&src_path.join("output.txt"), None));
}

/// Specifically designed to catch https://github.com/knope-dev/knope/issues/505
#[test]
fn pick_correct_commits_from_branching_history() {
    // Arrange a Git repo with branching history
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    create_branch(temp_path, "patch");
    commit(temp_path, "fix: A bug");
    switch_branch(temp_path, "main");
    merge_branch(temp_path, "patch");
    tag(temp_path, "v1.0.1");
    create_branch(temp_path, "breaking");
    commit(temp_path, "feat!: A breaking feature");
    switch_branch(temp_path, "main");
    merge_branch(temp_path, "breaking");
    tag(temp_path, "v2.0.0");
    switch_branch(temp_path, "breaking");
    merge_branch(temp_path, "main");
    commit(temp_path, "fix: Another bug");
    switch_branch(temp_path, "main");
    merge_branch(temp_path, "breaking");

    let source_path =
        Path::new("tests/prepare_release/pick_correct_commits_from_branching_history");
    for file in ["knope.toml", "Cargo.toml", "CHANGELOG.md"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");
    for file in ["CHANGELOG.md", "Cargo.toml"] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

#[test]
fn pick_correct_tag_from_branching_history() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    create_branch(temp_path, "v2");
    commit(temp_path, "feat!: Something new");
    tag(temp_path, "v2.0.0");
    switch_branch(temp_path, "main");
    commit(temp_path, "fix: A bug");

    let source_path = Path::new("tests/prepare_release/pick_correct_tag_from_branching_history");
    for file in ["knope.toml", "Cargo.toml", "CHANGELOG.md"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act.
    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
    actual_assert.success().stdout_eq("");
    for file in ["CHANGELOG.md", "Cargo.toml"] {
        assert().matches(
            Data::read_from(&source_path.join(format!("EXPECTED_{file}")), None),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

#[test]
fn test_cargo_workspace() {
    let source_dir = Path::new("tests/prepare_release/cargo_workspace");
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    copy_dir_contents(&source_dir.join("source"), temp_path);
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "first-package/v1.0.0");
    tag(temp_path, "second-package/v0.1.0");
    commit(temp_path, "feat(first-package): A feature");
    commit(temp_path, "feat(second-package)!: A breaking feature");

    let dry_run_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_path)
        .assert();
    let actual_output = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_path)
        .assert();

    dry_run_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_dir.join("dry_run_output.txt"),
            None,
        ));
    actual_output
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&source_dir.join("output.txt"), None));

    let expected_dir = source_dir.join("expected");
    assert_eq(
        Data::read_from(&expected_dir.join("first/Cargo.toml"), None),
        read_to_string(temp_path.join("first/Cargo.toml")).unwrap(),
    );
    assert_eq(
        Data::read_from(&expected_dir.join("second/Cargo.toml"), None),
        read_to_string(temp_path.join("second/Cargo.toml")).unwrap(),
    );
}
