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

```admonish warning
There used to be an older `[[packages]]` syntax. This is deprecated and will be removed in a future version. Please run `knope --upgrade` to upgrade your configuration automatically.
```

## Syntax

Each package, whether it's defined in the `[package]` section or in the `[packages]` section, can have these keys:

1. `versioned_files` is an optional array of files you'd like to bump the version of. They all must have the same version—as a package only has one version.
2. `changelog` is the (optional) Markdown file you'd like to add release notes to.
3. `scopes` is an optional array of [conventional commit scopes] which should be considered for the package when running the [`PrepareRelease`] step.
4. `extra_changelog_sections` is an optional array of extra sections that can be added to the changelog when running the [`PrepareRelease`] step.

### `versioned_files`

A package, by Knope's definition, has a single version. There can, however, be multiple files which contain this version (e.g., `Cargo.toml` for a Rust crate and `pyproject.toml` for a Python wrapper around it). As such, you can define an array of `versioned_files` for each package as long as they all have the same version and all are supported formats. If no file is included in `versioned_files`, the latest Git tag in the format created by the [`Release`] step will be used. The file must be named exactly the way that `knope` expects, but it can be in nested directories. The supported file types (and names) are:

1. `Cargo.toml` for Rust projects
2. `pyproject.toml` for Python projects using [PEP-621](https://peps.python.org/pep-0621/) or [Poetry](https://python-poetry.org)
3. `package.json` for Node projects
4. `go.mod` for Go projects using [modules](https://go.dev/ref/mod)

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

You may wish to add more sections to a changelog than the [defaults](./step/PrepareRelease.md#changelog-sections), you can do this by configuring custom [conventional commit footers](https://www.conventionalcommits.org/en/v1.0.0/#specification) to add notes to new sections in the changelog. By default, the footer `Changelog-Note` adds to the `Notes` section—the configuration to do that would look like this:

```toml
[package]
versioned_files = []
changelog = "CHANGELOG.md"
extra_changelog_sections = [
  { name = "Notes", footers = ["Changelog-Note"] }
]
```

## Examples

### A Single Package with a Single Versioned File

This is the relevant part of Knope's own `knope.toml`, where we keep release notes in a file called `CHANGELOG.md` at the root of the project and version the project using `Cargo.toml` (as this is a Rust project).

```toml
# knope.toml
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"
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

```admonish info
See [`PrepareRelease`] and [`Release`] for details on what happens when those steps are run for multiple packages.
```

[`bumpversion`]: ./step/BumpVersion.md
[`preparerelease`]: ./step/PrepareRelease.md
[`release`]: ./step/Release.md
[`command`]: ./step/Command.md
[request it as a feature]: https://github.com/knope-dev/knope/issues
[semantic versioning]: https://semver.org
[conventional commit scopes]: https://www.conventionalcommits.org/en/v1.0.0/#commit-message-with-scope
