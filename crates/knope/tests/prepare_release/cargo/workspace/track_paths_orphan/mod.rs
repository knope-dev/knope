use crate::helpers::{
    GitCommand::{Add, Commit, Tag},
    TestCase,
};

/// Commits touching only files outside every package's territory are dropped: the breaking
/// change to the README must not cause a major bump anywhere, and a package with no
/// matching commits is not released at all.
#[test]
fn commits_outside_all_territories_are_ignored() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Add("first/lib.rs"),
            Commit("feat: A feature in first"),
            Add("README.md"),
            Commit("fix!: A breaking change to repo infrastructure"),
        ])
        .run("release");
}
