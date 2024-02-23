use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `PrepareRelease` where the CHANGELOG.md file is missing and verify it's created.
#[test]
fn prepare_release_creates_missing_changelog() {
    TestCase::new(file!())
        .git([
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .run("release");
}
