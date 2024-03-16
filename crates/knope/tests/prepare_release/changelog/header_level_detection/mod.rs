use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn header_level_detection() {
    TestCase::new(file!())
        .git(&[
            Commit("Existing feature"),
            Tag("v1.0.0"),
            Commit("feat: We support custom header levels now 🎉"),
        ])
        .run("release --dry-run");
}
