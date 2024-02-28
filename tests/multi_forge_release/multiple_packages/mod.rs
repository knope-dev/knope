use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Verify that Release will operate on all defined packages independently
#[test]
fn multiple_packages() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("first/v1.2.3"),
            Tag("second/v0.4.6"),
            Commit("feat!: New breaking feature"),
        ])
        .run("release --dry-run");
}
