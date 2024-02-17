use std::path::Path;

use helpers::*;
use snapbox::{
    cmd::{cargo_bin, Command},
    file, Data,
};

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
    let data_path = Path::new("tests/git_release");
    let source_path = data_path.join("source");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    copy_dir_contents(&source_path, temp_path);

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
        .stdout_matches(file!["git_release/dry_run_output.txt"]);
    actual_assert
        .success()
        .with_assert(assert())
        .stdout_matches(file!["git_release/output.txt"]);
    assert().subset_matches(data_path.join("expected"), temp_path);
    let tags = get_tags(temp_path);
    assert_eq!(tags, vec!["v1.1.0"]);
}

/// Verify that Release will operate on all defined packages independently
#[test]
fn multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/git_release/multiple_packages");
    let source_path = data_path.join("source");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "first/v1.2.3");
    tag(temp_path, "second/v0.4.6");
    commit(temp_path, "feat!: New breaking feature");

    copy_dir_contents(&source_path, temp_path);

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
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));
    let actual_assert = actual_assert.success().with_assert(assert());
    assert().subset_matches(data_path.join("expected"), temp_path);
    assert_eq!(get_tags(temp_path), vec!["first/v2.0.0", "second/v0.5.0"]);
    actual_assert.stdout_matches(Data::read_from(&data_path.join("output.txt"), None));
}
