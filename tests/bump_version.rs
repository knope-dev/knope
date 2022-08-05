//! An integration test which runs the `prerelease` task defined in `knope.toml`.

use std::env::set_current_dir;

use clap::Parser;

use knope::{run, Cli};

use git_repo_helpers::*;

mod git_repo_helpers;

#[test]
fn tests() {
    // Arrange a folder with a knope file configured to bump versions and a file knope knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.2.3"); // Need to have stable version as tag if pre version in Cargo.toml.

    let knope_toml = temp_dir.path().join("knope.toml");
    std::fs::copy("tests/knope.toml", knope_toml).unwrap();

    set_current_dir(temp_dir.path()).unwrap();

    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Use a poor excuse for parametrization because setting the current dir doesn't work in parallel.
    let test_cases = [
        ("bump-pre", "1.2.3", "1.2.4-rc.0"),
        ("bump-pre", "1.2.3-rc.0", "1.2.4-rc.0"),
        ("bump-pre", "1.2.4-rc.0", "1.2.4-rc.1"),
        ("bump-release", "1.2.3-rc.0", "1.2.3"),
        ("bump-patch", "1.2.3", "1.2.4"),
        ("bump-minor", "1.2.3", "1.3.0"),
        ("bump-major", "1.2.3", "2.0.0"),
    ];

    for (workflow, current_version, expected_version) in test_cases {
        std::fs::write(
            &cargo_toml,
            format!(
                "[package]\nversion = \"{current_version}\"",
                current_version = current_version
            ),
        )
        .unwrap();

        let cli = Cli::parse_from(["knope", workflow]);
        run(cli).unwrap();
        let cargo_contents = std::fs::read_to_string(&cargo_toml).unwrap();
        assert_eq!(
            cargo_contents,
            format!(
                "[package]\nversion = \"{expected_version}\"",
                expected_version = expected_version
            )
        );
    }
}
