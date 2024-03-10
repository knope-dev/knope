use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn no_previous_tag() {
    TestCase::new(file!())
        .git(&[Commit("feat: Existing feature")])
        .run("release --dry-run")
}
