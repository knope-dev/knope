use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// If `PrepareRelease` is run with no `prerelease_label`, it should skip any prerelease tags
/// when parsing commits, as well as determine the next version from the previous released version
/// (not from the pre-release version).
#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("feat: Existing feature"),
            Tag("v1.0.0"),
            Commit("feat!: Breaking change"),
            Commit("feat: New feature"),
            Tag("v1.1.0-rc.1"),
        ])
        .run("release");
}
