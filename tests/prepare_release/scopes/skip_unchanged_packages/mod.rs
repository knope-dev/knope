use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[Commit("fix(first): Fix for first only")])
        .run("release");
}
