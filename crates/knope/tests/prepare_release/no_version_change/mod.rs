use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn is_an_error() {
    TestCase::new(file!())
        .git(&[Commit("docs: update README"), Tag("v1.2.3")])
        .run("release");
}
