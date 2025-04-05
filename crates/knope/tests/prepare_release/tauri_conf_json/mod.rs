use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v0.1.0"),
            Commit("feat!: New feature"),
        ])
        .run("release");
}
