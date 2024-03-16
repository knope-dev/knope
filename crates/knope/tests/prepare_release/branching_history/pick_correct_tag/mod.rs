use crate::helpers::{commit, create_branch, switch_branch, tag, TestCase};

#[test]
fn pick_correct_tag_from_branching_history() {
    let test = TestCase::new(file!());
    let temp_dir = test.arrange();
    let temp_path = temp_dir.path();

    commit(temp_path, "Initial commit");
    tag(temp_path, "v1.0.0");
    create_branch(temp_path, "v2");
    commit(temp_path, "feat!: Something new");
    tag(temp_path, "v2.0.0");
    switch_branch(temp_path, "main");
    commit(temp_path, "fix: A bug");

    test.assert(test.act(temp_dir, "prepare-release"));
}
