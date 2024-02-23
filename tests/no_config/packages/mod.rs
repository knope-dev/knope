use crate::helpers::{GitCommand::*, TestCase};

/// Run `knope release --dry-run` on a repo with supported metadata files.
#[test]
fn test_packages() {
    TestCase::new(file!())
        .git([
            Commit("feat: Existing Feature"),
            Tag("v1.0.0"),
            Commit("feat: Something"),
        ])
        .run("release --dry-run");
}
