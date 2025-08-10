use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn test_cargo_workspace() {
    TestCase::new(file!())
        .git(&[Commit("Initial commit"), Commit("feat: A feature")])
        .run("release");
}
