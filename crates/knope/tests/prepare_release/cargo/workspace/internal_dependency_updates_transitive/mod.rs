use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// Verify that bumps propagate transitively: `c` bumps, `b` (depends on c) gets a patch bump,
/// then `a` (depends on b) also gets a patch bump.
#[test]
fn propagates_transitively() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("a/v1.0.0"),
            Tag("b/v1.0.0"),
            Tag("c/v1.0.0"),
            Commit("feat(c)!: A breaking change in c"),
        ])
        .run("release");
}
