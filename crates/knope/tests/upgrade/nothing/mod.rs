use crate::helpers::TestCase;

/// Test running `--upgrade` when there is nothing to upgrade
#[test]
fn upgrade_nothing() {
    TestCase::new(file!()).run("--upgrade");
}
