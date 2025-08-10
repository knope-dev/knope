/// Specifically designed to catch <https://github.com/knope-dev/knope/issues/505/>
use crate::helpers::{TestCase, commit, create_branch, merge_branch, switch_branch, tag};

#[test]
fn pick_correct_commits_from_branching_history() {
    let test = TestCase::new(file!());
    let temp_dir = test.arrange();
    let temp_path = temp_dir.path();

    commit(temp_path, "Initial commit", "Knope <knope@knope.tech>");
    tag(temp_path, "v1.0.0");
    create_branch(temp_path, "patch");
    commit(temp_path, "fix: A bug", "Knope <knope@knope.tech>");
    switch_branch(temp_path, "main");
    merge_branch(temp_path, "patch");
    tag(temp_path, "v1.0.1");
    create_branch(temp_path, "breaking");
    commit(
        temp_path,
        "feat!: A breaking feature",
        "Knope <knope@knope.tech>",
    );
    switch_branch(temp_path, "main");
    merge_branch(temp_path, "breaking");
    tag(temp_path, "v2.0.0");
    switch_branch(temp_path, "breaking");
    merge_branch(temp_path, "main");
    commit(temp_path, "fix: Another bug", "Knope <knope@knope.tech>");
    switch_branch(temp_path, "main");
    merge_branch(temp_path, "breaking");

    test.assert(test.act(temp_dir, "prepare-release"));
}
