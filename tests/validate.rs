use std::fs::copy;
use std::path::Path;

use snapbox::cmd::{cargo_bin, Command};

use git_repo_helpers::*;

mod git_repo_helpers;

/// Run `--validate` with a config file that has lots of problems.
#[test]
fn test_validate() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/validate");
    init(temp_path);
    commit(temp_path, "Initial commit");
    tag(temp_path, "1.0.0");
    copy(source_path.join("knope.toml"), temp_path.join("knope.toml")).unwrap();

    let assert = Command::new(cargo_bin!("knope"))
        .arg("--validate")
        .current_dir(temp_path)
        .assert();
    assert.failure().stderr_eq_path("tests/validate/output.txt");
}
