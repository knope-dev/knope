use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn apply_scopes() {
    TestCase::new(file!())
        .git(&[
            Commit("fix(first): Fix for first only"),
            Commit("feat(both): Shared feat"),
            Commit("feat(second)!: Breaking change for second only"),
        ])
        .run("release");
}
