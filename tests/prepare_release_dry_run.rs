//! An integration test which runs the `prepare-release-dry-run` task defined in `dobby.toml`.

use std::env::set_current_dir;

use tempfile::TempDir;

use dobby::{command, run};
use git_repo_helpers::*;

mod git_repo_helpers;

#[test]
fn test() {
    // Arrange a git repo which has an existing commit and release tag.
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();
    init(path);
    commit(path, "feat: Feature in existing release");
    tag(path, "1.2.3");
    commit(path, "feat: Feature in new release");

    // Copy a dobby.toml into the new repo which defines the `prerelease` task.
    let dobby_toml = path.join("dobby.toml");
    std::fs::copy("tests/dobby.toml", dobby_toml).unwrap();
    // Create a metadata file that Dobby can read versions from.
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    let cargo_contents = "[package]\nversion = \"1.2.3\"\n";
    std::fs::write(&cargo_toml, cargo_contents).unwrap();
    // Create a changelog to make sure Dobby doesn't modify it
    let changelog = temp_dir.path().join("CHANGELOG.md");
    let changelog_contents = "## 1.2.3\n\n- Feature in existing release\n";
    std::fs::write(&changelog, changelog_contents).unwrap();

    // Act.
    set_current_dir(path).unwrap();
    let matches = command().get_matches_from(vec!["dobby", "prepare-release-dry-run"]);
    run(&matches).unwrap();

    // Assert nothing has changed.
    assert_eq!(std::fs::read_to_string(cargo_toml).unwrap(), cargo_contents);
    assert_eq!(
        std::fs::read_to_string(changelog).unwrap(),
        changelog_contents
    );
}
