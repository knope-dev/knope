use crate::helpers::TestCase;

/// Test upgrading from step-level `ignore_conventional_commits` to top-level
#[test]
fn upgrade_ignore_conventional_commits() {
    TestCase::new(file!()).run("--upgrade");
}
