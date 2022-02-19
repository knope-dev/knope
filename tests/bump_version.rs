//! An integration test which runs the `prerelease` task defined in `dobby.toml`.

use std::env::set_current_dir;

use dobby::{command, run};

#[test]
fn tests() {
    // Arrange a folder with a dobby file configured to bump versions and a file dobby knows how to bump.
    let temp_dir = tempfile::tempdir().unwrap();

    let dobby_toml = temp_dir.path().join("dobby.toml");
    std::fs::copy("tests/dobby.toml", dobby_toml).unwrap();

    set_current_dir(temp_dir.path()).unwrap();

    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Use a poor excuse for parametrization because setting the current dir doesn't work in parallel.
    let test_cases = [
        ("bump-pre", "1.2.3", "1.2.4-rc.0"),
        ("bump-pre", "1.2.3-rc.0", "1.2.3-rc.1"),
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

        let matches = command().get_matches_from(["dobby", workflow]);
        run(&matches).unwrap();
        let cargo_contents = std::fs::read_to_string(&cargo_toml).unwrap();
        assert_eq!(
            cargo_contents,
            format!("[package]\nversion = \"{}\"\n", expected_version)
        );
    }
}
