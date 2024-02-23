use crate::helpers::{GitCommand::*, TestCase};

#[test]
fn release_dry_run() {
    TestCase::new(file!())
        .git([
            Commit("feat: Existing"),
            Tag("v1.0.0"),
            Commit("feat: New Feature"),
        ])
        .run("release --dry-run");
}
