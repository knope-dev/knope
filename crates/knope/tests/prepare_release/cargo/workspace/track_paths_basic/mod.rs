use crate::helpers::{
    GitCommand::{Add, Commit, Tag},
    TestCase,
};

/// With `track_paths` and no explicit `paths`, commits are routed to packages by the
/// parent directories of their versioned files.
#[test]
fn commits_route_by_versioned_file_directories() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first-package/v1.0.0"),
            Tag("second-package/v0.1.0"),
            Add("first/lib.rs"),
            Commit("feat: A feature in first"),
            Add("second/lib.rs"),
            Commit("fix: A fix in second"),
        ])
        .run("release");
}
