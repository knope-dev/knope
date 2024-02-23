use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn release_dry_run() {
    TestCase::new(file!())
        .git([
            Commit("feat: Existing Feature"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("feat: New Feature"),
        ])
        .run("release --dry-run");
}
