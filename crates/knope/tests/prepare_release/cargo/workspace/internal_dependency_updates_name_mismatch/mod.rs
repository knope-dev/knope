use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Propagation works even when the name a dependency goes by inside a file (here: the
/// published crate name `second-lib`) differs from its package key (`second-package`) —
/// the relationship comes from which package's `versioned_files` contains the entry, not
/// from matching the name.
#[test]
fn propagates_when_crate_name_differs_from_package_key() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("fix(second-package): A fix in second"),
        ])
        .run("release");
}
