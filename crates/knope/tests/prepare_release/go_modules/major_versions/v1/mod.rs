use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn major_versions() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .expected_tags(&["v1.1.0"])
        .run("release");
}
