use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn is_an_error() {
    TestCase::new(file!())
        .git(&[Commit("docs: update REAMDME")])
        .run("release");
}
