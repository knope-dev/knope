use std::{
    fs::{copy, read_to_string},
    path::Path,
};

use helpers::*;
use rstest::rstest;
use snapbox::cmd::{cargo_bin, Command};

mod helpers;

/// Run a `PrepareRelease` in a repo and verify that the changelog is updated based on config.
#[rstest]
#[case(Some("CHANGELOG.md"))]
#[case(Some("CHANGES.md"))] // A non-default name
#[case(None)]
fn prepare_release_changelog_selection(#[case] changelog: Option<&str>) {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/changelog/changelog_selection");

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
        .with_assert(assert())
        .stdout_matches_path(expected_dry_run_output);
    actual_assert
        .success()
        .stdout_matches_path(source_path.join("output.txt"));

    for changelog_name in all_changelogs {
        match changelog {
            Some(changelog) if changelog_name == changelog => {
                assert().matches_path(
                    source_path.join("EXPECTED_CHANGELOG.md"),
                    read_to_string(temp_path.join(changelog_name)).unwrap(),
                );
            }
            _ => {
                assert().matches_path(
                    source_path.join("CHANGELOG.md"),
                    read_to_string(temp_path.join(changelog_name)).unwrap(),
                );
            }
        }
    }
    assert().matches_path(
        source_path.join("expected_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
}

#[test]
fn notes() {
    // Arrange a knope project with a merge commit.
    // Make a directory at a known path
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Existing versions");
    tag(temp_path, "first/v1.0.0");
    tag(temp_path, "second/v0.1.0");
    commit(
        temp_path,
        "chore: something\n\nChangelog-Note: A standard note",
    );
    commit(
        temp_path,
        "chore(first): something\n\nChangelog-Note: Standard note first only",
    );
    commit(
        temp_path,
        "chore(second): something\n\nChangelog-Note: Standard note second only",
    );
    commit(
        temp_path,
        "chore: something\n\nChangelog-First-Note: A custom note",
    );
    commit(temp_path, "chore: something\n\nSpecial: Special note");
    commit(temp_path, "chore: something\n\nWhatever: Whatever note");

    let source_path = Path::new("tests/prepare_release/changelog/extra_changelog_sections");
    for file in ["knope.toml", "Cargo.toml", "pyproject.toml"] {
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
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
    actual_assert.success().stderr_eq("");
    assert().matches_path(
        source_path.join("EXPECTED_Cargo.toml"),
        read_to_string(temp_path.join("Cargo.toml")).unwrap(),
    );
    assert().matches_path(
        source_path.join("EXPECTED_pyproject.toml"),
        read_to_string(temp_path.join("pyproject.toml")).unwrap(),
    );
    assert().matches_path(
        source_path.join("EXPECTED_FIRST_CHANGELOG.md"),
        read_to_string(temp_path.join("FIRST_CHANGELOG.md")).unwrap(),
    );
    assert().matches_path(
        source_path.join("EXPECTED_SECOND_CHANGELOG.md"),
        read_to_string(temp_path.join("SECOND_CHANGELOG.md")).unwrap(),
    );
}

#[test]
fn header_level_detection() {
    let source_path = Path::new("tests/prepare_release/changelog/header_level_detection");
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: We support custom header levels now 🎉");

    copy(
        source_path.join("CHANGELOG.md"),
        temp_path.join("CHANGELOG.md"),
    )
    .unwrap();
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();
    copy(source_path.join("Cargo.toml"), temp_path.join("Cargo.toml")).unwrap();

    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert();

    dry_run_assert
        .success()
        .with_assert(assert())
        .stderr_eq("")
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
    actual_assert.success().stderr_eq("");
    assert().matches_path(
        source_path.join("EXPECTED_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
}
