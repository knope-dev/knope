use crate::helpers::{GitCommand::*, TestCase};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[Commit("Initial commit"), Tag("v1.2.3")])
        .run("bump");
}
