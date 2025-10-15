use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn deno_workspace_with_dependencies() {
    TestCase::new(file!())
        .git(&[
            Commit("feat(@scope/first-package): Add new feature"),
            Commit("fix(@scope/second-package): A bug fix"),
        ])
        .run("release");
}
