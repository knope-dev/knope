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

We use [snapbox](https://crates.io/crates/snapbox) for most of the integration tests in the `tests` dir. This allows us to run commands end-to-end and compare the output to what we expect (making it much clearer when things change).

The general workflow for a snapshot test is:

1. Create a new directory in `tests` (optionally nested in a subdirectory relevant to what you're testing) which contains all the setup files you need (e.g., a `knope.toml` and a `Cargo.toml`).
2. Create a temp directory and copy all the source files over.
3. Use the functions from `tests/git_repo_helpers` to set up a git repo and add any commits/tags you need (for testing releases).
4. Run the command and verify the output using snapbox.

A good example of this is the `prerelease_after_release` test in `tests/prepare_release.rs`.

[cargo-binstall]: https://github.com/cargo-bins/cargo-binstall
