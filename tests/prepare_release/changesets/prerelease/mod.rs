use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn changesets() {
    TestCase::new(file!())
        .git(&[
            Commit("feat!: Existing feature"),
            Tag("v1.2.3"),
            Commit("feat: A new shared feature from a conventional commit"),
        ])
        .run("release");
}
