use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn changesets() {
    TestCase::new(file!())
        .git(&[
            Commit("feat!: Existing feature"),
            Tag("first/v1.2.3"),
            Tag("second/v0.4.6"),
            Commit("feat: A new shared feature from a conventional commit"),
        ])
        .run("release");
}
