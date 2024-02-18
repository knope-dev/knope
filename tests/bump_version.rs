//! An integration test which runs the `prerelease` task defined in `knope.toml`.

use std::{fs::read_to_string, path::Path};

use helpers::*;
use rstest::rstest;
use snapbox::{
    cmd::{cargo_bin, Command},
    Data,
};

mod helpers;

/// Test all the `BumpVersion` rules.
#[rstest]
#[case("bump-pre", "1.2.3", "1.2.4-rc.0")]
#[case("bump-pre", "1.2.3-rc.0", "1.2.4-rc.0")]
#[case("bump-pre", "1.2.4-rc.0", "1.2.4-rc.1")]
#[case("bump-release", "1.2.4-rc.0", "1.2.4")]
#[case("bump-patch", "1.2.3", "1.2.4")]
#[case("bump-minor", "1.2.3", "1.3.0")]
#[case("bump-major", "1.2.3", "2.0.0")]
fn bump_version(
    #[case] workflow: &str,
    #[case] current_version: &str,
    #[case] expected_version: &str,
) {
    // Arrange a folder with a knope file configured to bump versions and a file knope knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.2.3"); // Need to have stable version as tag if pre version in Cargo.toml.
    let source_path = Path::new("tests/bump_version");

    let knope_toml = temp_dir.path().join("knope.toml");
    std::fs::copy(source_path.join("knope.toml"), knope_toml).unwrap();
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    std::fs::write(
        cargo_toml,
        format!("[package]\nname = \"default\"\nversion = \"{current_version}\"\n"),
    )
    .unwrap();

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg(workflow)
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg(workflow)
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert.success().stdout_matches(Data::read_from(
        &source_path.join(format!(
            "{workflow}_{current_version}_{expected_version}_dry_run_output.txt"
        )),
        None,
    ));
    actual_assert.success().stdout_eq("");

    assert().matches(
        Data::read_from(
            &source_path.join(format!(
                "{workflow}_{current_version}_{expected_version}_cargo.toml"
            )),
            None,
        ),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

/// Test all the `BumpVersion` rules when multiple packages are present.
#[rstest]
#[case("bump-pre")]
#[case("bump-patch")]
#[case("bump-minor")]
#[case("bump-major")]
fn multiple_packages(#[case] workflow: &str) {
    // Arrange a folder with a knope file configured to bump versions and a file knope knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.2.3"); // Need to have stable version as tag if pre version in Cargo.toml.
    let data_path = Path::new("tests/bump_version/multiple_packages");
    let source_path = data_path.join("source");
    let expected_path = data_path.join(workflow);

    copy_dir_contents(&source_path, temp_path);

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg(workflow)
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg(workflow)
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .stdout_matches(Data::read_from(
            &expected_path.join("dry_run_output.txt"),
            None,
        ))
        .stderr_eq("");
    actual_assert.success().stdout_eq("").stderr_eq("");
    assert().subset_matches(expected_path.join("expected"), temp_path);
}

/// Test all the `BumpVersion` rules when multiple packages in pre-release versions are present.
#[rstest]
#[case("bump-pre")]
#[case("bump-release")]
fn multiple_packages_pre(#[case] workflow: &str) {
    // Arrange a folder with a knope file configured to bump versions and a file knope knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    // Need to have stable version as tag for each package.
    tag(temp_path, "rust/v0.1.2");
    tag(temp_path, "python/v3.4.5");
    tag(temp_path, "javascript/v6.7.8");
    let data_path = Path::new("tests/bump_version/multiple_packages_pre");
    let source_path = data_path.join("source");
    copy_dir_contents(&source_path, temp_path);
    let workflow_path = data_path.join(workflow);

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg(workflow)
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg(workflow)
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .stdout_matches(Data::read_from(
            &workflow_path.join("dry_run_output.txt"),
            None,
        ))
        .stderr_eq("");
    actual_assert.success().stdout_eq("").stderr_eq("");
    assert().subset_matches(workflow_path.join("expected"), temp_path);
}

#[test]
fn override_version_single_package() {
    // Arrange a folder with a knope file configured to bump versions and a file knope knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    let current_version = "0.1.0";
    tag(temp_path, &format!("v{current_version}"));
    let source_path = Path::new("tests/bump_version");

    let knope_toml = temp_dir.path().join("knope.toml");
    std::fs::copy(source_path.join("knope.toml"), knope_toml).unwrap();
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    std::fs::copy(source_path.join("Cargo.toml"), cargo_toml).unwrap();

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("bump-major")
        .arg("--override-version=1.0.0")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("bump-major")
        .arg("--override-version=1.0.0")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert.success().stdout_matches(Data::read_from(
        &source_path.join("override_dry_run_output.txt"),
        None,
    ));
    actual_assert.success().stdout_eq("");

    assert().matches(
        Data::read_from(&source_path.join("override_cargo.toml"), None),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

#[test]
fn override_version_multiple_packages() {
    // Arrange a folder with a knope file configured to bump versions and a file knope knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.2.3"); // Need to have stable version as tag if pre version in Cargo.toml.
    let data_path = Path::new("tests/bump_version/multiple_packages");
    let source_path = data_path.join("source");
    let expected_path = data_path.join("override");

    copy_dir_contents(&source_path, temp_path);

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("bump-major")
        .arg("--override-version=rust=1.0.0")
        .arg("--override-version=python=4.3.2")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("bump-major")
        .arg("--override-version=rust=1.0.0")
        .arg("--override-version=python=4.3.2")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .stdout_matches(Data::read_from(
            &expected_path.join("dry_run_output.txt"),
            None,
        ))
        .stderr_eq("");
    actual_assert.success().stdout_eq("").stderr_eq("");

    assert().subset_matches(expected_path.join("expected"), temp_path);
}
