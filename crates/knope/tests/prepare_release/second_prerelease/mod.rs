use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a pre-release.
#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("An old prerelease which should not be checked"),
            Tag("v1.1.0-rc.2"),
            Commit("feat: New feature in first RC"),
            Tag("v1.0.0"),
            Tag("v1.1.0-rc.1"),
            Commit("feat: New feature in second RC"),
        ])
        .run("prerelease");
}
