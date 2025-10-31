use crate::helpers::{GitCommand::*, TestCase};

#[test]
fn readme_version() {
    TestCase::new(file!())
        .git(&[
            Commit("initial commit"),
            Tag("0.1.0"),
            Commit("feat: A new feature"),
        ])
        .run("release");
}
