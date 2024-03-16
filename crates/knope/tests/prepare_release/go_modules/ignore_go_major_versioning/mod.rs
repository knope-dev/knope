use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn ignore_go_major_versioning() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("v1.0.0"),
            Commit("fix!: Breaking change"),
            Tag("v2.0.0"),
            Commit("fix: A fix"),
        ])
        .expected_tags(&["v2.0.1"])
        .run("release");
}
