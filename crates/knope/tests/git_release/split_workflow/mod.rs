use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `Release` for a repo not configured for GitHub simulating a state where `PrepareRelease`
/// has already been run in a separate workflow.
///
/// # Expected
///
/// A new tag should be added to the repo.
#[test]
fn git_release() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .expected_tags(&["v1.1.0"])
        .run("release --verbose");
}
