use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn override_version() {
    TestCase::new(file!())
        .git(&[Commit("feat: Existing feature"), Tag("v0.1.0")])
        .run("release --override-version=1.0.0");
}
