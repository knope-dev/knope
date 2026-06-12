use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// When a dependency's version is only tracked in a shared workspace manifest (owned by no
/// package), Knope can't derive the relationship — `internal_dependencies` declares it
/// explicitly so the dependent still gets released.
#[test]
fn explicit_internal_dependencies_propagate() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("fix(second-package): A fix in second"),
        ])
        .run("release");
}
