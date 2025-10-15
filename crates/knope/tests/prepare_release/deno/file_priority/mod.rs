use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn deno_json_takes_priority_over_jsonc_and_package_json() {
    TestCase::new(file!())
        .git(&[Commit("fix(@scope/test-package): A bug fix")])
        .run("release");
}
