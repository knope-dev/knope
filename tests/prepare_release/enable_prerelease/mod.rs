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

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using the `--prerelease-label` argument.
#[test]
fn with_option() {
    TEST_CASE.run("prerelease --prerelease-label=rc");
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using the `KNOPE_PRERELEASE_LABEL` environment variable.
#[test]
fn with_env() {
    TEST_CASE
        .env("KNOPE_PRERELEASE_LABEL", "rc")
        .run("prerelease");
}

/// Run a `PrepareRelease` as a pre-release in a repo which already contains a release, but set
/// the `prerelease_label` at runtime using both the `--prerelease-label` argument and the
/// `KNOPE_PRERELEASE_LABEL` environment variable.
///
/// The `--prerelease-label` argument should take precedence over the environment variable.
#[test]
fn prerelease_label_option_overrides_env() {
    TEST_CASE
        .env("KNOPE_PRERELEASE_LABEL", "alpha")
        .run("prerelease --prerelease-label=rc");
}
