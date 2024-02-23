use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release.
#[test]
fn prerelease_after_release() {
    TestCase::new(file!())
        .git([
            Commit("Initial commit"),
            Tag("v1.0.0"),
            Commit("feat: New feature in existing release"),
            Tag("v1.1.0"),
            Commit("feat!: Breaking feature in new RC"),
        ])
        .run("prerelease");
}
