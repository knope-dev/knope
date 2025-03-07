use std::{thread::sleep, time::Duration};

use crate::helpers::{TestCase, commit, create_branch, merge_branch, switch_branch, tag};

#[test]
fn merge_commits() {
    let test = TestCase::new(file!());
    let temp_dir = test.arrange();
    let temp_path = temp_dir.path();

    commit(temp_path, "Initial commit");
    create_branch(temp_path, "feature");
    commit(temp_path, "feat: A new feature");
    switch_branch(temp_path, "main");
    // Even if the latest tag commit is newer than the merged, the ancestors from the merge should be processed
    sleep(Duration::from_secs(1));
    commit(temp_path, "feat: existing feature");
    tag(temp_path, "v1.2.3"); // The current stable version
    merge_branch(temp_path, "feature");

    test.assert(test.act(temp_dir, "release"));
}
