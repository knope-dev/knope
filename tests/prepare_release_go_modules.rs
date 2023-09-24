use std::{
    fs::{copy, create_dir, create_dir_all, read_to_string},
    path::Path,
};

use snapbox::cmd::{cargo_bin, Command};

use crate::helpers::{assert, commit, get_tags, init, tag};

mod helpers;

/// Go modules have a peculiar way of versioning in that only the major version is recorded to the
/// `go.mod` file and only for major versions >1. This tests that.
#[test]
fn major_versions() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/go_modules/major_versions");

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
        .with_assert(assert())
        .stdout_matches_path(source_path.join("1.1_dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    assert().matches_path(
        source_path.join("EXPECTED_1.1_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert().matches_path(
        source_path.join("EXPECTED_1.1_go.mod"),
        read_to_string(temp_path.join("go.mod")).unwrap(),
    );
    let tags = get_tags(temp_path);
    assert_eq!(tags, vec!["v1.1.0"]);

    // Arrange 2—version goes to 2.0
    commit(temp_path, "feat!: Breaking change");

    // Act 2—cannot bump to v2 without override
    Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("failed_2.0_output.txt"));

    // Act 2—version goes to 2.0
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .arg("--override-version=2.0.0")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--override-version=2.0.0")
        .current_dir(temp_dir.path())
        .assert();

    // Assert 2—version goes to 2.0
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("2.0_dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    assert().matches_path(
        source_path.join("EXPECTED_2.0_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert().matches_path(
        source_path.join("EXPECTED_2.0_go.mod"),
        read_to_string(temp_path.join("go.mod")).unwrap(),
    );
    let tags = get_tags(temp_path);
    assert_eq!(vec!["v2.0.0"], tags);
}

/// In addition to the >2.x rules above, there is also a tagging pattern that must be kept-to
#[test]
fn subdirectories() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/go_modules/subdirectories");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    // This is the version of the Go package, but there is no project-wide tag, so _both_ commits should be included.
    tag(temp_path, "sub_dir/v1.0.0");
    commit(temp_path, "feat: New feature");

    for file in ["knope.toml", "CHANGELOG.md"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    let sub_dir = temp_path.join("sub_dir");
    create_dir(&sub_dir).unwrap();
    copy(source_path.join("go.mod"), sub_dir.join("go.mod")).unwrap();

    // Act
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert 1
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("1.1_dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    assert().matches_path(
        source_path.join("EXPECTED_1.1_CHANGELOG.md"),
        read_to_string(temp_path.join("CHANGELOG.md")).unwrap(),
    );
    assert().matches_path(
        source_path.join("EXPECTED_1.1_go.mod"),
        read_to_string(sub_dir.join("go.mod")).unwrap(),
    );
    let tags = get_tags(temp_path);
    assert_eq!(vec!["sub_dir/v1.1.0", "v1.1.0"], tags);
}

#[test]
fn version_determination() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.2.3");
    let source_path = Path::new("tests/prepare_release/go_modules/version_determination");
    create_dir(temp_path.join("with_comment")).unwrap();
    tag(temp_path, "with_comment/v0.1.0"); // Comment should override tag
    create_dir(temp_path.join("without_comment")).unwrap();
    tag(temp_path, "without_comment/v1.2.3");
    commit(temp_path, "feat: A feature");

    let versioned_files = [
        "knope.toml",
        "Cargo.toml", // Mix in another type of file for good measure
        "go.mod",
        "with_comment/go.mod",
        "without_comment/go.mod",
    ];

    for file in versioned_files {
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
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    for file in versioned_files {
        assert().matches_path(
            source_path.join(format!("EXPECTED_{file}")),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
}

/// When you get to major version 2 or above, it's [recommended](https://go.dev/blog/v2-go-modules)
/// that you stick all that code in a new `v{major}` directory. So v2.*.* code goes in a directory
/// named `v2`. This is not a submodule named v2, of course, so the tag is still `v2.*.*`. Basically,
/// having the latest code for every major version on a single branch.
///
/// So... when working on a `go.mod` file in a directory matching a major version (`v\d+`), we need
/// to:
///     1. Only consider tags that match the major version
///     2. Only use _parent_ directories (not the version directory) in tag prefixes (reading and writing)
#[test]
fn major_version_directories() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/prepare_release/go_modules/major_version_directories");

    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    tag(temp_path, "v2.0.0");
    tag(temp_path, "sub_dir/v1.0.0");
    tag(temp_path, "sub_dir/v2.0.0");
    commit(temp_path, "fix(v1): A fix");
    commit(temp_path, "feat(v2): New feature");

    create_dir_all(temp_path.join("sub_dir")).unwrap();
    create_dir_all(temp_path.join("v2/sub_dir")).unwrap();

    for file in [
        "knope.toml",
        "CHANGELOG.md",
        "go.mod",
        "sub_dir/go.mod",
        "v2/sub_dir/go.mod",
        "v2/go.mod",
        "v2/CHANGELOG.md",
    ] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .arg("--verbose")
        .current_dir(temp_dir.path())
        .assert();
    let actual_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .current_dir(temp_dir.path())
        .assert();

    // Assert
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches_path(source_path.join("dry_run_output.txt"));
    actual_assert.success().stdout_eq("");
    for file in [
        "CHANGELOG.md",
        "go.mod",
        "sub_dir/go.mod",
        "v2/CHANGELOG.md",
        "v2/go.mod",
        "v2/sub_dir/go.mod",
    ] {
        assert().matches_path(
            source_path.join(format!("EXPECTED_{file}")),
            read_to_string(temp_path.join(file)).unwrap(),
        );
    }
    let tags = get_tags(temp_path);
    assert_eq!(
        tags,
        vec![
            "sub_dir/v1.0.1",
            "sub_dir/v2.1.0",
            "v1.0.1",
            "v1/v1.0.1",
            "v2.1.0",
            "v2/v2.1.0"
        ]
    );
}
