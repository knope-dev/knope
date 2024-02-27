use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn version_determination() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("v1.2.3"),
            Tag("with_comment/v0.1.0"), // Comment should override tag
            Tag("without_comment/v1.2.3"),
            Commit("feat: A feature"),
        ])
        .run("prepare-release");
}
