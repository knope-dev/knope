use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn package_json_with_dependencies() {
    TestCase::new(file!())
        .git(&[Commit("feat: Add new feature")])
        .run("release");
}
