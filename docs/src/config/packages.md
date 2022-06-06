# `[[packages]]`

The `[[packages]]` array is where you tell `knope` about the packages (programs, libraries, modules, etc.) you want to manage using it. This array is required to direct [`BumpVersion`] and [`PrepareRelease`] to modify the correct files.

```admonish warning
Currently only [one package](https://github.com/knope-dev/knope/issues/153) and [one file per package](https://github.com/knope-dev/knope/issues/149) is supported.
```

## Syntax

1. `versioned_files` is an array of files you'd like to bump the version of.
2. `changelog` is the (optional) Markdown file you'd like to add release notes to.

### Example

```toml
# knope.toml
[[packages]]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"
```

This is the relevant part of Knope's own `knope.toml`, where we keep release notes in a file called `CHANGELOG.md` at the root of the project and version the project using `Cargo.toml` (as this is a Rust project).

## Supported Formats for Versioning

These are the types of files that can be added to `versioned_files` (and targetted using steps like [`BumpVersion`] and [`PrepareRelease`]. The file must be named exactly the way that `knope` expects, but it can be in nested directories.

1. `Cargo.toml` for Rust projects
2. `pyproject.toml` for Python projects (using [Poetry's metadata](https://python-poetry.org))
3. `package.json` for Node projects

[`bumpversion`]: ./step/BumpVersion.md
[`preparerelease`]: ./step/PrepareRelease.md
