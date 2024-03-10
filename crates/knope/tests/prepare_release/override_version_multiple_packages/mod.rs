use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn override_version_multiple_packages() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("first/v0.1.0"),
            Tag("second/v1.2.3"),
            Tag("third/v4.5.5"),
            Commit("fix: A bug fix"),
        ])
        .run("release --override-version=first=1.0.0 --override-version=second=4.5.6");
}
