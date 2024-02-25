use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing Feature"),
            Tag("v1.0.0"),
            Commit("feat: Something"),
        ])
        .run("release --dry-run")
}
