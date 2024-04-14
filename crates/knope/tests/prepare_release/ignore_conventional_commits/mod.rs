use crate::helpers::{GitCommand::*, TestCase};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing"),
            Tag("v1.0.0"),
            Commit("feat!: Breaking change that should be ignored"),
        ])
        .run("prepare-release");
}
