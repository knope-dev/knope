use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `PrepareRelease` in a repo with multiple versionable files—verify only the selected
/// one is modified.
#[test]
fn prepare_release_selects_files() {
    TestCase::new(file!())
        .git([
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .run("release");
}
