use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Same as the Cargo workspace test, but for `package.json` manifests: the dependency on
/// `second-package` is read from `first/package.json` with no extra configuration.
#[test]
fn relationships_are_read_from_package_json() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Commit("fix(second-package): A fix in second"),
        ])
        .run("release");
}
