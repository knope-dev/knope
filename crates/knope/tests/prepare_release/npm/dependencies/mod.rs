use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn package_json_with_dependencies() {
    TestCase::new(file!())
        .git(&[
            Commit("initial"),
            Tag("v1.0.0"),
            Commit("feat: Add new feature"),
        ])
        .run("release");
}
