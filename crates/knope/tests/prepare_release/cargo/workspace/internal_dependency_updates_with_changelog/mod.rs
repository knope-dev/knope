use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn dependent_changelog_gets_updated_dependencies_section() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("feat(second-package): A feature in the dep"),
        ])
        .run("release");
}
