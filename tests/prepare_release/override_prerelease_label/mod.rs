use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

const TEST_CASE: TestCase<5, 0> = TestCase::new(file!()).git([
    Commit("Initial commit"),
    Tag("v1.0.0"),
    Commit("feat: New feature in existing release"),
    Tag("v1.1.0"),
    Commit("feat!: Breaking feature in new RC"),
]);

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but change
/// the configured `prerelease_label` at runtime using the `--prerelease-label` argument.
#[test]
fn with_option() {
    TEST_CASE.run("prerelease --prerelease-label=alpha");
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but change
/// the configured `prerelease_label` at runtime using the `KNOPE_PRERELEASE_LABEL` environment variable.
#[test]
fn with_env() {
    TEST_CASE
        .env("KNOPE_PRERELEASE_LABEL", "alpha")
        .run("prerelease");
}
