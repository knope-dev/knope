use std::path::{Path, PathBuf};

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

const OUT_DIR: &'static str = "out";

const STDOUT_FILE: &'static str = "stdout.log";

const STDERR_FILE: &'static str = "stderr.log";

const DRY_RUN_STDOUT_FILE: &'static str = "dryrun_stdout.log";

impl TestCase {
    /// Create a new `TestCase`. `file_name` should be an invocation of `file!()`.
    pub fn new(file_name: &'static str) -> Self {
        let working_dir = tempfile::tempdir().unwrap();
        let data_path = Path::new(file_name).parent().unwrap();
        copy_dir_contents(&data_path.join("in"), working_dir.path());
        Self {
            working_dir,
            data_path: data_path.into(),
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
        for (key, value) in self.env {
            real = real.env(key, value);
            dry_run = dry_run.env(key, value);
        }
        dry_run = dry_run.arg("--dry-run");

        let real_assert = real.assert().with_assert(assert());

        if self.data_path.join(STDERR_FILE).exists() {
            real_assert
                .failure()
                .stderr_matches(Data::read_from(&self.data_path.join(STDERR_FILE), None));
        } else {
            let output = if self.data_path.join(STDOUT_FILE).exists() {
                Data::read_from(&self.data_path.join(STDOUT_FILE), None)
            } else {
                "".into()
            };
            real_assert.success().stdout_matches(output);
        }
        if self.data_path.join(DRY_RUN_STDOUT_FILE).exists() {
            dry_run
                .assert()
                .success()
                .with_assert(assert())
                .stdout_matches(Data::read_from(
                    &self.data_path.join(DRY_RUN_STDOUT_FILE),
                    None,
                ));
        }

        if self.data_path.join(OUT_DIR).exists() {
            assert().subset_matches(self.data_path.join(OUT_DIR), path);
        }
    }
}

pub enum GitCommand {
    Commit(&'static str),
    Tag(&'static str),
}
