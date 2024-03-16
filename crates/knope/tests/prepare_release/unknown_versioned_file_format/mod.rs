use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Run a `PrepareRelease` where the configured `versioned_file` is not a supported format
#[test]
fn snapshot_error() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .run("release");
}
