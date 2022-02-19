//! An integration test which runs the `prerelease` task defined in `dobby.toml`.

use dobby::{command, run};
use std::env::set_current_dir;
use std::io::Write;
use std::process::Command;

#[test]
fn test() {
    // Arrange a git repo which has an existing commit and pre-release tag.
    let temp_dir = tempfile::tempdir().unwrap();
    let output = Command::new("git")
        .arg("init")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Configure fake Git user.
    let output = Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("fake@dobby.dev")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Fake Dobby")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg("feat: New feature in first RC")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .arg("tag")
        .arg("v1.1.0-rc.1")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Create a changelog which contains that pre-release.
    let changelog = temp_dir.path().join("CHANGELOG.md");
    let mut changelog_file = std::fs::File::create(&changelog).unwrap();
    writeln!(&mut changelog_file, "## 1.1.0-rc.1\n").unwrap();
    writeln!(&mut changelog_file, "### Features\n").unwrap();
    writeln!(&mut changelog_file, "- New feature in first RC\n").unwrap();

    // Add a new commit which should be included in the new pre-release.
    let output = Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg("feat: New feature in second RC")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Copy a dobby.toml into the new repo which defines the `prerelease` task.
    let dobby_toml = temp_dir.path().join("dobby.toml");
    std::fs::copy("tests/dobby.toml", dobby_toml).unwrap();
    // Create a metadata file that Dobby can read versions from.
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    let mut cargo_toml_file = std::fs::File::create(&cargo_toml).unwrap();
    writeln!(&mut cargo_toml_file, "[package]").unwrap();
    writeln!(&mut cargo_toml_file, "version = \"1.1.0-rc.1\"").unwrap();

    // Act.
    set_current_dir(temp_dir.path()).unwrap();
    let matches = command().get_matches_from(vec!["dobby", "prerelease"]);
    run(&matches).unwrap();

    // Assert.
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let changelog_contents = std::fs::read_to_string(changelog).unwrap();
    let lines = changelog_contents.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "## 1.1.0-rc.2");
    assert_eq!(lines[2], "### Features");
    assert_eq!(lines[4], "- New feature in second RC");
}
