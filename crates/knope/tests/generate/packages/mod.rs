use crate::helpers::TestCase;

#[test]
fn generate_packages() {
    TestCase::new(file!()).run("--generate");
}
