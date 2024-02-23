use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn prepare_release_pyproject_toml() {
    TestCase::new(file!())
        .git([
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat!: New feature"),
        ])
        .run("release");
}
