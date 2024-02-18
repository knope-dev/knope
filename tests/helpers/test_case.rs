use std::path::PathBuf;

use snapbox::{
    cmd::{cargo_bin, Command},
    Data,
};
use tempfile::TempDir;

use crate::helpers::{assert, commit, copy_dir_contents, init, tag};

pub struct TestCase {
    working_dir: TempDir,
    data_path: PathBuf,
    env: Vec<(&'static str, &'static str)>,
}

impl TestCase {
    pub fn new(file_name: &str, test_name: &str) -> Self {
        let working_dir = tempfile::tempdir().unwrap();
        let data_path = PathBuf::from("tests").join(file_name).join(test_name);
        copy_dir_contents(&data_path.join("source"), working_dir.path());
        Self {
            working_dir,
            data_path,
            env: Vec::new(),
        }
    }

    pub fn git(self, commands: &[GitCommand]) -> Self {
        let path = self.working_dir.path();
        init(path);
        for command in commands {
            match command {
                GitCommand::Commit(message) => {
                    commit(path, message);
                }
                GitCommand::Tag(name) => {
                    tag(path, name);
                }
            }
        }
        self
    }

    pub fn env(mut self, key: &'static str, value: &'static str) -> Self {
        self.env.push((key, value));
        self
    }

    pub fn run(self, command: &str) {
        let parts = command.split_whitespace().collect::<Vec<_>>();
        let path = self.working_dir.path();

        let mut real = Command::new(cargo_bin!("knope")).current_dir(path);
        let mut dry_run = Command::new(cargo_bin!("knope")).current_dir(path);

        for arg in parts {
            real = real.arg(arg);
            dry_run = dry_run.arg(arg);
        }
        dry_run = dry_run.arg("--dry-run");

        real.assert()
            .success()
            .with_assert(assert())
            .stdout_matches(Data::read_from(&self.data_path.join("output.txt"), None));
        dry_run
            .assert()
            .success()
            .with_assert(assert())
            .stdout_matches(Data::read_from(
                &self.data_path.join("dry_run_output.txt"),
                None,
            ));

        assert().subset_matches(self.data_path.join("expected"), path);
    }
}

pub enum GitCommand {
    Commit(&'static str),
    Tag(&'static str),
}
