use crate::helpers::{GitCommand::*, *};

const TEST_CASE: TestCase = TestCase::new(file!()).git(&[
    Commit("feat: Existing Feature"),
    Tag("v1.0.0"),
    Commit("feat: Something"),
]);

/// Run `knope release --dry-run` on a repo with a GitHub remote to test that integration.
#[test]
fn https_remote() {
    TEST_CASE
        .with_remote("https://github.com/knope-dev/knope.git")
        .run("release --dry-run");
}

/// Run `knope release --dry-run` on a repo with a GitHub remote to test that integration.
#[test]
fn ssh_remote() {
    TEST_CASE
        .with_remote("git@github.com:knope-dev/knope.git")
        .run("release --dry-run");
}
