use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn release_assets() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .run("release --dry-run");
}
