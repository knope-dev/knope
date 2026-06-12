use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// With no `{ path, dependency }` entries and no `internal_dependencies`, the relationship
/// is read from the manifests themselves: `first-package`'s `Cargo.toml` declares a
/// dependency on `second-package`, so opting in to `update_internal_dependencies` is all
/// the configuration needed.
#[test]
fn relationships_are_read_from_manifests() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("fix(second-package): A fix in second"),
        ])
        .run("release");
}
