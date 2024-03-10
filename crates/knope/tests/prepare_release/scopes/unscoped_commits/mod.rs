use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn apply_to_all_packages() {
    TestCase::new(file!())
        .git(&[
            Commit("fix(first): Fix for first only"),
            Commit("feat: No-scope feat"),
            Commit("feat(second)!: Breaking change for second only"),
        ])
        .run("release");
}
