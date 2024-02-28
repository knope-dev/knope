use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn apply_all_commits() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: No scope feature"),
            Commit("feat(scope)!: New breaking feature with a scope"),
        ])
        .run("release");
}
