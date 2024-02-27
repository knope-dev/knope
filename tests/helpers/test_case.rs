use std::{
    io::stderr,
    path::{Path, PathBuf},
};

use snapbox::{
    cmd::{cargo_bin, Command, OutputAssert},
    Data,
};
use tempfile::TempDir;

use crate::helpers::{assert, commit, copy_dir_contents, get_tags, init, tag};

pub struct TestCase {
    file_name: &'static str,
    git: &'static [GitCommand],
    env: Option<(&'static str, &'static str)>,
    remote: Option<&'static str>,
    expected_tags: Option<&'static [&'static str]>,
}

impl TestCase {
    /// Create a new `TestCase`. `file_name` should be an invocation of `file!()`.
    pub const fn new(file_name: &'static str) -> Self {
        Self {
            file_name,
            env: None,
            git: &[],
            remote: None,
            expected_tags: None,
        }
    }

    pub const fn git(self, commands: &'static [GitCommand]) -> TestCase {
        TestCase {
            file_name: self.file_name,
            remote: self.remote,
            git: commands,
            env: None,
            expected_tags: self.expected_tags,
        }
    }

    pub fn with_remote(mut self, remote: &'static str) -> TestCase {
        self.remote = Some(remote);
        self
    }

    pub fn expected_tags(mut self, expected_tags: &'static [&'static str]) -> Self {
        self.expected_tags = Some(expected_tags);
        self
    }

    /// Set up a new temporary directory with the contents of the `in` directory (if any).
    /// Initialize a git repository and run the commands in `git`.
    pub fn arrange(&self) -> TempDir {
        let working_dir = tempfile::tempdir().unwrap();
        let path = working_dir.path();

        let in_dir = self.in_dir();
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

        working_dir
    }

    /// Run `command` in `working_dir` with any `self.env` set.
    pub fn act(&self, working_dir: TempDir, command: &str) -> Asserts {
        let data_path = self.data_path();
        let parts = command.split_whitespace().collect::<Vec<_>>();
        let mut real = Command::new(cargo_bin!("knope"))
            .current_dir(working_dir.path())
            .with_assert(assert());
        let mut dry_run = Command::new(cargo_bin!("knope"))
            .current_dir(working_dir.path())
            .with_assert(assert());

        for arg in parts {
            real = real.arg(arg);
            dry_run = dry_run.arg(arg);
        }
        if let Some((key, value)) = self.env {
            real = real.env(key, value);
            dry_run = dry_run.env(key, value);
        }
        dry_run = dry_run.arg("--dry-run");

        let dry_run = if Self::dry_run_stdout(data_path).exists()
            || Self::dry_run_stderr(data_path).exists()
        {
            Some(dry_run.assert())
        } else {
            None
        };

        Asserts {
            dry_run,
            real: real.assert(),
            working_dir,
        }
    }

    pub fn assert(&self, asserts: Asserts) {
        let Asserts {
            real,
            dry_run,
            working_dir,
        } = asserts;
        let data_path = self.data_path();
        let dry_run_stdout_file = Self::dry_run_stdout(data_path);
        let dry_run_stderr_file = Self::dry_run_stderr(data_path);
        if dry_run_stdout_file.exists() {
            dry_run
                .unwrap()
                .success()
                .stdout_matches(Data::read_from(&dry_run_stdout_file, None));
        } else if dry_run_stderr_file.exists() {
            dry_run
                .unwrap()
                .failure()
                .stderr_matches(Data::read_from(&dry_run_stderr_file, None));
        }

        let stderr_file = data_path.join("stderr.log");
        if stderr_file.exists() {
            real.failure()
                .stderr_matches(Data::read_from(&stderr_file, None));
        } else {
            let stdout_file = data_path.join("stdout.log");
            let output = if stdout_file.exists() {
                Data::read_from(&stdout_file, None)
            } else {
                "".into()
            };
            real.success().stdout_matches(output);
        }

        let path = working_dir.path();

        let in_dir = self.in_dir();
        if in_dir.exists() {
            let mut out_dir = data_path.join("out");
            if !out_dir.exists() {
                out_dir = in_dir;
            }
            assert().subset_matches(out_dir, path);
        }

        if let Some(expected_tags) = self.expected_tags {
            let actual_tags = get_tags(path);
            pretty_assertions::assert_eq!(expected_tags, actual_tags);
        }
    }

    fn dry_run_stderr(data_path: &Path) -> PathBuf {
        data_path.join("dryrun_stderr.log")
    }

    fn dry_run_stdout(data_path: &Path) -> PathBuf {
        data_path.join("dryrun_stdout.log")
    }

    /// Runs `.arrange()`, `.act()`, and `.assert()`.
    pub fn run(self, command: &str) {
        self.assert(self.act(self.arrange(), command));
    }

    pub fn env(self, key: &'static str, value: &'static str) -> TestCase {
        TestCase {
            file_name: self.file_name,
            git: self.git,
            remote: self.remote,
            env: Some((key, value)),
            expected_tags: self.expected_tags,
        }
    }

    fn data_path(&self) -> &Path {
        Path::new(self.file_name).parent().unwrap()
    }

    fn in_dir(&self) -> PathBuf {
        self.data_path().join("in")
    }
}

pub struct Asserts {
    real: OutputAssert,
    dry_run: Option<OutputAssert>,
    working_dir: TempDir,
}

#[derive(Clone, Copy, Debug)]
pub enum GitCommand {
    Commit(&'static str),
    Tag(&'static str),
}
