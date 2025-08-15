use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[Commit("Initial commit"), Tag("v1.2.3-rc.0")])
        .run("bump");
}
