use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `PreRelease` then `Release` for a repo not configured for GitHub.
///
/// # Expected
///
/// Version should be bumped, and a new tag should be added to the repo.
#[test]
fn git_release() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .expected_tags(&["v1.1.0"])
        .run("release");
}
