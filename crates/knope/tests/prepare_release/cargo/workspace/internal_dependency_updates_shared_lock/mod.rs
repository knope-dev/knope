use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Sharing a lock file is not a dependency relationship: `first-package` owns no manifest
/// that `second-package` writes into, so `second-package`'s release must not propagate to
/// it even though `first-package` opted into patch bumps.
#[test]
fn shared_lock_file_does_not_propagate() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("fix(second-package): A fix in second"),
        ])
        .run("release");
}
