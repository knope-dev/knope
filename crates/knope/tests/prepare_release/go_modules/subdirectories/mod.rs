use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// In addition to the >2.x rules above, there is also a tagging pattern that must be kept-to
#[test]
fn subdirectories() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("sub_dir/v1.0.0"),
            Tag("v1.0.0"),
            Commit("feat: New feature"),
        ])
        .expected_tags(&["sub_dir/v1.1.0", "v1.1.0"])
        .run("release");
}
