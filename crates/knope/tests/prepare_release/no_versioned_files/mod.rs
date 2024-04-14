use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// If `PrepareRelease` is run with no `versioned_files`, it should determine the version from the
/// previous valid tag.
#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .expected_tags(&["v1.1.0"])
        .run("release");
}
