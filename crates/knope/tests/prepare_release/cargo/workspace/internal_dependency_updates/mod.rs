use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// By default, a dependent gets its dependency strings updated but is _not_ released itself —
/// propagation is opt-in via `update_internal_dependencies`.
#[test]
fn dependents_are_not_bumped_by_default() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("feat(second-package): A feature in the dep"),
        ])
        .run("release");
}
