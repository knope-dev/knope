use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn test() {
    TestCase::new(file!())
        .git([Commit("Initial commit"), Tag("v1.2.3")])
        .run("bump --override-version=rust=1.0.0 --override-version=python=4.3.2");
}
