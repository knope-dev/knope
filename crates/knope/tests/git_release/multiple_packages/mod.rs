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
        .expected_tags(&["first/v2.0.0", "second/v0.5.0"])
        .run("release");
}
