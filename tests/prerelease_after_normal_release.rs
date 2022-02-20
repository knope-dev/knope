//! An integration test which runs the `prerelease` task defined in `dobby.toml`.

use std::env::set_current_dir;
use std::io::Write;

use dobby::{command, run};
use git_repo_helpers::*;

mod git_repo_helpers;

#[test]
fn test() {
    // Arrange a git repo which has an existing commit and release tag.
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path();
    init(path);
    commit(path, "feat: New feature in existing release");
    tag(path, "1.1.0");

    // Create a changelog which contains that release.
    let changelog = temp_dir.path().join("CHANGELOG.md");
    let mut changelog_file = std::fs::File::create(&changelog).unwrap();
    writeln!(&mut changelog_file, "## 1.1.0\n").unwrap();
    writeln!(&mut changelog_file, "### Features\n").unwrap();
    writeln!(&mut changelog_file, "- New feature in exsting release\n").unwrap();

    // Add a new commit which should be included in the new pre-release.
    commit(path, "feat!: Breaking feature in new RC");

    // Copy a dobby.toml into the new repo which defines the `prerelease` task.
    let dobby_toml = temp_dir.path().join("dobby.toml");
    std::fs::copy("tests/dobby.toml", dobby_toml).unwrap();
    // Create a metadata file that Dobby can read versions from.
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    let mut cargo_toml_file = std::fs::File::create(&cargo_toml).unwrap();
    writeln!(&mut cargo_toml_file, "[package]").unwrap();
    writeln!(&mut cargo_toml_file, "version = \"1.1.0\"").unwrap();

    // Act.
    set_current_dir(temp_dir.path()).unwrap();
    let matches = command().get_matches_from(vec!["dobby", "prerelease"]);
    run(&matches).unwrap();

    // Assert.
    let changelog_contents = std::fs::read_to_string(changelog).unwrap();
    let lines = changelog_contents.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "## 2.0.0-rc.0");
    assert_eq!(lines[2], "### Breaking Changes");
    assert_eq!(lines[4], "- Breaking feature in new RC");
}
