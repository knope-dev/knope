use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// A second prerelease where the commits since the last prerelease imply a _lower_ stable
/// target (a fix) than the existing prerelease line (which shipped a feature). The new
/// prerelease must continue the existing line (1.1.0-rc.1), not regress to 1.0.1-rc.0,
/// and its changelog must only contain the new commits.
#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("v1.0.0"),
            Commit("feat: New feature in first RC"),
            Tag("v1.1.0-rc.0"),
            Commit("fix: A fix after the first RC"),
        ])
        .run("prerelease");
}
