use crate::helpers::TestCase;

/// Run `--generate` on a repo with no remote.
#[test]
fn generate_no_remote() {
    TestCase::new(file!()).run("--generate");
}
