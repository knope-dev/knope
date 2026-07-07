use crate::helpers::{
    GitCommand::{Add, Commit, Tag},
    TestCase,
};

/// A single commit touching files in multiple packages' territories applies to all of them.
#[test]
fn commit_touching_multiple_territories_applies_to_each() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Add("first/lib.rs"),
            Add("second/lib.rs"),
            Commit("feat: A cross-cutting feature"),
        ])
        .run("release");
}
