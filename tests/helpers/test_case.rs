use std::{
    io::stderr,
    path::{Path, PathBuf},
};

use snapbox::{
    cmd::{cargo_bin, Command},
    Data,
};
use tempfile::TempDir;

use crate::helpers::{assert, commit, copy_dir_contents, init, tag};

pub struct TestCase<const GIT_LENGTH: usize, const ENV_LENGTH: usize> {
    file_name: &'static str,
    git: [GitCommand; GIT_LENGTH],
    env: [(&'static str, &'static str); ENV_LENGTH],
    remote: Option<&'static str>,
}

impl TestCase<0, 0> {
    /// Create a new `TestCase`. `file_name` should be an invocation of `file!()`.
    pub const fn new(file_name: &'static str) -> Self {
        Self {
            file_name,
            env: [],
            git: [],
            remote: None,
        }
    }

    pub const fn git<const GIT_LENGTH: usize>(
        self,
        commands: [GitCommand; GIT_LENGTH],
    ) -> TestCase<GIT_LENGTH, 0> {
        TestCase::<GIT_LENGTH, 0> {
            file_name: self.file_name,
            remote: self.remote,
            git: commands,
            env: [],
        }
    }
}

impl<const GIT_LENGTH: usize, const ENV_LENGTH: usize> TestCase<GIT_LENGTH, ENV_LENGTH> {
    pub fn with_remote(mut self, remote: &'static str) -> TestCase<GIT_LENGTH, ENV_LENGTH> {
        self.remote = Some(remote);
        self
    }
    pub fn run(self, command: &str) {
        let working_dir = tempfile::tempdir().unwrap();
        let parts = command.split_whitespace().collect::<Vec<_>>();
        let path = working_dir.path();
        let data_path = Path::new(self.file_name).parent().unwrap();

        let in_dir = data_path.join("in");
        if in_dir.exists() {
            copy_dir_contents(&in_dir, path);
        }

        init(path);
        for command in self.git {
            match command {
                GitCommand::Commit(message) => {
                    commit(path, message);
                }
                GitCommand::Tag(name) => {
                    tag(path, name);
                }
            }
        }

        let mut real = Command::new(cargo_bin!("knope"))
            .current_dir(path)
            .with_assert(assert());
        let mut dry_run = Command::new(cargo_bin!("knope"))
            .current_dir(path)
            .with_assert(assert());

        for arg in parts {
            real = real.arg(arg);
            dry_run = dry_run.arg(arg);
        }
        for (key, value) in self.env {
            real = real.env(key, value);
            dry_run = dry_run.env(key, value);
        }
        dry_run = dry_run.arg("--dry-run");

        let dry_run_stdout_file = data_path.join("dryrun_stdout.log");
        let dry_run_stderr_file = data_path.join("dryrun_stderr.log");
        if dry_run_stdout_file.exists() {
            dry_run
                .assert()
                .success()
                .stdout_matches(Data::read_from(&dry_run_stdout_file, None));
        } else if dry_run_stderr_file.exists() {
            dry_run
                .assert()
                .failure()
                .stderr_matches(Data::read_from(&dry_run_stderr_file, None));
        }

        let stderr_file = data_path.join("stderr.log");
        if stderr_file.exists() {
            real.assert()
                .failure()
                .stderr_matches(Data::read_from(&stderr_file, None));
        } else {
            let stdout_file = data_path.join("stdout.log");
            let output = if stdout_file.exists() {
                Data::read_from(&stdout_file, None)
            } else {
                "".into()
            };
            real.assert().success().stdout_matches(output);
        }

        let out_dir = path.join("out");
        if out_dir.exists() {
            assert().subset_matches(out_dir, path);
        }
    }
}

impl<const GIT_LENGTH: usize> TestCase<GIT_LENGTH, 0> {
    pub fn env(self, key: &'static str, value: &'static str) -> TestCase<GIT_LENGTH, 1> {
        TestCase {
            file_name: self.file_name,
            git: self.git,
            remote: self.remote,
            env: [(key, value)],
        }
    }

    pub fn envs<const ENV_LENGTH: usize>(
        self,
        envs: [(&'static str, &'static str); ENV_LENGTH],
    ) -> TestCase<GIT_LENGTH, ENV_LENGTH> {
        TestCase {
            file_name: self.file_name,
            git: self.git,
            remote: self.remote,
            env: envs,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum GitCommand {
    Commit(&'static str),
    Tag(&'static str),
}
