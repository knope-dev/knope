# Basic development tasks, run with `just`. Less strict than `just ci`.
default: test reformat lint

# Run things the same way CI does
ci: test lint check-format

# Install all dependencies that are not already installed (requires `cargo-binstall`)
install-all-dependencies: install-lint-dependencies

# Run a local webserver for testing the docs.
serve-docs:
    npm run --prefix docs start

test:
    cargo t

lint:
    cargo clippy -- -D warnings
    cargo-deny check

# Reformat all files, requires `npx` and `install-lint-dependencies`
reformat:
    cargo +nightly fmt
    npx prettier **/*.md docs/* --write
    taplo format

check-format:
    cargo +nightly fmt -- --check
    taplo format --check
    npx prettier **/*.md docs/* --list-different

# Install dependencies for `lint`, `default`, `check-format`, `reformat`, and some of `ci`. Requires `cargo-binstall`
install-lint-dependencies:
    cargo binstall --no-confirm cargo-deny taplo-cli {{binstall_args}}

binstall_args := ""
