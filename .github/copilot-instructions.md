# Copilot Instructions for Knope

## Project Overview

Knope is a command line tool that automates tedious development tasks, particularly around versioning, changelog generation, and release management. It works with conventional commits, changesets, and integrates with GitHub/Gitea for automated releases.

## Architecture

This is a Rust workspace with three main crates:

- `crates/knope` - The main CLI application
- `crates/knope-config` - Configuration parsing and validation
- `crates/knope-versioning` - Semantic versioning logic

## Development Workflow

### Prerequisites

- Rust (stable and nightly toolchains)
- [mise](https://mise.jdx.dev) - Tool version manager and task runner
- Node.js 24+ (managed by mise)

### Setup

```bash
# Install mise
curl https://mise.run | sh

# Install all tools and dependencies
mise install

# Install documentation dependencies
mise run install-docs-dependencies
```

### Common Tasks

Use `mise tasks` to see all available tasks. Most common ones:

- `mise run dev` - Run tests, reformat, and lint (default task)
- `mise run test` - Run all tests
- `mise run ci` - Run checks the same way CI does
- `mise run reformat` - Reformat all code (Rust, TOML, docs)
- `mise run lint` - Run all linters

## Coding Standards

### Rust Code

- **Formatting**: Use `cargo +nightly fmt` (requires nightly for unstable features)
  - Import grouping: `StdExternalCrate`
  - Import granularity: `Crate`
- **Linting**: Strict clippy configuration with `deny` on all warnings
- **Safety**: `unsafe_code` is forbidden
- **Error Handling**: No `panic!`, `expect`, `unwrap`, `todo!`, `unimplemented!` in production code (allowed in tests via `clippy.toml`)
- **Indexing**: No direct indexing/slicing to avoid panics
- **Logging**: Use `tracing`, never `print!` or `eprintln!`
- **Warnings**: All warnings are denied

### TOML Files

- Format with `taplo format`

### Markdown/Documentation

- Format with Prettier
- Check with Vale for style/grammar
- Docs are built with Astro/Starlight in the `docs` directory

## Testing

### Snapshot Tests

Most tests are end-to-end snapshot tests located in `crates/knope/tests/`. Each test has:

1. An `in` directory with input files
2. An `out` directory with expected output files (if files should change)
3. `stdout.log` or `stderr.log` for expected output
4. Optional `dryrun_stdout.log` or `dryrun_stderr.log` for `--dry-run` output

### Test Structure

```rust
#[test]
fn test_name() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("v0.1.0"),
        ])
        .env("KNOPE_PRERELEASE_LABEL", "alpha")
        .run("prepare-release --prerelease-label alpha")
}
```

### Updating Snapshots

```bash
# Update all snapshots
SNAPSHOTS=overwrite mise run test

# Update specific test
mise run snapshot test_name
```

## CI/CD

- Tests run on Ubuntu and Windows
- CI includes: tests, linting, formatting checks, docs building
- Vale checks documentation style
- `cargo-deny` checks dependencies, licenses, and security advisories

## Dependencies

- New dependencies must pass `cargo-deny` checks
- Allowed licenses: MIT, Apache-2.0, ISC, MPL-2.0, BSD-3-Clause, Unicode-3.0, Zlib, CDLA-Permissive-2.0
- Security advisories are checked against rustsec database

## Documentation

- Source: `docs/` directory (Astro/Starlight)
- Local dev: `mise run serve-docs`
- Build: `mise run build-docs` (checks for broken links)
- Format: `mise run reformat-docs`
- Edit links are at the bottom of each doc page

## Configuration

The project uses several configuration files:

- `mise.toml` - Task runner and tool versions
- `Cargo.toml` - Workspace configuration
- `knope.toml` - Knope's own release configuration
- `clippy.toml` - Clippy overrides for tests
- `rustfmt.toml` - Rustfmt configuration
- `taplo.toml` - TOML formatter configuration
- `deny.toml` - Dependency and license checks
- `.vale.ini` - Documentation linting

## When Making Changes

1. Always run `mise run dev` before committing
2. For Rust code: ensure `cargo +nightly fmt` passes
3. For new features: add snapshot tests following existing patterns
4. Update documentation if adding/changing user-facing features
5. Verify your changes work with `mise run ci`
6. Never commit code that doesn't pass all checks
