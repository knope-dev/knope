use crate::helpers::TestCase;

#[test]
fn get_version() {
    TestCase::new(file!()).run("get-version");
}
