use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn dependent_with_minor_policy_gets_minor_bump() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("fix(second-package): A fix in the dep"),
        ])
        .run("release");
}
