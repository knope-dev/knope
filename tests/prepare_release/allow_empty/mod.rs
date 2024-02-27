use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Old feat"),
            Tag("v1.0.0"),
            Commit("docs: Update README"),
        ])
        .run("prepare-release");
}
