use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn test_cargo_workspace() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("feat(first-package): A feature"),
            Commit("feat(second-package)!: A breaking feature"),
        ])
        .run("release");
}
