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
            Commit("feat!: Breaking change"),
        ])
        .expected_tags(&["v2.0.0"])
        .run("release --override-version=2.0.0");
}
