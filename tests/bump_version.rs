//! An integration test which runs the `prerelease` task defined in `knope.toml`.

use std::fs::read_to_string;
use std::path::Path;

use rstest::rstest;
use snapbox::assert_eq_path;
use snapbox::cmd::{cargo_bin, Command};

use git_repo_helpers::*;

mod git_repo_helpers;

/// Run a `PrepareRelease` in a repo with multiple versionable files—verify only the selected
/// one is modified.
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
        &cargo_toml,
        format!(
            "[package]\nversion = \"{current_version}\"",
            current_version = current_version
        ),
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
    dry_run_assert
        .success()
        .stdout_eq_path(source_path.join(format!(
            "{workflow}_{current_version}_{expected_version}_dry_run_output.txt"
        )));
    actual_assert.success().stdout_eq("");

    assert_eq_path(
        source_path.join(format!(
            "{workflow}_{current_version}_{expected_version}_cargo.toml"
        )),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}
