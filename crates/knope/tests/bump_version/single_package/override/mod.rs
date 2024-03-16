use crate::helpers::TestCase;

#[test]
fn test() {
    TestCase::new(file!()).run("bump --override-version=1.0.0");
}
