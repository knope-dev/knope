use crate::helpers::{
    GitCommand::{Add, Commit, Tag},
    TestCase,
};

/// Explicit `paths` override the versioned-file fallback: a commit touching `docs/` routes
/// to the first package because `docs` is in its configured paths.
#[test]
fn explicit_paths_override_versioned_file_directories() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Add("docs/guide.md"),
            Commit("feat: A documented feature for first"),
            Add("second/lib.rs"),
            Commit("fix: A fix in second"),
        ])
        .run("release");
}
