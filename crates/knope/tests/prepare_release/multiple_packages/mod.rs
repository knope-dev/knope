use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn operates_on_all_packages_independently() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("first/v1.2.3"),
            Tag("second/v0.4.6"),
            Commit("feat!: New breaking feature"),
        ])
        .run("release");
}
