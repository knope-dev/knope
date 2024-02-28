# Contributing

Knope is open to all kinds of contributions—if you want to contribute code there are a few helpful hints.

## Docs

The [docs website](https://knope.tech) is built using [starlight](https://starlight.astro.build).
The source is contained in the `docs` directory.
The easiest way to find a document to edit is to go to that doc on the website and click on "Edit page"
at the bottom.

### Running locally

`npm --prefix docs install`, then `npm --prefix docs start` (or `just serve-docs`).

### Docs linting

CI will fail if the docs are not formatted correctly or there are broken relative links.
Use `just reformat` to reformat the docs and `just build-docs` to check for broken links.

## `just` and `justfile`

[`just`](https://just.systems/man/en/chapter_1.html) is a command runner (like `make`) which makes it easier to run
common tasks the same way on many platforms. Specifically, you can run the same sorts of commands that CI does to
replicate failures (or prevent them) locally! Start by installing
via [your favorite method](https://just.systems/man/en/chapter_4.html) (like [`cargo binstall just`][cargo-binstall]).
Then, run `just -l` to see all the available commands.

## Formatting

This project uses `rustfmt` to format Rust code, but depends on unstable features (for example, sorting imports).
You need to install the nightly toolchain (for example, with `rustup toolchain install nightly`) before formatting the
code.

[Prettier](https://prettier.io) formats Markdown (via [`npx`](https://docs.npmjs.com/cli/v7/commands/npx))
and [Taplo](https://crates.io/crates/taplo-cli) formats TOML. `just install-all-dependencies` will install
Taplo (via [cargo-binstall], which you must install manually), but won't install NPM.

## Snapshot Tests

Most of the tests for Knope are end-to-end snapshot tests in the `tests` directory, where one directory/module
corresponds to one test.
To create a new test:

1. Copy a test directory
2. Add it as a `mod` to whatever directory is above it.
3. Change the contents of `in` as required for what you're testing.
4. Change the contents of `out` to match what `in` should look like after running the command (for example, increased
   versions)
5. Change `mod.rs` to have the setup & command invocation that you want
6. Run the test with `just` (or `cargo test`)
7. If the test fails, you can run `SNAPSHOTS=overwrite just` to update the snapshots

### How snapshot tests work

Most snapshot tests look like this:

```rust
#[test]
fn name_of_test() {
    TestCase::new(file!())  // Corresponds to a directory you make for this test
        .git(&[              // Run Git commands as needed to set up the test
            Commit("Initial commit"),
            Tag("v0.1.0"),
        ])
        .env("KNOPE_PRERELEASE_LABEL", "alpha")  // Set environment variables as needed
        .run("prepare-release --prerelease-label alpha")  // The command you want to run, omitting the binary name
}
```

This test must be in a "test directory," which has the following:

1. An `in` directory with all the files that the command needs to run.
   `TestCase::run` will create a temporary directory, copy the contents of `in` into it, and run the command from there.
2. An `out` directory, if the command should alter the files in `in`.
3. If the command should succeed (exit with a 0 status code):
   1. A `stdout.log` file if the command produces an output.
   2. A `dryrun_stdout.log` file if `.run()` should _also_ execute the command with `--dry-run` to snapshot the output.
4. If the command should fail (exit with a non-0 status code):
   1. A `stderr.log` file if the command should fail and produce an error message.
   2. A `dryrun_stderr.log` file if `.run()` should _also_ execute the command with `--dry-run` to snapshot the output.

If neither of `stdout.log` or `stderr.log` are present, the command should succeed and produce no output.

`TestCase` leverages [snapbox](https://crates.io/crates/snapbox) under the hood, so you can set `SNAPSHOTS=overwrite` to
update the snapshots if you've made changes to the test.

The setup functions (`new`, `git`, `env`) of `TestCase` are `const`,
so you can define them once and reuse them in multiple cases,
when slightly different setups should produce the same results
(for example, in `tests/prepare_release/override_prerelease_label/mod.rs`).

[cargo-binstall]: https://github.com/cargo-bins/cargo-binstall
