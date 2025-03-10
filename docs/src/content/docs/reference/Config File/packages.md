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
Each file must have the same version number as all the other files.

An entry in this array can either be a string, containing the path to a file, or an object containing a `path` and
specifying a `dependency` within the file to update:

```toml
[package]
versioned_files = [
    "Cargo.toml",
    "package.json",
    { path = "crates/knope/Cargo.toml", dependency = "knope-versioning" }
]
```

All paths should be relative to the config file.

Knope determines the type of the file using its name (independent of its path),
so `blah/Cargo.toml` is a `Cargo.toml` file.

Knope supports the following file names:

### `Cargo.toml`

For versioning Rust projects. Must contain `version` and `name` fields in the `package` table, like so:

```toml title="Cargo.toml"
[package]
name = "my-package"
version = "1.0.0"
```

If you specify `dependency`, Knope will search for it in the `workspace.dependencies`,
`dependencies`, and `dev-dependencies` tables:

```toml title="knope.toml"
[package]
versioned_files = [
    { path = "crates/knope/Cargo.toml", dependency = "knope-versioning" }
]
```

```toml title="Cargo.toml" {6}
[package]
name = "something-else"
version = "1.0.0"

[dependencies]
knope-versioning = "1.0.0"
```

### `Cargo.lock`

Knope can keep dependencies of a Rust project up to date by specifying a `Cargo.lock` file. By default,
the dependency name is the package name in the first `Cargo.toml` file listed in `versioned_files`.
You can override this by specifying the `dependency` field manually.
If you provide neither, Knope will error.

### `gleam.toml`

A [Gleam configuration file](https://gleam.run/writing-gleam/gleam-toml/).

`dependency` isn't supported yet.

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

To omit the major version from the module line (e.g., for binaries, where it doesn't matter much),
use the [`ignore_go_major_versioning`](#ignore_go_major_versioning) option.

`dependency` isn't yet supported.

### `package.json`

For JavaScript or TypeScript projects, must contain a root-level `version` field:

```json title="package.json"
{
  "version": "1.0.0"
}
```

`dependency` isn't yet supported.

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

`dependency` isn't yet supported.

### `pom.xml`

For Java projects using [Maven](https://maven.apache.org), must contain a `<version>` field in the `<project>` section:

```xml
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.mycompany.app</groupId>
  <artifactId>my-app</artifactId>
  <version>1.2.3</version>
</project>
```

Neither `dependencies` nor multi-module projects are support yet.

### `pubspec.yaml`

For Dart projects, must contain a `version` field:

```yaml title="pubspec.yaml"
version: 1.0.0
```

`dependency` isn't yet supported.

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

Assets can either be a single "glob" string, or a list of files to upload to a GitHub release.
They do nothing without [GitHub configuration](/reference/config-file/github).
Assets are per-package.
When specifying an exact list, each asset can optionally have a `name`, this is what it'll appear as in GitHub releases.
The `name` defaults to the file name (the final component of the path).

:::caution
Knope doesn't yet support uploading assets to Gitea, declaring both `[gitea]` and assets is an error.
:::

### A single glob string

```toml title="knope.toml"
[package]
assets = "artifact/*"  # Upload all files in the artifact directory
```

### A list of files

```toml
[package]

[[package.assets]]
path = "artifact/my-binary-linux-amd64.tgz"
name = "linux-amd64.tgz"

[[package.assets]]
path = "artifact/my-binary-darwin-amd64.tgz"  # name will be "my-binary-darwin-amd64.tgz"
```

## `ignore_go_major_versioning`

Go has special rules about major versions above 1. Specifically, the module line in `go.mod` must end in the major version.
By default, Knope follows these rules,
so if there is no major version at the end of the module line,
Knope will assume you're updating the latest 1.x or 0.x tag.

To ignore this rule, and always use the latest tag (even if it doesn't match the module line), set `ignore_go_major_versioning` to `true` in the package config:

```toml title="knope.toml"
[package]
versioned_files = ["go.mod"]
ignore_go_major_versioning = true
```

:::tip

To maintain multiple major versions of a Go module, check out [this recipe](/recipes/multiple-major-go-versions)

:::
