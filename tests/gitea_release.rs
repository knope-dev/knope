use std::{fs::copy, path::Path};

use helpers::*;
use snapbox::{
    cmd::{cargo_bin, Command},
    Data,
};

mod helpers;

/// Run a `PreRelease` then `Release` for a repo not configured for gitea.
///
/// # Expected
///
/// Version should be bumped, and a new tag should be added to the repo.
#[test]
fn gitea_release() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/gitea_release");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act. Cannot run real release without integration testing gitea.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
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
}

/// Verify that Release will operate on all defined packages independently
#[test]
fn multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/gitea_release/multiple_packages");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "first/v1.2.3");
    tag(temp_path, "second/v0.4.6");
    commit(temp_path, "feat!: New breaking feature");

    copy_dir_contents(&data_path.join("source"), temp_path);

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
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));
}

#[test]
fn separate_prepare_and_release_workflows() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/gitea_release/separate_prepare_and_release_workflows");
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

    // Run the actual release (but dry-run because don't test gitea)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
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
}

#[test]
fn release_assets_not_allowed() {
    // Arrange a package that's ready to release with some artifacts
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/gitea_release/release_assets");
    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    Command::new(cargo_bin!("knope"))
        .arg("--validate")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr_eq(Data::read_from(&source_path.join("stderr.txt"), None));
}

#[test]
fn auto_generate_release_notes() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/gitea_release/separate_prepare_and_release_workflows");
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

    // Run the actual release (but dry-run because don't test gitea)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("auto_generate_dry_run_output.txt"),
            None,
        ));
}

#[test]
fn no_previous_tag() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/gitea_release/no_previous_tag");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    copy_dir_contents(&data_path.join("source"), temp_path);

    // Run the actual release (but dry-run because don't test gitea)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));
}

#[test]
fn version_go_mod() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/gitea_release/version_go_mod");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    tag(temp_path, "go/v1.0.0");
    commit(temp_path, "feat: New feature");
    copy_dir_contents(&data_path.join("source"), temp_path);

    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    commit(temp_path, "chore: Prepare release");

    assert().subset_matches(data_path.join("expected"), temp_path);

    // Run the actual release (but dry-run because don't test gitea)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));
}
