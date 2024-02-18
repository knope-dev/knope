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

[`just`](https://just.systems/man/en/chapter_1.html) is a command runner (like `make`) which makes it easier to run common tasks the same way in multiple platforms. Specifically, you can run the same sorts of commands that CI does to replicate failures (or prevent them) locally! Start by installing via [your favorite method](https://just.systems/man/en/chapter_4.html) (personally, I use [`cargo binstall just`][cargo-binstall]). Then, run `just -l` to see all the available commands.

## Formatting

We use `rustfmt` to format Rust code, but we depend on unstable features (e.g., sorting imports). You need to install the nightly toolchain (e.g., `rustup toolchain install nightly`) before formatting the code.

We also use [Prettier](https://prettier.io) to format Markdown (via [`npx`](https://docs.npmjs.com/cli/v7/commands/npx)) and [Taplo](https://crates.io/crates/taplo-cli) for formatting TOML. `just install-all-dependencies` will install Taplo (via [cargo-binstall], which must be installed separately), but will not install NPM.

## Snapshot Tests

Most of the tests for Knope are end-to-end snapshot tests in the `tests` directory. Generally speaking, a test looks like this:

```rust
#[test]
fn name_of_test() {
    TestCase::new("this_file_name/name_of_test")  // Corresponds to a directory you make for this test
        .git(&[  // Run Git commands as needed to set up the test
            Commit("Initial commit"),
            Tag("v0.1.0"),
        ])
        .env("KNOPE_PRERELEASE_LABEL", "alpha")  // Set environment variables as needed
        .run("prepare-release --prerelease-label alpha")  // The command you want to run, omitting the binary name
}
```

Each `.rs` file in `tests` should have a directory named the same.
Each test within the file should have a directory named the same as the test.
That test directory must contain:

1. A `source` directory with all the files that the command needs to run. These will be copied to a temporary directory.
2. An `expected` directory with all the files that the command should produce or modify. These will be compared to the temporary directory after the command runs.
3. A `stdout.txt` file, this will match the output of the command.
4. A `dry_run_stdout.txt` file, this will match the output of the command when run with `--dry-run`.

`TestCase` leverages [snapbox](https://crates.io/crates/snapbox) under the hood, so you can set `SNAPSHOTS=overwrite` to update the snapshots if you've made changes to the test.

[cargo-binstall]: https://github.com/cargo-bins/cargo-binstall
