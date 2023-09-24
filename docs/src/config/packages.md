# Packages

Packages are how you tell `knope` about collections of files that should be tracked with [semantic versioning]. At least one package must be defined in order to run the [`BumpVersion`] or [`PrepareRelease`] steps.

There are two ways to define packages, if you only have one package, you define it like this:

```toml
[package]
# package config here
```

If you have multiple packages, you define them like this:

```toml
[packages."<name>"]  # where you replace <name> with the name of the package
# package config here

[packages."<other_name>"]  # and so on
# package config here
```

## Syntax

Each package, whether it's defined in the `[package]` section or in the `[packages]` section, can have these keys:

1. `versioned_files` is an optional array of files you'd like to bump the version of. They all must have the same version—as a package only has one version.
2. `changelog` is the (optional) Markdown file you'd like to add release notes to.
3. `scopes` is an optional array of [conventional commit scopes] which should be considered for the package when running the [`PrepareRelease`] step.
4. `extra_changelog_sections` is an optional array of extra sections that can be added to the changelog when running the [`PrepareRelease`] step.
5. `assets` is a list of files that should be included in the release along with the name that should appear with them. These are only used for GitHub releases by the [`Release`] step.

### `versioned_files`

A package, by Knope's definition, has a single version. There can, however, be multiple files which contain this version (e.g., `Cargo.toml` for a Rust crate and `pyproject.toml` for a Python wrapper around it). As such, you can define an array of `versioned_files` for each package as long as they all have the same version and all are supported formats. If no file is included in `versioned_files`, the latest Git tag in the format created by the [`Release`] step will be used. The file must be named exactly the way that `knope` expects, but it can be in nested directories. The supported file types (and names) are:

1. `Cargo.toml` for Rust projects
2. `pyproject.toml` for Python projects using [PEP-621](https://peps.python.org/pep-0621/) or [Poetry](https://python-poetry.org)
3. `package.json` for Node projects
4. `go.mod` for Go projects using [modules](https://go.dev/ref/mod)

#### A special note on `go.mod`

Go modules don't normally have their entire version in their `go.mod` file, only the major component and only if that component is greater than 1. However, this makes it difficult to track versions, specifically between [`PrepareRelease`] and [`Release`] if they are run in separate workflows. To bypass this, Knope will add a comment in the module line after the module path containing the full version—like `module github.com/knope-dev/knope // v0.0.1`. If a version exists in that format, it will be used. If not, the version will be determined by the latest Git tag.

Updating the version of a `go.mod` file with Knope will completely rewrite the module line, adding in the expected comment syntax. If you have another comment here, you'll want to move it before running Knope. If you have a suggestion for how to improve versioning for Go, please [open an issue][request it as a feature].

#### Other file formats

Want to bump the version of a file that isn't natively supported? [Request it as a feature] and, in the meantime, you can write a script to manually bump that file with the version produced by [`BumpVersion`] or [`PrepareRelease`] using a [`Command`] step, like this:

```toml
[package]
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

```admonish warning
The `Version` variable in the [`Command`] step cannot be used when multiple packages are defined. This is a temporary limitation—if you have a specific use case for this, please [file an issue][request it as a feature].
```

### `extra_changelog_sections`

You may wish to add more sections to a changelog than the [defaults](./step/PrepareRelease.md#changelog-sections), you can do this by configuring custom [conventional commit footers](https://www.conventionalcommits.org/en/v1.0.0/#specification) and/or [changeset types](https://github.com/knope-dev/changesets#change-type) to add notes to new sections in the changelog.

By default, the commit footer `Changelog-Note` adds to the `Notes` section—the configuration to do that would look like this:

```toml
[package]
versioned_files = []
changelog = "CHANGELOG.md"
extra_changelog_sections = [
  { name = "Notes", footers = ["Changelog-Note"] }
]
```

To leverage that same section for changeset types, we could add the `types` key:

```toml
[package]
versioned_files = []
changelog = "CHANGELOG.md"
extra_changelog_sections = [
  { name = "Notes", footers = ["Changelog-Note"], types = ["note"] }
]
```

### `assets`

Assets is a list of files to upload to a GitHub release. They do nothing without [GitHub configuration](./github.md). Assets are per-package. Each asset can optionally have a `name`, this is what it will appear as in GitHub releases. If `name` is omitted, the final component of the path will be used.

```toml
[package]
versioned_files = ["Cargo.toml"]

[[package.assets]]
path = "artifact/my-binary-linux-amd64.tgz"
name = "linux-amd64.tgz"

[[package.assets]]
path = "artifact/my-binary-darwin-amd64.tgz"  # name will be "my-binary-darwin-amd64.tgz"
```

## Examples

### A Single Package with a Single Versioned File and multiple Assets

This is the relevant part of Knope's own `knope.toml`, where we keep release notes in a file called `CHANGELOG.md` at the root of the project and version the project using `Cargo.toml` (as this is a Rust project).

```toml
# knope.toml
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[package.assets]]
path = "artifact/my-binary-linux-amd64.tgz"
name = "linux-amd64.tgz"

[[package.assets]]
path = "artifact/my-binary-darwin-amd64.tgz"
name = "darwin-amd64.tgz"
```

### A Single Package with Multiple Versioned Files

If your one package must define its version in multiple files, you can do so like this:

```toml
# knope.toml
[package]
versioned_files = ["Cargo.toml", "pyproject.toml"]
changelog = "CHANGES.md"  # You can use any filename here, but it is always Markdown
```

### Multiple Packages

If you have multiple, separate packages which should be versioned and released separately—you define them as separate, named packages. For example, if `knope` was divided into two crates—it might be configured like this:

```toml
# knope.toml
[packages.knope]
versioned_files = ["knope/Cargo.toml"]
changelog = "knope/CHANGELOG.md"

[packages.knope-utils]
versioned_files = ["knope-utils/Cargo.toml"]
changelog = "knope-utils/CHANGELOG.md"
```

By default, the package names (e.g., `knope` and `knope-utils`) will be used as package names for changesets. No additional config is needed to independently version packages via changesets. If you want to target conventional commits at a specific package, you need to add the [`scopes` key](./step/PrepareRelease.md#mono-repos-and-multiple-packages).

```admonish warning
When you have one `[package]`, the package name "default" will be used for changesets. If you switch to a multi-package setup, you will need to update all changeset files (in the .changeset directory) to use the new package names.
```

```admonish info
See [`PrepareRelease`] and [`Release`] for details on what happens when those steps are run for multiple packages.
```

### Multiple Major Versions of Go Modules

The [recommended best practice](https://go.dev/blog/v2-go-modules) for maintaining multiple major versions of Go modules is to include every major version on your main branch (rather than separate branches). In order to support multiple go modules files in Knope, you have to define them as separate packages:

```toml
# knope.toml
[packages.v1]
versioned_files = ["go.mod"]
scopes = ["v1"]

[packages.v2]
versioned_files = ["v2/go.mod"]
scopes = ["v2"]
```

This allows you to add features or patches to just the major version that a commit affects and release new versions of each major version independently.

```admonish warning
If you use this multi-package syntax for go modules, you **cannot** use Knope to increment the major version. You'll have to create the new major version directory yourself and add a new package to `knope.toml` for it.
```

[`bumpversion`]: ./step/BumpVersion.md
[`preparerelease`]: ./step/PrepareRelease.md
[`release`]: ./step/Release.md
[`command`]: ./step/Command.md
[request it as a feature]: https://github.com/knope-dev/knope/issues
[semantic versioning]: https://semver.org
[conventional commit scopes]: https://www.conventionalcommits.org/en/v1.0.0/#commit-message-with-scope
