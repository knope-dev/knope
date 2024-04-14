use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn override_default_sections() {
    TestCase::new(file!())
        .git(&[
            Commit("Existing feature"),
            Tag("v1.0.0"),
            Commit("fix!: Something you hopefully don't care about"),
            Commit("fix: Something you do care about"),
            Commit("feat: Something new"),
        ])
        .run("release");
}
