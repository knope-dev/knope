---
title: Packages
---

A package is a set of files that Knope releases together with the same version.
Knope can increase this version based on changes that affect the package.

A project can either consist of a single package:

```toml title="knope.toml"
[package]
# package config here
```

Or multiple packages:

```toml title="knope.toml"
[packages."<name>"]  # where you replace <name> with the name of the package
# package config here

[packages."<other_name>"]  # and so on
# package config here
```

## `versioned_files`

The files within a package that contain the current version.
This is an array of strings, each of which is a file path relative to the `knope.toml` file.
Each file must have the same version number as all the other files.

Knope determines the type of the file using its name (independent of its path),
so `blah/Cargo.toml` is a `Cargo.toml` file.

Knope supports the following file names:

### `Cargo.toml`

For versioning Rust projects. Must contain a `[package.version]` field, like so:

```toml title="Cargo.toml"
[package]
version = "1.0.0"
```

### `pyproject.toml`

For Python projects using [PEP-621](https://peps.python.org/pep-0621/) or [Poetry](https://python-poetry.org).
Must contain either a `[project.version]` or `[tool.poetry.version]` value, respectively.
If it has both values, they must be the same.

```toml title="pyproject.toml"
[project]  # PEP-621
version = "1.0.0"

[tool.poetry]  # Poetry
version = "1.0.0"
```

### `package.json`

For JavaScript or TypeScript projects, must contain a root-level `version` field:

```json title="package.json"
{
  "version": "1.0.0"
}
```

### `go.mod`

For Go projects using [modules](https://go.dev/ref/mod).
Must contain a module line
which must end in the major version for any greater than 1. Can optionally contain a comment
containing the _full_ version.
If this comment isn't present, Knope uses the latest matching Git tag to find the version.

```text title="go.mod"
module github.com/knope-dev/knope // v0.0.1
```

```text title="go.mod"
module github.com/knope-dev/knope/v2 // v2.0.0
```

## `changelog`

The relative path to a Markdown file you'd like to add release notes to.

```toml title="knope.toml"
[package]
changelog = "CHANGELOG.md"
```

## `scopes`

An array of conventional commit scopes that Knope should consider for the package.
If not defined, Knope will consider _all_ scopes.
Commits with no scope are always considered.

```toml title="knope.toml"
[packages.knope]
scopes = ["knope", "all"]

[packages.changesets]
scopes = ["changesets", "all"]
```

## `extra_changelog_sections`

An array of objects defining more sections for the changelog (or overrides for the default sections).
Each object can optionally have an array of `footers` or an array of `types`.

:::tip
Check out the [custom changelogs recipe](/recipes/customizing-changelogs) for a full example of how to use this feature.
:::

```toml
[package]
extra_changelog_sections = [
    { name = "Security", footers = ["Security-Note"], types = ["security"]}
]
```

## `assets`

Assets is a list of files to upload to a GitHub release. They do nothing without [GitHub configuration](../github).
Assets are per-package. Each asset can optionally have a `name`, this is what it will appear as in GitHub releases.
The `name` defaults to the final part of the path.

```toml
[package]

[[package.assets]]
path = "artifact/my-binary-linux-amd64.tgz"
name = "linux-amd64.tgz"

[[package.assets]]
path = "artifact/my-binary-darwin-amd64.tgz"  # name will be "my-binary-darwin-amd64.tgz"
```
