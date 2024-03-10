use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn replace_version() {
    TestCase::new(file!())
        .git(&[Commit("Initial")])
        .run("replace-version");
}
