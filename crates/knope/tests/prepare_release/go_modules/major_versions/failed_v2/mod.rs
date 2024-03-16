use crate::helpers::GitCommand::{Commit, Tag};
use crate::helpers::TestCase;

#[test]
fn major_versions() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat!: Breaking change"),
        ]).expected_tags(&["v1.1.0"])
        .run("release");
}