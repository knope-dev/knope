# Basic development tasks, run with `just`. Less strict than `just ci`.
default: test reformat lint

# Run things the same way CI does
ci: test lint check-format build-docs

# Install all dependencies that are not already installed (requires `cargo-binstall`)
install-all-dependencies: install-book-dependencies install-lint-dependencies

# Run a local webserver for testing the docs.
serve-book:
    mdbook serve docs --open

# Build the docs, checks for some common issues (like broken links).
build-docs:
    mdbook build docs

test:
    cargo t

lint:
    cargo clippy -- -D warnings
    cargo deny check

# Reformat all files, requires `npx` and `install-lint-dependencies`
reformat:
    cargo +nightly fmt
    npx prettier **/*.md --write --prose-wrap=never
    taplo format

check-format:
    cargo +nightly fmt -- --check
    taplo format --check
    npx prettier **/*.md --list-different --prose-wrap=never

# Install dependencies for `serve-book`, `build-book`, and some of `ci`. Requires `cargo-binstall`
install-book-dependencies:
    cargo binstall --no-confirm mdbook mdbook-linkcheck mdbook-admonish

# Install dependencies for `lint`, `default`, `check-format`, `reformat`, and some of `ci`. Requires `cargo-binstall`
install-lint-dependencies:
    cargo binstall --no-confirm cargo-deny taplo-cli
