# `[[packages]]`

The `[[packages]]` array is where you tell `knope` about the packages (programs, libraries, modules, etc.) you want to manage using it. This array is required to direct [`BumpVersion`] and [`PrepareRelease`] to modify the correct files.

```admonish warning
Currently only [one package](https://github.com/knope-dev/knope/issues/153) at a time is supported.
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

### `versioned_files`

A package, by Knope's definition, has a single version. There can, however, be multiple files which contain this version (e.g., `Cargo.toml` for a Rust crate and `pyproject.toml` for a Python wrapper around it). As such, you can define an array of `versioned_files` for each package as long as they all have the same version and all are supported formats. If no file is included in `versioned_files`, the latest Git tag in the format created by the [`Release`] step will be used. The file must be named exactly the way that `knope` expects, but it can be in nested directories. The supported file types (and names) are:

1. `Cargo.toml` for Rust projects
2. `pyproject.toml` for Python projects (using [Poetry's metadata](https://python-poetry.org))
3. `package.json` for Node projects
4. `go.mod` for Go projects using [modules](https://go.dev/ref/mod)

Want to bump the version of a file that isn't natively supported? [Request it as a feature] and, in the meantime, you can write a script to manually bump that file with the version produced by [`BumpVersion`] or [`PrepareRelease`] using a [`Command`] step, like this:

```toml
[[packages]]
versioned_files = []  # With no versioned_files, the version will be determined via Git tag
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "my-command-which-bumps-a-custom-file-with version"
variables = { "version" = "Version" }
```

[`bumpversion`]: ./step/BumpVersion.md
[`preparerelease`]: ./step/PrepareRelease.md
[`release`]: ./step/Release.md
[`command`]: ./step/Command.md
[request it as a feature]: https://github.com/knope-dev/knope/issues
