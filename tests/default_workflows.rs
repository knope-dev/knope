mod helpers;

use std::path::Path;

use helpers::*;
use rstest::rstest;
use snapbox::cmd::{cargo_bin, Command};
use tempfile::tempdir;

#[rstest]
#[case::cargo_workspace("cargo_workspace")]
#[case::github("github")]
#[case::gitea("gitea")]
#[case::no_forge("no_forge")]
fn test(#[case] case: &str) {
    let asset_dir = Path::new("tests/default_workflows").join(case);
    let source_dir = asset_dir.join("source");
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    copy_dir(&source_dir, temp_path);
    init(temp_path);
    commit(temp_path, "feat: Existing Feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New Feature");

    let help_assert = Command::new(cargo_bin!("knope"))
        .arg("--help")
        .current_dir(temp_path)
        .assert();
    let help_document_change_assert = Command::new(cargo_bin!("knope"))
        .arg("document-change")
        .arg("--help")
        .current_dir(temp_path)
        .assert();
    let help_release_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--help")
        .current_dir(temp_path)
        .assert();
    let release_dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_path)
        .assert();

    help_assert
        .success()
        .with_assert(assert())
        .stdout_eq_path(asset_dir.join("help.txt"));
    help_document_change_assert
        .success()
        .with_assert(assert())
        .stdout_eq_path(asset_dir.join("help_document_change.txt"));
    help_release_assert
        .success()
        .with_assert(assert())
        .stdout_eq_path(asset_dir.join("help_release.txt"));
    release_dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(asset_dir.join("release_dry_run.txt"));
}
