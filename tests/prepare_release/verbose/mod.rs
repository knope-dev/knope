use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

/// The PrepareRelease step should print out every commit and changeset summary that will be included,
/// which packages those commits/changesets are applicable to,
/// and the semantic rules applicable to each change, as well as the final rule and version selected
/// for each package when the `--verbose` flag is provided.
#[test]
fn verbose() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("first/v1.2.3"),
            Tag("second/v0.4.6"),
            Commit("feat: A feature"),
            Commit("feat!: A breaking feature"),
            Commit("fix: A bug fix"),
            Commit("fix!: A breaking bug fix"),
            Commit(
                "chore: A chore with a breaking footer\n\nBREAKING CHANGE: A breaking change",
            ),
            Commit("feat(first): A feature for the first package"),
            Commit("feat: A feature with a separate breaking change\n\nBREAKING CHANGE: Another breaking change"),
    ]).run("release --verbose");
}
