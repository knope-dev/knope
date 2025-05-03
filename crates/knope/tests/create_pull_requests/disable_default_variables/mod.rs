use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `PreRelease` then `Release` for a repo not configured for gitea.
///
/// # Expected
///
/// Version should be bumped, and a new tag should be added to the repo.
#[test]
fn gitea_release() {
    TestCase::new(file!())
        .git(&[Commit("feat: Existing feature"), Tag("v1.0.0")])
        .run("pr --dry-run"); // Cannot run real release without integration testing gitea.
}
