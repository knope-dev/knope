use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// When you get to major version 2 or above, it's [recommended](https://go.dev/blog/v2-go-modules)
/// that you stick all that code in a new `v{major}` directory. So v2.*.* code goes in a directory
/// named `v2`. This is not a submodule named v2, of course, so the tag is still `v2.*.*`. Basically,
/// having the latest code for every major version on a single branch.
///
/// So... when working on a `go.mod` file in a directory matching a major version (`v\d+`), we need
/// to:
///     1. Only consider tags that match the major version
///     2. Only use _parent_ directories (not the version directory) in tag prefixes (reading and writing)
#[test]
fn major_version_directories() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("v1.0.0"),
            Tag("v2.0.0"),
            Tag("sub_dir/v1.0.0"),
            Tag("sub_dir/v2.0.0"),
            Commit("fix(v1): A fix"),
            Commit("feat(v2): New feature"),
        ])
        .expected_tags(&[
            "sub_dir/v1.0.1",
            "sub_dir/v2.1.0",
            "v1.0.1",
            "v1/v1.0.1",
            "v2.1.0",
            "v2/v2.1.0",
        ])
        .run("release --verbose");
}
